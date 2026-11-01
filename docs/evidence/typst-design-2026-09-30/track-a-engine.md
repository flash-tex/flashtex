# Typst design, track A: engine integration architecture

- **Lane:** TYPST-DESIGN-A (kabir-claude, mac-m5pro-kabir), 2026-09-29/30.
- **Question:** how a second engine, Typst, plugs into FlashTeX behind
  `display-list-v3` so that editing feels as good as LaTeX, without touching
  the LaTeX work. Input to rewriting DESIGN.md §15.
- **Typst version:** 0.15.1 (crates.io, 2026-07-17; tag `v0.15.1`,
  `9dfd3a08500b7896045f907433cf7b4b02434fad`), the latest stable.
- **Machine:** M5 Pro, 15 cores, 24 GB, macOS 26 (Darwin 25.6). Other agents'
  engine benchmarks ran throughout. The 1-minute load average was **8 to 60**,
  and every raw row records it. Typst's compile is mostly single-threaded, so
  that load inflates wall times. Rows marked *(load)* were taken at a load
  above 20 and are not used for conclusions.
- **Prototype (throwaway, not committed):**
  `/Users/kubar/.claude/jobs/80e4852c/tmp/typst-a/proto`. It is a
  1,100-line Rust embedder (`tbench`) over the unmodified 0.15.1 crates, plus
  a Swift Core Graphics pixel-diff harness in `…/typst-a/px/`.
- **Raw data:** [`raw-a/`](raw-a/) holds one JSON row per run, and the traces
  are summarised as text.

Measured numbers and my inferences are kept apart. **Verified** means I measured
it here or read it in the 0.15.1 source. **Belief** means an inference I did not
test.

---

## 0. Answer in brief

1. **Typst compiles the whole document on every edit.** There is no API that
   returns pages as they are laid out, or the edited page first. `typst::compile`
   returns one finished `PagedDocument` after all introspection iterations. For
   this lane's long test documents, the verified keystroke latency (edit → new
   document) is:

   | pages | standard `typst::compile`, p95 | seeded compile (§2.3), p95 |
   |---|---|---|
   | 10 | 4.8–5.2 ms | 1.6–2.6 ms |
   | 100 | 53–60 ms | 16.4–17.4 ms |
   | 300 | 205–227 ms | 60–64 ms |
   | 1,000 | 1,000–1,400 ms | 353–387 ms |

   The LaTeX side's target is ≤ 16 ms p95 for the edited page at any length.
   With unmodified Typst **that target holds only up to about 100 pages.**
2. **The largest mitigation is ours to build: seed the compile loop with the
   previous compile's introspector.** `typst::compile` starts every compile
   from an empty introspector, so a document with an outline, references and
   citations runs **4 layout iterations per keystroke**. Seeding the loop, with
   the same public crates, converges in **1 iteration**:
   - 300 pages: 190 → 60 ms p50;
   - 1,000 pages: about 1,000 → 351 ms;
   - output identical to `typst::compile` on **all 192 of 192** validated
     edits (d10, d100 and d300).
3. **Pixel parity with Typst's own PDF is achievable, with one condition.**
   Glyph positions must come from typst-pdf's content stream, which is the same
   method the LaTeX side uses (v3 §4.2), and must stay in f64 bp.
   - With those positions, Core Text drawing of the display list gave **0
     differing pixels** against Core Graphics' rendering of the exported PDF on
     2 text pages (7,039 glyphs) at 2× and 3×.
   - Typst's own frame positions come within 5.8 × 10⁻⁵ bp of the PDF's, and
     still give 31–5,324 differing pixels.
   - Rounding the positions to v3's integer sp also breaks exactness: 118–581
     pixels, each off by up to 3 levels.
4. **Architecture:** run a separate `flashtex-typst-host` process for each open
   Typst document.
   - It is our own MIT code, linked to the Apache-2.0 typst crates and the MIT
     `flashtex-display-list` crate.
   - It speaks the same socket protocol as `flashtex-host`, and nothing links
     it to the GPL engine.
   - A separate process is required for crash, hang and memory isolation in any
     case, whatever the licence says (§7).
5. **`display-list-v3` gaps.** It lacks:
   - OpenType fonts and glyph ids;
   - exact f64 origins;
   - ICC colour spaces, alpha, spot colours;
   - gradients, tilings, SVG images, colour glyphs, stroked text;
   - page labels and bleed;
   - on-demand span resolution.

   Every gap has a backward-compatible extension proposed in §3.3. The largest
   is **"PDF islands"**: the constructs v3 can't express are drawn from a
   typst-pdf fragment. That needs no protocol change at all.

---

## 1. Embedding API (verified from source)

### 1.1 What an embedder implements

`typst_library::World` (re-exported as `typst::World`,
`crates/typst-library/src/lib.rs:60`) must be `Send + Sync` and provide:

| method | purpose | our host |
|---|---|---|
| `library() -> &LazyHash<Library>` | standard library; `Library::builder().with_inputs(..).with_features(..).build()` (needs `typst::LibraryExt`) | built once: **0.9 ms** |
| `book() -> &LazyHash<FontBook>` | metadata of every font | `typst_kit::fonts::FontStore` |
| `font(index) -> Option<Font>` | lazily load font `index`; may be called with stale indices during memo validation (0.14 panic fix, PR 6117) | `FontStore::font` |
| `main() -> FileId` | entry file | |
| `source(FileId) -> FileResult<Source>` | a parsed Typst file | editor buffers held as `Source`, updated by **`Source::edit(range, text)`** (incremental reparse, keeps span numbers stable) |
| `file(FileId) -> FileResult<Bytes>` | any other file (images, bib, data, fonts, plugins) | project root plus the package cache |
| `today(offset) -> Option<Datetime>` | `datetime.today()` | pinned per compile, for reproducibility |

Optional `typst_ide::IdeWorld` (with `upcast`, `packages`, `files`) enables
completion, tooltips, jumps and so on. `typst::compile::<PagedDocument>(&world)
-> Warned<SourceResult<PagedDocument>>` is the whole compile API. `comemo`'s
cache is global to the process; the embedder must call **`comemo::evict(n)`**.

### 1.2 Crates (0.15.1)

| crate | role | we use |
|---|---|---|
| `typst` | `compile`, `trace`; re-exports `typst-library` | yes |
| `typst-syntax` | lexer, incremental parser, `Source`, `Span`, `FileId` | yes (via `typst`) |
| `typst-eval` | evaluation (`eval(world, library, traced, sink, route, source)`, public) | yes, for the seeded loop (§2.3) |
| `typst-library` | the standard library, `World`, `Engine`, `Frame`/`FrameItem`, introspection | yes |
| `typst-realize` | show-rule realisation | indirect |
| `typst-layout` | layout; **`PagedDocument`, `Page` live here** (`typst_layout::PagedDocument`) | yes |
| `typst-pdf` | PDF export (via **krilla** 0.8.2 + pdf-writer 0.15; `PdfOptions { page_ranges, tagged, standards, … }`) | yes: export, and the source of exact positions (§5) |
| `typst-render` | tiny-skia raster (PNG export) | no (reference only: 1.7–8 ms per page at 2×) |
| `typst-svg` | SVG export | no |
| `typst-html`, `typst-bundle` | HTML and bundle targets (experimental) | no |
| `typst-ide` | completion, tooltips, `jump_from_click`, `jump_from_cursor`, definitions | yes |
| `typst-kit` | helpers for embedders: `FontStore`, `fonts::{embedded, system, scan}`, `FileStore`/`FileLoader`, `SystemPackages`/`UniversePackages`, `SystemDownloader`, `Watcher`, `Time` | yes (fonts, packages) |
| `typst-assets` | embedded fonts (feature `fonts`: Libertinus Serif, New Computer Modern (+Math), DejaVu Sans Mono = **17 faces**), ICC profiles, hyphenation data | yes |
| `typst-timing` | Chrome-trace scopes | benchmarks only |

### 1.3 How typst-cli and tinymist embed it

- **typst-cli `SystemWorld`** (`crates/typst-cli/src/world.rs`) keeps:
  - a `LazyHash<Library>`;
  - a `LazyLock<FontStore>`, so the system-font scan happens only on first use;
  - a `FileStore<SystemFiles>`, which caches files and tracks dependencies for
    `watch`;
  - a `Time`, fixed via `SOURCE_DATE_EPOCH`.

  `typst watch` calls `comemo::evict(10)` after each compile
  (`watch.rs:82`).
- **tinymist** (checked at `f3b00eb5`, 2026-09-18, on typst 0.15.1) also calls
  plain `typst::compile`.
  - After each compile it runs `comemo::evict(10)` on a CPU worker thread
    (`tinymist-project/src/compiler.rs:981-993`), because all projects share one
    cache.
  - Its preview (`typst-preview`) turns the document into reflexo's vector IR
    and ships **deltas** to a webview.
  - `tinymist-viewer` is a new WGPU/Vello native viewer fed with the same
    incremental vector data. Neither renders the edited page before the whole
    document is done.

### 1.4 API stability history (from the changelogs in `docs/content/changelog/`)

| release | date | embedder-visible breaks |
|---|---|---|
| 0.6 | 2023-06 | `World` reworked for packages ("breaking change for implementors") |
| 0.7 | 2023-08 | parser split into `typst-syntax`; `World::today` fix needed downstream |
| 0.8 | 2023-09 | `Span`/`FileId` made type-safe: every error path must be handled |
| 0.9 | 2023-10 | `typst::ide` extracted to `typst-ide`; `'static` bounds removed |
| 0.10 | 2023-12 | `typst` + `typst-library` merged; `typst-pdf`/`typst-svg`/`typst-render` split out |
| 0.11 | 2024-03 | first release on crates.io since 0.1 (0.2–0.10 were git-only) |
| 0.12 | 2024-10 | `World: Send + Sync` (multithreaded compiler); `World::main` signature changed; `Tracer` removed for `Warned<T>`; `typst-kit` added |
| 0.13 | 2025-02 | `compile` generic over `PagedDocument`/`HtmlDocument` |
| 0.14 | 2025-10 | crate split (`Library: Default` removed → `LibraryExt`); `PdfOptions.tagged`; `World::font` panic fix; PDF export rewritten on krilla; new incremental algorithm (PR 6683); MSRV 1.88 |
| 0.15 | 2026-06 | `typst-kit` **completely reworked** (PRs 7710, 8026); `DiagSpan`; variable fonts; spot colours; bundle target; baseline changes (silent layout shifts); MSRV 1.92 |

**Verified:** every minor release since 0.6 has broken embedders. Releases come
about every 4–8 months (0.11 → 0.15: 2024-03, 2024-10, 2025-02, 2025-10,
2026-06).

**Consequence (belief):**
- Pin `=0.15.1` exactly.
- Upgrade in a deliberate lane that re-runs the parity gate.
- Keep all typst-touching code in one crate, so each upgrade is one diff.

Output can also change between Typst versions (0.15's baseline shifts). The
preview and the export use the same crate, so parity is unaffected. Documents
written for an older Typst may still change appearance on upgrade.

---

## 2. Incremental performance (measured)

### 2.1 Test documents

Documents are generated by `tbench gen` (reproducible, seeded). Each is A4,
11 pt, justified, with numbered headings and a running header that uses
`context` and `counter(page)`. Each section contains:
- 3 paragraphs;
- an inline equation and a numbered display equation with a label;
- a citation to a BibTeX file with n/2 entries, and a footnote;
- a reference to the section's equation;
- every 3rd section, an SVG figure with a caption and a reference to it;
- every 5th section, a table.

Every document also has an `#outline()` and a `#bibliography`.

- **d10 / d100 / d300 / d1000:** 10 / 100 / 300 / 999 pages
  (13 / 139 / 416 / 1,387 sections; 28,738 / 305,481 / 917,306 /
  3,069,321 glyphs).
- **c300:** as d300, with a `#pagebreak()` every 14 sections (chapters): 300
  pages.
- **Ablations of d300:** `nooutline`, `nocite` (no citations or
  bibliography), `plain` (neither of those, no footnotes, no labels or
  references), `bare` (plain, and no running header or numbering).

**Edits: realistic typing** (`type_char`). Each keystroke inserts a new letter,
and every 7th a space, just before a marker at 5%, 50% or 95% of the document
(`EDITSTART`, `EDITMID`, `EDITEND`). A *backspace* run deletes characters of
the prose. **Every document state is new.**

A first run instead alternated insert and delete of the same character. That
made comemo return a cached document (0.17–10 ms at any size), and it was
discarded. Any Typst latency claim must use non-repeating edits.

Each edit is `Source::edit` plus a compile. p50 and p95 are over 40 keystrokes
per location, after 2 warm-up keystrokes.

### 2.2 Standard `typst::compile` (`raw-a/bench-typing-evict10.jsonl`, `comemo::evict(10)` after every compile, load 9–13)

| doc | cold compile | no-change recompile | keystroke p50 | keystroke p95 | changed pages per keystroke | footprint after 160 keystrokes |
|---|---|---|---|---|---|---|
| d10 | 76 ms | 0.19 ms | 4.3–4.7 ms | 4.8–5.2 ms | 1 | 108 MB |
| d100 | 518 ms | 2.3 ms | 50.6–54.5 ms | 53.1–60.1 ms | 1–2 (max 6) | 961 MB |
| d300 | 1,575 ms | 7.3 ms | 189–194 ms | 205–227 ms | 1 | 3,038 MB |
| c300 (chapters) | 486 ms | 11 ms | 134–137 ms | 149–164 ms (one run: 341 ms) | 1–2 | 1,023 MB |
| d1000 *(load 25–60)* | 12,012 ms | 50 ms | 916–1,146 ms | 1,001–1,399 ms | 1 | 10,596 MB |

- **The latency does not depend on where the edit is.** Start, middle, end and
  backspace are within noise of each other. The cost is the whole document every
  time.
- Pages whose frame hash changed per keystroke: **1–2**, and at most 6 in d100
  when a line re-broke across a page. So the *output* delta is as small as
  LaTeX's, but it arrives only after the whole compile.

**Where the time goes** (`typst-timing`, d300, one EDITMID keystroke,
`raw-a/trace-d300.summary.txt`; tracing overhead inflates this run to 433 ms):

- 4 layout iterations (`iter (1)`–`iter (4)`);
- `realize` (show rules) 238 ms in total, of which outline entries take 78 ms
  (1,250 calls) and `cite-group` 32 ms;
- `eval` 8.6 ms.

**Ablation** (`raw-a/bench-ablation.jsonl`, 20 keystrokes each; load 24–40, so
indicative only). Keystroke p50 at 270–292 pages:

| variant | keystroke p50 |
|---|---|
| bare | **33.5 ms** |
| plain (+ running header, heading and equation numbering) | 48 ms |
| nocite (+ outline, footnotes, labels) | 101 ms |
| full d300 | 190 ms |

Introspection-heavy features (outline, citations, references, counters) are
what make Typst's per-keystroke cost grow.

### 2.3 Mitigation measured: seed the fixed-point loop (`raw-a/seeded.jsonl`, `seeded-validate.jsonl`)

`typst::compile` (`crates/typst/src/lib.rs`, `compile_impl`) evaluates the
source, then calls `PagedDocument::create` with an introspector. The first time
that introspector is `EmptyIntrospector`, then it is the previous iteration's.
The loop stops when `constraint.validate(doc.introspector())` holds, after at
most 5 iterations.

Every piece of that loop is public:
- `typst_eval::eval`;
- `Engine { world, library, introspector, traced, sink, route }`;
- `Output::create` and `Output::introspector`;
- `comemo::Constraint`.

So the host can run the same loop with **the previous keystroke's final
introspector as the first one** (`compile_seeded` in the prototype). Results:

| doc | cold iterations | keystroke iterations (seeded) | seeded p50 | seeded p95 | vs standard p50 |
|---|---|---|---|---|---|
| d10 | 3 | 1 | 1.7–2.1 ms | 2.0–2.6 ms | 2.4× |
| d100 | 4 | 1 | 15.0–16.1 ms | 16.4–17.4 ms | 3.4× |
| d300 | 4 | 1 | 59.2–62.4 ms | 60.0–64.7 ms | 3.2× |
| c300 | 4 | 1 | 45.8–46.6 ms | 46.8–50.1 ms | 2.9× |
| d1000 (load 11–13) | 4 | 1 | 350.6–356.3 ms | 352.5–386.7 ms | ≈ 2.9× |

- **Correctness (verified):** for 16 keystrokes at each of 4 locations in d10,
  d100 and d300, every page of the seeded document had the same `hash128` as a
  standard `typst::compile` of the same source. That is **0 mismatches in 192
  edits**.
- **Belief, and why:** seeding reaches the same fixed point whenever the
  previous introspection answers are still valid. In a pathological document
  with several fixed points it could settle on a different one than a
  from-empty compile would. The standard compile would then still converge,
  and still stay within its 5-iteration cap. The host should therefore:
  - re-run the standard `typst::compile` when idle and replace the pages if they
    differ;
  - always use the standard compile for export.
- **Where the rest goes** (seeded d300, `raw-a/trace-d300-seeded.summary.txt`,
  69 ms under tracing):
  - `realize` 35 ms (outline entries 12, `cite-group` 9);
  - page run 22 ms;
  - `eval` 5 ms.

  In `bare` (`trace-d300bare-seeded.summary.txt`, 40 ms) the page run is
  16 ms, `eval` 5 ms and `realize` 4 ms. **Nothing we can do outside Typst makes
  these O(document) passes O(page).**

### 2.4 Memory and eviction (`raw-a/mem.jsonl`)

| run | footprint |
|---|---|
| d300 seeded, `evict(10)` after every compile, 1,000 keystrokes (load 9–14) | 575 MB → 969 MB (100) → 992 MB (1,000): **flat**. Keystroke p50 60–62 ms throughout, p95 64–79 ms |
| d300 seeded, **no eviction**, 300 keystrokes | 571 MB → 7.6 GB (100) → 14.7 GB → **21.7 GB** (300): **+70 MB per keystroke**. `evict(0)` afterwards took 6.1 s and left 9.5 GB |
| d300 standard, `evict(10)`, 1,000 keystrokes | 2.9–3.4 GB (the 3 extra iterations cache 3 extra layouts); p95 spikes to 1 s at load 35 |
| `evict(10)` itself (d300 seeded) | p50 9.5–10.3 ms, p95 25–43 ms (load 12–22) |

**Consequences:**
- Eviction is mandatory.
- It must run **after** the pages are sent, off the compile's critical path, as
  tinymist does.
- Seeded compiles use about ⅓ of the memory of standard ones.
- Budget about 1 GB per open 300-page document and about 3–4 GB at 1,000
  pages (d1000 seeded: 2.9–3.6 GB).
- comemo's cache lives only in memory, so **reopening a document is always a
  cold compile**: 1.6 s at 300 pages, 12 s at 1,000 under load. The LaTeX
  side's ≤ 100 ms reopen (persisted S₀) has no Typst equivalent. The app should
  show the previous session's page rasters, keyed by the v3 content hash, while
  the cold compile runs.

### 2.5 Edited page first? No: a whole document per compile

- **Verified:** `layout_document` realises the whole document, splits it into
  page *runs* at `pagebreak`s, and lays the runs out in parallel
  (`engine.parallelize`, `crates/typst-layout/src/pages/mod.rs`). It then
  finalises the pages and returns them all at once.
- There is no callback and no page streaming. There is no cancellation either:
  a compile that has started finishes.
- Inside a run, flow layout is sequential. It uses memo hits for the
  paragraphs and blocks that did not change.
- The engine therefore cannot put the edited page on the socket before the rest
  of the document is done, the way `flashtex-host` does after a `\shipout`.

| | LaTeX (`flashtex-host`, DESIGN §5) | Typst 0.15.1 (this lane) |
|---|---|---|
| unit of work per keystroke | restart at the checkpoint before the edit, stop at convergence | re-evaluate and re-realise the whole document; memo hits for unchanged paragraphs and runs |
| edited page available | as soon as it ships (≤ 16 ms p95 target) | after the whole compile (table in §0) |
| later pages | background; marked stale | arrive in the same compile (only 1–2 changed) |
| cost grows with | pages between restart and convergence | total document length × introspection density |

**Mitigations, in order of value:**

1. **Seeded introspector** (§2.3): 2.4–3.4×. This is ours to build.
2. **Chapter page-runs:** a `#pagebreak()` between chapters splits layout into
   independently memoised runs, laid out in parallel. For 300 pages that gives
   **46 vs 60 ms** seeded (c300 vs d300), and a cold compile of **486 vs
   1,575 ms**. The cost is the document author's structure, not ours to impose.
   The app can suggest it for long documents.
3. **Send the viewport page first once the compile ends.** Per page, the
   display-list encoding costs 30 µs (9.2 ms for all 300 pages) and a
   single-page PDF export 0.5 ms (§5.2). The first page on the socket therefore
   comes about 1 ms after the compile, and the other 0–5 changed pages follow.
4. **Coalesce keystrokes.** One compile runs at a time, and the newest edit
   state wins, so typing at 10 keys/s against a 60 ms compile never queues. If
   the compile takes longer than the gap between keys, latency is bounded by
   about 2× the compile time. Show the previous pages (not marked stale) until
   the new document lands: in Typst the old pages are mostly still right.
5. **Page-diff hashing off the latency path.** `hash128` over every page frame
   costs 16–21 ms at 300 pages when there is one run (d300). That is 0.8 ms in
   c300, where unchanged runs return cached frames. **Belief:** hashing in
   parallel over 15 cores, or first comparing frame `Arc` identity, brings it
   under 2 ms. Hash the viewport page first.
6. **Not recommended:** compiling a truncated document (the current chapter
   only) for a provisional frame. It breaks parity (counters, references, page
   numbers) and relies on assumptions about the document's structure.
7. **Upstream:** incremental realisation, or a page-streaming callback in Typst.
   Both are outside our control. We could propose one to the Typst maintainers,
   with the numbers in §2.2 as motivation.

**Verdict (verified numbers, belief about users):**
- Up to about 100 pages, Typst meets the §1.2 target with the seeded compile
  (p95 16.4–17.4 ms).
- At 300 pages the edited page takes about 60–65 ms. Most users won't notice
  that at typing speed with coalescing, though it is visible on fast repeats.
- At 1,000 pages it takes about 350–390 ms, and **§1.2 is not met**. Document
  that limit rather than hide it.

---

## 3. Mapping Typst frames to `display-list-v3`

### 3.1 Typst's output model (verified, `typst-library/src/layout/frame.rs` etc.)

A `Page` has:
- a `frame` (size, baseline, and items `Arc<LazyHash<Vec<(Point, FrameItem)>>>`);
- `fill: Smart<Option<Paint>>`;
- `bleed: Sides<Abs>`;
- `numbering`, `supplement` and the logical page `number`.

`FrameItem` =
- `Group(GroupItem { frame, transform, clip: Option<Curve>, label, parent })`;
- `Text(TextItem { font: FontInstance (font + variation coords), size, fill: Paint, stroke: Option<FixedStroke>, lang, region, text, glyphs: Vec<Glyph { id: u16, x_advance, x_offset, y_advance, y_offset: Em, range, span: (Span, u16) }> })`;
- `Shape(Shape { geometry: Line | Rect | Curve(Move/Line/Cubic/Close), fill: Option<Paint>, fill_rule, stroke: Option<FixedStroke { paint, thickness, cap, join, dash, miter_limit }> }, Span)`;
- `Image(Image { kind: Raster | Svg | Pdf, … }, Size, Span)`;
- `Link(Destination: Url | Position(page, point) | Location, Size)`;
- `Tag(Tag)`, which is introspection only and draws nothing.

`Paint` = `Solid(Color)`, `Gradient(Linear | Radial | Conic)` or `Tiling`.

`Color` = `Process(Luma | Oklab | Oklch | Rgb | LinearRgb | Cmyk | Hsl | Hsv)`
or `Spot(colorant, tint)`.

Typst's `pt` is the PostScript point, i.e. **bp** (1/72 in), not TeX's pt.

### 3.2 Construct-by-construct mapping

**Census** (`raw-a/census.json`). The prototype compiled each of Typst's
3,829 test-suite snippets (`tests/suite`, split at `--- name ---`, with
typst-dev-assets v0.15.1 fonts and assets). 2,203 compiled; most of the rest are
intentional error tests. Across 2,623 pages it counted:

| construct | count |
|---|---|
| glyphs | 85,787 |
| text runs | 29,241 |
| text runs with a stroke | 31 |
| shapes (line / rect / curve) | 5,593 / 1,985 / 594 |
| dashed strokes | 116 |
| images (raster / SVG / PDF) | 73 / 24 / 6 |
| links (url / position / location) | 73 / 2 / 595 |
| groups | 5,657 (133 transformed, 23 clipped) |
| solid paints | 36,723 |
| gradients (linear / radial / conic) | 583 / 81 / 74 |
| tilings | 37 |
| colour spaces (luma / sRGB / CMYK / HSL / HSV / spot / Oklab / Oklch / linear RGB) | 32,848 / 3,783 / 26 / 23 / 23 / 15 / 2 / 2 / 1 |
| paints with alpha < 1 | 164 |
| variable-font runs | 175 |
| colour-glyph runs (COLR/sbix/CBDT/SVG) | 64 |
| skewed text runs | 60 |

| Typst construct | how typst-pdf writes it (krilla 0.8.2) | v3 today | proposal |
|---|---|---|---|
| Text: glyph ids of an OpenType font (CFF, CFF2, TrueType; collections) | `Type0`/`CIDFontType0C` or `CIDFontType2` subset, Identity-H; positions from `cm` + `Tm` + `TJ`, all numbers **f32** (`pdf-writer` `push_float` → ryu) | **gap**: `FONT.format` = `type1` only; `opentype`/`truetype` reserved | E1 |
| Glyph origin | f32 in bp user space | sp integers (i32) | E2 (exact f64 bp origins) |
| Variable-font instance (`FontInstance.variations`) | subsetter instantiates a static CID font (`subset_with_variations`) | gap | E1 `variations`; pixel parity untested (**risk**) |
| Colour glyphs (COLR v0/v1, sbix, CBDT, SVG) | Type 3 fonts whose glyph procedures are images or vector drawings (`krilla/src/text/type3.rs`) | gap (Type 3 reserved) | E5 PDF island |
| Text fill: solid | `cs`/`scn` in ICCBased sGray (N=1) or sRGB, DeviceCMYK, Separation; components quantised **to u8** (`to_vec4_u8`) | partial: DeviceGray/RGB/CMYK only, f64 | E3 |
| Text fill: gradient or tiling | shading or tiling pattern as the text's fill | gap (flagged `sh`/pattern INCOMPLETE) | E5 |
| Text stroke (`FixedStroke`) | `Tr` 1/2 with width, cap, join, dash | partial: `TEXT_RENDER` exists, but no line width, cap, join or dash for text | E4 (state item) or E5 |
| Skewed, rotated or scaled text (group transforms) | `cm` | yes: `MATRIX` glyph matrix + page-space origin (the host flattens transforms) | none |
| Shape: line, rect, cubic curve; fill rule; stroke width, cap, join, miter, dash + phase | path ops, f32 | **yes**: `PATH` with paint bits, stroke parameters and matrix | emit the f32 values krilla writes |
| Text decoration (underline, strike, overline, highlight) | ordinary shapes in the frame | yes (`PATH`) | none |
| Group clip (`Curve`) | `W n` | yes: `CLIP` + `SAVE`/`RESTORE` | none |
| Alpha < 1 (fill or stroke) | ExtGState `ca`/`CA` | gap (`gs` INCOMPLETE) | E3 alpha |
| Gradients: linear, radial | axial/radial shading, sampled in the gradient's space (0.15 fixed sampling) | gap | E5 (or a later native `SHADING`) |
| Gradient: conic | no native PDF conic; krilla approximates it | gap | E5 (native would have to copy krilla's approximation) |
| Tiling | tiling pattern | gap | E5 |
| Spot colour (`color.spot`) | Separation with a process fallback | gap (flagged) | E3 |
| Image: raster (PNG, JPEG, GIF, WebP), including from bytes | JPEG passed through; others decoded and re-encoded as Flate + ICC + SMask; `/Interpolate` from `image.scaling` | partial: `png`/`jpeg` by **file path** only | E6 |
| Image: SVG | converted to PDF vector content (krilla-svg over usvg) | gap | E5 |
| Image: PDF page | embedded as a form XObject | yes (`IMAGE` type `pdf`, file + page) if it comes from a file | E6 for bytes |
| Link: URL | URI annotation | yes (kind 4) | none |
| Link: `Position` | GoTo page + point | yes (kind 3 "N spec") | none |
| Link: `Location` (unresolved in the frame) | resolved through the introspector; labelled headings become named destinations (0.15) | yes once the host resolves it (kind 1 name or 3 page) | none |
| Named destinations (labels) | `/Dests` | yes (`DESTS`) | none |
| Page fill (background) | a filled rectangle | yes (`PATH`/`RULE`) | none |
| Page bleed (`page.bleed`) | MediaBox is enlarged; trim box present | gap: `box` = MediaBox only | E7 |
| Page label (`numbering`: "iv", "A-3") | `/PageLabels` | gap: `counts[10]` is TeX's `\count0–9` | E7 (host puts the physical page in `counts[0]`, the rest 0) |
| Outline / bookmarks, document metadata, PDF tags | outline, metadata, structure tree | out of scope (not drawn) | none |
| `Tag` items | used for tagging | no drawing | none (§4 uses them) |

### 3.3 Proposed extensions (backward-compatible)

v3's rules (§3 of the spec):
- a new **section tag**, **message kind** or **JSON key** is a minor change;
- a new **item opcode** is a major one, because a reader must not guess at it.

The proposals keep to those rules. Where a new opcode is unavoidable, it is
**negotiated**: the host sends it only to a client whose `HELLO` says
`[3, 2]` and lists the capability. A 3.1 client never receives one; it gets
`INCOMPLETE` pages and falls back to `DONE.pdf`, exactly as today. The protocol
owner has to rule on whether "negotiated opcode" counts as a minor change.

| # | extension | kind | what it carries |
|---|---|---|---|
| **E1** | `FONT.format: "opentype"` (the value is already reserved) | JSON keys (minor) | `face_index`, `variations: [[tag, value]]`, `units_per_em`, `ps_name`, `file` + `program_sha256`. GLYPH `code` is the glyph id; `encoding` is absent. Key = SHA-256(tag, format, sha256(program), face index, variations). **Allow an empty program when `file` is readable and its sha256 matches**: system CJK collections are 20+ MB, and the app can map the file. |
| **E2** | section tag 7 **`ORIGINS_F64`** | new section (minor) | For each GLYPH item in order, `f64 x, f64 y` in page space, **bp**, top-left, y down. These are the values the PDF viewer computes from the content stream (§5.2). A reader that knows the section uses it; others keep the sp values (off by ≤ 7.6 × 10⁻⁶ bp). Add the section to the §4.6 hash. |
| **E3** | section tag 8 **`COLORSPACES`** (id → ICC profile bytes and key, or Separation(name, fallback)); items **`FILL_COLOR_CS`/`STROKE_COLOR_CS`** `u16 cs, u8 n, f32[n]` and **`ALPHA`** `u8 fill/stroke, f32` | section (minor) + negotiated opcodes | Exact colour management: the client builds a `CGColorSpace(iccData:)` from the same profile bytes typst-pdf embeds. Components are the u8-quantised values the PDF has. |
| **E4** | item **`LINE_STATE`** `f64 width, u8 cap, u8 join, f64 miter, dash` for `TEXT_RENDER` 1/2 | negotiated opcode | Stroked text (31 of 29,241 runs in the census). Using E5 instead avoids the opcode. |
| **E5** | **PDF islands**: for a construct v3 can't express, the host exports it with typst-pdf and emits an `IMAGE` of type `pdf` (existing v3 3.1) over its bounding box. The host copies the element into a one-page frame of that size and exports with `page_ranges` and `tagged: false`. | **none needed** (uses the existing `IMAGE` `pdf`); optionally a `PDF_DATA` message to send bytes instead of a file | Gradients, tilings, conic gradients, SVG images, colour glyphs, gradient- or tiling-filled text, spot colours until E3. Core Graphics draws the same PDF operators as the export, so parity holds **by construction** (belief; not yet measured for islands). |
| **E6** | `IMAGE` JSON: `data` (a key for an `IMAGE_DATA` binary message) in place of `file`; types `gif`, `webp`; `interpolate`; `icc` | JSON keys + new message kind (minor) | Typst images can come from bytes (`image(bytes)`, packages). For parity the host sends **the pixels the PDF has** (after typst-pdf's decode and ICC handling), not the source file. |
| **E7** | section tag 9 **`PAGE_META`** (JSON) | new section (minor) | `label` (the page-numbering string), `number` (logical), `bleed` [l t r b], `trim` box, `engine: "typst"`. It keeps `counts` TeX-only, which is the §15 engine-neutrality check. |
| **E8** | messages **`RESOLVE`** `{id, span, col}` → **`RESOLVED`** `{file, byte, line, col}`, and **`LOCATE`** `{file, byte}` → `{hits: [[page, x, y]]}` | new message kinds (minor) | Pull-based source mapping, where SyncTeX needs push (§4). |

Units and precision:
- Typst lays out in f64 **bp**. Converting to sp is exact enough for
  hit-testing, search and caching.
- It is **not** exact enough for zero-tolerance pixels. Measured: positions
  rounded to sp gave 118–581 pixels off by 1–3 levels at 2× and 3× (§5.2),
  hence E2.
- Paths and matrices already carry the PDF's own f64 numbers in v3, so they
  need nothing new.

**Per-page content hash (Typst):** keep v3 §4.6, SHA-256 over the encoded
sections. Add `ORIGINS_F64`, `COLORSPACES` and `PAGE_META` to the hash;
`SPAN` and `col` stay excluded. To decide *which* pages to re-encode, compare
Typst's own `hash128` per page frame (above). The display-list encoding is
about 30 µs per page. The measured encoding (v3-like, 15 bytes per glyph) was
14.2 MB for the 917k glyphs of d300, 47.5 MB for d1000. It only runs for
changed pages.

---

## 4. Source mapping (verified)

**Typst side:**
- Every glyph carries `(Span, u16)`: the syntax node and the byte offset within
  its text.
- Shapes and images carry a `Span`.
- `Tag` items bracket every located element.
- A `Span` is a **stable id**: numbered syntax nodes whose numbers survive
  incremental edits (`typst-syntax/src/span.rs`). That is the model v3 §5.3
  asks for ("span ids … outlive compiles"), and it comes for free here.

Measured on d300 (`raw-a/jumps-d300.jsonl`):

| operation | API | cost |
|---|---|---|
| click → source (file, byte offset) | `typst_ide::jump_from_click` | **4–37 µs** |
| cursor → (page, point) | `typst_ide::jump_from_cursor` (a linear scan of every page frame for the span) | **2.9–3.2 ms** at 300 pages |
| resolve **every** distinct glyph span on all pages to a line (v3's eager `SOURCES` re-declaration) | `WorldExt::range` + `Lines::byte_to_line` | **507 ms** for 25,644 spans (917k glyphs) |

- Eagerly re-declaring the lines of moved spans after every edit, as the LaTeX
  host does, is too slow for Typst at scale.
- Byte offsets move on every edit, and so do lines. The span ids themselves
  don't.
- **Proposal (E8):**
  - Send SPAN ids, and `col` = the glyph's `u16` offset within its span.
  - Declare lines only for the spans of pages actually sent (about 85 spans,
    1.7 ms per page).
  - Serve clicks and forward search on demand through `RESOLVE` and `LOCATE`
    from the host's current `Source`. That costs microseconds per click and
    about 3 ms per forward search at 300 pages. **Belief:** an index from span
    to page built after each compile makes forward search O(1).

Compared with the LaTeX side:
- LaTeX's mapping is `(file, line)` + column captured when each node is
  allocated, and is weaker inside macros (the position where the macro call
  ended).
- Typst's is exact to the syntax node, including inside markup functions and
  show rules (the span of the content element).
- Neither needs a separate SyncTeX file.

---

## 5. Fonts and pixel parity

### 5.1 Discovery (verified)

- **Embedded:** `typst_kit::fonts::embedded()` gives 17 faces in **2.5 ms**.
  These are the defaults: Libertinus Serif, New Computer Modern and its Math
  font, DejaVu Sans Mono.
- **System:** `fonts::system()` uses fontdb, including the Adobe fonts folder
  on macOS. It found **786 faces in 177 ms** here, and loads each face lazily
  on first use. Load it lazily, as the CLI does (`LazyLock`), so a document
  that uses only the embedded fonts never pays for the scan.
- **Project:** `fonts::scan(dir)` (the CLI's `--font-path`).
- **Order:** `FontStore` indices follow push order. The CLI pushes its sources
  in a fixed order; fallback follows the `FontBook` family selection.
- **Variable fonts** are new in 0.15: "Variable", "Var" and "VF" suffixes are
  trimmed from family names.

### 5.2 Drawing with Core Text, and the pixel-parity gate (measured)

The fonts are OpenType, so the app loads them directly from the same bytes Typst
used: `CGFont(CGDataProvider)`, or `CTFontManagerCreateFontDescriptorsFromData`
for collections. It then draws with `CTFontDrawGlyphs` using glyph ids.

Unlike LaTeX, there is no Type 1 conversion. typst-pdf embeds **CFF subsets
that keep the original charstrings and hints**. The prototype checked this with
fontTools on all 4 embedded subsets: `BlueValues`, `StdHW`/`StdVW` and
`StemSnap` are identical to the original fonts, and the hint operators are
present.

**Experiment** (`px/pxdiff.swift`, `raw-a/pixel-parity.jsonl`):
- Core Graphics draws the typst-pdf page (`CGPDFDocument`, sRGB bitmap,
  antialiasing on, font smoothing off).
- Core Text draws the page's glyph runs and lines.
- The two bitmaps are compared per channel.
- Pages: d10 page 2 (3,412 glyphs: text, math, footnotes) and d300 page 150
  (3,627 glyphs).
- Three sources of glyph origins were tried:
  - **typst**: Typst's frame positions (f64);
  - **pdfpos**: re-derived from typst-pdf's own content stream the way a
    viewer does (CTM, `Tm`, `Tf`, `TJ`, `/W`, all from the numbers as written;
    `px/pdfpos.py`);
  - **pdfpos-sp**: pdfpos rounded to v3 sp.

| origins | 1× default | 1× no-quantize | 2× default | 2× no-quantize | 3× default | 3× no-quantize |
|---|---|---|---|---|---|---|
| typst (p2 / p150) | 43,884 / 46,295 | 1,225 / **0** | 62 / 31 | 78 / 43 (≤ 2 levels) | **0** / 408 (≤ 1) | 5,324 (≤ 3) / 408 (≤ 1) |
| **pdfpos** (p2 / p150) | 43,884 / 46,295 | 1,202 / **0** | 62 / 31 | **0 / 0** | **0** / 408 (≤ 1) | **0** / 408 (≤ 1) |
| pdfpos-sp (p2 / p150) | 43,884 / 46,295 | 1,242 / 21 | 62 / 31 | 118 / 137 (≤ 3) | 0 / 408 | 527 / 581 (≤ 3) |

(Each cell is the number of differing pixels. "No-quantize" means
`setShouldSubpixelQuantizeFonts(false)` on both contexts. At 1× default the
differences go up to 118/134 levels.)

**Findings:**
- **Verified:** Typst's frame positions and the PDF's differ by at most
  **5.8 × 10⁻⁵ bp**. krilla writes f32, and so do the `/W` widths
  (`505.99997`). Even that small a difference changes Core Graphics' glyph
  rasterisation at some positions: with the same context settings, Typst's
  origins give 78 (2×) and 5,324 (3×) differing pixels on page 2, where the
  PDF-derived origins give 0.
- **Verified:** with **pdfpos** origins, and subpixel quantisation off, the
  display list is **pixel-identical at 2×** (both pages) and at 3× on page 2.
  - The **408 pixels at 3× on page 150** are ≤ 1 level and appear with every
    origin source.
  - **Belief:** they come from the 4 fraction and footnote rules, which the
    harness draws from Typst's frame, not from PDF-derived coordinates.
- 1× is not exact on page 2 (1,202 pixels, ≤ 42 levels). Retina displays are
  2×, and this was not investigated further. The LaTeX preview lane's
  font-smoothing and quantisation settings (§6.2) should be reused and
  re-measured with Typst.
- **A pitfall found and fixed in the harness:** the display-list flip must use
  the PDF's MediaBox height (f32 `841.8898`), not Typst's f64 page height or the
  bitmap's ceiling. Otherwise every page is shifted by a fraction of a pixel.
  The host must write `box` from the PDF's numbers.

**Recommended method (mirrors v3 §4.2):**
- For each changed page, export that page with typst-pdf
  (`PdfOptions { page_ranges: [p..=p], tagged: false }`). That measured
  **0.44–0.53 ms**.
- Positions from that single-page export are **identical** (max difference
  0.0 bp) to the full-document export's.
- Read the glyph origins, path coordinates and colours back from that content
  stream into the display list, and keep glyph ids and spans from the frame.
  The k-th text object is the k-th `TextItem`: this held on both test pages
  (141 and 118 runs).
- Tagged export can't be combined with `page_ranges` ("cannot enable tagged PDF
  and export a page range"). A standalone one-page `PagedDocument` fails too
  ("tags weren't properly closed"). Hence `tagged: false`, which doesn't change
  positions.
- **Alternative (belief):** reproduce krilla's f32 arithmetic from the frame
  directly. It's cheaper, but ties us to krilla's internals across versions.
  Deriving from the PDF is exact by construction.

**Untested, and so the risks:**
- colours other than black (ICC sGray/sRGB colour management);
- alpha;
- gradients;
- raster, SVG and PDF images;
- colour glyphs (Type 3 in the PDF);
- variable-font instances: subsetter's instancing vs Core Text's variation
  interpolation.

E3 and E5 are designed so these reach parity by construction. Each needs its
own gate row before the gate is claimed.

---

## 6. Packages (verified from source)

- **Import:** `#import "@preview/name:1.2.3"`.
  - Versions are **exact**; there are no ranges, lockfile or solver.
  - Local namespaces live in `{data-dir}/typst/packages/{namespace}`.
  - Downloaded `@preview` packages go to `{cache-dir}/typst/packages/preview/`,
    fetched from `https://packages.typst.org/preview/{name}-{version}.tar.gz`
    (`typst-kit/src/packages.rs:329,360`). The package index is at
    `…/preview/index.json`.
  - The download goes to a temporary directory and is then renamed into place.
    **Nothing verifies its integrity:** there is no checksum or signature, and
    TLS is the only protection (`packages.rs:268-272`).
- **Offline:** a package that isn't cached fails the compile with a package
  error. **Belief:** the host should fetch it on a background thread and
  recompile, and never block a keystroke on the network.
  - The prototype set up no downloader, and so works fully offline.
  - FlashTeX can pre-seed the cache or vendor popular packages.
- **Reproducibility:** given the same compiler version, the same package
  versions and a pinned `today`, output is deterministic.
  - Universe versions are immutable by the registry's policy. That is a policy,
    not something the protocol enforces, so record package tarball hashes in the
    project.
  - Packages can read only their own files, plus project paths passed to them as
    the 0.15 `path` type.
  - Packages have no network, shell or clock access.
- **WASM plugins** (`typst-library/src/foundations/plugin.rs`) run in the
  **wasmi** interpreter (1.0.9):
  - They import only the two `typst_env` protocol functions: no WASI, so no
    files, clock or network.
  - Relaxed SIMD is disabled to keep them deterministic.
  - Functions must be pure. Typst doesn't enforce that; it caches results by
    argument. `plugin.transition` covers impure initialisation.
  - **There is no fuel or memory limit** (none configured). A plugin can loop
    forever or allocate up to wasm32's 4 GiB.
  - Typst's own `while` loops stop after 10,000 iterations, but `for` over a
    huge range runs unbounded.
  - **So the host needs a wall-clock watchdog** (§7).

---

## 7. Process architecture

**Recommendation (belief, from the measurements above):**

```
 Mac app (MIT, Swift) ── display-list-v3 (+E1–E8, negotiated) over Unix socket ──┐
   ├── flashtex-host        (GPL-2+, one per open .tex document)                  │
   └── flashtex-typst-host  (our code MIT; typst crates Apache-2.0; one per open .typ document)
```

- **Separate binary `flashtex-typst-host`,** in its own crate
  `crates/flashtex-typst-host`:
  - It depends on the `=0.15.1` typst crates and the MIT `flashtex-display-list`
    crate (the v3 reference encoder).
  - It never touches `flashtex-engine`, `tools/web2rust` or the LaTeX harnesses.
  - It builds in its own CI job: the typst dependency tree took 2 min 39 s,
    release, 6 jobs, under load.
- **The licence already rules out the alternative, and isolation does too:**
  - Apache-2.0 can't be combined with GPL-2.0-only terms.
  - Nuance (belief; needs the §3 legal review): the engine binary is
    distributable under **GPL v2 or v3** because of xpdf, and Apache-2.0 *is*
    compatible with GPLv3. A single GPLv3 host with both engines is therefore
    legally conceivable. We don't recommend it, because:
    - a Typst panic, a plugin hang or a 20 GB cache blow-up would take the LaTeX
      engine down with it;
    - release cadences and threading models differ (rayon);
    - D11's "public protocol at a process boundary" gives us that isolation for
      free.
  - A single MIT/Apache Typst host next to the GPL host is fine. Ship the
    Apache-2.0 `NOTICE` of the typst crates and the OFL notices of the embedded
    fonts.
- **One host process per open Typst document** (as with LaTeX):
  - comemo's cache is a process-wide static, so one process per document gives
    each its own memory budget.
  - Killing the process frees 1–4 GB at once (§2.4). `evict(0)` took up to
    6.1 s and returned only part of it.
  - Start-up (verified): library 0.9 ms, embedded fonts 2.5 ms, idle footprint
    **12.7 MB**, binary 46 MB stripped (the typst CLI is 44.7 MB). A 10-page
    cold compile takes 76–80 ms; the CLI's full run for d10 took 0.09 s with
    embedded fonts only and 0.19 s with the system-font scan.
- **Inside the host,** mirror `flashtex-host`'s COMPILE semantics:
  - **I/O thread:** decodes `COMPILE` messages. It applies `edits` with
    `Source::edit` (microseconds) and marks the newest state. A newer `COMPILE`
    supersedes one that has not started.
  - **Compile thread:** one compile at a time, using the seeded compile loop.
    For each compile it:
    1. diffs page hashes, viewport page first;
    2. exports changed pages to single-page PDFs;
    3. derives the display list;
    4. sends `PAGE` frames, then `PAGES`, then `DONE` (`mode: "incremental"`,
       `first_page_ms`);
    5. hands `comemo::evict(10)` (10 ms p50, 43 ms p95) to a background thread.
  - **Idle verification:** a standard `typst::compile` checks the seeded fixed
    point. `export` runs the standard compile, then `typst_pdf::pdf` (91 ms at
    300 pages, 446 ms at 1,000).
  - **Watchdog:** no Typst compile can be cancelled. The app kills and restarts
    a host whose compile passes a budget (for example 10 s, or an RSS ceiling),
    marks all pages stale, and cold-compiles. That handles plugin hangs,
    runaway `for` loops and panics.
- **Not recommended:** linking Typst into the app process (MIT and Apache are
  compatible). It would put multi-GB caches, rayon pools and possible hangs
  inside the UI process.

---

## 8. Risks

1. **Latency ceiling:** whole-document compiles make the §1.2 edited-page
   target unreachable beyond about 100 pages with unmodified Typst. Measured
   seeded p95 is 60–64 ms at 300 pages and 353–387 ms at 1,000. Needs an owner
   decision: accept and document the limit, or pursue upstream work.
2. **The seeded compile loop re-implements a private function**
   (`compile_impl`) over public-but-unstable APIs. It has to be re-checked on
   every Typst release, and verified against the standard compile when idle.
3. **API churn:** every 0.x minor release breaks embedders (§1.4). Pin exactly
   and upgrade deliberately.
4. **Parity:** exact only with PDF-derived f64 origins (E2) and matched Core
   Graphics text settings. 1× is not exact. Colour, alpha, gradients, images,
   colour glyphs and variable fonts are unmeasured, and rely on E3/E5 in
   principle.
5. **Memory:** 1 GB per 300-page document, 3–4 GB at 1,000 pages, and
   **+70 MB per keystroke without eviction**.
6. **Untrusted content:** packages are verified only by TLS, plugins have no
   resource limits, and nothing is cancellable. The watchdog restart is the
   only backstop.
7. **Reopen:** there is no persistable cache. A 300-page reopen costs a
   1.6 s cold compile, mitigated by showing cached rasters.
8. **Measurement caveat:** a busy machine (load 8–60). Absolute numbers may be
   5–30% high. The ratios (seeded vs standard, chaptered vs continuous) were
   taken back to back and hold.
