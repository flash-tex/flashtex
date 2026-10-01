# Engine-v3 pane: zoom, pinch and 512 px tiles (P3-V3-ZOOM-TILES, 2026-09-30)

**Superseded in part by the re-review of 4308a9365** ([`rereview/README.md`](rereview/README.md)): pages drawn whole no longer take a 128 MiB scale cap. They keep one full-scale, file-backed page raster per source, so every reachable scale is full scale. The cap is now a last resort at 1 GiB.

**Update after review of 0f57f8b70:** pages that are not tiled by translation no longer draw a page-sized raster per tile job. Their memory is bounded at every reachable scale (up to about 20 px/pt for letter and 32 for beamer), and the parity sweeps now run to 20 and 32 px/pt. See [`review-memory/README.md`](review-memory/README.md). The "cut from one raster" wording below describes the head before that change.

Machine: mac-m1max-a (M1 Max, 10 cores, built-in 120 Hz XDR panel plus a
60 Hz external display, macOS 26.3.1, **Low Power Mode on**). The machine is
shared with three CI runners and other agents' builds. Every run records
`uptime` and the runner processes (`scroll/runs.txt`); load averages ran
from 23 to 776 during this work. Governing text: `docs/design/engine-v2/DESIGN.md`
§1.2, §6.2, §3 and §9. The design follows #1228 (preview-v2 tiles) and its
review.

## What changed

- **Zoom.** The pane lays out at fit-to-width × `ShellModel.previewZoom`,
  so ⌘=, ⌘-, ⌘0, ⌘9 and the HUD readout (`previewFitScale`) work in v3.
  Page origins sit on device pixels, pages wider than the pane scroll
  horizontally, and a zoom change keeps the page point at the top centre of
  the viewport in place.
- **Pinch** (`EngineV3ScrollContainer.magnify`). While the gesture runs,
  the clip view gets a Core Animation `sublayerTransform` about the
  gesture's location, and tiles sample linearly; nothing is laid out or
  drawn. The zoom is committed once, in the same transaction that removes
  the transform, and the page point under the fingers stays put. The
  handler belongs to this pane's scroll view, which fixes two findings from
  #1228's review: the pinch is not process-global, and a cancelled gesture
  commits like an ended one.
- **Tiles above 3 px/pt** (`EngineV3PageTiles`, `DL3Renderer.rasterizeTiles`).
  - A page shows 512 px tiles of its visible rect plus a 256 px prefetch
    margin, over a 2 px/pt backdrop of the whole page. Tiles more than one
    tile beyond that are dropped.
  - Tiles are drawn on `flashtex.engine-v3.tiles` in parallel
    (`concurrentPerform`, up to 8 workers). They are IOSurfaces in Core
    Animation's BGRA format, tagged sRGB, so the commit neither copies nor
    colour-converts them: a main-thread `sample` during the scroll bench
    shows no `CA::Render::copy_image`.
  - The main thread only assigns `contents`, under a generation token. A
    job for a page, scale or content the view has left is skipped before it
    draws, and its result is dropped if the view moved on.
  - After a zoom, the previous scale's tiles stay up, stretched, until the
    new visible tiles land. After an edit, the old tiles stay until they
    are replaced.
  - At tiled scales the reader thread no longer draws whole pages.
- **Perf-audit items (PR #1252, #2 2026-09-30T04:50).**
  - Only pages whose content hash changed are redrawn.
  - Tiles are drawn in parallel.
  - `updateNSView` lays out only when the page revision, the zoom or the
    width changed.
  - The representable implements `sizeThatFits`, so SwiftUI does no Auto
    Layout measuring of the pane.

## Tiles are pixel-exact (zero tolerance)

Every tile must equal the same window of the whole-page raster, which is
what the parity gate compares with Core Graphics' rendering of the engine's
PDF. #1228 found the causes of mismatches; they apply to v3 as well:

- **Glyph phase** (`tileCoordinate`). A glyph lands on the pixel and phase
  `floor((d + 0.001)·N)`. When the page's sum `d + 0.001` sits within an
  ulp of a boundary, the tile's smaller sum can round the other way. Such
  an origin is moved 1e-7 px onto the side the page took.
- **Filled rules** (`tileEdge`). A rule's edges are passed in as the page's
  single-precision device coordinates.
- **Everything else is cut from one raster.** Pages with paths, clips,
  images, forms, stroked text or stroked rules are rasterised once per tile
  job and the tiles are cut from that raster.
  - Stroked rules were first measured by translation. Stroking the
    translated line gave 1–2,000 px per affected page. Filling the stroke's
    outline with page-rounded edges was exact everywhere except one rule:
    `proof-practice` at 3.75 px/pt showed a 1-level difference along one
    pixel row, on 16 pages. Six rounding models of the centre line and
    width did not reproduce Core Graphics' stroker there.
  - For glyph-and-rule pages, the raster is **partial**. It runs from the
    page's bottom-left device origin to the rects' right and top edges, so
    nothing is translated and a tile low on the page costs part of the page.
  - Pages with paths or images always get the whole raster. A partial
    raster changed 1–2 tiles of `beamer-visuals` page 3, a page with
    shadings.
  - PDF-fallback pages are cut from the whole PDF raster.

**Sweep** (`TileParityTests.testEveryParityFixtureTilesExactly`):
`FLASHTEX_V3_TILE_SWEEP=1`, over all 83 parity fixtures
(`tools/displaylist/check_positions.py`: 83/83 exact positions), 230
pages, 3.25–8 px/pt in 0.25 steps. Every tile is compared, zero tolerance.
Cut pages are cut one tile row per call, as the pane's jobs do.

| Run | Page × scale | Tiles | Differing tiles |
|---|---|---|---|
| Final (this head) | 4,600 (1,960 by translation) | 255,759 | **0** |
| Stroked rules as translated fills (rejected) | 4,600 | 255,759 | 80 (`proof-practice`, 3.75 px/pt) |
| Partial raster for path pages too (rejected) | 4,600 | 255,759 | 7 (`beamer-visuals` p3) |
| Stroked rules stroked by translation (rejected; 4, 6, 8 px/pt only) | 690 | 44,818 | 34 |

These checked-in tests always run:

- `testTilesAreExactWindowsOfTheCheckedInPages`: `beamer-overlays`,
  `tile-text` (`min-tabular`) and `tile-paths` (`beamer-madrid`), 440
  page × scale, 8,267 tiles, 0 differing.
- `testSurfaceTilesMatchTheWholePage`: the IOSurfaces the pane installs.
- `testTileCoordinateKeepsThePagesPhase`: #1228's reviewed value.

`EngineV3ZoomTilesTests` checks the real pane in a window that is never
shown, with a live `flashtex-host`:

- At zoom 4 the pane tiles, and every installed tile equals the
  whole-page raster.
- Fewer tiles than the page's are held, and the bitmap bytes held are less
  than one whole page.
- A pinch keeps the page point under its anchor.
- An edit while zoomed shows the new content's tiles.
- Back at fit, no tiles are held.

**Parity against the exported PDF** (the same sweep, whole reassembled
pages against `CGContext.drawPDFPage` of the engine's PDF):

- 4,270 of 4,400 page × scale are identical.
- In the other 130, the whole-page `DL3Renderer` raster already differs
  from the PDF raster at that scale: median 282 px, a glyph or two. The
  tiles equal that raster exactly, so this is not a tile effect. The
  existing gate pins 1, 2 and 4 px/pt; these rows are at other tiled
  scales.
- This looks like the known sp-rounding residual
  (`PreviewParityTests.floorPixels`) showing at more scales. It belongs to
  the renderer/display-list lane.
- Against PDFKit, the same 130 are "further from PDFKit than Core Graphics
  is", because Core Graphics and PDFKit agree on those pages.

## 8 px/pt: memory and time

`FLASHTEX_V3_TILE_BENCH=<fixture>:<page> swift test -c release -Xswiftc
-enable-testing --filter TileParityTests/testTileBenchmark`. The model is a
1100×900 pt pane at 2× (2200×1800 px plus the 256 px margin), scrolled to
the top of the page. Each figure is the median of 9 runs; load is in the
JSON (1-minute load 99 and 138).

| | `twelvept-plain` p1 (770 glyphs, tiles by translation) | `thesis-chapter` p1 (2,184 glyphs, stroked rules: cut) |
|---|---|---|
| Whole page (the pane without tiles) | 14.9 ms, **124.1 MB** | 42.6 ms, **128.5 MB** |
| Viewport tiles (25 × 512 px) | **4.1 ms** parallel (5.4 ms sequential), **26.2 MB** | **29.4 ms** parallel (147 ms one call per tile), **26.2 MB**; each job draws a partial page raster |
| Scroll step (a new row of 5 tiles) | 0.86 ms | 8.3 ms (a partial raster covering the rows down to the page bottom) |
| One tile | 0.28 ms | 5.6 ms |
| Backdrop at 2 px/pt | 1.7 ms, 7.9 MB | 2.0 ms, 8.2 MB |

`tilebench-*.json`.

## 120 Hz scroll with the pane on engine v3

Setup:

- `scripts/bench.sh`: the release app is never activated
  (`FLASHTEX_NO_ACTIVATE=1`, `FLASHTEX_KEYCHAIN_OFF=1`,
  `FLASHTEX_WINDOW_FRAME=40,40,1440,900`) and opens `dense.tex`.
- `FLASHTEX_V3_PPP=8` puts the pages at exactly 8 px/pt.
- `FLASHTEX_V3_SCROLL_BENCH=8` scrolls at 2,400 pt/s for 8 s, bouncing
  between the ends, from the 120 Hz screen's display link. The window is
  shown with `orderFrontRegardless`, which neither activates the app nor
  takes focus, and a watchdog guards the run.
- A dropped frame is a callback interval over 1.5 × 8.33 ms.
- `FLASHTEX_V3_TILE_THRESHOLD=1000` gives the "whole pages" runs.
- Only the bench's own PID is killed. `caffeinate -w` keeps the display on.

| Document, mode | Frames / dropped | Interval p50 / p99 / max | Bitmaps held (max) | Process footprint (max) | Tile jobs (off main), max job | Frames with a visible tile missing | Load (1 min) |
|---|---|---|---|---|---|---|---|
| dense.tex (2 pages), tiles, run 4 | 961 / **0** | 8.33 / 8.33 / 8.3 ms | **41 MB** | **108 MB** | 72 jobs (288 tiles), 3.5 ms | 0 | 28 |
| dense.tex, tiles, run 3 | 960 / **0** | 8.33 / 8.33 / 8.3 ms | 41 MB | 108 MB | 72, 4.0 ms | 0 | 395 |
| dense.tex, tiles, run 2 | 958 / 2 | 8.33 / 8.33 / 16.7 ms | 41 MB | 108 MB | 72, 4.8 ms | 0 | 424 |
| dense.tex, whole pages, run 2 | 960 / 0 | 8.33 / 8.33 / 8.3 ms | 248 MB | 314 MB | — | — | 23 |
| dense.tex, whole pages, run 1 | 961 / 0 | 8.33 / 8.33 / 8.3 ms | 248 MB | 314 MB | — | — | 411 |
| dense10.tex (6 pages), tiles | 961 / **0** | 8.33 / 8.33 / 8.3 ms | 41 MB | 109 MB | 77, 4.0 ms | 0 | 324 |
| dense10.tex, whole pages | 959 / 1 | 8.33 / 8.33 / 16.7 ms | 248 MB | 314 MB | — | — | 268 |
| dense-rules.tex (stroked rules: cut, partial raster), tiles | 960 / **0** | 8.33 / 8.33 / 8.3 ms | 41 MB | 159 MB | 76, 9.7 ms | 0 | 51 |
| dense-rules.tex, whole pages | 961 / 0 | 8.33 / 8.33 / 8.3 ms | 248 MB | 315 MB | — | — | 39 |

Notes on the table:

- Run 1 of the tiled bench (`tiles8-run1.json`: 50 dropped, 145–168 ms
  gaps at t ≈ 1.0–1.5 s) had `sample` attached to the process at that
  moment. Attaching suspends the process, so the run is kept only for its
  sample.
- The sample shows no `copy_image`: the IOSurface tiles are not converted
  on the main thread, which is #1228's colour-space finding, avoided here.
- The v3 pane was already off-main before this change, so the whole-page
  mode also holds 120 Hz on these documents. The gains from tiles are:
  - **memory**: 41 MB of bitmaps against 248 MB; footprint 108 MB against
    314 MB;
  - **time to a sharp viewport**: 4 ms of tiles against 15–20 ms for a whole
    page, and less for the scroll row that enters.

**Keystroke → pixels at 8 px/pt** (`EngineV3Bench`, 40 keys at 300 ms,
`dense.tex`, `typebench/*.json`, load 30–70):

| Mode | p50 | p95 |
|---|---|---|
| Tiles | 30.3 ms | 32.7 ms |
| Whole pages | 29.9 ms | 32.9 ms |

The compile dominates both, and tiles cost nothing measurable here.

## Screenshots

- `tiled-8ppt.png`: the pane at 8 px/pt (crop), showing the top of page 1
  sharp from tiles.
- `pinch-hold-2x.png`: `FLASHTEX_V3_PINCH_HOLD=2`, the transform held at 2×
  over the fit-to-width bitmaps, filtered linearly.

## Reproduce

```
cargo build --release -p flashtex-engine --bin flashtex-host --bin flashtex-initex
# pdflatex.fmt for the engine, then the parity fixtures:
FLASHTEX_POOL=$PWD/crates/flashtex-engine/pdftex.pool python3 tools/displaylist/check_positions.py \
    --engine target/release/flashtex-initex --formats FMTDIR -j 4
cd apps/mac
FLASHTEX_V3_TILE_SWEEP=1 FLASHTEX_V3_TILE_SWEEP_OUT=/tmp/sweep.json \
  swift test -c release -Xswiftc -enable-testing --filter 'TileParityTests|EngineV3ZoomTilesTests'
FLASHTEX_V3_TILE_BENCH=real-world__thesis-chapter:0 swift test -c release -Xswiftc -enable-testing \
  --filter TileParityTests/testTileBenchmark
bash docs/evidence/v3-zoom-tiles-2026-09-30/scripts/bench.sh <label> 8 3 8     # tiles
bash docs/evidence/v3-zoom-tiles-2026-09-30/scripts/bench.sh <label> 8 1000 8  # whole pages
```

The scripts carry this machine's scratch paths. Edit `S` and `W` at their top.
