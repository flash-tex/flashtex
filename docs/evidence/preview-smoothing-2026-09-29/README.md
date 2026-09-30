# Preview tiles and font smoothing — evidence (P0-PREVIEW-TILES, 2026-09-29)

Machine: mac-m1max-a (M1 Max, 10 cores, built-in 120 Hz XDR display, macOS 26.3).
The machine is shared with other agents' builds, so every run records `uptime`.
Governing text: `docs/design/engine-v2/DESIGN.md` §6.2 and Appendix B.3.

Dense page: `dense.tex` (article, `\footnotesize`, 3,675 glyphs on page 1),
`flashtex-render --tex dense.tex --v2 dense.json --font-dir apps/mac/Fonts`.
The JSON (1.7 MB) is not committed; regenerate it with that command. The tests
use `apps/mac/Tests/FlashTeXMacTests/Fixtures/display-list-v2-dense.json`
(840 KB): the same list with each run's clusters collapsed to one
(`python3 compact_clusters.py dense.json display-list-v2-dense.json`), so every
glyph, position, font and paint is unchanged.

## 1. Font smoothing (`GlyphRunRenderer.smoothFonts`)

Method: export page 1 through `GlyphRunRenderer.pdfData` (the app's export),
open it with `open -g -a Preview`, capture the window by ID
(`screencapture -x -o -l`), and compare its ink with our raster of the display
list at Preview's own scale (1.8735 px/pt, measured from the width of the
justified text block), smoothing off and on. Ink = Σ(255 − gray) over the text
block, per square point.

| Raster (1.8735 px/pt) | Preview ink ÷ ours | Preview dark px (<128) ÷ ours | Mean gray of the crop |
|---|---|---|---|
| ours, smoothing **off** | **1.2525** | 1.4745 | 230.33 |
| ours, smoothing **on**  | **0.9991** | 0.9881 | 224.03 |
| Preview.app             | —          | —      | 223.83 |

At 4 px/pt the same ratios are 1.2572 (off) and 0.9964 (on). Preview.app draws
PDF text with CoreGraphics' font smoothing (stem darkening).

Images: `preview-window.png` (the capture, half size), and 320×120 px crops of
the same text, shown 3× nearest-neighbour: `crop-preview-app.png`,
`crop-ours-smooth-on.png`, `crop-ours-smooth-off.png`.

**Decision: `smoothFonts` stays `false` for now; this needs the Commander.**
Smoothing on matches Preview.app, but it breaks V2Parity's zero tolerance:
CoreGraphics stem-darkens the ligature glyphs (fi, ffi) of the PDF-embedded
font differently from the same glyphs drawn through the CTFont. On the text
fixture that is 11 differing pixels at 1 px/pt and 13 at 2 px/pt (a few gray
levels, up to 14). With smoothing off there are 0. The spec requires both zero
tolerance and the smoothing that matches Preview.app, so zero tolerance wins
until the Commander chooses. Switching is one line; the writet1 export (§6.3)
changes the embedded fonts and should be measured again then.

Reproduce: `FLASHTEX_SMOOTHING_OUT=<dir> FLASHTEX_SMOOTHING_LIST=dense.json
FLASHTEX_SMOOTHING_SCALES=1.8735,4 swift test --filter PreviewSmoothingEvidenceTests`
writes `export.pdf` and the smoothing on/off rasters.

## 2. Tiles are exact windows of the page, at every tiled scale

Review of #1228 (flashtex-2a): at 6 px/pt, tile (4, 1) of dense page 1 differed
from the whole-page raster in 168 px (one glyph a pixel over). A sweep over the
tiling range (3.25–8 px/pt in 0.25 steps, every tile, zero tolerance) at the
reviewed head found 6 differing tiles: that glyph, a glyph on the math-nav
page at 5 px/pt, and four 1–3 px differences at rule ends (math-rules and
scripts at 7 px/pt, math-nav at 4.25 px/pt). A TikZ page (paths, a clip,
dashes) differed at almost every scale.

Causes, measured with a one-glyph probe (Helvetica 10 pt at 6 px/pt and 8 pt
at 3.25 px/pt, device x swept in steps down to 5e-7 px):

- CoreGraphics picks a glyph's pixel and subpixel phase as
  `floor((d + 0.001)·N)` per axis (N = 1, 2 or 4 phases by glyph size; the
  rendering changes at k/N − 0.001 exactly, on both axes, y measured from the
  bottom). A tile translates the page by whole pixels, which is exact, but
  `d + 0.001` rounds by magnitude: `6 × 354.6665` = 2127.99899999999980 px, and
  `+ 0.001` gives 2128 on the page but 79.99999999999980 in the tile (2048 px
  to the left). Positions are 7-digit decimals, so a device coordinate landing
  exactly on k/N − 0.001 happens about once per dense page at some scales.
- Rule edges are single-precision device coordinates, whose rounding also
  depends on magnitude (one gray level on a rule's end column).
- Paths: curve flattening, stroke expansion and edge clipping all depend on
  the device coordinates; snapping points did not fix them.

Fix (`GlyphRunRenderer.tileOrigin`, `tileRect`, `cutTiles`):

- A glyph origin within 1e-6 of a phase boundary is moved 1e-7 px onto the side
  the page's own sum took; every other origin passes unchanged.
- A rule's edges are passed as the page's single-precision values, which the
  whole-pixel translation leaves exact.
- A page with any path or image item is not tiled by translation: each tile
  job rasterizes the whole page once and cuts its tiles from it (exact by
  construction; glyph-and-rule pages keep the cheap translated tiles).

The whole-page raster and the export are untouched. `PreviewV2TileTests`:

- `testTilesArePixelExactWindowsOfTheWholePageAcrossTheTiledScales`: 3.25–8
  px/pt in 0.25 steps, dense (both pages; `Fixtures/display-list-v2-dense.json`,
  its runs' clusters collapsed to one each, glyphs and positions unchanged),
  text, math, math-rules and the TikZ page: **0 differing tiles** (7,154 tiles
  compared; it was 6 of them before, plus the TikZ page).
- The review's exact case, a dark sweep point, and `tileCoordinate` on the
  reviewed value.
- `testTiledPageEqualsTheExportedPDFRaster`: V2Parity at zero tolerance at 1, 2,
  3.5 and 6 px/pt on the text and dense pages, and the tiles equal the export.
- Before the fix, the same sweep over the text and math fixtures plus
  scripts, window and math-nav (not committed; 7 fixtures) found the 6 tiles
  above; with the fix, 0.

## 3. No drawing on the main thread (DESIGN §1.2)

`show`, scrolling and a committed zoom only queue tile jobs on
`V2TileGrid.queue`: the missing or stale visible tiles, then the prefetch
margin. A job draws its tiles in parallel and hands them to the main thread,
which installs them as layer contents (compositing only) if the view's tile
generation is unchanged; tiles of a page, scale or appearance the view has left
are dropped, and a job queued for one is skipped before it draws.
`dispatchPrecondition(.notOnQueue(.main))` guards the job. Until a tile lands
the page shows what it had: the 2 px/pt backdrop, the previous content's tile
in the same place (a keystroke), or, after a zoom, the previous scale's tiles
stretched to the new scale (linear) until every visible tile of the new scale
is up.

A sample of the main thread during the scroll bench then showed
`CA::Render::copy_image` redrawing every new tile (about 0.4 ms each) and each
backdrop (about 3 ms) in the CoreAnimation commit: the sRGB bitmaps were
converted to the window's colour space there (with the window set to sRGB, 148
samples fell to 3). Tiles and backdrops are now converted to the window's
colour space on the tile queue (`V2TileGrid.displayImage`) before they reach a
layer, and preview bitmaps use CoreAnimation's premultiplied BGRA. The sRGB
raster is the one the parity tests compare; the conversion is display only.

## 4. Tiles at 8 px/pt (dense page, release build)

`tile-bench.json`, from `FLASHTEX_TILE_BENCH_LIST=dense.json swift test -c release
-Xswiftc -enable-testing --filter PreviewV2TileTests/testTileBenchmark` at
2a2524530 (median of 9). The machine was heavily loaded: load average 366 with
all three CI runners (`mac-m1max-a-1..3`) busy, so the times are upper bounds;
the first measurement (#1228, load 7.8) is in brackets.

| | Before: whole page | After: tiles |
|---|---|---|
| Bitmap memory | 124.1 MB (4896×6336 px) | 31.5 MB (30 tiles for a 1100×900 pt pane at 2×, margin included) + 7.8 MB backdrop at 2 px/pt |
| Raster time | 21.8 ms [18.1] | 8.7 ms for all 30 tiles, 8 workers [3.0]; 15.9 ms sequential [6.6] |
| Scroll step | — | 1.86 ms for a new row of 6 tiles [0.68] |
| One 512 px tile | — | 0.32 ms [0.24] |
| Backdrop, 2 px/pt | — | 4.8 ms |

All of it runs on the tile queue. What made tiles fast (debug builds, load
15–50): culling to the items and glyphs whose ink box meets the tile (without
it one tile cost 1.4 ms); testing an item's bounds before binding it (refcount
contention between workers, 17 ms against 9 ms). Separate `CGFont`s per worker
gave nothing in release (3.4 ms against 3.3 ms).

## 5. In-app scroll at 8 px/pt, 120 Hz

`FLASHTEX_V2_SCROLL_BENCH=8` drives the preview's scroll view from its display
link at 2,400 pt/s, bouncing between the ends of the two-page document, and
counts dropped frames (a callback interval over 1.5× the 8.33 ms frame). The
bench moves the window to the fastest screen (the built-in 120 Hz panel; this
Mac also drives a 60 Hz external display) and records main run-loop passes
over 8 ms with what the preview did in them, the off-main tile jobs, frames
with visible tiles still missing, and the load average.

Launch (release app, never activated): `FLASHTEX_NO_ACTIVATE=1
FLASHTEX_PREVIEW_V2=1 FLASHTEX_V2_FILE=dense.json FLASHTEX_V2_SCROLL_BENCH=8
FLASHTEX_V2_SCROLL_BENCH_OUT=<json> .build/release/FlashTeXMac
-FlashTeX.PreviewZoom.v1 '<real>4</real>'` plus a 1,080 pt preview column
(argument-domain split frames), which puts the pages at exactly 8 px/pt.
`FLASHTEX_V2_TILE_THRESHOLD=1000` gives the "before" run (whole pages). Load and
runner state per run: `scroll-bench-runs.txt`.

| | Before: whole pages | Tiles, colour converted in the commit (f802987b7) | Tiles at 2a2524530, run 1 / run 2 |
|---|---|---|---|
| File | `scroll-bench-before-whole-page.json` | `scroll-bench-before-offmain-colour.json` | `scroll-bench-after-tiles.json` / `-run2.json` |
| Frames in 8 s | 42 | 952 | 957 / 955 |
| Dropped frames | 920 | 7 | **3 / 5** |
| Frame interval p50 / p99 / max | 155 / — / 462 ms | 8.33 / 8.33 / 34.0 ms | 8.33 / 8.33 / 22.2 ms; 8.33 / 8.33 / 23.2 ms |
| Longest main-thread scroll step | — | 1.5 ms | 3.7 / 6.8 ms |
| Frames with a visible tile still missing | — | 2 | 3 / 1 |
| Page bitmaps held (max) | 248 MB | 44 MB | 44 MB |
| Process footprint (max) | 952 MB | 174 MB | 130 / 125 MB |
| Load average; CI runners busy | 17; — | 12; none | 272–205; 2 of 3 |

**Zero dropped frames is not shown yet.** The remaining drops are one
reproducible pair when page 2 first scrolls into view (t ≈ 1.10 s: a 22–29 ms
main-thread pass followed by a 17–20 ms one in which page 2's first 10 tiles
are installed) and, in some runs, a single 16.7 ms interval. The long passes
contain no preview-v2 rasterization and no image conversion (0–1 samples of
`copy_image` in a sampled run); the main thread spends much of the scroll in
AppKit/SwiftUI window layout (`NSWindow layoutIfNeeded` → `NSHostingView.layout`, 13–18 %)
in every sampled window. Ruled out by experiment: the page shadow, the HUD's
page readout (`previewVisiblePage`), and the accessibility overlay (its lines were never built). Finding what
lays out the window at page 2's first appearance is the next step.
