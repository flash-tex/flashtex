# Beamer decks in the Mac app, engine-v3 preview (lane BEAMER-V3, step 4, 2026-10-03)

This step opened beamer decks in the Mac app with the engine-v3 preview (`FLASHTEX_ENGINE_V3=1`) and compared what the pane drew with Core Graphics' rendering of the PDF. The decks were the 13 BEAMER-V3 corpus decks and the committed `fixtures/real-world/beamer-*` fixtures. The app ran on copies in a scratch directory; no original was opened.

Machine: mac-m1max-a (M1 Max), shared with other lanes. Load averages were 7–94 during these runs (each `capture.json` records its own), so times here are upper bounds.

## How the app was driven

The app was built with `apps/mac/scripts/make-app.sh` from `origin/main` e908d4ad5 plus this lane's four branches, with `flashtex-host` from the same tree. Each run copied the bundle under its own bundle id (`tech.jay3332.flashtex.mac.bva`), so it neither read nor wrote the preferences of the owner's running FlashTeX: window frame, split positions, preview zoom. It was launched with `open -n --env ...` and environment hooks only, never Accessibility:

- `FLASHTEX_OPEN`, `FLASHTEX_NO_ACTIVATE=1`, `FLASHTEX_KEYCHAIN_OFF=1`, `FLASHTEX_V3_CACHE`
- `FLASHTEX_V3_CAPTURE_*`, including this lane's `_TILES`, `_EDIT` and `_NARROW_PREVIEW` (`EngineV3PageCapture.swift`)
- `FLASHTEX_V3_PPP=4.25` for the tile runs
- `-NSRequiresAquaSystemAppearance YES`, for the light (pixel-exact) preview

Two things about the launch matter for reading the results:

- `FLASHTEX_KEYCHAIN_OFF=1` is needed: without it a freshly ad-hoc-signed build blocks at launch on a Keychain prompt in `ConversionCredential`, before any window exists.
- The window manager on this Mac (AeroSpace) tiles new windows to half the screen, 900 pt wide. That is below the narrow-layout width, so `FLASHTEX_V3_CAPTURE_NARROW_PREVIEW=1` shows the preview column instead of the editor. The pane was 729 pt wide, which gives a fit-width scale of 2.86 px/pt for a 4:3 frame. The earlier `FLASHTEX_WINDOW_FRAME` hook loses to the tiling: the window alternated between 1,500 and 900 pt, and every 900 pt pass collapsed the preview column to 4 pt. That is the "collapsed pane" INFDESC-APP saw but could not explain.

The reference is pdflatex's PDF (MacTeX 2026, oracle only), drawn the way `DL3Renderer.rasterize(pdfPage:)` draws it, by a small Swift comparison program outside the repository. Every engine PDF, written by `flashtex-initex` from this tree and by the app's compile (`engine.pdf` in each capture), draws identically to pdflatex's at 2 px/pt on all 23 decks.

## Verified results (after the fixes; before/after in `captures/before-fixes/`)

**Page counts** all equal pdflatex's: warsaw-beaver 9, madrid-dolphin 6, metropolis 10, overlays-advanced 27, handout-2on1 2, graphics 7, tikz-overlays 15, notes 5, bibliography 3, allowframebreaks 14, widescreen-169 4, article-mode 1, long-deck 118.

**Geometry.** Each pane bitmap fills the pane's width, 1,038 px at 2× in a 729 pt pane, with the page's own aspect ratio and no crop or letterbox:

| deck | page size (bp) | pane bitmap |
|---|---|---|
| 4:3 decks | 362.83 × 272.13 | 1,038 × 779 |
| widescreen-169 | 453.54 × 255.12 | 1,038 × 584 |
| notes (double width) | 725.67 × 272.13 | 1,038 × 390 |
| handout-2on1 (portrait A4, two slides a sheet) | 595.28 × 841.89 | 1,038/1,039 × 1,469 (see host issue 1) |
| article-mode (letter) | 612 × 792 | 1,038 × 1,344 |

**Pixels.** The table counts pane bitmaps identical to Core Graphics' rendering of the PDF at the pane's own scale: the whole page at fit width (2.86 px/pt for 4:3), and the tiles at 4.25 px/pt (512 px tiles, as one image). "Drawn from the PDF" means the page or one of its forms is INCOMPLETE.

| deck | drawn from the PDF (INCOMPLETE) | captured pages with forms | fit width: identical | 4.25 px/pt tiles: identical |
|---|---|---|---|---|
| warsaw-beaver | 1-9 | 2-9 | 9/9 | 9/9 |
| madrid-dolphin | 2-6 | 2-6 | 6/6 | 6/6 |
| metropolis | - | - | 10/10 | 10/10 |
| overlays-advanced | 2-6,24-27 | 2-6,24-27 | 27/27 | 27/27 |
| handout-2on1 | 1 | 1 | 1/1 (page 2: 1 px narrower, host issue 1) | 2/2 |
| graphics | - | - | 0/7 (max delta 2) | 0/7 (max delta 3) |
| tikz-overlays | 2-3,11-15 | 11-15 | 15/15 | 15/15 |
| notes | 1-5 | - | 5/5 | 5/5 |
| bibliography | - | - | 2/3 (max delta 161) | 2/3 (max delta 161) |
| allowframebreaks | 1-14 | 1-14 | 14/14 | 14/14 |
| widescreen-169 | 1-3 | 2-3 | 4/4 | 4/4 |
| article-mode | - | - | 1/1 | 1/1 |
| long-deck (9 pages captured) | 74 of 118 | 2,40-42,117-118 | 9/9 | 9/9 |
| rw-beamer-blocks-columns, -default, -fragile, -overlays, -polish, -sans-operators | - | - | all | all |
| rw-beamer-madrid | 1,3-6 | 3-6 | 7/7 | 7/7 |
| rw-beamer-visuals | 1-4,6 | 2-4,6 | 7/7 | 7/7 |

- **Fallback rate.** Every page with beamer's shaded balls, a shaded headline (Warsaw, Madrid), TikZ opacity or shading, beamer buttons, or pgfpages' notes and handout sheets is drawn from the compile's PDF. That is 137 of the 284 pages captured here, and 74 of 118 in long-deck. Those pages are exact, but each costs a whole PDF draw.
- **Tiles** at 4.25 px/pt, including PDF-fallback pages and pages with forms and images: no seams, no missing visible tiles (`missingVisible` 0), no stale tiles (`stale` 0), and every tile exactly its grid rectangle (`wrongSize` 0). They match the whole-page rendering everywhere, with the same two exceptions as the whole page.
- **Remaining differences.** All are in included PDF images, drawn in the pane with `drawPDFPage` instead of as pdfTeX's Form XObject:
  - graphics: the `\logo` and PDF images differ by at most 2 levels (a few hundred px).
  - bibliography page 3: beamer's `beamericononline.pdf` (transparency groups and SMasks) differs by up to 161 levels, but only when several such PDFs share a page. Reproduction: a `standalone` page with `\includegraphics` of beamericonbook, beamericonarticle and beamericononline gives 3,050 px with max delta 161 between the direct draws and pdfTeX's page. Each icon alone gives at most 2. Two attempts were made (a transparency layer around the draw made it worse; the current colour has no effect). **Open, reported below.**

The same comparison at renderer level (`PreviewParityTests` over display lists that `flashtex-initex` wrote for these decks, at 1.43, 1.74, 2, 2.29, 2.86 and 4.25 px/pt) gives the same two remaining pages and the same pixel counts.

**Edits in long-deck** (`edits/`). The capture hook typed into the running app's editor:

- " EDITED" in frame 20's prose (3 overlay slides): the app reported pages [40, 41, 42] changed, which are exactly the pages pdflatex's PDF changes. No page stayed stale. Pages 38–45, 117 and 118 are identical to pdflatex's edited PDF. The incremental DONE came 3.6 s after the edit, 79 pages typeset.
- A fourth `\item<4->` in frame 20 (one more slide; every later page moves by one): 118 → 119 pages, pages 40–119 changed, as in pdflatex, and none stale. Pages 38, 40–45, 60 and 117–119 are identical to pdflatex's edited PDF.
- Changing item 2 of frame 20 at 4.25 px/pt (tiled), so that only slides 2 and 3 of the frame change in the PDF: the tiles and backdrops of pages 39–43 are identical to the edited PDF, with 0 stale and 0 missing tiles. The app counted page 40 as changed (its display-list hash changed, though its pixels did not), which is harmless.

**Search (engine-faithful, not changed).** Every glyph of a frame carries the `\end{frame}` line, as pdfTeX's SyncTeX records it (long-deck pages 40–42: 301 glyphs, all `main.tex:134`). So:

- Reverse search, a click anywhere on a slide, selects the `\end{frame}` line.
- Forward search (⌘-click, and follow-the-edit, which uses the same lookup) finds a place only from the 60 `\end{frame}` lines, each giving the frame's first slide. From a line inside a frame body it finds nothing, so the preview does not follow the caret while you type inside a frame.
- On the Outline page, the table-of-contents glyphs carry `main.toc` lines inside the host's project copy, which are not project files, so reverse search there selects nothing.

An app-side fallback (the next line that has a place) is possible without touching the engine; it is not done here.

## App fixes (this lane)

| PR | branch | root cause | test |
|---|---|---|---|
| #1450 | `agent/mac-claude-a/beamer-v3-app-zero-width-anchor` | The first real layout kept a reading anchor from the widthless pre-layout. With a restored zoom above fit, the pages opened scrolled to their right edge, cutting off the left of every slide (`images/before-anchor-fix-warsaw-window.png`). | `EngineV3PreviewNavTests.testFirstRealLayoutOpensAtTheLeftEdgeWhenZoomedIn` |
| #1451 | `agent/mac-claude-a/beamer-v3-app-image-edges` | PNG/JPEG images were drawn with antialiased edges, unlike Core Graphics' PDF image drawing: graphics deck up to 2,852 px, max delta 252 (`images/before-image-edge-fix-graphics-p4-diff.png`). | `RasterImageEdgeTests` |
| #1452 | `agent/mac-claude-a/beamer-v3-app-pdf-draw-lock` | Pages of one `CGPDFDocument` were drawn from several threads at once (the raster queue and the tile queue), and Core Graphics then draws shadings wrongly at random: balls up to 75 levels off, headlines 12–13 (`images/before-lock-fix-rw-madrid-p3-balls-diff.png`). The captures here were taken with a per-document lock, which still failed 3 of ~52 test runs; #1452 now draws each page from the drawing thread's own document of the PDF's bytes, which draws exactly as one at a time on all 23 decks' PDFs (every page, 2.86 and 4.25 px/pt, 3 concurrent rounds: 0 px differ), so the captures stand. | `PDFConcurrentDrawTests` |
| (this PR) | `agent/mac-claude-a/beamer-v3-app-evidence` | Evidence hooks: tiles, a typed edit, search lines, `engine.pdf`, narrow-layout preview. | `EngineV3PageCaptureTests` |

## Engine and host issues (reported, not fixed here)

1. **The PAGE box is not "as pdfTeX writes it".** handout-2on1 page 2 is drawn from the display list (not INCOMPLETE). Its pane bitmap is 1,038 px wide at 1.7437 px/pt, while the PDF's MediaBox `[0 0 595.276 841.89]` gives 1,039 px. So the box the host sends is A4's exact 595.2756 bp, not pdfTeX's 595.276 (display-list-v3 PAGE header: "`box`: the PDF box in bp … as pdfTeX writes it"). At some scales a page is then one pixel narrower or shorter than the PDF, and H in the y flip is off by up to 0.0002 bp. Repro: `\documentclass{beamer}\usepackage{pgfpages}\pgfpagesuselayout{2 on 1}[a4paper,border shrink=5mm]`, two frames; capture at `FLASHTEX_V3_PPP=1.7437`.
2. **A high fallback rate on beamer** (137 of 284 pages). Shadings (balls, headlines) and the pgfpages sheets are INCOMPLETE. Exact, but every such page is a whole PDF draw (with #1452, from each drawing thread's own copy of the PDF, so they still draw in parallel).

## Files

- `captures/fit/<deck>.capture.json`, `.cmp.txt`: fit width, after the fixes.
- `captures/tiles-4.25/`: 4.25 px/pt (tiles), after the fixes.
- `captures/before-fixes/`: the same comparisons before #1451/#1452.
- `edits/long-deck-edit-{text,item,tiles}.capture.json`, `.cmp.txt`: the three edits.
- `images/`: the screenshots and diff crops named above. `warsaw-window-p3.png` and `long-deck-after-item-edit-window-p43.png` are window screenshots by id. `open-bibliography-p3-online-icon-{pdflatex,pane}.png` is the open difference.
