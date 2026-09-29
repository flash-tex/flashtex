# Preview tiles and font smoothing — evidence (P0-PREVIEW-TILES, 2026-09-29)

Machine: mac-m1max-a (M1 Max, 10 cores, built-in 120 Hz XDR display, macOS 26.3).
The machine is shared with other agents' builds, so every run records `uptime`.
Governing text: `docs/design/engine-v2/DESIGN.md` §6.2 and Appendix B.3.

Dense page: `dense.tex` (article, `\footnotesize`, 3,675 glyphs on page 1),
`flashtex-render --tex dense.tex --v2 dense.json --font-dir apps/mac/Fonts`.
The JSON (1.7 MB) is not committed; regenerate it with that command.

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

## 2. Tiles at 8 px/pt (dense page, release build)

`tile-bench.json`, from `FLASHTEX_TILE_BENCH_LIST=dense.json swift test -c release
-Xswiftc -enable-testing --filter PreviewV2TileTests/testTileBenchmark`
(median of 9; load average 7.8):

| | Before: whole page | After: tiles |
|---|---|---|
| Bitmap memory | 124.1 MB (4896×6336 px) | 31.5 MB (30 tiles for a 1100×900 pt pane at 2×, margin included) + 7.8 MB backdrop at 2 px/pt |
| Raster time | 18.1 ms | 3.0 ms for all 30 tiles (8 workers); 6.6 ms sequential |
| Scroll step | — | 0.68 ms for a new row of 6 tiles |
| One 512 px tile | — | 0.24 ms |

What made tiles fast (debug-build measurements, load 15–50):
- Culling: a tile draws only the items, and within runs only the glyphs, whose
  ink box meets it. Without it, one tile cost 1.4 ms and 30 tiles cost more than
  the whole page.
- Testing an item's bounds before binding it: binding retains the run's shared
  arrays, and parallel workers contended on those refcounts (17 ms against 9 ms).
- Separate `CGFont` instances per worker were tried and dropped: in release
  builds they gave no gain (3.4 ms against 3.3 ms, interleaved runs).

Tiles are pixel-identical to the whole-page raster (text and math fixtures, at
8, 6.25 dark, 4 and 3.5 px/pt), and equal the exported PDF's raster at the
pinned 1 and 2 px/pt (`PreviewV2TileTests`).

## 3. In-app scroll at 8 px/pt, 120 Hz

`FLASHTEX_V2_SCROLL_BENCH=8` drives the preview's scroll view from its display
link at 2,400 pt/s, bouncing between the ends of the two-page document, and
counts dropped frames (a callback interval over 1.5× the 8.33 ms frame). The
release app was launched with `FLASHTEX_NO_ACTIVATE=1`, `FLASHTEX_V2_FILE=dense.json`,
zoom 400% (`-FlashTeX.PreviewZoom.v1 '<real>4</real>'`) and a 1,080 pt preview
column (argument-domain split frames), which puts the page at exactly 8 px/pt.
`FLASHTEX_V2_TILE_THRESHOLD=1000` gives the "before" run (whole pages).

| | Before: whole pages (`scroll-bench-before-whole-page.json`) | After: tiles |
|---|---|---|
| Frames in 8 s | 42 | 948 |
| Dropped frames | 920 | 13 (one 53 ms hitch; p99 interval 8.33 ms) |
| Frame interval p50 / max | 155 ms / 462 ms | 8.33 ms / 53 ms |
| Page bitmaps held (max) | 248 MB | 44 MB (tiles of two pages plus backdrops) |
| Process footprint (max) | 952 MB | 164 MB |
| Load average | 17 | about 24 |

The "after" run used the build just before the last change, which moved
prefetch-margin tiles off the main thread. Its largest synchronous tile pass
was 15.9 ms, the likely cause of the 13 drops. The rerun with off-main prefetch
could not be taken: the console session locked during the measurement, which
stops display links and window capture. **Zero dropped frames is not yet
demonstrated.** Rerun the same launch once the screen is unlocked.

Before, two 124 MB page bitmaps exceed the rasterizer's 192 MB retention cap,
so the pages are evicted and re-rasterized while scrolling.

## 4. Pinch

During a pinch the v2 preview's clip view gets a `sublayerTransform` about the
top centre of the viewport (which `PreviewAnchor` keeps across the committed
zoom) and every bitmap and tile samples linearly; the zoom is committed once
when the gesture ends and the transform removed in the same pass. Accessibility
is not granted on this machine, so no real pinch could be driven;
`FLASHTEX_V2_PINCH_HOLD=<m>` applies and holds a transform for screenshots, and
`PreviewV2TileTests` checks the transform's fixed point.
