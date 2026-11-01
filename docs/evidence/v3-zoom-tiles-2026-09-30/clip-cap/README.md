# Clip growth cap and page-raster fallback (review follow-ups of #1287)

**Problem (MEDIUM).** `clearClip` grows a page-context clip until each side
is clear of rule edges. Inside a page-wide cluster of rules with edges less
than about 4 px apart, or with a non-finite rule, the clip grew to the whole
page. That is about 500 MB of scratch per tile job at 16 px/pt, with up to 8
workers at once.

**Fix (`DL3Tiles.swift`).**

- `clearClip` moves each side at most `clipGrowthMax` (128) px. A 512 px
  tile's clip is then at most 768 px a side (2.25× its area). Its clipped
  raster backs at most `clipResidentBound` bytes, 24 MiB with 16 KB pages.
- `tileRoutes` gives each tile one of three routes: translation, a clip, or
  the page's raster. A tile whose clip would grow past the cap is cut from
  the page's `DL3PageRaster`. In the pane that is the source's kept raster
  (`EngineV3TileSource.render` passes `rasterizeTiles(pageRaster:)`), so it
  is drawn once per source, not once per job.
- **Size limit on the fallback (review of #1395).** A translatable or
  clip-exact page's scale is not capped (`tileScale` caps only pages drawn
  whole), so the fallback raster is used only while it fits
  `fallbackRasterMaxBytes` (`wholeRasterMaxBytes`, 1 GiB; `pageRasterFits`).
  Beyond that limit:
  - A tile keeps its uncapped clip: exact, and backed only in the clip's
    rows. A non-finite rule's clip is the whole page.
  - An unmapped clip leaves the tile nil. The pane retries it a bounded
    number of times (8) and never allocates the raster.
  - Measured at 16 px/pt with the limit lowered: the uncapped band clip
    backs 99.5 MB, and no page raster is drawn.
- **Uncapped clips are not drawn in parallel (delta review of #1395).**
  - Over the limit, the per-tile workers skip clips grown past
    `clipGrowthMax`. These clips are drawn after the parallel pass, one
    clipped raster per distinct clip at a time, and every tile sharing a
    clip is cut from that raster.
  - Peak mapped span (`peakLiveCutSpanBytes`): at most one uncapped clip for
    several band tiles at 16 px/pt, and exactly one page-sized clip for all
    tiles of a page with a rule that overflows at its scale.
  - With the parallel drawing restored, both assertions fail.
- **Release on leaving the keep set.** The pane now frees a translatable
  page's fallback raster when its tiles leave the keep set, as it already
  did for pages drawn whole. These rasters share the 2-slot LRU with pages
  drawn whole (`testDenseBandFallbackRastersShareTheKeptRasterBudget`).
- A rule whose RULE_GEOMETRY is not finite makes the page neither
  translatable nor clip-exact, so the pane draws it whole. A rule that
  overflows only at a given scale sends every tile at that scale to the
  page raster.
- A clipped raster that cannot be mapped no longer returns nil. Its tiles
  are cut from the page raster. Mapping only the clip's rows was rejected:
  it shifts the context's device origin, which is a translation and not
  exact.
- `rasterize(.dark)` now lays the dark ground over every pixel, as
  `rasterizeToSurface` does. Before, the partial last column and row were
  part-white. RGBA dark tiles therefore differed from it at fractional page
  edges (max delta 74–176), which is why the ORIGINS test had skipped dark
  RGBA; it now checks it.

**Tests.** `TileParityClipCapTests` and
`EngineV3PageTilesTests.testADenseRuleBandIsCutFromTheKeptRaster`. All are
zero tolerance, light and dark, IOSurface (all tiles at once and one by one)
and RGBA, at 3.25–16 px/pt.

| Case | Tiles compared | Differing |
|---|---|---|
| Dense clusters: page-wide mesh and a band, filled (translated) and half stroked (clip-exact) | 1,104 | 0 |
| Non-finite rules (NaN, ±∞, overflow at 1e308), at 3.25 / 8 / 16 px/pt | all | 0 |
| Stroked rules 0–1.2 px off every tile boundary on a clip-exact page (164 of 214 clips grown) | 1,284 | 0 |
| Fast path: 400 rules away from tile boundaries, alone and over `beamer-overlays` p1 | 2,058 | 0 |
| Unmapped clip rasters (forced), translated and `tile-text` | all | 0 |
| ORIGINS+RULE_GEOMETRY, now with dark RGBA | 4,248 | 0 |

- **Fast path.** On the synthetic page 85 of 107 tiles are drawn by
  translation. The 22 others are only the tiles that the two deliberate
  on-boundary rules reach.
- **Checked-in pages.** The cap moves no tile of the checked-in fixtures
  onto a page raster: 3,374 translated, 4,038 clipped and 0 page-raster at
  3.25–20 px/pt.

**Memory, letter page at 16 px/pt.** The test band has rules 3.1 px apart
across the middle fifth of the page.

- **Before.** A tile in the band grew its clip to 9792×2540 px, a
  99.5 MB window of the 496 MB page raster. A page-wide mesh grows it to
  the whole page.
- **Now.** No clipped raster backs more than `clipResidentBound`
  (25,165,824 bytes). The band's tiles come from one page raster per job.
  In the pane that is the one kept, purgeable raster per source.

**Viewport bench.** `testTileBenchmark`, release, median of 9, 25 viewport
tiles. Each run's load average is recorded in its JSON.

| Page | Viewport before → after (ms) | Scroll row | One tile |
|---|---|---|---|
| beamer-overlays p1–3 at 8 px/pt | 6.1–6.8 → 4.6–4.7 | 0.8–1.1 → 0.6–0.7 | 0.2 (same) |
| beamer-overlays p1–3 at 16 px/pt | 8.7–11.8 → 5.7–7.9 | 1.8–2.7 → 1.2–1.4 | 0.2 (same) |
| tile-text (clip-exact) at 8 / 16 px/pt | 19.1 / 15.2 → 8.0 / 10.4 | 3.9 / 3.2 → 1.7 / 2.1 | 1.8 → 1.0 |

- Before was run at load 8.4–10.7 and after at load 6.3–16.3. The
  differences are within machine noise: routing adds no measurable cost, and
  no bench page changes route.
- An earlier after-run at load ~85 (11–30 ms) was discarded.
- The numbers match #1287's `rule-edges/bench-after` (4.7–4.9 and 6.3–6.6 ms).
