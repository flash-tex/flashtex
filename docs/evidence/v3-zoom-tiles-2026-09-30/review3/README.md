# Third review of #1287 @2cd7cfc8e: no disk writes, a raster budget, debounced redraws

The third review (NOT-READY) had one major finding. The kept page raster was a
`MAP_SHARED` mapping of an unlinked temp file, so every redraw wrote the whole
raster to the SSD. A probe measured 620 MB for 5 × 124 MB rasters. That
happened on every compile that changed a whole-drawn page, every zoom step and
every appearance toggle.

Machine: mac-m1max-a, shared with three CI runners, Low Power Mode on. Load is
recorded per run.

## 1. Purgeable memory, a budget, a debounce

- **Anonymous purgeable memory** (`DL3PageRaster`: `mach_vm_allocate` with
  `VM_FLAGS_PURGABLE`).
  - The raster is nonvolatile only while it is drawn and while tiles are cut,
    and volatile otherwise.
  - Volatile memory is not counted in the process's footprint, and the kernel
    may purge it. `cut` then returns nil and the holder draws the raster again
    (`testAPurgedRasterIsDrawnAgain`).
  - Nothing is ever written to disk.
- **Budget:** at most **2** kept rasters over all pages, least recently cut
  first out (`EngineV3RasterHolder.budget`, `testKeptRastersStayWithinTheBudget`).
- **Debounce:** while edits keep changing a zoomed page that is drawn whole,
  its current tiles stay up (stale). Its raster is drawn once the edits pause
  for **300 ms**, with the newest content, not once per keystroke-compile.
  - The same pending content shown again (a scroll, another page arriving)
    does not restart the debounce.
  - Neither does a PDF fallback swapped in at DONE: it keeps the keystroke's
    compile, so the latency is stamped when the tiles land.
  - Covered by `testEditsOnAPageDrawnWholeAreDebounced`.
- **Tile keys use form hashes, not arrival.** They are built from the page
  content, the appearance and the **hashes** of the forms the page draws. A
  form re-sent unchanged, or a fallback PDF reloaded at DONE, no longer
  redraws an identical raster; the trace showed two draws per compile before
  this.

**Measured, the raster itself** (`TileParityTests`, beamer `tile-paths` p1,
`proc_pid_rusage` `ri_diskio_byteswritten`):

| Scale | Raster | Draw | First block cut | Footprint growth while idle | Disk written |
|---|---|---|---|---|---|
| 12 px/pt | 57 MB | 13 ms | 1.1 ms | +12.6 MB (= the 12 cut tiles) | **0** |
| 16 px/pt | 101 MB | 22 ms | 1.1 ms | +16.8 MB (= the 16 cut tiles) | **0** |
| 20 px/pt | 158 MB | 33 ms | 1.2 ms | +16.9 MB (= the 16 cut tiles) | **0** |

**Typing while zoomed in the app.** `EngineV3Bench` drives 40 keystrokes, or
20 at 2 s intervals, into the editor and stamps each one up to its page's
tiles on screen.
- Release app, never activated.
- A run counts only with a pane of at least 600×600 pt (`typebench/retries.txt`).

| Document, 16 px/pt | Interval | Keystroke → tiles p50 / p95 | **Disk written** | Peak footprint | Rasters drawn (tile jobs) |
|---|---|---|---|---|---|
| beamer-madrid p1 (paths, forms, PDF fallback; drawn whole) | 300 ms | 3,923 / 6,923 ms | **0.8 MB** | 264 MB | 4 deferred → 6 jobs for 40 keys |
| beamer-madrid p1 | 2,000 ms | 1,372 / 1,992 ms | **1.9 MB** | 173 MB | |
| beamer-madrid p1, no debounce | 2,000 ms | 718 / 741 ms | 1.9 MB | 130 MB | |
| beamer-madrid p1, whole pages (untiled) | 2,000 ms | 717 / 793 ms | 1.9 MB | 302 MB | |
| dense.tex (glyphs; tiles by translation) | 300 ms | **41 / 49 ms** | 1.3 MB | 126 MB | |

What the typing runs show:
- **Disk.** The app writes 0.8–1.9 MB in a whole typing session, which is
  logging (glyph runs write the same). Before, the raster files wrote about
  100–775 MB per redraw.
- **The debounce's price.** On a whole-drawn page it adds about 650 ms per
  keystroke when compiles finish between keys. That is the 300 ms pause, plus
  the pause starting again when the page's PDF fallback is swapped in at DONE
  (beamer-madrid p1 is a fallback page), plus the draw.
- **Fast typing.** The beamer compile takes about 1.3 s here, so 300 ms
  typing outruns it. The stale tiles stay up and only a few rasters are drawn
  (6 tile jobs for 40 keys), as the review asked; each keystroke's latency is
  then mostly the compile.
- **Glyph pages** have no debounce: 41 ms p50.

## 2. Failures leave no hole

When a raster cannot be drawn (no memory), the job's tiles are asked for again
shortly. The retries are bounded, at most 8 per source with growing delays,
and the backdrop shows in the meantime
(`testTilesThatCouldNotBeDrawnAreRetried`, which uses a test hook that makes
the next draws fail). There is no file backing, so there is no disk-full case.

## 3. Host tests

`EngineV3TestHost.awaitReady` **fails** the test (XCTFail, then throws) when
the host fails to start or is not ready within 120 s. Only "no host built" and
"no TeX Live" skip.

## 4. Tests

- `EngineV3PageTilesTests.testAPDFFallbackSourceKeepsItsLightRasterAcrossAppearances`:
  - a source with `pdf != nil` gets the media box's grid;
  - its tiles are exact light and dark;
  - one raster serves both appearances (finding 6).
- `EngineV3ZoomTilesTests` checks:
  - page 2 (TikZ) in **dark** mode against the dark whole page;
  - tile exactness on the edited page after the edit step;
  - horizontal follow (finding 5).

## 5. Caret follow and forward search scroll horizontally

`follow(_:)` scrolls horizontally too when the target is outside the margin,
centring it, so the target and its flash are in view at zoom. Tested in
`EngineV3ZoomTilesTests`: a target at the right edge of page 1 that starts out
of view ends up within the clip bounds.

## 6. Nits

- A PDF-fallback raster's identity has no appearance in it
  (`EngineV3TileSource.rasterIdentity`), so a dark toggle reuses it.
- `DL3PageRaster.checkedSize` refuses a scale that is not finite and positive,
  more than 1 Mpx a side, or an overflowing `W·4·H`. `pixelSize` returns 0×0
  for those instead of trapping (`testBadScalesYieldNoRasterAndNoTrap`).

## Parity (zero tolerance)

| Sweep | Page × scale | Tiles | Differing |
|---|---|---|---|
| All 83 fixtures, 3.25–8 px/pt | 4,600 | 255,759 | **0** |
| Whole-drawn (paths, forms, images, PDF fallbacks) and stroked-rule pages at 12, 16 and 20 px/pt | 126 | 32,287 | **0** |
| The same beamer pages at 16 and 20 px/pt, the pane's IOSurfaces | 54 | 7,371 | **0** |
| Checked in: light, **dark** for every tile kind, the pane's block and ring sets, PDF fallbacks, and the highest reachable scales | — | — | **0** |
