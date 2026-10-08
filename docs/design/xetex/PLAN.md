# Unicode mode from XeTeX: plan and phase S0 results

Owner request (2026-10-04): full custom font support -- any system font,
OpenType, and OpenType math. **Owner decision (2026-10-04):** "recreate/rewrite
whatever we need from XeTeX; FlashTeX should not need to ship a separate TeX".
FlashTeX therefore ships **its own engine programs only**: two FlashTeX binaries over one shared runtime (§3.3). XeTeX's capabilities
(Unicode input, native OpenType and system fonts, OpenType math) become
FlashTeX features, selected per document as an engine mode, matching
`xelatex`. There is no bundled `xetex` binary, no second TeX distribution, no
xdvipdfmx port and no TeX Live binary in the product path.

The typesetting core comes from `xetex.web`, ported the way the pdfTeX engine
was (`docs/design/engine-v2/DESIGN.md` D1, D3, §4.1): `tools/web2rust`
translates the unmodified `xetex.web` with WEB change files (phase S0, done).
XeTeX's C/C++ layer is not linked. What it does is rewritten in FlashTeX's
runtime (§3.1): fonts and shaping, and output through the display list and
FlashTeX's own PDF writer (§3.2). `.sty` files and fonts come from the user's
TeX Live when present and from the no-TeX-Live bundle (GitHub Release assets)
otherwise (D12, §3.4). The preview carries native glyphs as
`FONT.format: "opentype"` (DESIGN.md §15.4, E1). DESIGN.md governs; the owner
decision is its §13 row of 2026-10-04.

Lane: XETEX-S0 (mac-claude-a), #1489. Crate: `crates/flashtex-xetex`. This
revision (S1–S3 for the owner decision): branch
`agent/mac-claude-a/xetex-plan-v2`. The mode switch and its UX across the
Classic (pdfTeX), Unicode (this plan) and Modern (Typst) modes belong to the
engine-modes proposal (#1520); this plan covers only what the Unicode mode needs from
the engine and how it shares one runtime and one engine interface with the pdfTeX engine
(§3.3).

## 1. What the spike found (2026-10-04)

- `xetex.web` is 34,428 lines (TeX Live 2026, XeTeX 0.999998, e-TeX 2.6).
- `web2rust` tangled it and stopped at §744 (a local `const` section).
- 84% of the pdfTeX engine's change-file hunks match `xetex.web` as written.
- The C/C++ layer is about 10.7k lines, over HarfBuzz 12.3.2, FreeType
  2.14.1, ICU, TECkit and Graphite2.
- Risks: glyph metrics through FreeType; the shaping version lock (output
  depends on the exact HarfBuzz/FreeType); C pointers kept in `mem` (native
  word nodes' glyph arrays, `font_layout_engine`); XDV output.

## 2. Architecture and licensing

- `crates/flashtex-xetex` (GPL-2.0-or-later, its own `LICENSE` with XeTeX's
  MIT notice): `src/generated/` (74,253 lines, never edited) is web2rust's
  translation of `third_party/xetex/xetex.web` (unmodified, pinned at
  `tags/texlive-2026.1`, `third_party/xetex/README.md`) with the change files
  of `changes/` (3,136 lines, `changes/README.md`).
- It **shares the pdfTeX engine's runtime through its public interface**,
  changing nothing in `crates/flashtex-engine`: the word space (`arena`), the
  checked array index (`ix`), the command-line parser (`cli`), the run's
  configuration and kpathsea (`system::configure`, `find_input`,
  `find_file`, `runsystem`), dates and MD5. XeTeX's own system layer is
  `src/system.rs` (Pascal's file model, XeTeX's Unicode input files and
  `input_line`, the routines tex.ch and xetex.ch supply) and
  `src/xetex_ext.rs` (XeTeX's C parts; MIT code, ported).
- It is **excluded from the root workspace** (`exclude` in `Cargo.toml`), with
  its own `Cargo.lock`, so its build never slows the workspace's builds and
  gates; `scripts/gate.sh` treats it as a standalone crate (clippy and tests
  only when its files change). Its translation is still checked on every run
  of web2rust's tests (`tests/drift.rs`, second test). S3 brings it into the workspace, behind the same engine interface as
  the pdfTeX engine, in a binary of its own (§3.3).
- DESIGN.md §3: change files only on `xetex.web` (its e-TeX notice forbids
  modified copies); XeTeX's MIT code may be ported; nothing MIT links the
  crate. `scripts/license-boundary-allow.txt` lists `flashtex-xetex` with the
  reason (a GPL executable and workspace of its own); the boundary check is
  clean. HarfBuzz (MIT), FreeType (FTL or GPL-2), ICU (Unicode licence),
  TECkit (LGPL-2.1 or CPL) and Graphite2 (LGPL-2.1 or MPL) are all usable on
  the GPL side of §3; which of them can also be shared on the MIT side is
  §3.1's question. No Apache-2.0-only code is used (§3.2).

## 3. Phases and acceptance gates

| Phase | Scope | Acceptance gate |
|---|---|---|
| **S0** (done, #1489) | web2rust translates `xetex.web` into `crates/flashtex-xetex`; TFM fonts only; native fonts, TECkit, ICU, Graphite and pictures stubbed as "not found"; XDV for TFM text | P-T1 (box dumps + `\tracingall` log) and exit status equal to TeX Live 2026's `xetex -no-pdf` on the plain-TeX lockstep cases that need no pdfTeX primitive plus XeTeX-specific ones (target about 200); XeTeX's own tests (`xetex-*.test`) pass; XDV byte-identical after the two normalisations; pdftex.web's translation byte-identical (drift test). **Met: §4.** |
| **S1** Native fonts in FlashTeX's runtime | The engine side of §3.1: the handle and state work of §4.7; `find_native_font`, `XeTeXFontInst`/`XeTeXLayoutInterface`'s metrics and shaping, native word nodes and their glyph-info arrays, `\XeTeXglyph*`, `\XeTeXfeature*`, `\XeTeXinterchartoks` classes, `define_native_font` and the glyph records in XDV; font lookup by file name (bundle, TeX Live, project) and by name through the platform-free index (§3.1); TECkit mappings (`mapping=tex-text`), input encodings and normalisation, `\XeTeXlinebreaklocale` | P-T1 + XDV equal to `xetex -no-pdf` (font path normalised, §4.7) on a native-font corpus: plain-TeX cases with OpenType fonts by file name (Latin Modern and TeX Gyre OTF from TeX Live), then by name, then `xelatex` documents with `fontspec` against `xelatex -no-pdf`; a test that fails when a pinned shaping or metrics library changes version; the pdfTeX engine's lockstep unchanged |
| **S2** OpenType math, pictures, and output | OpenType math (`XeTeXOTMath.cpp`: the `MATH` table, variants, assemblies, kerns, `\Umath...` with OpenType fonts); pictures (`XeTeX_pic.c`: PNG, JPEG, BMP, PDF bounds, `\XeTeXpicfile`, `\XeTeXpdffile`); Graphite2 (no AAT, §3.1); the **output path of §3.2**: `ship_out` to `display-list-v3` (native glyph runs as `FONT.format: "opentype"`), and FlashTeX's PDF writer from the display list, with the `\special`s that the `xetex`/`xdvipdfmx` drivers of real packages emit | P-T1 + XDV equal on `unicode-math` documents (amsmath + unicode-math with Latin Modern Math, STIX Two, Libertinus Math) and on `\XeTeXpicfile`/`graphicx` documents; **PDF parity against `xelatex`'s PDF, visual and structural (§3.5), not byte-level**, on the same corpus plus hyperref, xcolor and TikZ documents |
| **S3** Shared runtime, Unicode binary, product integration | §3.3: a shared runtime crate and one FlashTeX engine trait over both translated engines, **two binaries (`flashtex-host` for Classic, `flashtex-host-unicode` for Unicode), the mode chosen per document, a switch starting the other binary**; the incremental system (§5 of DESIGN.md) for Unicode-mode documents (checkpoints include the handle tables, §4.7); the `xelatex` format built by FlashTeX from `latex.ltx` (D12, §3.4); the no-TeX-Live bundle's Unicode-mode files; the preview and Export/Print through the host | The S1 and S2 corpora at P-T1/XDV in the host, incremental (edit replay equal to a cold run); PDF parity (§3.5) on the same corpora in the host; a no-TeX-Live gate (bundle only, no TeX Live on the machine) for Unicode mode; DESIGN.md §1.2's latency targets for Unicode-mode documents; each binary's size, start-up and memory measured (§3.3); `flashtex-host` unchanged after the runtime split: P-T1 lockstep, P-T2, the web2rust drift test, and its size and start-up |

### 3.1 Fonts and shaping in FlashTeX's runtime (S1)

- **What is rewritten, from XeTeX's MIT source:** `XeTeX_ext.c` (native word
  nodes, glyph-info arrays, `make_font_def`, `make_xdv_glyph_array_data`,
  feature and variation parsing of the font name), `XeTeXFontInst.cpp` and
  `XeTeXLayoutInterface.cpp` (metrics, shaping calls, glyph bounds),
  `XeTeXFontMgr.cpp`'s name matching (full name, PostScript name, family plus
  style, `/B` `/I` `/BI`, the `:` and `/` options), and `XeTeXOTMath.cpp`
  in S2. The functions the change files already declare (`changes/ext.ch`) stay
  the interface; S0's integer handles become handles into tables that the
  runtime owns and that checkpoints save with the word space (§4.7).
- **Reuse before building (DESIGN.md §1):** shaping uses **HarfBuzz** and
  metrics **FreeType**, vendored at TeX Live 2026's exact versions (HarfBuzz
  12.3.2, FreeType 2.14.1, sha256-pinned like `third_party/zlib`), because
  XeTeX's output depends on them and no other implementation meets the parity
  bar (rustybuzz follows a different HarfBuzz version: a belief to confirm
  before S1 relies on it). TECkit's mapping engine is vendored or ported for
  `.tec` files (`tex-text.tec` comes from the bundle or TeX Live); ICU for
  encodings, normalisation and line breaking is pinned the same way, at the
  version TeX Live 2026 builds with.
- **Vendored (S1):** [`third_party/harfbuzz`](../../../third_party/harfbuzz/README.md) and [`third_party/freetype`](../../../third_party/freetype/README.md), built by `crates/flashtex-xetex/fontlibs`, and [`third_party/teckit`](../../../third_party/teckit/README.md) (TECkit 2.5.13's engine, used under its LGPL-2.1-or-later option, with TeX Live's zlib), built by `crates/flashtex-xetex/teckit`; all pinned by `crates/flashtex-xetex/tests/pinned_libs.rs`. A TFM font's `:mapping=` goes through tex.ch's ML\TeX `effective_char` as in TeX Live (`changes/mltex.ch`).
- **Font lookup, platform-free.** XeTeX asks Core Text (macOS) or fontconfig
  (elsewhere) for the installed fonts; FlashTeX does not. A name is resolved
  by XeTeX's own matching rules, rewritten, over a platform-free index of the
  installed and project fonts (`crates/font-discovery` is the existing
  candidate: it already reads `name`, `OS/2` and `MATH` with no platform API).
  A name then resolves to the same face as `xelatex` whenever both see the
  same set of font files; where Core Text or fontconfig enumerate a different
  set, the difference is measured, not assumed away (§5).
- **One font system for every mode.** The index, font loading and the font
  resources of the display list and the PDF writer are shared with the
  Classic and Modern modes (the engine-modes proposal). Shaping stays per
  mode: Unicode mode shapes exactly as XeTeX does, with the pinned HarfBuzz.
- **Out of scope: AAT-only fonts** (Commander ruling, 2026-10-04). XeTeX
  shapes them through Core Text on macOS; FlashTeX takes no Core Text
  dependency, so a font with AAT tables and no OpenType layout is shaped by
  HarfBuzz as any other font, and where that differs from `xelatex` on macOS
  the difference is documented, not fixed. Vendoring HarfBuzz, FreeType and
  ICU is not shipping a separate TeX: they are libraries.
- **Licensing (§3 of DESIGN.md; a belief until the §3 legal review):**
  HarfBuzz (MIT), FreeType (FTL), ICU (Unicode licence) and the font index
  (MIT) can live on the MIT side and so be shared with the Typst host; TECkit
  (LGPL-2.1 or CPL) and Graphite2 (LGPL-2.1 or MPL-2.0) are linked only into
  the GPL engine. Code ported from xetex.web's change files stays in the GPL
  crate.

### 3.2 Output: display list and FlashTeX's PDF writer (S2)

- `ship_out` in Unicode mode emits `display-list-v3` (DESIGN.md §6.1), as the
  pdfTeX engine's display-list writer does: native glyph runs carry glyph ids
  and `FONT.format: "opentype"` (E1), with `face_index`, variations and the
  font file's digest. The preview needs nothing else (DESIGN.md §6.2).
- **PDF comes from the display list through FlashTeX's own writer**, not from
  XDV and not from xdvipdfmx. The writer is new: the pdfTeX engine's backend
  (DESIGN.md §6.3) writes from its own `ship_out` and keeps doing so for
  Classic mode, where P-T2 binds. The Unicode-mode writer reuses that
  backend's pieces where they apply: TeX Live's zlib, PNG inclusion that copies
  IDAT unchanged, PDF inclusion through the ported `pdftoepdf`. It adds
  OpenType embedding (CFF and TrueType subsets, `face_index`, variations) and
  `ToUnicode` maps from the glyph-to-text clusters. Candidates to evaluate
  before building (§1's rule, a decision for S2): `pdf-writer`, `krilla` and
  `subsetter` (dual MIT or Apache-2.0, to confirm, used under MIT).
- **Licence rule (Commander ruling, 2026-10-04): no Apache-2.0-only code.**
  It would force the binary to GPLv3 only (xpdf already limits it to GPL v2
  or v3, §3 of DESIGN.md). Reused code is MIT, BSD, or dual MIT/Apache-2.0
  used under MIT.
- **Specials.** Real packages pick the `xetex` or `xdvipdfmx` driver
  (`xetex.def`, `hxetex.def`, `l3backend-xetex.def`, `pgfsys-xetex.def`,
  xcolor) and talk to the output through dvipdfmx's `\special` language
  (`pdf:bann`/`eann`, `pdf:dest`, `pdf:outline`, `pdf:docinfo`, `pdf:image`,
  `pdf:literal`, `color push`/`pop`, `x:` specials, `papersize`). FlashTeX
  interprets that language in the display list and the writer, from its
  documentation and measured behaviour, and does not port xdvipdfmx. The set
  is closed by counting the specials the S2 and S3 corpora emit; an unknown
  special is a structured diagnostic, never a silent drop.
- XDV stays as the **parity instrument**: `xetex.web`'s own DVI code writes it
  when asked (lockstep and tests), and the product path does not.

### 3.3 One engine interface, two binaries (S3)

Owner ruling on the engine-modes proposal's Q10 (2026-10-04): **SPLIT**, two
FlashTeX engine programs rather than one. Classic is `flashtex-host`;
Unicode mode (and later the FlashTeX native mode, §3.6) is a second binary,
for example `flashtex-host-unicode`. Both are FlashTeX's own builds over one
shared runtime library crate; neither is `xetex` or xdvipdfmx.

- **A shared runtime crate.** What both engines use moves out of
  `crates/flashtex-engine` into a runtime library crate: the word space
  (`arena`), the checked index (`ix`), `cli`, the resolver and kpathsea set-up,
  dates and MD5, the host's socket protocol, the incremental machinery
  (checkpoints, convergence, memoisation, macro replay), the display-list
  writer and diagnostics. The font system of §3.1 and the PDF writer of §3.2
  sit beside it, so a fix lands once for both binaries.
- **The engine interface trait is kept.** Both translated engines are
  web2rust output of the same shape (a `Globals` word space, `tex_body`,
  `ship_out`), and both implement one FlashTeX engine trait: start a job from
  a configuration, run to the next `\shipout` or checkpoint, save and restore
  state (the word space plus each engine's side tables: for Unicode mode the
  handle tables of §4.7), emit display-list pages and diagnostics. The host
  code, the incremental system, the display-list writer and the PDF export are
  written once against the trait and instantiated per binary.
- **Two binaries, the mode per document.** `flashtex-host` links only the
  pdfTeX engine; `flashtex-host-unicode` links only the XeTeX-derived engine,
  so Classic's binary, size and start-up do not change with Unicode's
  libraries (HarfBuzz, FreeType, ICU, TECkit, Graphite2). The app starts the
  binary the document's mode names (the engine-modes proposal, #1520, owns the
  manifest key, detection and the switch); **a mode switch starts the other
  binary**. This also keeps kpathsea's process-wide configuration
  (`crates/flashtex-engine/src/resolver.rs`: one resolver per process) to one
  program name per binary (`xetex` for Unicode: `TEXINPUTS.xetex`,
  `OPENTYPEFONTS` etc.).
- **Measured for each binary at S3:** the binary's size, its cold start-up
  (process start to the first protocol reply, and to the first page of a
  warm-format document), and its resident memory, with Classic's numbers
  before and after the runtime split reported side by side.
- **What has to move first, without changing the pdfTeX engine's output:**
  the shared runtime above into its own crate; the `flashtex-xetex` crate into
  the root workspace (it is excluded today, §2); every process-wide `static`
  of both engines into per-engine state (§4.7 for this crate; the pdfTeX
  engine has 49 `static`/`thread_local!` lines outside `generated/`, not yet
  classified as mutable or constant). Each step is a refactor gated by the
  pdfTeX engine's P-T1 lockstep, P-T2 and the drift test.

### 3.4 Files: the format and the bundle (S3)

- The `xelatex` format is built by FlashTeX's Unicode mode from `latex.ltx`
  (TeX Live's `xelatex.ini` and its hyphenation patterns), cached by content
  hash, as D12 does for pdflatex. No format from TeX Live is loaded, and no TeX
  Live binary runs.
- The no-TeX-Live bundle must carry what Unicode-mode documents read: the
  `xetex`/`xdvipdfmx` drivers, fontspec, unicode-math, polyglossia and their
  dependencies, `tex-text.tec` and the other TECkit mappings, and the OpenType
  fonts the defaults name (Latin Modern and Latin Modern Math, TeX Gyre). Its
  manifest gets a Unicode-mode section checked by the S3 no-TeX-Live gate.

### 3.5 Parity targets

- **P-T1 and XDV, against `xelatex`/`xetex -no-pdf`.** Box dumps and the
  `\tracingall` log at every `\shipout`, with DESIGN.md §1.1's normalisation,
  and the XDV byte for byte after the two normalisations of §4.2 (the
  preamble's date comment, and the font file's path in `define_native_font`).
  This is the primary gate for Unicode mode, as P-T1 is for Classic.
- **PDF: visual and structural, not byte-level.** Without xdvipdfmx, font
  subsets, object order and compression legitimately differ. Structural: the
  same page count and page boxes; per page the same glyphs (font by
  PostScript name and face, glyph id) at the same positions within
  xdvipdfmx's output precision (a belief to measure: positions rounded to
  0.01 bp); the same links, destinations, outline entries and document
  information; the same images and their placement; the same colours.
  Visual: both PDFs rasterised by one rasteriser (Core Graphics, as the
  preview gate does, DESIGN.md §6.2) at 2×, with 0 differing pixels as the target and
  any floor measured and stated, as DESIGN.md §6.2 states its 1× floor.
- Byte-level PDF parity (P-T2) stays Classic mode's export gate and is not a
  Unicode-mode target.

### 3.6 After S3: the FlashTeX native mode

A **"FlashTeX" native mode** is planned after the modes proposal's M3
(Unicode in the product), specified in the engine-modes proposal (#1520). It
is opt-in and FlashTeX-only, built on the Unicode core and shipped in the
Unicode binary: the project's `[fonts]` applied automatically, modern
defaults, single-pass cross-references, and output pinned to a FlashTeX
version. It does not match any other engine by design, so it never replaces
Unicode mode's `xelatex` parity, and S1–S3 add nothing for it beyond keeping
the engine trait and the runtime crate open to a third configuration.

## 4. Phase S0 results (2026-10-04)

All numbers below are **verified** (measured on mac-m1max-a, MacTeX 2026 as
the oracle) unless marked otherwise.

### 4.1 What compiles

- `web2rust` translates the whole of `xetex.web` with the 17 change files:
  1,721 sections, 203,413 tokens, 1,427 pool strings, 449 routines into
  `src/generated/` (`scripts/xetex-regenerate.sh`). The crate builds in
  release and debug, clippy-clean (`-D warnings`) and rustfmt-clean.
- Native fonts, TECkit, ICU, Graphite and pictures are stubs that answer as
  TeX Live's XeTeX does when nothing is found (`src/xetex_ext.rs`); the C
  pointers `xetex.web` keeps (glyph-info arrays in `mem`, layout engines,
  mappings, OpenType assemblies, picture paths) are integer handles
  (`changes/ext.ch`), which is the model S1 fills in.

### 4.2 P-T1 and XDV against `xetex -no-pdf`

`scripts/xetex-lockstep.sh` (tools/xetex-lockstep):

| set | cases | P-T1 log + exit status equal | XDV identical after normalisation |
|---|---|---|---|
| tools/lockstep cases with no pdfTeX-only primitive (`suite.txt`, selected by `--select`: 1308 of 1464; 156 use a pdfTeX-only primitive) | 1308 | 1308 | 1308 of 1308 |
| XeTeX-specific cases (`tools/xetex-lockstep/cases`) | 53 | 53 | 53 of 53 |
| XeTeX's own tests (bug73, ctrlsym, filedump; format dump and load included) | 3 | 3 (also equal to the committed `.log`) | n/a |
| **total** | **1364** | **1364** | **1361 of 1361** |

The spike named two XDV normalisations, the preamble's date comment and a
native font's path. The harness implements the first; the second is S1 work
(below), since S0 writes no `define_native_font` record. With
`SOURCE_DATE_EPOCH` pinned, the first changes no byte in S0 either. The XDV is written by `xetex.web`'s own DVI
code (S0 needed no separate writer); byte identity holds through the
preamble, `set`/`fnt_def` records, the postamble and the 223 padding.

### 4.3 web2rust changes, and pdftex.web unchanged

Each is something pdftex.web never does (`tools/web2rust/README.md`, "What
xetex.web needs beyond pdftex.web"): a local `const` section; C's bare
`break`; `addressof(x)` as a `var` argument; `--first-string 65536` (the
numbering of Omega's `otangle`, which TeX Live tangles xetex.web with); and
the unary-minus-before-`and` clash, refused like the other C/Pascal
precedence clashes. Proof that pdfTeX is unchanged: `cargo test --release -p
web2rust` passes, including `tests/drift.rs`, which regenerates
`crates/flashtex-engine/src/generated/` and `pdftex.pool` from pdftex.web and
compares them byte for byte with the committed ones.

### 4.4 Review against TeX Live's own program

The WEB the change files produce was diffed against TeX Live's merged XeTeX
WEB (`tie` of xetex.web, tex.ch0, tex.ch, the feature files, SyncTeX's,
xetex.ch, char-warning-xetex.ch and tex-binpool.ch). It found, beyond what
the lockstep cases had caught: XeTeX's one-word SyncTeX layout (box nodes 8
words, not pdfTeX's 9); web2c's identity `hi`/`ho` (XeTeX's `min_halfword` is
negative); the string-pool dump of `str_start` (only the pool strings'
starts); the empty trie's size `max_hyph_char`; and two tex.ch bug fixes,
[25.369] (`\noexpand\endwrite`) and [26.449] (`\mkern` with a non-mu
dimension). All are in the change files now.

**Finding for the pdfTeX engine (not changed by this lane):** neither tex.ch
fix [25.369] nor [26.449] is re-specified in `crates/flashtex-engine/changes/`
(pdftex.web has the pre-fix text, TeX Live's pdftex gets the fixes from
tex.ch). **Measured** for [26.449]: on the input of
`tools/xetex-lockstep/cases/x053-mkern-nonmu.tex` with tools/lockstep's
prelude and invocation, TeX Live's pdftex and the engine (main at
f2012b591) differ: the engine reports `Illegal unit of measure (mu
inserted)` and `Dimension too large` after `\mkern\dimen0`, and its box is
9127.08324pt wide where pdftex's is 26.74988pt. For [25.369] the input of
`x052-noexpand-endwrite.tex` gives equal logs, so a difference there is
still only a belief. The fix is a one-hunk change each in the engine's
`web2c.ch` (the XeTeX port's `web2c.ch` has both), plus a lockstep case.

### 4.5 Speed (measured, load-sensitive)

INITEX with every tracing switch on (the lockstep invocation), median of 11
runs: `001-edef-basic` xetex 1.10 s, port 1.31 s; `x001-utf8-text` xetex
0.99 s, port 1.65 s; `089-mathcode-class` xetex 1.56 s, port 1.70 s. INITEX
is not the product path, and S0 adds none of the engine's optimisations; the
gap is a belief to be profiled in S1 (first suspect: the terminal written a
byte at a time while `\tracingonline` copies the whole trace to it).

### 4.6 Not done in S0 (by design)

`crates/flashtex-xetex/changes/README.md`, "Not re-specified": native
fonts, TECkit, ICU encodings, pictures, the output driver (`-no-pdf` is the
only mode), tex.ch's hashed `\hyphenation` table, ML\TeX, encTeX, source
specials, SyncTeX output, `\XeTeXinputencoding` on the terminal level. None
can show in a TFM-only document that does not name them.

### 4.7 Required in S1 (from the review of #1489)

- **Handles are engine state.** The handle allocator of `changes/ext.ch`'s
  C pointers (a counter and a free list, with the tables behind it: glyph-info
  arrays, layout engines, mappings, OpenType assemblies, picture paths) is
  saved and restored with the word space, so a checkpoint (DESIGN.md §5.2)
  restores the handles the `mem` words it restores refer to.
- **No process-wide state.** `static PROTRUSION` (`src/xetex_ext.rs`, hz.cpp's
  protrusion codes), `START` (the run's start time) and the statics of
  `src/system.rs` (`FIRST_LINE`, `NO_PDF`, `TERMINATING`, `TEX_INPUT_TYPE`,
  `FULL_NAME_OF_FILE`) move into per-engine state, so a resident host can run
  several engines and restore one.
- **The XDV font-path normalisation is implemented** in
  `tools/xetex-lockstep/run.py`: the file name inside each
  `define_native_font` record is replaced by its base name before the
  comparison (the record's other bytes stay compared), with a test.

## 5. Risks carried into S1–S3

| risk | mitigation |
|---|---|
| Glyph metrics through FreeType (heights, depths, italic corrections, bounding boxes) | Rewrite `XeTeXFontInst.cpp`/`XeTeXLayoutInterface.cpp`'s metrics against the vendored FreeType; compare every `\XeTeXglyphbounds`, `\fontcharht` etc. with xetex in lockstep cases |
| Shaping version lock: shaped output depends on HarfBuzz's exact version (and line breaking on ICU's) | Vendor HarfBuzz 12.3.2, FreeType 2.14.1 and TeX Live 2026's ICU as TeX Live builds them (sha256 in `third_party/<lib>/README.md`); a test pins the versions; an upgrade is a lane of its own with a lockstep run against the matching TeX Live |
| C pointers in `mem` | Handles (S0's model): the runtime owns the glyph arrays; `copy_native_glyph_info`/`free_native_glyph_info` map to copy/free of a handle; checkpoints (DESIGN.md §5.2) snapshot the handle tables with the word space (§4.7) |
| XDV for native fonts (`define_native_font`, glyph arrays, `set_text_and_glyphs`) | `make_font_def` and `make_xdv_glyph_array_data` rewritten from XeTeX_ext.c, compared byte for byte with the path normalised |
| Font lookup by name without Core Text or fontconfig picks another face than `xelatex` | XeTeX's matching rules rewritten over the platform-free index (§3.1); P-T1 cases use fonts by file name first; name lookups are measured against `xelatex` on a named set of macOS and TeX Live fonts, and every difference is reported with the two candidate faces |
| PDF parity without xdvipdfmx: the `\special` language is large and partly undocumented | The set is closed by measurement on the S2/S3 corpora (§3.2); structural comparison (§3.5) names the first differing object; unknown specials are diagnostics |
| Splitting out the shared runtime changes the pdfTeX engine | Each step of §3.3 is a refactor gated by the pdfTeX engine's P-T1 lockstep, P-T2 and the drift test, and by `flashtex-host`'s size and start-up; the two binaries never share a process, so never kpathsea state |
| Build time: a second 74k-line generated crate in the workspace | Measure the workspace build with `flashtex-xetex` in it before S3 moves it; keep the path-filtered gate (§2) until the number is known |
