# Engine-v3 preview in the Mac app (P3-APP-V3, 2026-09-29/30)

Lane **P3-APP-V3** (kabir-claude, mac-m5pro-kabir). It follows DESIGN.md §3 (the
licence boundary is a process boundary), §6.1–6.2 (display list, preview
renderer) and §12 P3 (app integration behind a flag; preview parity at zero
tolerance). Branch `agent/kabir-claude/app-v3`, PR #1247. How to use it:
[`apps/mac/docs/engine-v3-preview.md`](../../../apps/mac/docs/engine-v3-preview.md).

The machine is an M5 Pro running macOS 26.6 (`raw/environment.txt`). It is
shared with other lanes; the load average was 10–22 during the runs.

## Verified

### Decoder parity: 83/83 fixtures

`FlashTeXDisplayListV3Tests.testEveryParityFixtureMatchesTheReferenceDecoder`
decodes each fixture's `display.dl3` in Swift and compares the canonical text
(every decoded field, floats bit for bit) with the Rust reference decoder's
`dl3-dump --canonical` for the same file.

- **83/83 fixtures identical**: 230 pages, 133,676 items, 118,899 glyphs.
- The inputs are what `tools/displaylist/check_positions.py` leaves in
  `target/dl3-positions/`. Positions: 83/83 exact, at the tree of this branch.
- The checked-in fixture (`beamer-overlays.dl3`) runs on every `swift test`,
  against the SHA-256 of its Rust canonical text.
- `testDecodingFailsClosed` covers truncated bodies and frames, unknown opcodes
  and dangling path references (spec §7).

### Type 1 decision: neither (a) nor (b); load the programs directly into Core Graphics

- **What was measured.** `CGFont(CGDataProvider)` loads a Type 1 PFB on macOS 26
  (`CMR10`, 132 glyphs). `getGlyphWithGlyphName` resolves names, and
  `showGlyphs` draws them.
- **Why that is the right choice.** The PDF's embedded subsets carry the same
  charstrings (writet1), and Core Graphics is also the rasteriser that draws
  them when it renders the PDF. Drawing the FONT program through the same
  rasteriser is the shortest route to identical pixels.
- **Cost.** No conversion step (a) and no second rasteriser (b).
- **Portability.** The protocol stays unchanged and renderer-agnostic: it
  carries the Type 1 programs. A Linux or Windows client would load the same
  programs with FreeType, which reads Type 1 natively. Engine-side CFF
  conversion (a) remains possible if Apple removes Type 1 from `CGFont`.

### Preview pixel parity against Core Graphics' rendering of the engine's PDF

`FlashTeXPreviewV3Tests.testEveryParityFixtureIsPixelIdenticalToThePDF`
(`raw/parity-vs-coregraphics.json`) rasterises every page of all 83 fixtures at
1×, 2× and 4× in two ways:

- through `DL3Renderer`;
- through `CGContext.drawPDFPage` of the engine's PDF (byte-identical to
  pdflatex's for these fixtures, P-T2), into an identically configured sRGB
  bitmap.

A pixel counts as different if any RGBA channel differs.

| scale | page renders identical | differing pixels |
|---|---|---|
| 1× | 216/220 | 85 |
| 2× | **220/220** | **0** |
| 4× | **220/220** | **0** |

- **The 1× floor.** Four pages differ at 1×, and never at 2× or 4×:
  - `inline-math` p1: 45 px
  - `inline-math` p2: 11 px
  - `proof-practice-21242` p11: 23 px
  - `proof-practice-21242` p13: 6 px

  Each is a single glyph whose origin lies on Core Graphics' subpixel boundary
  at 1×. The display list rounds positions to sp (spec §4.2, at most 7.6·10⁻⁶
  bp), and the PDF's decimal position (accumulated by Core Graphics in floating
  point) falls on the other side of that boundary.

  The renderer restores pdfTeX's three-decimal grid for y. That fixed a whole
  line of `thesis-chapter` (2,161 px at 1×). x inside a `TJ` is not on any grid.
  The test allows at most 64 px per page, at 1× only. **Removing the floor
  needs protocol 3.2 to carry the PDF's exact origin.** That is an engine-lane
  change, proposed and not made here.
- **PDF fallback.** 30 page renders (10 pages × 3 scales) are drawn from the PDF
  instead, as spec §4.7 requires:
  - beamer-madrid pages 1, 3–6;
  - beamer-visuals pages 1–4 and 6.

  Their pages or forms are INCOMPLETE: `gs` transparency, or shaded-ball forms.
  A page counts as needing the fallback when any form it draws is INCOMPLETE,
  not only when the page itself is flagged.

The table is the result after these fixes, each found by this sweep:

- **FONT encodings from the engine are all `.notdef` for fonts whose built-in
  encoding is written `dup 1/uni6301 put`.** This affects the arphic gbsnu CJK
  subfonts in `unicode-accents` and cost 3,160 px at 4×. The cause is that the
  engine's `displaylist/mod.rs` `builtin_encoding` splits on whitespace. The
  renderer now falls back to the program's built-in encoding for any code the
  sent encoding leaves at `.notdef`, which is also what a viewer does for a
  font without `/Encoding`. **Engine bug, reported to the Commander, not fixed
  here** (GPL side, another lane).
- **Images.** Samples are now drawn in the PDF's device colour space, as pdfTeX
  embeds them (`/DeviceRGB`, with no sRGB chunk, gamma or profile applied).
  Interpolation stays at `.default`: `.none`, `.medium` and `.high` each
  differed.
- **Pages drawing INCOMPLETE forms** now fall back to the PDF (see above).

### Against PDFKit (`raw/parity-vs-pdfkit.json`)

PDFKit's `PDFPage.draw(with:to:)` is not the same as Core Graphics'
`drawPDFPage` for the same PDF. Measured on the engine's PDFs: 2,114,050 px over
the 660 page renders, mostly thin rules and some glyphs at 1× and 2×. So the
PDFKit test (`testEveryParityFixtureIsAsCloseToPDFKitAsCoreGraphicsIs`) asserts
that the preview is no further from PDFKit than Core Graphics' own rendering
is, plus the 1× floor.

- **It holds on every page.** The preview's distance from PDFKit equals Core
  Graphics' on all but the four 1× floor pages.
- **The zero-tolerance gate stays Core Graphics, as DESIGN.md §6.2 states it.**

### Keystroke → pixels in the app (`raw/keystroke-plain-*.json`)

**Setup**

- Release build of the app (`swift build -c release`), `FLASHTEX_V3_BENCH`
  (`EngineV3Bench.swift`), and `flashtex-host` from this tree
  (`target/release`).
- Documents: `docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py` `doc(n, False)`
  (plain article + amsmath) for n = 10, 120 and 1000.
- 60 keystrokes per document, 300 ms apart. They alternate: an `x` inserted
  after the first `with ` (a prose word on page 1, which is on screen), then
  deleted.
- The pane was at 1.14 px/pt (default window, Retina).

**What is measured.** The keystroke is stamped (`CLOCK_UPTIME_RAW`) just before
`NSTextView.insertText` / `deleteBackward`. The end point is the main thread's
`CATransaction.commit()` + `flush()` that installs the new bitmap of the first
page the resulting compile changed. No debounce applies; every keystroke is a
COMPILE.

| document | pages | samples | **p50** | **p95** | key → hook | hook → COMPILE sent | sent → page decoded on main | raster | page on main → commit |
|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | 60/60 | **16.0 ms** | **18.8 ms** | 1.28 | 0.08 | 11.87 | 0.95 | 2.81 |
| plain-120 | 121 | 60/60 | **16.8 ms** | **18.5 ms** | 2.42 | 0.09 | 11.17 | 0.96 | 2.79 |
| plain-1000 | 1001 | 60/60 | **34.1 ms** | **37.7 ms** | 11.50 | 0.37 | 17.92 | 0.95 | 2.81 |

The phase columns are medians.

- **"key → hook"** is the editor's own work before the change reaches
  `ShellModel.updateActiveText`, where the engine-v3 hook is now the first
  statement. For the 1,000-page document that is 11.5 ms. It is spent in the
  text view and `SourceEditorView` turning a 2.5 MB NSTextStorage into a String;
  it is not in this lane's code, which takes 0.37 ms. That is the largest
  app-side cost left at 1,000 pages.
- **"sent → page on main"** is the host (restart from the checkpoint, typeset,
  write the frames) plus decoding on the reader thread.
- Earlier runs, during heavier load from other lanes, gave 16–18 / 20–25 ms
  (10 pages), 18–20 / 22–40 ms (120 pages) and 32–62 / 38–157 ms (1,000 pages).

**Format on first use.** The host built `pdflatex.fmt` from TeX Live 2026 in
4,752.8 ms, then warmed up in 177.5 ms (`raw/host-first-start.txt`). The pane
shows a spinner, a timer and "Preparing the pdfLaTeX format from your TeX
Live…" meanwhile.

**On screen.** `raw/plain-120-page1-on-screen.png` is the bitmap installed in
page 1's layer at the end of the plain-120 run.

### Gates

| gate | result |
|---|---|
| `swift build`, `swift test` (apps/mac) | see the PR's final comment for the tip SHA's run |
| decoder parity, all fixtures | 83/83 |
| preview pixel parity, Core Graphics | 2×, 4×: 220/220 identical; 1×: 216/220 (floor above) |
| licence boundary (`scripts/check-license-boundary.sh`) | clean. The MIT targets `FlashTeXDisplayListV3` and `FlashTeXPreviewV3` link no engine code; the app spawns `flashtex-host` |
| `scripts/gate.sh pr` | passed (at 8c1fd1f80) |
| no drawing on the main thread | yes. `DL3Renderer.rasterize` runs on `flashtex.engine-v3.raster` (concurrent queue). The main thread only assigns `layer.contents`. Decoding and resource loading run on the socket reader thread |
| old path unchanged with the flag off | with the flag off (the default): `PreviewPane` shows the v2/v1 pane as before, `updateActiveText` runs `scheduleAutoCompile()` as before, and no host is started |

## Follow-up (2026-09-30): keystroke → pixels by stage

This work is on branch `agent/kabir-claude/app-v3-2`. Each stage is timed in
the app (`EngineV3Latency`) and also emitted as an os_signpost (subsystem
`tech.jay3332.flashtex.mac`, category `EngineV3Latency`). Setup:

- Release build, 60 keystrokes per document, window 1440×900.
- Load average 2–4. The owner restricted CPU-heavy work that night, so
  benchmarks ran only once the load was below 8.
- Raw data: `raw/stages/*.json`; summary: `raw/stages/stages.txt`.

What changed, in keystroke order:

1. **Edit hook on the text storage (fast path).** The byte splice comes
   straight from `NSTextStorage`'s edited range, before the editor's own work.
   Key → COMPILE sent now takes 0.5–1.2 ms instead of 3.4 / 3.6 / 14.2 ms
   (10 / 120 / 1,000 pages).
2. **Rasterise and commit on the socket reader thread.** The page is drawn
   into an IOSurface (zero-copy) and committed there, into the page view's
   hosting layer, so the main thread is not on the path. Before:
   - with a CGImage, `CATransaction.commit` + `flush` took 3.5 ms;
   - main-thread queueing took 1–3 ms p50 while the editor worked through the
     keystroke.

   Pixel identity with the RGBA parity raster is checked by
   `testScreenLayoutDrawsTheSamePixels`, for both the BGRA layout and the
   IOSurface.
3. **Robust re-layout.** Frame changes of the clip view re-lay out the pages,
   and a page arriving without a view gets one. This fixes an "all
   off-screen" start seen on the first launch after a build.

**Fast path, p50 / p95 (ms):**

| document | key → hook | hook → sent | host first page | decode + prepare | raster | raster → commit | **key → commit** | commit → next frame |
|---|---|---|---|---|---|---|---|---|
| plain-10 | 0.74 / 1.07 | 0.41 / 0.62 | 19.74 / 21.43 | 0.09 | 1.57 / 1.79 | 0.16 / 0.28 | **22.9 / 25.0** | 15.0 / 18.6 |
| plain-120 | 0.30 / 0.42 | 0.27 / 1.14 | 13.21 / 14.39 | 0.07 | 1.18 / 1.24 | 0.13 / 0.14 | **15.4 / 17.2** | 13.9 / 18.8 |
| plain-1000 | 0.24 / 0.43 | 0.26 / 6.97 | 17.92 / 24.45 | 0.06 | 1.07 / 1.35 | 0.12 / 0.14 | **22.9 / 31.6** | 15.3 / 21.5 |

**The same build with the fast path off** (`FLASHTEX_V3_FAST_EDITS=0`), key →
commit p50 / p95: 23.6 / 26.0 (10 pages), 18.3 / 19.5 (120) and 32.3 / 41.7
(1,000). Of that, key → hook is 3.3, 3.4 and 13.9 ms.

**What bounds it now: the host.** The app's own stages total 1.7–2.9 ms p50.
The host's `first_page_ms` (COMPILE received → first re-typeset page) is 13 to
20 ms p50 and is spread between 9 and 22 ms from one keystroke to the next.
Two A/B tests were run on plain-10:

- **`viewport`.** A first run suggested it cost 6 ms (14.5 vs 20.3 ms p50),
  and the app now sends it only when the view is past page 1. The final runs
  without it gave the same 19.7 ms, so that difference was noise.
- **A shorter checkpoint interval** (`--timed 0.004` instead of 0.02 s): no
  change (19.5 ms).

The ≤ 16 ms p95 target therefore needs the host's restart of the edited page
to get faster (lane P4). On the app side what is left is:

- the display's frame: commit → next frame is 12–15 ms p50 on this 60 Hz
  path, and the pages are committed as soon as they exist, with no extra
  frame of waiting;
- about 1.2–1.6 ms of raster, which is at 1.14 px/pt.

`FLASHTEX_V3_VIEWPORT` and `FLASHTEX_V3_TIMED` stay in as A/B switches.

## Forward and reverse search against pdflatex's SyncTeX (2026-09-30)

Source mapping in the engine-v3 pane (`EngineV3SourceMap.swift`,
`FlashTeXPreviewV3/DL3SourceIndex.swift`) is built from the display list's
`SOURCES` + `SPAN` + per-glyph `col` (protocol §5.3). No protocol or engine
change was needed.

What it does:

- **Reverse search.** A click on the preview finds the glyph under the point,
  then its file, line and byte column. It opens `\input`/`\include` files the
  editor has not opened yet, and selects the character.
- **Forward search.** The caret, ⌘⇧J or ⌘-click in the editor finds the glyph
  at the caret's column on the first page that shows its line. The pane
  scrolls to it and flashes it.
- **Following edits.** The existing caret follower (CaretFollow.swift) drives
  it with the same rules as the old panes: debounced, scrolling only when the
  target is outside the comfort band, and a scroll by hand pausing it until
  the next edit.

**Oracle.** `tools/displaylist/synctex_oracle.py` recompiles each fixture's
converged sources with pdflatex `-synctex=1` (the same layout; the engine's
PDF is byte-identical). `FlashTeXPreviewV3Tests.SourceMapOracleTests` then
asks TeX Live's `synctex` CLI about:

- 30 glyphs per fixture (reverse search: `synctex edit` at the glyph's ink
  centre);
- 15 source lines per fixture (forward search: `synctex view`).

**Results, all 83 fixtures:**

| | agreement |
|---|---|
| reverse: same file and line | **2,421 / 2,490 (97.2 %)** |
| reverse: within one line | **2,482 / 2,490 (99.7 %)** |
| forward: same page | **749 / 759 (98.7 %)** |
| forward: our box overlaps a SyncTeX box | **745 / 759 (98.2 %)** |

`raw/synctex/disagreements.json` lists every miss. They have two causes:

- **Off-by-one lines.** SyncTeX attributes a paragraph's boxes to the line
  where the box was started, so the first words of a line that continues a
  paragraph report the previous line; the display list names the character's
  own line.
- **Auxiliary files.** Text from `.toc`/`.vrb` files is attributed differently
  (beamer's verbatim frames), and SyncTeX points at other pages for it.

## Dark preview (2026-09-30)

The preview's existing dark toggle (the title bar's moon, whose default
follows the Appearance setting and the system) now also drives the engine-v3
pane (`DL3Appearance.dark`).

What changes in dark mode:

- **The page ground** is gray 0.125.
- **Colours the page's items set** (text, rules, paths, forms) have their HSL
  lightness inverted onto [ground, 1], with hue and saturation kept. Black ink
  becomes white. A page's own white boxes become the ground. Beamer's blue
  becomes a lighter blue, and its blocks become dark boxes.
- **Text is kept readable:** its lightness is at least 0.72, so hyperref's
  pure-blue links read on the dark ground.
- **Images are drawn untouched.**
- **Pages that fall back to the PDF** (INCOMPLETE) get the same treatment on
  the whole bitmap: Core Image `CIColorInvert` followed by a `CIHueAdjust` of
  π. On those pages images are inverted too, because the PDF's pixels cannot
  be told apart from its ink.

Light mode is unchanged. The zero-tolerance parity sweep after this change
gives the same numbers: 2× and 4× 220/220 identical, 1× 216/220 (the floor).

**Tests** (`DarkAppearanceTests`, rendering through the app's renderer, not a
screen capture):

- black ↔ white;
- hue kept;
- the dark page is dark overall (mean luminance < 90 against > 180 for light);
- more than 95 % of the light page's ink pixels are light ink in dark mode;
- a PNG figure's pixels are identical in both appearances (> 99 % of samples).

**Evidence pairs:** `raw/dark/*-light.png` and `raw/dark/*-dark.png`, for
hyperref-toc (links), beamer-blocks-columns (a PNG figure) and beamer-madrid
(theme colours).

## Fonts beyond Type 1 (lane P3-FONTS-2, 2026-09-30)

This work is on branch `agent/kabir-claude/app-v3-4`, which merges
`agent/kabir-claude/p3-fonts-2` (#1265) before it lands. The pane now sends
`"font_formats": ["type3", "truetype", "opentype"]`.

- **`type3`:** the `T3B1` masks (`DL3Type3`, MIT) are drawn as 1-bit image
  masks (`/Decode [1 0]`) in the fill colour, through
  [w 0 0 h llx lly] × `font_matrix` × the glyph matrix. Interpolation is
  `.medium`, the smoothing Core Graphics applies to a Type 3 mask; measured
  on the PK documents, `.none`, `.low` and `.default` all differ.
- **`truetype`/`opentype`:** the font file is loaded into `CGFont`, and the
  glyph for each code is found in this order:
  - the glyph named `encoding[code]`;
  - `uniXXXX` names through the Unicode cmap;
  - `indexN` as glyph N;
  - subfonts through `subfont[code]` and a Unicode cmap. A non-Unicode cmap
    is reported as a problem, and the page falls back to the PDF.
- **Fonts with `problem`:** their pages fall back to `DONE.pdf`.

**Pixel parity** against Core Graphics' rendering of the engine's PDF, on
lane P3-FONTS-2's 4 font documents (`dl-docs/`, display lists from this
tree's engine; positions 4/4 exact):

| scale | identical page renders |
|---|---|
| 1× | 3/4 (one Type 3 page: 1 px) |
| 2× | **4/4** |
| 4× | 3/4 (the Computer Modern Type 3 page: 26 px, max Δ 20) |

- The Type 3 residue is mask resampling at the sp-rounded origin, the same
  cause as the 1× Type 1 floor. The test allows at most 64 px with max Δ ≤ 32
  on Type 3 pages.
- **Coverage gap:** the Clear Sans (TrueType) document drew with Type 1
  fonts here, because this Mac's TeX Live has no Clear Sans TrueType. So the
  TrueType path is not pixel-verified; the OpenType (GFS Bodoni) page is
  identical at every scale.
- **No regression:** the 83 parity fixtures are unchanged (2×/4× 220/220,
  1× 216/220), and decoder parity is 4/4 on the font documents and 83/83 on
  the fixtures.

## Instant reopen (owner decision 8A, 2026-09-30)

Implemented in `EngineV3Snapshot.swift`.

**What is stored, per project:**

- PNGs of the pages near the viewport and the first ones (at most 12);
- every page's size;
- the SHA-256 of every editor document, as last sent to the host (not
  newer text typed during the compile);
- the modification time and size of every other input file in the project
  folder (`.tex`, `.bib`, `.sty`/`.cls`, images, ...:
  `EngineV3Snapshot.inputExtensions`), as listed when the project copy was
  last synced.

It is written 1.5 s after an `ok` compile, on a utility queue, and only if
those input files are still as they were at the sync (otherwise the pages
may not show a change made outside the app, so nothing is written). Everything
stored stays under a 256 MB budget; the least recently written project goes
first.

**When a project opens** (`documentURL`'s didSet, in the open's own run-loop
turn):

- if every editor document still hashes the same, the stored pages go on
  screen at once, marked stale;
- every other input file is then checked on the snapshot queue (off the
  main thread): same time and size, none added or removed. A chapter, `.bib`
  or figure changed outside the app, or a folder that cannot be listed in
  full (more than 20,000 entries), drops the stored pages;
- each stored page stays dimmed until the compile's raster of that page is
  committed. A `STARTED` with `keep:false`, the open's own compile clearing
  the page list, a host restart, a `PAGE` that grows the count, the host's
  `current` ranges, and a `DONE` that did not send it never clear it: every
  write of the stale marks adds the stored pages not yet replaced, and the
  page view keeps a stored bitmap dimmed until its replacement is on the
  layer (re-reviews of #1332, 2026-10-01);
- this needs no host and no fonts: the images are decoded and committed on
  the raster queue;
- the compile's pages then replace them.

**Measured** (`scripts/openbench.sh`, release build, private cache root, load
4–6; `raw/open/open-o6-*.json`), before the input-file check was added. Times
run from `replaceProject` to the first page bitmap committed.

| document | reopen in the running app: stored pages | first current page | at launch: stored pages | at launch, no snapshot: first page |
|---|---|---|---|---|
| plain-10 | **16 ms** | 64 ms | 111 ms | 458 ms |
| plain-120 | **29 ms** | 64 ms | 146 ms | 485 ms |
| plain-1000 | **26 ms** | 244 ms | 365 ms | 631 ms |

- **In-app reopen** meets the ≤ 100 ms target at every size.
- **Re-measured with the input check off the main thread** (2026-10-01,
  `raw/open/open-r2-*.json`, one run each, load about 2.7; single runs, so
  differences of ±20 ms are noise): reopen in the running app, stored pages
  29 / 28–48 / 16–20 ms (plain-10 / 120 / 1000; plain-10's in-app reopen
  shows no snapshot in both the old and the new runs); first current page
  86 / 63–87 / 231–235 ms; at launch, stored pages 101 / 139 / 356 ms; at
  launch, no snapshot, first page 502 / 495 / 592 ms. No regression is
  visible against the table above at this precision.
- **At launch** the time includes creating the window and the editor. For the
  1,000-page document that is the 2.5 MB text going into the editor before
  the pane exists, which is not this lane's code.

**Tests** (`EngineV3SnapshotTests`):

- save, load and invalidation by content hash, and old page images removed;
- a chapter and a `.bib` changed outside the app, and a figure added, each
  invalidate the snapshot; a non-input file does not;
- the stored pages stay stale after `STARTED keep:false`, the host's
  `current` ranges and a `DONE` that did not send them, until each page
  arrives; end to end with a host, both stored pages are stale at `STARTED`
  and the other one still is at the first `PAGE`;
- an input changed after the pages were shown drops them (the background
  check); unchanged inputs keep them;
- a second model opening the same file has the stored pages, stale,
  synchronously in `openTex`, before any host runs, and the compile then
  replaces them.

## Key → presented (owner request, 2026-09-30)

**What is measured.** `EngineV3PresentProbe` (`EngineV3Present.swift`) is a
transparent 1×1 `CAMetalLayer` inside each page layer, created only in
measurement runs. It has `presentsWithTransaction`, so its drawable is
presented by the same Core Animation transaction that installs the page's
new bitmap. The drawable's `presentedTime` is therefore when that page went
on screen.

**How it was run.**

- Built-in Liquid Retina XDR (ProMotion, 120 Hz).
- The release app, with a private cache root.
- The bench window was ordered in front without activating the app. An
  occluded window's frames are never shown, and every `presentedTime` is 0.
- `scripts/presab.sh`: 2 rounds, interleaved, 40 keystrokes per run, load
  4.5–8.2.
- Raw data in `raw/present/`: `pb*` is the shipped configuration, `pa*` the
  frame-rate boost.

p50/p95 in ms, shipped configuration:

| document | key → commit | commit → presented | **key → presented** |
|---|---|---|---|
| plain-10 | 12.7 / 21.6 | 19.1 / 23.1 | **30.9 / 39.1** |
| plain-120 | 13.3 / 20.2 | 19.5 / 23.6 | **33.3 / 40.4** |
| plain-1000 | 24.5 / 30.6 | 19.5 / 23.6 | **41.1 / 49.3** |

**Where the commit → presented time goes.**

- **The floor is the window server.** Across every run, commit → presented
  was never below 15.9–16.0 ms, which is two 120 Hz frames. The rest, up to
  one more frame, is where in the frame the commit landed.
- **Nothing the app does comes after the commit.** The commit is explicit
  and flushed at once, from the raster or reader thread. No implicit
  transaction and no run-loop turn stand in between.
- **Metal presentation is not faster.** Presenting the probe with Metal on
  its own (`FLASHTEX_V3_PRESENT_TX=0`) instead of with the transaction gives
  the same result: p50 21.6 vs 21.4 ms, minimum 16.0 in both. So drawing the
  pages through Metal would not go faster either.
- **Probe cost.** It adds ~0.3 ms to the commit and flush (p50 0.46 vs
  0.12 ms, `np*`), in measurement runs only.

**What was tried.**

- **Display-link frame-rate boost** (`EngineV3FrameRateBoost`, a
  `CADisplayLink` asking for 120 Hz while typing): no gain.
  - key → presented p50 was 30.9 / 35.4 / 48.6 ms with it and
    30.9 / 33.3 / 41.1 without.
  - In both arms, frames went on screen on the 120 Hz grid (presented times
    fall on odd multiples of 8.33 ms). macOS already raises the rate when
    the window's content changes.
  - It is now off by default (`FLASHTEX_V3_BOOST=1` turns it on).
- **Aligning commits to the frame deadline**: not done, because it can only
  delay a page. The first changed page already commits the moment its
  bitmap exists, and the frame it makes is the earliest possible.
- **Implicit transaction fixed.** `setStale` changed the page layer's
  opacity in an implicit, animated transaction, a 0.25 s fade when a stale
  page came back. It is now explicit, with no actions.

**Result.** key → presented ≈ key → commit + 16–24 ms of window server.
What the app can still cut is key → commit, and that is mostly the host's
first page.

**Not used.** A later run of the final build (`presfin.sh`) was made while
another lane's host benchmark held a core (load 7.4 → 9.9). Its
key → commit tripled, so it was discarded. Its commit → presented agreed:
p50 20–21 ms, minimum 15.9–16.0.

## Beliefs, not verified here

- The 1× floor would go to zero if protocol 3.2 carried the PDF's exact origin
  alongside the sp one, or if the app matched Core Graphics' own accumulation.
  Not tried.
- `CGFont` Type 1 support could disappear in a later macOS. The fallbacks are
  (a) or (b), and the protocol would not change.

## Gaps (next)

- **Editor side.** At 1,000 pages the editor's String conversion costs 11.5 ms
  per keystroke. Hooking NSTextStorage's edited range, with the byte offset
  kept incrementally, would remove it.
- **Not in the pane yet:**
  - SyncTeX (spans → caret, click → source);
  - zoom and pinch;
  - 512 px tiles (lane #1228);
  - diagnostics in the Problems panel.
- **Host lifetime.** There is one host per app session, not per document, and
  it outlives an app crash.
- **Bundling.** `make-app.sh` does not bundle `flashtex-host` + `pdftex.pool`
  into `Contents/Helpers` yet. The locator already looks there.

## Reproducing

```
cargo build --release -p flashtex-engine --bin flashtex-host --bin flashtex-initex
cargo build --release -p flashtex-display-list --bins
# a pdflatex.fmt for the engine (as docs/evidence/host-unify-2026-09-29/scripts/gates-setup.sh)
FLASHTEX_POOL=$PWD/crates/flashtex-engine/pdftex.pool python3 tools/displaylist/check_positions.py \
    --engine target/release/flashtex-initex --formats FMTDIR -j 4
cd apps/mac && swift test --filter 'FlashTeXDisplayListV3Tests|FlashTeXPreviewV3Tests'
FLASHTEX_V3_PARITY_OUT=/tmp/parity swift test --filter FlashTeXPreviewV3Tests   # writes the JSON reports
swift build -c release --product FlashTeXMac && bash ../../docs/evidence/app-v3-preview-2026-09-29/scripts/runbench.sh
```

(`runbench.sh` expects the generated documents in `.scratch/bench/plain-N/main.tex`
under the worktree; generate them with `gen.doc(n, False)`.)
