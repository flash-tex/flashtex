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
