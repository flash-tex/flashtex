# Re-review of #1287 @4308a9365: full-scale tiles on every page, with a kept page raster

The re-review (NOT-READY) found that the scale cap from the previous round
broke the requirement: tiles must be pixel-identical to the whole-page render
**at the requested zoom**. Pages with paths, images, clips, forms, stroked text
or a PDF fallback were shown stretched above about 8.3 px/pt (letter) or
17.9 px/pt (beamer).

Machine: mac-m1max-a, shared with three CI runners, Low Power Mode on. Load is
recorded per run.

## 1. One full-scale page raster per source, kept and reused (`DL3PageRaster`)

A page drawn whole now has **one** raster per source, where a source is one
page content at one scale. It is drawn at full scale on the tile queue by the
first job that needs it (`EngineV3RasterHolder`). Every later job of that
source cuts its tiles from it: the visible block, the prefetch ring and every
scroll step.

It is freed:

- before the next source's raster is drawn (content or zoom change);
- when the page's keep set becomes empty (the page scrolled away);
- on `removeAll` and teardown.

The pixels live in a **file-backed mapping**: `MAP_SHARED` over a file in
`$TMPDIR/flashtex-page-rasters`, unlinked as soon as it is created.

- `msync` starts the write-back as soon as the page is drawn. After that the
  pages are clean file cache, which the kernel evicts under pressure and reads
  back on demand.
- The raster is not part of the process's physical footprint.
- A crash leaves no file behind.

The scale cap stays only as a last resort, at **1 GiB**: a letter page at
23.5 px/pt, a 4:3 beamer frame at about 52 px/pt. The pane reaches about
19.3 px/pt on a letter page in a 14-inch MacBook Pro, so every reachable scale
on this machine is full scale.

This also resolves the review's minors 4 and 5:

- **Minor 4: redraw per job and zoom step.** Each source is drawn once. In
  `EngineV3PageTilesTests.testAPathPageKeepsOneRasterPerSource`, the visible
  job, the prefetch job and the scroll-step jobs share 1 raster, and a new
  scale draws exactly 1 more. The pane test with a TikZ page asserts
  `rastersDrawn == 1` after scroll steps.
- **Minor 5: non-seamless stretched tiles.** No tiles are stretched at any
  reachable scale. Every installed tile is a 1:1 window of the full-scale
  raster: its source's `pixelsPerPoint` equals the screen's, which
  `EngineV3ZoomTilesTests` asserts for both pages.

**Measured, `DL3PageRaster`** (TileParityTests; `tile-paths` p1 is beamer
with shadings; the letter page is `tile-text` drawn whole):

| Page, scale | Raster (file-backed) | Draw | First 4×4 block cut | Footprint growth |
|---|---|---|---|---|
| beamer, 12 px/pt | 57 MB | 31–64 ms | 1.0 ms | +12.6 MB (= the 12 tiles' surfaces) |
| beamer, 16 px/pt | 101 MB | 49–121 ms | 1.3 ms | +16.8–17.1 MB (= the 16 tiles) |
| beamer, 20 px/pt | 158 MB | 97–186 ms | 1.2 ms | +17.3–17.7 MB (= the 16 tiles) |
| letter, 19.3 px/pt (the most the pane reaches here) | 722 MB | 201 ms | 4.7 ms | +3.9 MB |
| letter, 23.5 px/pt (**the 1 GiB bound**) | 1,074 MB | 712 ms | 1.4 ms | +2.2 MB |

The last two rows come from the opt-in `testPageRasterCostAtTheLastResortBound`
(`FLASHTEX_V3_RASTER_BENCH=1`). They are the floor for the cap: at 1 GiB the
draw takes 0.7 s, while the footprint barely moves.

## In the app: peak footprint, 120 Hz, time to first tile

Setup:

- `scripts/benches3.sh` and `benches4.sh`: release app, never activated,
  710×846 pt pane, 2,400 pt/s for 8 s at 120 Hz.
- Runs whose recorded pane was below 600×600 pt were discarded
  (`scroll/runs.txt`).

Columns: **Kept raster** is the largest `DL3PageRaster`, which is file cache,
not footprint. **Clipped** is the largest resident clipped raster. **Missing**
counts frames in which a visible tile had not landed yet.

| Document (page kind) | px/pt | Frames / dropped | Bitmaps | Kept raster | Clipped | **Footprint (peak)** | Missing | Load |
|---|---|---|---|---|---|---|---|---|
| `beamer-madrid` (paths, forms) | 8 | 960 / 1 | 37 MB | 25 MB | — | **117 MB** | 0 | 10 |
| `beamer-madrid` | 16 | 954 / 7 | 34 MB | 101 MB | — | **144 MB** | 7 | 7 |
| `beamer-madrid`, run 2 | 16 | 958 / 3 | 34 MB | 101 MB | — | **143 MB** | 9 | 5 |
| `beamer-madrid` | 20 | 959 / 2 | 33 MB | 158 MB | — | **164 MB** | 27 | 7 |
| `beamer-visuals` (forms, PDF fallbacks) | 8 | 954 / 6 | 37 MB | 25 MB | — | **120 MB** | 0 | 9 |
| `beamer-visuals` | 16 | 957 / 4 | 34 MB | 101 MB | — | **149 MB** | 6 | 6 |
| `beamer-visuals` | 20 | 956 / 4 | 33 MB | 158 MB | — | **178 MB** | 8 | 6 |
| `beamer-visuals`, run 2 | 20 | 959 / 1 | 33 MB | 158 MB | — | **178 MB** | 9 | 6 |
| `min-tabular` (table rules: clipped) | 8 / 16 / 20 | 960 / 0 each | 37 MB | — | 10 / 12 / 13 MB | **116–119 MB** | 0 | 5–10 |
| `dense.tex` (glyphs: by translation) | 8 / 16 / 20 | 954–958 / 2–6 | 45 MB | — | — | **117–118 MB** | 0 | 6–8 |

For comparison, the previous round with the 128 MiB cap had `beamer-madrid`
at 243 MB (16 px/pt) and 280 MB (20 px/pt), and the version before it drew a
page-sized raster per job.

**Time to first tile.** A path page's first tiles wait for its raster to be
drawn: 70–130 ms per page at 16–20 px/pt in the app (largest job). While that
page scrolls in, a few frames show its backdrop ("Missing": 6–27 frames of
about 960). Later jobs of the page only cut, in about 1 ms. From the first
queued job to the first tiles on screen:

| Document | px/pt | First tile p50 / max (n sources) | Largest tile job |
|---|---|---|---|
| `beamer-madrid` (every page drawn whole) | 16 | 87 / 113 ms (8) | 112 ms |
| `beamer-madrid` | 20 | 106 / 212 ms (6) | 211 ms |
| `beamer-visuals` (some pages drawn whole) | 16 | 4 / 101 ms (8) | 100 ms |
| `beamer-visuals` | 20 | 5 / 126 ms (6) | 125 ms |
| `dense.tex` (glyphs) | 16 / 20 | 3 / 5 ms, 5 / 5 ms | 6–8 ms |

(`scroll/r4-*.json`, load 8–12. The `r4-visuals-16` run in `runs.txt` drew no
kept raster during its timed window and is not used here; its repeat,
`r4-visuals-16-b`, is.)

## Merge with main: dark preview, forward and reverse search (#1254 follow-ups)

Main gained the v3 pane's dark preview (e8897b21f) and its forward/reverse
search with follow-the-edit (549549b3b). The merge conflicted in
`EngineV3Preview.swift` and `DL3Renderer.swift`, and kept both sides:

- **`drawStream`** takes `appearance` and `tile`. Dark text's fill switch
  comes after tile culling, so a culled glyph changes no state.
- **`EngineV3ScrollView`** takes `zoom`, `follow` and `dark`. `updateNSView`
  sets the appearance, lays out only on changed inputs, then follows.
- **`resized`, `follow`, `flash` and `mouseDown`** (reverse search) are kept
  as main wrote them, on top of the zoomed layout.
- **The raster plan** keeps `appearance` and stays empty at tiled scales.

**Tiles render in the pane's appearance too.**
- Main's `contentKey` already puts the appearance in each tile source's key,
  so a toggle makes a new source.
- Display-list tiles are drawn dark with the same full-pixel ground as
  `rasterizeToSurface(…, appearance: .dark)`, in all three kinds: translated,
  clipped, and kept raster.
- PDF-fallback tiles get the same per-pixel Core Image pass (invert, then
  rotate hue by half a turn), applied tile by tile.

These tests check that every kind equals the dark whole page:
- `TileParityTests.testDarkTilesEqualTheDarkWholePage` (4 and 12 px/pt);
- `EngineV3PageTilesTests.testDarkTilesOnAPathPage`;
- the pane test, which compares in the pane's appearance.

## 2. CI and the helper script

The `scripts/ci/build-helpers.sh` change is **reverted**. That script is also
used by `release.yml`, and the change would have made the GPL engine a
required release helper.

`EngineV3OpenTests`, `EngineV3InstanceTests` and `EngineV3ZoomTilesTests` now
skip cleanly (`EngineV3TestHost`) in three cases:

- no host is built;
- no TeX Live answers `kpsewhich pdflatex.ini`;
- the host fails to start, or isn't ready within 120 s.

Before, they failed after 90 s. Running them in CI needs a workflow step,
which is Commander-owned.

## 3–6. Minors

- **3. Re-request after a skip.** Tiles skipped while out of the keep set are
  requested again when the skip is reported: the callback updates with the
  last visible rect. Covered by
  `EngineV3PageTilesTests.testSkippedTilesAreRequestedAgainWhenWantedAgain`,
  which is deterministic (it holds the tile queue).
- **4. Tests.**
  - The pane test's document has a table (stroked rules, clipped) on page 1
    and a TikZ drawing (paths, kept raster) on page 2. Both pages' tiles are
    checked exact at full scale.
  - Skip and cancel behaviour is asserted.
  - The memory test's single-tile and 5×4 sets are parity-checked.
  - Clipped parity is checked on the pane's block and ring sets at three
    viewports and 8/16/20 px/pt.
  - The PDF-fallback raster is checked against `rasterize(pdfPage:)`.
- **5. Teardown.** `EngineV3PageTiles.deinit` bumps the generation and frees
  the raster. `EngineV3Session.stop()` drops every page's tiles (`dropAllTiles`).
- **6. Nits.**
  - The `DL3Tiles.swift` header is rewritten.
  - The `clipExact` claim now names what was measured.
  - A PDF fallback's tile grid comes from its media box, the same size as its
    raster (`DL3PageRaster.gridSize`).
  - `measureResidency` is read under a lock.

## Parity (zero tolerance, every tile against the whole-page raster)

| Sweep | Page × scale | Tiles | Differing |
|---|---|---|---|
| All 83 parity fixtures, 3.25–8 px/pt in 0.25 steps (`sweep-all-3.25-8.json`) | 4,600 | 255,759 | **0** |
| Path, form, image and PDF-fallback pages (`beamer-default`, `beamer-madrid`, `beamer-visuals`, `beamer-blocks-columns`) with stroked-rule pages (`min-tabular`, `thesis-chapter`, `lab-report`, `hw1`) and a glyph page (`twelvept-plain`) at **12, 16 and 20 px/pt** (`sweep-12-16-20.json`) | 126 | 32,287 | **0** |
| The same beamer pages at 16 and 20 px/pt, the pane's IOSurface tiles (`sweep-surfaces-16-20.json`) | 54 | 7,371 | **0** |
| Checked in, every run: `tile-text`, `tile-paths` and `beamer-overlays` at 3.25–8 px/pt | 440 | 8,267 | **0** |

Also checked in, every run:

- **Highest reachable scales.** `tile-text` at 12, 16 and 20 px/pt;
  `tile-paths` at 16, 24 and 32; `beamer-overlays` at 20 and 32.
- **The pane's rect sets** (block, ring, single tile and 5×4 block).
- **PDF fallback.** `DL3PageRaster(pdfPage:)` against `rasterize(pdfPage:)`.
