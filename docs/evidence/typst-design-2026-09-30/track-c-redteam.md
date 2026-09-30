# Typst as a second engine: Track C red-team (licensing, risks, alternatives)

- **Lane:** TYPST-DESIGN-C (read-only review; this file is the only change).
- **Reviewer:** claude-opus-5-5 (kabir-claude TYPST-DESIGN-C, mac-m5pro-kabir), 2026-09-29/30.
- **Under review:** proposed DESIGN.md §15 "Typst support" (branch
  `agent/kabir-claude/design-typst`, `1f6fda3cb`), against DESIGN.md §1, §3, §4.4–4.5,
  §6, §9, §14 on main `67a2ea078` and `docs/protocol/display-list-v3.md` (v3.1).
- **Labels:** **VERIFIED** = read in a primary source or measured here, with the source
  or method given. **REPORTED** = stated by a third party I could not check further.
  **BELIEF** = my inference; treat it as a hypothesis.
- **Measurement machine:** Apple M5 Pro, 15 cores, 24 GB, macOS 26.6.2, Typst CLI
  0.15.1 (Homebrew), **under heavy load from other agents (load average 24–32)**. The
  absolute numbers are therefore pessimistic. Ratios and orders of magnitude are the
  useful part. Scripts are described in §7 so the numbers can be reproduced.

## 0. Verdict in one screen

The plan's shape is right: a separate MIT/Apache host process, upstream crates rather
than a re-implementation, and Typst ranked below LaTeX. But §15 as written has three
problems that block implementation, plus licensing hygiene that has to be decided first.

| # | Finding | Severity |
|---|---|---|
| B1 | `display-list-v3` cannot express Typst output. **Every** page of a plain Typst document uses ICC-based colour and Type0/CIDFontType0 (CFF) fonts. v3 marks the first INCOMPLETE and reserves the second as "refused" (VERIFIED §2.1). §15's claim that v3 "stays free of TeX-only assumptions" is false today: the spec carries `tex_name`, TFM sizes, Type 1 encodings, pdfTeX `/Fm<n>` ids and sp-only coordinates. | **Blocking.** A protocol extension has to come first, and it is a change to a component the LaTeX path shares. |
| B2 | §15's gates can't be met with unmodified upstream crates. The ≤ 16 ms p95 edit target measured at **28–79 ms p95** (one-page raster included) at 301–1,000 pages. The ≤ 100 ms reopen target measured at **0.7–2.3 s** cold, and comemo caches can't be persisted. "Positions equal the exported PDF" needs krilla's f32 rounding, which sp can't represent below 256 pt (VERIFIED §2.2–2.3). | **Blocking.** Set Typst-specific targets. |
| B3 | One bundled Typst version breaks existing projects. Measured on 0.15.1: **12 of 94** Universe templates from the ≤ 0.12 era fail to compile (plus one broken template), and **4 of 80** from the 0.13–0.14 era fail. The Typst web app pins the compiler per project and has an upgrade assistant (VERIFIED §2.4). | Blocking for **release**, not for a prototype. |
| L | Licensing is sound in principle; the separate process is the right boundary. But §15 gives the wrong reason ("Apache-2.0 is incompatible with GPL-2.0"). There are also four concrete hazards: a GPL-3 font compiled into the "MIT/Apache" binary, a trademark-authorization requirement for commercial use, Universe packages under their own licences (7% copyleft), and the shared MIT crate as a route to an accidental combination (§1). | Needs decisions before code; not fatal. |

## 1. Licensing

### 1.1 Is the process boundary sufficient? Yes, and the reasoning in §15 needs correcting

- **VERIFIED.** The GPL FAQ, #MereAggregation
  (<https://www.gnu.org/licenses/gpl-faq.en.html#MereAggregation>): "pipes, sockets and
  command-line arguments are communication mechanisms normally used between two
  separate programs … But if the semantics of the communication are intimate enough,
  exchanging complex internal data structures, that too could be a basis to consider
  the two parts as combined". The Typst host never talks to the GPL engine at all. Both
  talk to the MIT app through a public, versioned, engine-independent wire format. That
  is weaker coupling than the LaTeX case DESIGN §3/D11 already relies on. **Verdict:
  the Typst host is a separate program and carries no GPL obligation.**
- **VERIFIED, and a correction to §15.** The FSF licence list
  (<https://www.gnu.org/licenses/license-list.en.html#apache2>) says Apache-2.0 is
  "compatible with version 3 of the GNU GPL … not compatible with GPL version 2". The ASF
  agrees (<https://www.apache.org/licenses/GPL-compatibility.html>: "Apache 2 software
  can therefore be included in GPLv3 projects"). The engine is GPL-2.0-or-later, and it
  links xpdf 4.06, which is GPL v2 or v3 only (DESIGN §3), so its binary is
  distributable under GPLv3. Linking Typst into the engine would therefore not be
  impossible. It would force the engine binary to **GPLv3-only** and erase the
  GPL-2-or-later posture. Keep the separate process, but give the true reasons:
  - it keeps the engine's GPLv2 option;
  - it keeps the Typst side free of any GPL obligation;
  - it isolates crashes and memory (§2.5);
  - it isolates the build and dependencies (§4).
- **The real accidental-combination risk is the shared crate** `crates/display-list-v3`
  (`flashtex-display-list`, MIT). The GPL engine links it, and so would the Typst host.
  Two ways it could go wrong:
  1. **Apache code flows into it.** Examples: copying Typst frame/font code into the
     shared decoder, or adding an Apache-2.0-only dependency. The engine binary would
     then contain Apache-2.0 code: fine under GPLv3, incompatible under GPLv2 (FSF
     above), so the engine silently becomes GPLv3-only.
  2. **GPL code flows into it.** Example: moving the PDF content-stream interpreter
     `crates/flashtex-engine/src/displaylist/interp.rs` (1,315 lines, GPL by location)
     into the shared crate so the Typst host can reuse it. Both the crate and the Typst
     host would then become GPL.

  `scripts/check-license-boundary.sh` checks only that nothing depends on
  `flashtex-engine` (check A) and that the iPad links no GPL (check B). **It does not
  check code provenance or the licences of the shared crate's dependencies (VERIFIED,
  read on main).** Guard-rails are in §4.
- **The MIT app talking to an Apache host** raises no issue: both licences are
  permissive, and Apache-2.0 §4 applies to whatever we distribute (below). **BELIEF:**
  linking `typst-ide` into the app would also be legally fine, but it would put 400+
  crates and Typst's release cadence into the app build. Keep it in the host.

### 1.2 Exact obligations when the DMG ships `flashtex-typst-host`

Apache-2.0 §4 (<https://www.apache.org/licenses/LICENSE-2.0>, VERIFIED text):

- **(a)** Give recipients a copy of the Apache-2.0 licence.
- **(b)** "Cause any modified files to carry prominent notices stating that You changed
  the files". This is triggered by any `[patch]` or fork of a Typst crate, even a
  one-line fix. §15 says "unmodified", so make that a rule: no patching without a
  recorded notice.
- **(c)** Retain the copyright, patent, trademark and attribution notices in any
  **source** we distribute.
- **(d)** Include a readable copy of the attribution notices in each upstream `NOTICE`
  file "within a NOTICE text file distributed as part of the Derivative Works … or,
  within a display generated by the Derivative Works" (for example, About → Acknowledgements).

The upstream NOTICE files the host inherits (VERIFIED by reading each one):

- **`typst/typst` NOTICE** (398 lines, v0.15.1). It covers:
  - MIT: clrs.cc colours, rustup, rustc, rust-analyzer, lipsum;
  - **LPPL-1.3: translations adapted from Babel and cleveref**;
  - BSD-3: Skia blending in typst-render;
  - Apache-2.0 with LLVM exceptions: `powi`.

  <https://github.com/typst/typst/blob/v0.15.1/NOTICE>
- **`typst/typst-assets` NOTICE** (1,552 lines, v0.15.1), for the fonts compiled into the
  binary by the `embedded-fonts` feature and for other data:

  | Asset | Licence |
  |---|---|
  | Libertinus Serif | **OFL-1.1** (Reserved Font Names "Linux Libertine", "Biolinum", "STIX Fonts") |
  | New Computer Modern except NewCM10-Regular | **GUST Font License 1.0** |
  | **NewCM10-Regular.otf** | **GPL-3.0-or-later + Font Exception + Distribution Exception** |
  | DejaVu Sans Mono | Bitstream Vera licence and public domain |
  | Foxit `.pfb` fonts | BSD-3 (PDFium) |
  | ICC profiles | CC0 |
  | HTML/MathML data | WHATWG CC-BY-4.0/BSD-3 and W3C document licence |

  <https://github.com/typst/typst-assets/blob/v0.15.1/NOTICE>
- **`typst/hayagriva` NOTICE**: "The Creative Commons BY-SA 3.0 … applies to … The CSL
  styles and locales found in `archive/`". Those are compiled in. The obligation is to
  attribute and include the licence URI. Share-alike binds only if we modify the styles.
  <https://github.com/typst/hayagriva/blob/main/NOTICE>
- The rest of the dependency tree: **425 packages** in Typst 0.15.1's `Cargo.lock`
  (VERIFIED count). Each needs its licence text reproduced in binary distribution
  (MIT/BSD/ISC/Zlib/Unicode). Generate the list with `cargo about` (or similar) at DMG
  build time and fail the build on any unknown licence.

**The NewCM10-Regular hazard (VERIFIED text of the exception).** The Distribution
Exception says: "If you distribute the fonts as they are in your program, and your program
follows a license compatible with GPL version 3 … these fonts do not by themselves cause
the resulting program to be covered by the GNU General Public License … If however you
distribute a copy of the fonts that modifies either the glyphs … or the glyph-set … this
exception is invalidated". So an Apache/MIT host may carry the unmodified font, but:

1. we must ship the GPL-3 text, and meet GPL-3 §6 for the font as "object code".
   **BELIEF:** pointing to the corresponding source on CTAN/GitLab satisfies this, but
   legal review should confirm;
2. it conflicts with DESIGN §1's own rule, "nothing GPL on the MIT side";
3. it rules out ever running this binary on the iPad or the App Store.

**Recommendation:** build the host **without** `typst-kit/embedded-fonts`. Ship the four
default families as separate files in the app bundle's `Resources/` (aggregation), or
load New Computer Modern from the user's TeX Live, which carries it. Either way it is
kept out of the executable.

### 1.3 Typst Universe packages

- **VERIFIED:** Universe accepts "an OSI-approved license or a version of CC-BY, CC-BY-SA,
  or CC0" (<https://github.com/typst/packages/blob/main/docs/licensing.md>). It also
  allows non-open assets such as university logos when "the copyright holder has a
  policy that clears distribution".
- **VERIFIED (tally of `https://packages.typst.org/preview/index.json` on 2026-09-29).**
  There are 1,635 packages (4,831 versions). The licences of the newest versions:

  | Licence | Packages |
  |---|---|
  | MIT | 1,156 |
  | MIT-0 | 140 |
  | Apache-2.0 | 61 |
  | Unlicense | 49 |
  | GPL/LGPL/AGPL families | **119 (7.3%)**, including **20 AGPL** |
  | LPPL-1.3c | 3 |

- **Obligation:** none while the **user** fetches packages, as `typst` itself does. We
  would be *distributing* them if we:
  - pre-seed a cache in the DMG;
  - bundle "starter templates";
  - mirror Universe;
  - sync caches to another machine.

  Then each package's licence applies. GPL/AGPL packages would need corresponding
  source, which for WASM plugins means the plugin's Rust/C source, not the `.wasm`.
  **Rule:** never bundle Universe packages wholesale. Bundle only named, reviewed
  templates, preferably MIT-0 or 0BSD.
- **Terms of use (REPORTED/BELIEF).** The typst.app Terms (§9,
  <https://typst.app/terms>) govern "Customers" of Typst's Services: accounts on the web
  app. They forbid "access or search of data in an interface other than the explicitly
  public interface (e.g. scraping)". Package download through the CDN the open-source
  CLI uses is the public interface, and I found no separate CDN terms. Still:
  - use `typst-kit`'s downloader;
  - set a FlashTeX User-Agent;
  - cache locally;
  - avoid bulk prefetching.

  `typst-kit` itself warns that "the index format of the official package registry is
  not specified" (VERIFIED, `crates/typst-kit/src/packages.rs`).

### 1.4 Trademark and naming

- **VERIFIED** (<https://typst.app/legal/brand/>): "You are free to use the Typst name
  and logo to refer to our web app and the Typst compiler. **Commercial uses of name and
  logo always require our authorization.**" Also: "The name of the project must not imply
  that it is the Typst web app / IDE or compiler … NOT OKAY: typst-ide, typst-app, typstc,
  typst". A modified logo must not be taken to be endorsed by them, and the unmodified
  "t" can't be our project's logo.
- **Consequences:**
  - "FlashTeX Typst" as a product or edition name is risky: it reads as a Typst IDE.
  - "FlashTeX compiles Typst documents" / "Typst support" is nominative use.
  - If FlashTeX is sold or monetised in any way, their policy requires **written
    authorization** before marketing uses the name. Ask Typst GmbH early.
  - **BELIEF:** they are likely to say yes, and the conversation also covers CDN
    etiquette (§1.3).
  - Never use the Typst "t" as the `.typ` document icon.
  - The binary name `flashtex-typst-host` is internal and prefixed by our name. Fine.
- **Typst GmbH's commercial web app.** Apache-2.0 has no field-of-use restriction, and
  §3 grants a patent licence, so there is no legal conflict. There is a relationship
  risk: a polished native competitor to their paid product, using their registry. That
  is another reason to contact them rather than surprise them. Whether "Typst" is a
  registered mark was not verified (web search was unavailable to this lane).

## 2. Technical risks (measured where possible)

### 2.1 The display list cannot carry Typst pages (B1)

- **VERIFIED.** Compiling a plain text-and-maths document (`p10.typ`, §7) and a feature
  sample with Typst 0.15.1, then scanning the PDFs:
  - the text document has `/ICCBased` ×1, `/Type0` ×3, `/CIDFontType0` ×6,
    `/FontFile3` ×3 (CFF) and `/Identity-H` ×3;
  - the feature sample adds `/ShadingType` (gradient), `/PatternType` (tiling) and `/ca`
    (fill opacity).
- v3.1 has:
  - fonts: `format` `type1` or `none`, with "`truetype`, `opentype`, `type3` … reserved
    (the engine refuses those fonts today)"; glyphs are addressed by 8-bit `code` →
    256-name `encoding`;
  - colour: only DeviceGray/RGB/CMYK;
  - INCOMPLETE for `gs` (transparency), `sh` (shading), "a pattern or separation colour
    space".

  So every Typst page would arrive INCOMPLETE. Its fallback, "renders the PDF page
  instead (`DONE.pdf`)", costs a whole-document export (§2.3).
- **Required design change:** a **v3.2 (additive) or v4 spec** before any Typst host
  code. It needs:
  - OpenType/CFF/TrueType font resources addressed by glyph id, with CID→GID mapping;
  - ICC-based colour;
  - axial and radial shadings (plus Typst's conic gradients, which krilla samples);
  - tilings as forms;
  - constant alpha (`CA`/`ca`) and soft masks;
  - stroke dash, cap and join;
  - Type 3 glyphs (colour and bitmap emoji);
  - span ids that map to Typst `Span`s (file plus byte range, not line/column).

  Most of this also benefits LaTeX (TikZ opacity and shadings, xcolor transparency), so
  it isn't Typst-only work. But it changes the shared decoder and the Swift renderer: the
  LaTeX preview path.

### 2.2 Position parity with `typst-pdf`

- **VERIFIED.** `typst-pdf` 0.15 writes through krilla 0.8.2
  (`typst/Cargo.toml` v0.15.1), and krilla's geometry is `f32`
  (`crates/krilla/src/geom.rs`: `pub struct Point { pub x: f32, pub y: f32 }`). Typst's
  own layout is `f64`.
- **Consequence (VERIFIED arithmetic).** An f32 in [256, 512) pt has an ulp of 2⁻¹⁵ pt
  (2 sp) and is exact in sp. Below 256 pt the ulp is finer than 1 sp, so sp can't hold
  the PDF's number exactly.
- The LaTeX host gets exactness by interpreting the PDF content stream "in exact decimal
  arithmetic". For Typst the equivalent is one of two things:
  1. replicate krilla's f64→f32 rounding and its TJ/advance arithmetic when building items
     from Frames, and carry f32 (or f64) coordinates in the extended protocol; or
  2. interpret krilla's content streams, which costs a PDF export per edit and is
     rejected in §2.3.

  **BELIEF:** option 1 works and passes a zero-pixel gate. It needs a positions harness
  like `docs/evidence/display-list-v3-2026-09-29/positions-fixtures.json` from day one.

### 2.3 Performance and the §1.2 targets (B2)

**VERIFIED** with `typst watch` on generated documents: headings, an outline,
"page X of Y" footers, numbered equations, cross-references and tables (§7).

**Timing sources:**
- **Typst-reported:** Typst's own reported compile time. It covers layout plus the named
  export, and excludes the file-watch debounce.
- **Cold:** a `typst compile` to PDF, median of 3 runs.

**Edit types:**
- **Small edit:** one character in a paragraph about 40% of the way through the document.
- **Reflow edit:** `#lorem(90)` inserted in section 2, which shifts every later page.

**Cold compiles:**

| Document | Cold compile to PDF | Max RSS |
|---|---|---|
| 11 pages | 34 ms | 50 MB |
| 101 pages | 231 ms | 261 MB |
| 301 pages | 677 ms | 715 MB |
| 1,000 pages | 2,314 ms | 2,336 MB |

**Edits in a long-lived watch process (Typst-reported ms):**

| Document | Edit | Export | Median | p95 | Watch-process RSS (first → max) |
|---|---|---|---|---|---|
| 301 pages | small, 40× | 1 page, PNG at 144 ppi | 28.7 | 79.4 | 1.14 → 1.55 GB |
| 301 pages | reflow, 30× | 1 page, PNG | 26.3 | 28.4 | 1.17 GB |
| 301 pages | small, 40× | whole PDF | 76.6 | 89.5 | 1.14 → 1.56 GB |
| 1,000 pages | small, 40× | 1 page, PNG | 40.8 | 59.0 | 3.7 → **5.1 GB** |
| 1,000 pages | reflow, 30× | 1 page, PNG | 42.8 | 51.3 | 3.8 GB |
| 1,000 pages | small, 40× | whole PDF | 762 | 1,400 | 3.8 → **5.2 GB** |
| 1,000 pages | reflow, 30× | whole PDF | 268 | 300 | 3.4 GB |

(The 1,000-page whole-PDF runs swing between 268 and 762 ms under the ambient load and
memory pressure. Treat them as "hundreds of ms".)

**What this means:**

- **Typst's incremental layout is genuinely good:** about 26–43 ms median with one-page
  raster up to 1,000 pages, and reflow edits are no worse than small ones. But:
  - **It misses ≤ 16 ms p95.** Separating layout from the one-page raster was not
    possible with the CLI: `--timings` records only the first compile. A one-page PNG
    costs about 23 ms more than a one-page SVG (§2.7), so **BELIEF:** layout alone is
    about 10–25 ms. That needs an in-process harness to verify.
  - **`typst::compile` has no page streaming, no viewport-first and no cancellation**
    (VERIFIED, `crates/typst/src/lib.rs`: it returns the whole document after up to
    `MAX_ITERS` introspection passes, with no cancel hook). The host can't show the
    edited page before layout of the whole document finishes, and it can't abandon a
    superseded compile. v3.1's "a COMPILE while one is running supersedes it" must be
    implemented as "finish, then discard" or "kill the worker".
  - **Reopen ≤ 100 ms is unreachable.** Cold is 0.7 s at 301 pages and 2.3 s at 1,000.
    comemo's caches are in-memory only, so there is no S₀ equivalent. Show the last
    exported PDF (or a cached display list) instantly and mark it stale until the first
    compile lands. Say so in §15.
  - **The preview must never go through `typst-pdf`.** A whole-PDF export on every edit
    is 77–90 ms at 301 pages and hundreds of ms to 1.4 s at 1,000 pages. Export only on
    demand.
- **Memory is the sharpest risk.**
  - **VERIFIED:** `typst watch` evicts with `comemo::evict(10)` after every compile
    (`crates/typst-cli/src/watch.rs:82`). Even so, a 1,000-page document holds 3–5 GB
    RSS.
  - A 24 GB Mac running one LaTeX host per open document (checkpoint budget 1 GiB by
    default) plus one or two large Typst documents is at risk.
  - **Required:** a per-host memory budget, tuned eviction, and a host restart when the
    budget is exceeded (cold start is the price). One Typst host per document, as for
    LaTeX, so one document can't evict another.

### 2.4 Version churn and pinning (B3)

- **VERIFIED:** releases roughly every 4–8 months (tags):
  - 0.12: 2024-10-18
  - 0.13: 2025-02-19
  - 0.14: 2025-10-24
  - 0.15: 2026-06-15

  Every release carries breaking changes. Counted in `docs/content/changelog/*.typ`:
  7, 11, 17, 9 and 23 "breaking" markers for 0.11–0.15. Examples:
  - 0.13 removed `style`, `state.display` and `locate` compatibility;
  - 0.15 removed `path`, `pattern` and `pdf.embed`, and banned backslashes in paths.
- **VERIFIED:** the web app pins. From the 0.14 release post
  (<https://typst.app/blog/2025/typst-0.14>): "projects would always use the latest
  compiler version unless explicitly pinned … We are now phasing out the 'Latest' option
  … The upgrade assistant … compiles your document with both versions, makes a verdict,
  and lists new errors and warnings."
- **VERIFIED (measured):** `typst init` plus `typst compile` on 0.15.1 for every Universe
  **template** package whose newest version declares a minimum compiler ≤ 0.12:
  - **13 of 95 fail**;
  - one of those (`sunny-famnit`) is a template bug (missing `.bib`), so **12 of 94
    (13%) are version breaks**. Examples:
    - "cannot add string and type";
    - "unknown variable: style";
    - "module `pdf` does not contain `embed`";
    - "only element functions can be used as selectors".
  - For the 0.13–0.14 era: **4 of 80 (5%) fail**.

  Plain `#import` of 145 sampled packages all passed. Import evaluates only the top
  level, so it is not evidence of compatibility.
- **Required design change:**
  - pin the Typst version per project (a FlashTeX project file; `.typ` sources carry no
    pin);
  - ship **at least the current and previous minor** as separate host binaries (the
    Homebrew CLI is **44.7 MB** each, arm64, with fonts embedded);
  - offer an upgrade assistant modelled on the web app's: compile both, diff the
    diagnostics.

  §15's "upstream crates unmodified" should also say "one host binary per supported
  Typst minor, selected by the project pin".
- The Rust API churns too: `World`, `IdeWorld` and frame item types change between
  minors. **BELIEF:** each bump costs the host several engineer-days plus re-running the
  positions harness. Budget that as recurring maintenance.

### 2.5 Determinism, fonts and reproducibility

- **VERIFIED:**
  - two compiles with a fixed `--creation-timestamp` give byte-identical PDFs;
  - `World::today` is pinned by `creation_timestamp` (`typst-cli/src/world.rs`), so
    §4.5's per-session date pinning maps directly.
- **VERIFIED: system fonts make output machine-dependent.** `#set text(font:
  "Helvetica")` compiles to a different PDF with and without system fonts. The fallback
  produces only a **warning** ("unknown font family: helvetica"), not an error. This Mac
  exposes 380 font families.
- **Required:**
  - preview and export use the same host and font set, so they agree on one machine;
  - record a per-project font manifest (family → file SHA-256) and flag missing or
    changed fonts prominently. A warning buried in diagnostics won't do: the user sends
    the project to a co-author and it silently reflows.

### 2.6 Packages: network, privacy and supply chain

- **VERIFIED:**
  - `typst-kit` downloads `https://packages.typst.org/preview/{name}-{version}.tar.gz`
    and unpacks it **with no hash check** (`crates/typst-kit/src/packages.rs`);
  - the index has **no hash field** (its keys are authors, compiler, description,
    entrypoint, exclude, keywords, license, name, repository, updatedAt, version);
  - Typst has **no lockfile**. A maintainer's comment on typst#7981
    (<https://github.com/typst/typst/issues/7981>) says resolution is deterministic
    because versions are exact, "as long as you trust Typst Universe".
  - Universe stopped accepting in-place edits to published versions, so a checksum can
    be relied on (<https://github.com/typst/packages/issues/2671>). nixpkgs has
    historically had to override stale hashes (REPORTED in the same thread).
- **VERIFIED: typst#5454, "Symlinks can be used to escape the project sandbox", is
  OPEN.** Our `World` owns file access, so it must canonicalise paths and refuse to
  escape `root`, exactly like §4.5's "reads confined to the project".
- **Required:**
  - write a FlashTeX-side lock (`package@version` → SHA-256 of the tarball) on first
    fetch and verify it on every later fetch;
  - an offline mode, and a first-use prompt before contacting packages.typst.org (the
    IP address and package names go to Typst GmbH; disclose this in the privacy text);
  - a "vendor packages into the project" action. Typst's own discussion points at
    vendoring (typst PR #5729, REPORTED).

### 2.7 WASM plugins and runaway documents

- **VERIFIED** (`crates/typst-library/src/foundations/plugin.rs`):
  - plugins run in `wasmi` with `wasmi::Config::default()`, which has **no fuel metering
    and no store limiter**;
  - the only imports are two `typst_env` functions (no WASI, no filesystem, no network),
    so the sandbox is good;
  - but **a plugin loop can't be interrupted**, and each instance can grow wasm32 memory
    up to 4 GiB, with one instance per thread.
- **VERIFIED** (`typst-eval/src/flow.rs`, `typst-library/src/engine.rs`): `while` loops
  stop at 10,000 iterations ("loop seems to be infinite"), and call depth is capped at
  80. `for` over a huge range is not bounded.
- **Required:**
  - run compiles in a worker the host can kill (or a sub-process);
  - apply wall-time and RSS limits per compile;
  - restart transparently.

  This is the §4.5 "resource limits" rule applied to Typst. It matters because users open
  downloaded projects.

### 2.8 Preview renderer choice

- **VERIFIED** (11-page document, median of 5, CLI):

  | Export | Time |
  |---|---|
  | PDF, all pages | 21.2 ms |
  | SVG, all pages | 21.9 ms |
  | PNG at 144 ppi, all pages | 55.7 ms |
  | PNG at 288 ppi, all pages | 117.9 ms |
  | PNG at 144 ppi, one page | 45.4 ms |
  | SVG, one page | 22.3 ms |

  So the tiny-skia raster adds about 23 ms for one page at 2×. DESIGN B.3 gives Core
  Graphics/Core Text at 0.75 ms for a 2× full page, on a heavier page.
- typst-render also rasterises glyph outlines with its own code, not Core Text, so it
  can't meet §6.2's zero-pixel check against Core Graphics' rendering of the exported PDF.

## 3. Product and UX risks

- **Two languages, one app.** "The app selects the engine by file type" is not enough.
  A project needs an explicit root file and engine (a folder may contain both `.tex` and
  `.typ`, for example a Typst paper with a LaTeX-built figure). Mixed projects need a
  defined answer: each root compiles with its own engine; no cross-inclusion; a PDF from
  one engine can be included in the other (`\includegraphics`, `image("x.pdf")`, which
  Typst 0.14+ supports).
  - Shared `.bib` files will format differently: hayagriva with CSL versus
    biber/BibTeX with `.bst`/biblatex. Users will read that as a FlashTeX bug. Say so in
    the UI.
- **Feature asymmetry.**
  - LaTeX gets the resident incremental engine, viewport-first and ≤ 100 ms reopen.
    Typst gets none of those (§2.3), and the gap will be visible on large documents.
  - Typst's source mapping (span → byte range, `typst_ide::jump_from_click`/
    `jump_from_cursor`) is *finer* than our LaTeX spans.
  - **BELIEF:** the Typst side will feel second-class exactly where users compare it
    with tinymist/VS Code: completion quality, rename, references, formatting.
- **Tooling maturity versus "just as nice as LaTeX".**
  - **VERIFIED:** `typst-ide` 0.15.1 exports only `autocomplete`, `tooltip`,
    `definition`, `jump_from_click`/`jump_from_cursor`, `analyze_*` and `named_items`
    (`crates/typst-ide/src/lib.rs`).
  - **VERIFIED:** tinymist (Apache-2.0, 3.5k stars, v0.15.8 on 2026-09-08) adds:
    - semantic tokens, code actions, formatting (typstyle), document highlight and links;
    - folding, references, inlay hints, colour provider, code lens, rename and signature
      help;
    - linting, test/coverage/profiling;
    - an incremental preview with cross-jump
      (<https://github.com/Myriad-Dreamin/tinymist>).

    "As nice as" the best Typst editing today means tinymist parity, not `typst-ide`.
- **Competitors:**
  - **Typst web app:** collaboration, version pinning and an upgrade assistant (§2.4).
  - **tinymist:** in VS Code, Zed, Neovim and others (via LSP).
  - **The CLI:** `typst watch`.

  Where FlashTeX can **genuinely** be better:
  - a native Mac app with a 120 Hz Core Graphics preview, pixel-identical to the
    exported PDF (tinymist previews SVG in a webview);
  - offline-first, with integrity-locked packages (§2.6), which the web app and CLI
    don't offer;
  - LaTeX and Typst in one tool;
  - **BELIEF:** a Typst-only iPad path. Apache-2.0 has no App Store conflict, provided
    the GPL-3 font is not compiled in (§1.2). Typst is the only engine that could ever
    run *on* the iPad.
- **Maintenance for a small team.** It's a recurring cost: version bumps every 4–8
  months across N shipped versions, a protocol extension, the positions harness, and
  packaging and legal notices. **BELIEF:** about 0.5 engineer ongoing after the initial
  build. That is small next to the pdfTeX port, but not zero, and it competes for the
  same Commander attention.

## 4. Process risk to the LaTeX roadmap, and guard-rails

These are the concrete ways Typst could impede LaTeX, each with a guard-rail.

1. **The Cargo workspace.** VERIFIED: root `Cargo.toml` has `members = ["crates/*",
   "tools/web2rust"]`. A Typst host placed at `crates/flashtex-typst-host` would:
   - join the root workspace;
   - put Typst's ~425-crate tree into the **shared `Cargo.lock`**;
   - be built by every `cargo test --workspace` and `gate.sh full`;
   - inherit the pinned `release` profile that perf-bench baselines depend on;
   - let Typst's MSRV (1.92 at 0.15.1, bumped by each release) pull the toolchain
     forward for the engine.

   The engine today depends only on `flashtex-display-list`, `sha2` and `cc`.
   **Guard-rail:** put the host in its own top-level directory (e.g. `typst-host/`) with
   **its own workspace and `Cargo.lock`**, excluded from the root. It depends on
   `flashtex-display-list` by path only.
2. **CI.**
   - Typst jobs run only on changes under `typst-host/**` (path filters). They are never
     part of the required LaTeX PR check or the LaTeX merge queue.
   - They count against §9.5's "at most 3 branches per machine in CI".
   - They are the first jobs cancelled or deferred when runners are busy.
   - Their build outputs are covered by `clean-worktrees.sh`.
   - Measure Typst build time on first landing and put it in the lane report (not
     measured here).
3. **The shared protocol and decoder** (`crates/display-list-v3`,
   `docs/protocol/display-list-v3.md`, and the Swift renderer):
   - extensions are additive minor versions behind `HELLO.capabilities`;
   - the LaTeX host is never required to emit them;
   - every change runs the LaTeX parity fixtures and the positions check (93,509 glyphs
     exact);
   - CODEOWNERS on those paths names the LaTeX display-list owner;
   - spec changes are batched and reviewed once per design review (§14), not trickled in.

   IDE requests (completion, tooltip, jump) go in a **separate `ide-v1` message family**,
   so `COMPILE` semantics and the LaTeX host are untouched.
4. **The licence CI check** (extends §9.6):
   - `flashtex-display-list` and every dependency of `flashtex-engine` must be MIT/BSD/
     ISC/Zlib-compatible or dual-licensed with such an option (a `cargo-deny` allowlist
     per crate);
   - no Apache-2.0-only crate may reach the engine;
   - a provenance check that no file under `crates/flashtex-engine` is copied into the
     shared crate or the Typst host (header plus similarity check);
   - the Typst workspace may not depend on `flashtex-engine`;
   - the DMG build fails if the third-party notices are incomplete.
5. **The app.**
   - No Typst UI work until the P3 "app integration behind a flag" for the LaTeX v3
     client has landed. Typst then *reuses* that client rather than growing a second one
     in parallel during P3/P5 churn.
   - Typst UI stays behind a feature flag until its own gates pass.
   - Typst lanes don't edit `V2PreparedPage`/`GlyphRunRenderer`, except through a
     reviewed protocol extension (point 3).
6. **People and attention.**
   - At most one Typst lane at a time, staffed only when LaTeX lanes are fully staffed.
   - Typst reports are batched into the Commander's normal checkpoint; there are no
     Typst-driven design reviews outside §14.
   - Typst never pre-empts a LaTeX landing in the merge queue (already in §15, and it
     should be enforced by queue priority, not goodwill).
7. **Sequencing.** Begin with the protocol-extension design and a throwaway spike: an
   in-process harness that measures layout-only incremental latency and memory at
   100/300/1,000 pages, and checks that one-page display lists built from frames are
   krilla-exact. Only then build the host. Both are cheap and settle B1 and B2 before
   any app or protocol code lands.

## 5. Alternatives

| Choice | Verdict | Why |
|---|---|---|
| **typst-ide in our host** vs **tinymist as an LSP server** | **typst-ide in-host first**. Add `typstyle` (formatting) and evaluate `tinymist-query` (Apache-2.0, on crates.io at 0.15.8) as an in-host library later. | tinymist is a second process with its own `World` and its own compiles. That doubles a footprint already measured at 1–5 GB for large documents (§2.3). The app has no LSP client, and tinymist's Typst version would drift from the project's pinned host version. typst-ide shares the host's `World`, comemo cache and frames (needed for `jump_from_click`). The price: fewer features than tinymist until we add them (§3). |
| **typst-render (tiny-skia) bitmaps** vs **our Core Graphics renderer from the display list** | **Our renderer.** Use typst-render only as a debugging aid. | It is about 23 ms per page at 144 ppi versus 0.75 ms (B.3), it can't meet the §6.2 zero-pixel check against Core Graphics' rendering of the PDF, it is CPU-only and has no tiling. The one advantage is that it works without the protocol extension, so it's acceptable only as a prototype stopgap. |
| **Export PDF on every edit and show it (PDFKit)** vs **frames → display list** | **Frames → display list** (with krilla-exact rounding, §2.2). PDF is for export, and as a "render the PDF page" fallback on small documents only. | The whole-PDF export is 77–90 ms at 301 pages and hundreds of ms to 1.4 s at 1,000 pages (§2.3), plus B.3's +25 ms per update and +31 ms cold for PDFKit. It would also give up per-page caching. |
| **One Typst version** vs **several** | **Several:** current and previous minor at least, pinned per project, with an upgrade assistant. | §2.4: 5–13% of older templates break on 0.15.1, and the web app pins. |
| **Embedded fonts in the binary** vs **fonts as bundle files** | **Bundle files.** | Keeps the GPL-3 NewCM10-Regular out of the executable (§1.2) and keeps the iPad option open. |

## 6. Required changes to proposed §15 (summary)

1. Replace "Apache-2.0 is incompatible with GPL-2.0, so …" with the accurate reasoning
   (§1.1). Add the shared-crate licence and provenance guard-rails (§4, point 4).
2. Add: build without embedded fonts, ship fonts as files; generate NOTICE and
   third-party licences in the DMG; no patching without Apache §4(b) notices; never
   bundle Universe packages wholesale; a FlashTeX-side package lock (SHA-256); an
   offline mode and a network prompt; the trademark rule (nominative use only, and
   authorization from Typst GmbH before commercial marketing).
3. Replace "emitting display-list-v3" with "emitting a reviewed additive extension of
   display-list-v3 (fonts by glyph id, ICC colour, shadings, alpha, tilings, Type 3)",
   and delete the claim that v3 is already engine-neutral.
4. Replace "Edit latency meets §1.2" with Typst targets set from a layout-only harness
   (§4, point 7). State that reopen is a stale-then-refresh preview, not ≤ 100 ms.
5. Add version pinning per project, several host versions and an upgrade assistant.
6. Add resource limits: a killable compile worker, a wall-time limit and an RSS budget
   with host restart (§2.3, §2.7). Add a font manifest per project.
7. Add the workspace, CI, protocol-ownership, app-sequencing and staffing guard-rails
   (§4, points 1–6).

## 7. Method (reproducible)

- **Documents:** generated Typst files. Each unit is:
  - a heading with a label;
  - `lorem(180)` with `@eq` and `@sec` references;
  - a numbered display equation;
  - `lorem(140)` plus a unique `EDITMARK<i>`;
  - a 3×2 table with maths;
  - `lorem(60)`.

  The preamble has `outline()` and a footer with `counter(page).display("1 of 1", both:
  true)`. 12, 120, 360 and 1,200 units give 11, 101, 301 and 1,000 pages (page counts
  read from `/Type /Pages /Count`).
- **Cold runs:**
  `typst compile --ignore-system-fonts --creation-timestamp 0 in.typ out.pdf`, 3 runs,
  median. RSS is `ru_maxrss` of the children.
- **Incremental runs:**
  - start `typst watch --ignore-system-fonts --creation-timestamp 0 in.typ out` (with
    `--pages P` and a `-{p}.png` output for the one-page variant);
  - wait for the first status line;
  - then N times: sleep 0.25 s, rewrite the file with the edit toggled, and parse
    "compiled successfully in X" from stderr;
  - RSS is `ps -o rss=` of the watch process after each edit.
- **Template compatibility:**
  - for every package in `index.json` (newest version per name) in the chosen compiler
    buckets, run `typst init @preview/name:version dir`;
  - compile the `[template].entrypoint` with a private `--package-cache-path`;
  - non-templates (init fails) are skipped. Random seed 7.
- **PDF features:** regex counts over the uncompressed dictionary keys of the output
  PDFs.
- **Sources:** every file and licence was read at the tag named (typst v0.15.1,
  typst-assets v0.15.1) through the GitHub API.
- **Not done:** web search was unavailable, so trademark registration status and any
  Universe CDN terms beyond typst.app/terms are unverified; the Typst build time in our
  CI was not measured; there is no in-process layout-only benchmark.
