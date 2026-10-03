# "Infinite Descent" in the Mac app (lane INFDESC-APP, 2026-10-03)

Owner priority (2026-10-03): FlashTeX must compile and preview the whole of
Clive Newstead's *An Infinite Descent into Pure Mathematics*
(`~/Documents/infdesc/infdesc.tex`, 95 `.tex` files under `book/`; reference
`infdesc.pdf`, 592 pages). The app ran on a copy in a scratch directory; the
original folder was never written.

Machine: mac-m1max-a (M1 Max), shared with other lanes. **Load averages were
16–170 during these runs** (recorded with each run below), so every time here
is load-sensitive and an upper bound, not a quiet-machine figure.

App: built from `origin/main` 8aee5e3be plus the two PRs of this lane
(`apps/mac/scripts/make-app.sh`, `flashtex-host` from the same main in
`Contents/Helpers`). Driven only by environment hooks (`FLASHTEX_OPEN`,
`FLASHTEX_NO_ACTIVATE=1`, `FLASHTEX_V3_CACHE`, `FLASHTEX_V3_CAPTURE_*`,
`FLASHTEX_V3_BENCH*`), never Accessibility; screenshots by window id.

## Verified (measured on this machine)

### Engine v3 (`FLASHTEX_ENGINE_V3=1`), default host (external tools off, as main ships)

| | value | run |
|---|---|---|
| Whole book compiles | yes: `ok`, 0 errors, 44 warnings, 580 pages | `capture-v3-default.json` |
| First page on screen | 3.4 s after the open (format cached, no S₀) | same |
| All pages (first DONE) | 281 s; 5 passes, 2,896 pages typeset (load 54–77) | same |
| Host memory (`flashtex-host` phys_footprint) | peak 1.41 GB, 1.29 GB after the compile | same |
| App memory | peak 198 MB | same |
| Pages drawn from the PDF (INCOMPLETE) | none | same |
| Index | **missing**: 580 pages, 12 fewer than the reference | see "Index" below |

Text of every page against the reference (Ghostscript `txtwrite`, per-page
character multisets, order-insensitive): pages 1–574 match except page 2
(the build date) and page 10 (the table of contents' page numbers of the four
indexes, which do not exist yet).

### Engine v3 with external tools (makeindex), the behaviour of #1344

`flashtex-host --external-tools auto` (a wrapper standing in for #1344's
per-COMPILE `"external_tools": "auto"`, which is not on main):

| | value |
|---|---|
| First DONE | 54 s, 1 pass, 576 pages (tools run before the `.aux` passes) |
| makeindex, then the follow-up compile (`cause: tools`) | DONE at 119 s: **592 pages** |
| Text against the reference | **591/592 pages identical**; page 2 differs only by the build date |
| Host memory | peak 1.58 GB |

The four indexes (topics, vocabulary, notation, LaTeX commands) are typeset
and match the reference (`compare-p575.png`). The multi-pass state settled:
cleveref references, `lastpage`, the table of contents (its index entries
included) and the hints/solutions files are as in the reference.

**Why the index is missing without tools.** imakeidx's `[makeindex]` runs
`makeindex infdesc.idx` through `\write18` (restricted shell escape allows
it), and the engine runs it exactly as pdfTeX would. But the app compiles
with an output directory (`…/out`) separate from the source copy, and
makeindex looks for `infdesc.idx` in the current directory: "Input index
file infdesc.idx not found" ×4 in the host's stderr. TeX Live 2026's makeindex
ignores `TEXMF_OUTPUT_DIRECTORY` (checked: same error with it set), so pdflatex
with `-output-directory` fails the same way. latexmk-style external tools in
the host (#1296) run `makeindex` in the output directory and fix it; #1344
turns them on in the app for trusted projects.

### pdflatex oracle (MacTeX 2026, pdfTeX 1.40.29), for comparison

In place (no output directory), imakeidx running makeindex itself: 3 passes
(588, 592, 592 pages; `.aux`, `.toc`, `.sol`, `.hnt`, `.thm`, `.idx` identical
after pass 2), 91–100 s per pass at load ~150.

### Old engine (default, flag off)

Compiles to "recovered: preview shown with provisional rendering": 846 errors,
1,479 warnings, 710 "not implemented", 574 pages; the pane shows font-map text
instead of pages, and a CTAN fetch sheet asks to install `imakeidx`,
`lastpage`, `refcount`, `xkeyval`, … (`old-engine-window.png`,
`old-engine-fetch-sheet.png`). The old engine is frozen (D13); nothing was
changed there.

### Preview correctness (pane bitmaps against the reference)

`compare-p*.png`: left, the reference PDF (Ghostscript, 144 dpi); right, the
bitmap the engine-v3 pane itself drew for that page (`EngineV3PageCapture`,
`installedImage`), in the app's dark preview (the system appearance here):

- `compare-p2.png` title page: **before the fix the logo (an included PDF,
  `book/media/logo.pdf`) was missing** from the pane, while the exported PDF
  had it. Fixed in PR A (#1407); `compare-p2-fixed.png` is after.
- `compare-p100.png` theorem/example/exercise boxes (ntheorem `framed` with
  TikZ backgrounds; mdframed is loaded but its environments are not used):
  match.
- `compare-p210.png` a tikz-cd diagram: matches.
- `compare-p575.png` the first index page (with tools): matches.
- `compare-p569.png` (a PNG figure) and `compare-p570.png` (a whole included
  PDF page, the "Practice page"), after the fix: match.

The after-fix images (`compare-p2-fixed.png`, `-p569`, `-p570`) were drawn by
the pane's renderer (`DL3Renderer`, light) from the pages the host sent,
because in that run the pane was collapsed (380 × 56 pt; the cause was not
pinned down: a later launch had the 1,500 pt window at once, with the split
positions restored from the app's shared defaults deciding the pane width).
`EngineV3PageCapture` records the pane frame and which source drew each page. `pdf-image-repro-before.png` /
`-after.png` are the pane's own bitmaps of a two-image test document.

The pixel-level gate (Core Graphics' rendering of the same compile's PDF
against the pane, light mode) is `EngineV3PDFImageTests` for the PDF-image
fix; the general parity gate is `PreviewParityTests` (P3).

### Keystroke latency, edit in a middle chapter

`FLASHTEX_V3_BENCH` with `FLASHTEX_V3_BENCH_FILE=book/number-theory/modular-arithmetic.tex`
(chapter 8; the edit is on page 264), 4 keystrokes 150 s apart, load 76–190:

- Opening the chapter file (no text change) made a **cold full compile:
  658 s, 5 passes** (`cold_reason`: the preamble ran `\write18`).
- Keystroke 1 → the first re-sent page 19 s later, then pages from page 1 at
  about one per second, all unchanged; **the edited page 264 arrived 83 s
  after the key** (it was off screen: the bench's pane was collapsed, so no
  key → pixels sample was recorded).
- Keystrokes 2–4: **no page at all in the remaining 7.5 minutes.** A
  superseded compile finishes its run silently (protocol §6.5), and a cold
  run of this book is 5 passes, so each newer COMPILE waits for it.

So typing in a middle chapter is not usable today; the cause is on the engine
side (issues 1 and 2 below), not in the app's edit path (the app sends each
keystroke at once, measured 1.7–2.9 ms app-side in #1254).

Typing by someone at the machine in a running evidence app (53 keystrokes in
`getting-started.tex`, 06:08–06:13Z) hit the same wall: all 53 COMPILEs
waited behind a 446 s tools round and were answered `cancelled`; the last one
then took 821 s (5 passes, first page after 123 s, 115 s of it queued).

## App fixes (this lane)

- **PR A #1407 `agent/mac-claude-a/infdesc-app-pdf-images`: included PDF pages
  were blank in the v3 pane.** Three causes in `DL3Renderer`: the matrix of a
  PDF image is pdfTeX's Form XObject `cm`, whose form space is the included
  page's own coordinates in bp (not the unit square); a `CGPDFPage` does not
  retain its document; the host sends the box in scaled points (see engine
  issues). After review: no second origin shift (the `cm` carries it),
  pdfTeX's form `/Matrix` for `/Rotate` 90/180/270, and an LRU bound on the
  image cache. Tests: `EngineV3PDFImageTests` (end to end, pixel-identical to
  the compile's PDF), `PDFImageRenderTests` (host-free, 0 px against Core
  Graphics for a box away from the origin, a smaller crop box, scaling,
  rotation, dark), `PDFImageBoxTests`. The after-fix images here were made
  before the review fix; both included PDFs of the book have their box at
  the origin, where the two agree.
- **PR B `agent/mac-claude-a/infdesc-app-evidence`** (stacked on A):
  - `FLASHTEX_WINDOW_FRAME` resized whichever window was first 0.5 s after
    launch, which can be a 500 × 500 helper window rather than the main one
    (the first run here kept a 900 pt main window and a collapsed preview). It
    now waits up to 2 min for the main "FlashTeX" window, holds the frame and
    logs once when it applied it (0.5 s after launch in the check run).
  - `FLASHTEX_V3_CAPTURE_*` (`EngineV3PageCapture`): waits until the host
    is quiet, shows chosen pages, writes the pane's own bitmaps (or, when the
    pane does not show the page within 30 s, the same renderer's drawing of
    it) and a timing and memory summary; `EngineV3PagesView.scrollToPage`.
  - `FLASHTEX_V3_BENCH_FILE` / `_TIMEOUT`: type in a chosen project file
    (a middle chapter) and wait longer than 300 s for the first compile.
  - Tests: `EngineV3PageCaptureTests`.

Not app problems: project-relative `\input` across the 95 files resolved
(every page's text matches); the PNG figure draws; no hang or crash in the
app during these runs.

## Engine and host issues for INFDESC-ENGINE (reported, not fixed here)

1. **Every edit is a cold full run.** `DONE.cold_reason`: "the preamble ran an
   external command (write18)" (`host/mod.rs` `Key::check`, `barriers`). S₀
   is never used for this book, so a keystroke re-typesets from page 1 (an
   edit compile took 820 s and 2,960 typeset pages at load ~150; first page
   123 s after the key, of which 115 s queued). Find which preamble
   `\write18` sets the barrier (the only `runsystem` calls in the log are
   imakeidx's four makeindex calls at the end of the document).
2. **A superseded compile blocks the newer one for minutes.** A cancelled or
   superseded run "finishes the page it is on and the rest of the run without
   sending" (§6.5); on this book the rest of the run is up to 5 passes of
   ~580 pages. While a `cause: tools` round ran (5 passes, 446 s), 53
   keystroke COMPILEs (ids 2–54) waited and were then all answered
   `cancelled` (§6.4 says a newer COMPILE supersedes the cycle); in the bench,
   keystrokes 2–4 got no page in 7.5 minutes. Stopping at the next page (or
   checkpoint) instead of finishing the passes would bound this.
3. **5 passes where pdflatex needs 3.** Default host: 5 passes, 2,896 pages
   typeset for 580 pages; pdflatex converges after pass 3 (files identical
   after pass 2). `converged_at` is null in every DONE.
4. **PDF image box units.** `displaylist/mod.rs` `dl_image_info` sends a PDF
   image's `width`/`height`/`orig_x`/`orig_y` in scaled points
   (`bp2int`); protocol §5.2 says bp. The app now reads an implausible box
   from the file, but the host should send bp.

## Files

- `capture-v3-default.json`, `capture-v3-tools.json`: `EngineV3PageCapture`
  summaries (ms from the open; DONEs as main applied them, sampled every
  20 ms, so DONEs arriving together appear once).
- `compare-p*.png`, `v3-window-p210.png`, `old-engine-*.png`,
  `pdf-image-repro-before.png` / `pdf-image-repro-after.png`.
