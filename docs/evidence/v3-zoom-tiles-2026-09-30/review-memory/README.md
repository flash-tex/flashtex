# Review of #1287 @0f57f8b70: bounded memory at every reachable scale

**Superseded in part by the re-review of 4308a9365** ([`rereview/README.md`](../rereview/README.md)): pages drawn whole no longer take a 128 MiB scale cap. They keep one full-scale, file-backed page raster per source, so every reachable scale is full scale. The cap is now a last resort at 1 GiB.

The review (NOT-READY) found one major problem. A page that is not tiled by
translation cut its tiles from a raster about the size of the whole page. That
covers pages with stroked rules, paths, images, forms or a PDF fallback: 132 of
230 fixture pages. The raster was rebuilt for every tile job: 470 MB at
16 px/pt and 743 MB at 20 px/pt on a letter page with table rules, and
101–158 MB on a beamer page.

The pane reaches fit × zoom (≤ 4) × backing (2) px/pt:

- about 19 px/pt for a letter page;
- about 32 px/pt for a 4:3 beamer frame in a 1,512 pt pane, the full width
  of a 14-inch MacBook Pro.

Machine: mac-m1max-a, shared with three CI runners, Low Power Mode on. Every
run records its load and runner state (`scroll/runs.txt`).

## What changed

Pages now tile in one of three ways.

1. **Glyphs and filled rules only: by translation, unchanged.** Every tile is
   drawn on its own and only its own pixels are allocated.
2. **Glyphs and rules, stroked rules included** (`DL3Renderer.clipExact`):
   **a clipped page raster.**
   - The tile job draws with the page's own context, with the same device
     origin and nothing translated, over an anonymous `mmap` the size of the
     page.
   - It clips to the requested tiles (pixel-aligned rects) and fills only
     them.
   - The system backs only the pages of memory that are written. The resident
     size is the tiles' rows rounded to 16 KB pages, **whatever the scale**
     (`cut-raster-residency.txt`, measured with `mincore`, tile-text p1, a
     letter page with table rules):

| Requested | 8 px/pt | 16 px/pt | 20 px/pt | Before (page raster) |
|---|---|---|---|---|
| One 512 px tile, top right | 9.4 MB | 9.3 MB | 9.4 MB | 124 / 496 / 776 MB |
| Viewport, 5×4 tiles | 40 MB | 55 MB | 58 MB | 124 / 496 / 776 MB |

   A pixel-aligned clip changes no pixel inside it on these pages. The sweeps
   below cover them up to 20 px/pt, and the checked-in `tile-text` up to
   20 px/pt.
3. **Paths, clips, images, forms, stroked text, and PDF-fallback pages: a
   whole-page raster at a capped scale.**
   - The clip is **not** exact on these pages. On `tile-paths` (beamer with
     shadings) it changed 1–17 px of one tile per page at 4–7.75 px/pt, and
     clip margins did not fix it (`clip-margin-probe.txt`):

     | Clip margin | Differing tiles (8,267 compared) |
     |---|---|
     | 0 px | 39 |
     | 4 px | 32 |
     | 16 px | 11 |
     | 64 px | 10 |
     | 256 px | 5 |

   - So these pages still draw the whole page. Their tile scale is capped
     (`DL3Renderer.tileScale`) so that the raster stays at most
     `wholeRasterMaxBytes` = 128 MiB. That is a letter page at 8 px/pt, or a
     4:3 beamer frame at 17.9 px/pt.
   - Above the cap, the tiles are stretched on screen with linear filtering,
     as the backdrop is. They remain exact windows of the page raster at
     their own scale.

**Minor findings, also fixed:**

- **Queued jobs are skipped when their tiles are no longer wanted.** Each
  page's tile generation now holds its keep set (the viewport plus the
  margin plus one tile), guarded by a lock. A job draws only tiles still in
  it, so a fast scroll leaves no backlog of drawing behind it.
- **Dropped page views cancel their jobs.** Dropping a page view calls
  `tiles.removeAll()`, which bumps the generation.
- **CI runs the pane test.** `scripts/ci/build-helpers.sh` now builds
  `flashtex-host` (`FLASHTEX_HOST`), so the Mac job no longer silently skips
  `EngineV3ZoomTilesTests` and the other engine-v3 integration tests. Only the
  helper script changed; the workflow file is untouched.

## Parity (zero tolerance, every tile against the whole-page raster)

| Sweep | Page × scale | Tiles | Differing |
|---|---|---|---|
| All 83 parity fixtures, 3.25–8 px/pt in 0.25 steps | 4,600 | 255,759 | **0** |
| 9 fixtures at 12, 16 and 20 px/pt: `min-tabular`, `thesis-chapter`, `listings-manual`, `lab-report`, `hw1`, `twelvept-plain`, `min-hrulefill` (stroked rules, clipped), `beamer-madrid`, `beamer-visuals` (paths, whole) | 99 | 34,035 | **0** |
| Checked in (`testTilesAreExactWindowsOfTheCheckedInPages`), 3.25–8 px/pt | 440 | 8,267 | **0** |
| Checked in (`testTilesAreExactAtTheHighestReachableScales`): `tile-text` at 12, 16 and 20; `tile-paths` at 16, 24 and 32; `beamer-overlays` at 20 and 32 | — | — | **0** |

Against the engine's PDF, 4,270 of 4,400 page × scale match Core Graphics.
The other 130 are unchanged from before: the whole-page `DL3Renderer` raster
already differs from the PDF raster at those scales, and the tiles equal the
page raster.

## In the app: peak memory and 120 Hz at 8, 16 and 20 px/pt

Setup:

- `scripts/benches.sh` drives release app runs that are never activated.
  `FLASHTEX_V3_PPP` fixes the scale.
- The pane is 710×846 pt: the split positions are passed as launch
  arguments, and the bench hides the Problems panel.
- The bench scrolls at 2,400 pt/s for 8 s from the 120 Hz screen's display
  link.
- A run counts only if its recorded pane is at least 600×600 pt. The window's
  saved layout varies between launches, and 5 attempts of beamer-8 were
  discarded (`scroll/runs.txt`).

Columns: **Bitmaps** is tiles plus backdrops held at most. **Cut raster** is
the largest resident raster of a tile job. **Footprint** is the process's
peak physical footprint.

| Page (kind) | px/pt | Frames / dropped | Max interval | Bitmaps | Cut raster | Footprint | Tile jobs (max) | Load |
|---|---|---|---|---|---|---|---|---|
| `min-tabular` (table rules: clipped) | 8 | 960 / 0 | 8.3 ms | 37 MB | 10 MB | 119 MB | 61 (11.6 ms) | 51 |
| `min-tabular` | 16 | 960 / **0** | 8.3 ms | 37 MB | 12 MB | 118 MB | 70 (17.9 ms) | 42 |
| `min-tabular`, run 2 | 16 | 939 / 21 | 88 ms | 37 MB | 12 MB | 137 MB | 70 (19.7 ms) | 31 |
| `min-tabular` | 20 | 960 / 0 | 8.3 ms | 37 MB | 13 MB | 118 MB | 72 (12.9 ms) | 28 |
| `beamer-madrid` (paths: whole, capped) | 8 | 960 / 0 | 8.4 ms | 37 MB | 25 MB | 141 MB | 82 (18.0 ms) | 57 |
| `beamer-madrid` | 16 | 961 / **0** | 8.3 ms | 34 MB | 101 MB | 243 MB | 77 (49.5 ms) | 35 |
| `beamer-madrid`, run 2 | 16 | 960 / **0** | 8.3 ms | 34 MB | 101 MB | 241 MB | 77 (36.3 ms) | 37 |
| `beamer-madrid` (capped at 17.9) | 20 | 960 / 0 | 8.3 ms | 22 MB | 134 MB (the cap) | 280 MB | 68 (48.8 ms) | 22 |
| `dense.tex` (glyphs: by translation) | 8 | 960 / 0 | 8.4 ms | 45 MB | — | 120 MB | 72 (4.4 ms) | 51 |
| `dense.tex` | 16 | 961 / 0 | 8.3 ms | 45 MB | — | 117 MB | 74 (4.8 ms) | 29 |
| `dense.tex` | 20 | 960 / 0 | 8.3 ms | 45 MB | — | 128 MB | 74 (4.0 ms) | 26 |
| `dense-rules.tex`, 6 pages (stroked rules: clipped) | 16 | 953 / 7 | 42.5 ms | 45 MB | 12 MB | 145 MB | 76 (18.6 ms) | 33 |
| `dense-rules.tex`, run 2 | 16 | 961 / **0** | 8.3 ms | 45 MB | 12 MB | 128 MB | 76 (19.0 ms) | 32 |

What the table shows:

- **Tile coverage.** No frame had a visible tile missing in any of these runs.
- **Memory on clipped pages** (table and stroked-rule pages) is flat in
  scale: the cut raster is 10–13 MB and the footprint 118–145 MB. Before,
  it was up to 776 MB transient at 20 px/pt.
- **Memory on path pages** is bounded by the cap: 101 MB at 16 px/pt and
  134 MB at the cap. Tile jobs run one at a time on a serial queue, so at
  most one such raster exists.
- **Dropped frames.**
  - Two runs dropped frames: `min-tabular` run 2 (21) and `dense-rules` run 1
    (7).
  - In both, the drops were long main-thread passes (up to 88 ms) while no
    visible tile was missing, the same as the other load-sensitive hitches in
    these benches.
  - The other run of each of those documents dropped 0.
