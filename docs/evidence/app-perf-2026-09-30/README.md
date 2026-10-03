# APP-PERF-AUDIT: the Mac app from keystroke to pixels (2026-09-30)

Lane APP-PERF-AUDIT (kabir-claude, mac-m5pro-kabir: Apple M5 Pro, 15 cores, 24 GB,
macOS 26.6). Branch `agent/kabir-claude/app-perf` from `origin/main` 67a2ea078.
Governing text: [DESIGN.md](../../design/engine-v2/DESIGN.md) §1.2 (keystroke to
pixels ≤ 16 ms p95; scroll and zoom at 120 Hz, no drawing on the main thread),
§6.2 (preview renderer) and §5.6 ("profile first; every item needs a measured win").

**Read the load column.** Other agents were building on this Mac throughout. The
1-minute load average was 6 to 74 (it is recorded for every cell under
`raw/<cell>/load.txt`). Latencies that depend on the old engine's compile time
are marked as affected by load. The editor-only before/after pairs were run
interleaved, old then new, so both halves of a pair saw the same load. The
Commander paused all measurements at load ~50, so continuous scroll, pinch-zoom
and Allocations were **not measured by this lane** (see "Not measured").

## Method

- **Build.** Release `FlashTeXMac` (`swift build -c release`). The producer is
  `flashtex-render` (render-pipeline, the old engine) built from this checkout.
  It drives the default display-list-v2 pane.
- **Driving without OS events or permissions.** The existing in-app
  `TypingBench` (`FLASHTEX_TYPING_BENCH=<script>`) types
  `tools/typing-bench/typed-200.txt` (200 characters, with "naïve café" at
  character 97) into the real `NSTextView` every 30 ms. There is no synthetic
  OS input, AppleScript, screen capture or Accessibility. The bench gained:
  - `FLASHTEX_TYPING_BENCH_AT=mid-paragraph`: a caret among words on page 1.
  - `FLASHTEX_TYPING_BENCH_NO_WORKER=1`: editor-only typing, with no compile
    results and no preview frames.
  - `FLASHTEX_TYPING_BENCH_START_UNPAINTED=1`: start typing despite F1 below.
  - Logs of why the bench was not ready, and of launch time.
- **os_signpost.** `FLASHTEX_SIGNPOSTS=1` (`PerfSignposts.swift`, off by
  default: `OSSignposter.disabled`, one `Bool` test per hook). It records:
  - intervals: `editorChange`, `syntaxFlush`, `commitUserChange`, `bufferCopy`,
    `braceHighlight`, `modelUpdate` and `renderPass`;
  - events: `keyDown`, `compileSend`, `resultApplied` and `pageDraw`.

  The existing `FlashTeXLog` timeline gives the preview stages (send, receive,
  validate, preraster, publish, blit): `tools/typing-bench/timeline.py`.
- **Profiling.** `xcrun xctrace record --template 'Time Profiler' --attach <pid>`
  ran for the whole cell. The exports were summarised with `scripts/tp_summary.py`
  (main thread, innermost app frame, inclusive, `--focus`, `--children`) and
  `scripts/mainthread.py` (main-thread ms per traced keystroke).
- **Documents.** `scripts/gen.py` (the P4-L2-L3 generator, pages 10/120/1000).
  `plain-10` is 41 KB, `plain-120` is 498 KB and `plain-1000` is 4.1 MB.
- **Where the traces are.** The `.trace` bundles (1.4 GB) are not in git. They
  are at `/tmp/appperf/runs/<cell>/trace.trace` on mac-m5pro-kabir. Their
  summaries are in `raw/<cell>/`: `summary.txt`, `timeline.txt`,
  `bench-summary.json`, `startup.txt`, `events.txt` and `series-*.txt`.

## Ranked findings

The owner column assigns each item to a lane. P3-APP-V3 is the new engine
behind the flag (flashtex-host plus display-list-v3). Tiles is mac-claude-a
(#1228: PreviewV2View and GlyphRunRenderer). The "this lane" rows are fixed on
this branch.

| # | Finding | Measured cost | Fix | Gain (measured / expected) | Owner |
|---|---|---|---|---|---|
| F1 | **A large document never shows its first page.** The v2 reply is over the producer's line limit, so the app asks for a 16-page window at page 1. The producer narrows it around its centre (`PageWindow::fitting`) and serves pages 4–14. The viewer sits on page 1, which is "not loaded", and nothing re-requests it because the visible page never changed. | plain-120 and plain-1000: **no first paint within 60 s** (3 runs, `raw/base-plain-120-mid-paragraph`, `base-plain-1000-mid-paragraph`, `base-plain120`). One run painted after 16 s, when an unrelated model change re-rendered the pane. | `V2Window.Controller.servedFrame`: a published windowed frame that lacks the viewer's page re-requests the served (fitted) page count from the viewer. This happens once, and the count only shrinks. | **Measured:** first page 1.66 s after launch (plain-120) and 47.6 s (plain-1000, of which 42 s is the old engine's startup compiles, F5). Both were at load 41–45. | **this lane** (fixed) |
| F2 | **The whole display list is re-sent and re-validated on every keystroke.** One 10-page frame is a 13.5 MB JSON line (`display_list … line 13496661 B`). Typing on page 1 shifts every later page's source byte offsets, so the page cache reuses **0/10** pages. | 10 pages, typing on page 1: validate/decode/prepare **67.7 ms** p50 per frame. Pre-raster is 12.4 ms. Main-thread admission (recv→val) is 24.6 ms p50 and 41 ms p95. Transport of the sibling line (v1→recv) is 14.9 ms. Keystroke to paint is **188 ms p50** and result to paint is 137 ms. Even with 10/10 pages reused (typing after `\end{document}`), validate costs 33 ms. `raw/base-plain-10-mid-paragraph`, `base-plain10`, `base-plain10-end`. | display-list-v3 (§6.1): per-page content hashes, and source spans keyed by stable span ids rather than byte offsets. Send only the changed pages, in a binary encoding, and validate only those. | Expected: the per-frame app cost scales with the pages that changed (1 page ≈ 1/10 of the above), so about 67 → <7 ms and result→paint about 137 → ~15 ms at 10 pages. The frame size falls from 13.5 MB to the changed pages only. | **P3-APP-V3** |
| F3 | **The main thread is saturated while typing.** Every keystroke re-lays out the SwiftUI hosting tree, even with no compile results arriving. | Editor-only (no worker) main-thread work per keystroke (the time in stacks containing each function; the rows overlap): **12.1 ms** (10 pages), 15.3 ms (120), 26.4 ms (1000). That includes `GraphHost.flushTransactions` 4.5–5.0 ms, `CA::Transaction::commit` 4.2–5.5 ms, `NSHostingView.layout` 2.2–2.5 ms and `CompletingTextView.draw` 0.9–1.8 ms. With the preview attached (10 pages): **24–31 ms per keystroke, 75–93 % of wall time busy** at 30 ms typing. `raw/base-plain-*-editor-mid-paragraph`, `stf*-plain-10-*`. | Stop the per-keystroke SwiftUI invalidation. The editor stays the source of truth. The model takes edits without mutating observed state on every character (for example the `documents` array feeding `ContentView`, `WordCountStatusItem`, `StatusBreadcrumb` and the App scene's menus). Views read change-only mirrors, as FT-071 did for the menus. | Expected: 8–10 ms per keystroke of main-thread work removed. That is the difference between a frame per keystroke and dropped frames at 120 Hz. **Tried and reverted:** `sizeThatFits` on four `NSViewRepresentable`s (editor, anchor probe, page accessibility overlay, page bitmap). Layout-trait measuring was unchanged at 1.2–2.6 ms/key in 6 interleaved runs, so there was no measured gain. | app shell (next lane); P3-APP-V3 for the result path |
| F4 | **A `\begin`/`\end` line scans the whole document on every keystroke.** The brace highlight runs `EditorNavigation.environmentPairs` whenever the caret's line holds `\begin{`/`\end{`. That includes every one-line `equation` in these documents. It walks the text through `character(at:)` on the bridged Swift buffer and builds a `Use` for every command. | `editorChange` p50, typing inside an equation line: **95.0 / 95.5 ms** (1000 pages) and **12.3 / 11.9 ms** (120 pages). On prose, 120 pages cost 2.4 ms while the text was ASCII and 12 ms once it held "ï" (`raw/tp-plain120` series). | A single-pass UTF-16 scanner (`forEachEnvironmentUse`) that follows `uses(in:)`'s walk exactly and allocates only `\begin`/`\end` arguments, run over the text storage itself. `AppPerfAuditTests` checks it equals the old construction on 26 corner cases and 3,000 random documents. | **Measured, interleaved old/new:** 1000 pages 95 → **9.8 ms** (braceHighlight 5.9 ms of it). 120 pages 12.1 → **2.2 ms**. | **this lane** (fixed) |
| F5 | **Startup compiles the same revision four times** (plain-1000). mac-1 (rules/font-hints only) is discarded when the display-list capabilities switch on (16 s). mac-2 fails as over-limit after 9.3 s. mac-3 is a 16-page window (14.2 s), narrowed. mac-4 is the refit window (4.9 s; F1). | 42 s to the first live frame on plain-1000 at load 45 (`raw/fixwin-plain-1000-mid-paragraph/events.txt`). On 10 pages the discarded mac-1 costs one small compile. | The first request should carry the final capability set. The window should be engaged up front from the source size (or the producer auto-windows), with the fitted page count. | Expected: one compile instead of four at startup, about 42 s → ~10 s on the old engine. With the new engine the principle holds: capability negotiation must not throw work away. | **P3-APP-V3** (request path) |
| F6 | **Auto-close copied the whole buffer for every character typed.** `autoClose` built `nativeText(of:)` (a full UTF-8 transcode into a buffer of 3× capacity) before checking whether the character can open anything. | 4.2 ms per keystroke at 4 MB. `editorChange` was 8.7 ms, of which the second copy was about half. | `AutoClose.mayClose` (shared EditorCore) skips `closer`'s text argument exactly when `closer` cannot read it. `AppPerfAuditTests` checks it is exact. | **Measured, interleaved:** prose at 1000 pages, `editorChange` 8.74 / 8.68 → **4.51 / 4.48 ms**. | **this lane** (fixed) |
| F7 | **The whole buffer is copied per keystroke into the model** (`bufferCopy`, the binding push). Each copy is allocated at 3× capacity, and the model keeps it, so an open document costs 3× its size in RAM. | 4.24 ms p50 at 4 MB; 0.67 ms at 500 KB (`raw/sig-plain-1000-…`, `fix1-plain-120-…`). | **Tried and reverted:** splicing the edit into the previous String gave 3.7–4.4 ms ASCII and 5.3 ms non-ASCII, the same cost (allocation and page faults dominate). Recommendation: pass edits (range + text) to the model and the engine instead of whole strings. DESIGN §5 edits are deltas anyway. | Expected: −4 ms/key at 4 MB and 2/3 of the document RAM. | P3-APP-V3 (edit protocol), then app shell |
| F8 | **The old engine's compile dominates latency beyond 10 pages.** | send→result p50: 33 ms (10 pages), 391 ms (120), 5.1–7.2 s (1000), at load 6–17. Keystroke to paint at 120 pages is 1.75 s p50; nearly every keystroke is coalesced (195/200). | The new engine: L2/L3 incremental plus L4 viewport first (§5). | §1.2 targets | P3-APP-V3 / engine |
| F9 | **Preview frames wait for the busy main thread.** recv→val (a frame decoded on the reader thread → validation starts after main-thread admission) and deliver/pub→paint. | 24.6 ms p50 (41 ms p95) plus 4.9 ms plus 10.6 ms per frame at 10 pages. | Falls out of F3. Admission should not need the main thread (hop straight to the loader queue). | Expected: −25 ms p50 per frame | P3-APP-V3 (new loader) |
| F10 | **Pre-raster is whole-page and serial.** It reran on every frame for every page whose token changed, which with F2's 0 % reuse was every page. | 12.4 ms p50 per 10-page frame (≈1.2 ms per page at 2×). `GlyphRunRenderer.draw` is 146 ms over a 5 s typing run on all threads. | Tiles (#1228) for zoom memory. Raster only changed pages, which needs F2's stable per-page identity. Raster pages in parallel. | Expected: 12 → ~1.5 ms per keystroke frame | tiles (mac-claude-a) + F2 |
| F11 | **Idle is clean.** | A settled app (10 pages, worker attached) over 40 s: **1 ms** sampled on all threads (the Nearby listener's network path evaluator). Over 60 s: 320 ms app CPU during settling, 0 ms producer, 0 idle wake-ups in `top`'s 5 s sample. The only periodic UI is Nearby's 1 Hz `TimelineView`, shown only in that panel. `raw/idle-tp`, `raw/idle-plain10`. | none | — | — |
| F12 | **Memory ceilings** (read from code, not measured with Allocations). The page bitmap cache is `V2PageRasterizer.maxBytes` 192 MB and the prepared-page cache is `V2PageCache` 96 MB/64 pages. At 8 px/pt one whole page is 124 MB (#1228), so the bitmap cache holds one to two pages at high zoom. On all threads the reader appended each 13.5 MB line through `Data.append` (563 ms over the 10-page run). | see text | Tiles (#1228) and F2 remove the large items. | — | tiles, P3-APP-V3 |

### Keystroke → paint on the old engine (for the record, with preview)

`raw/base-plain-10-mid-paragraph` (load 6–17): 200 keystrokes, 76 paints, 124
coalesced.

| Stage (p50) | ms |
|---|---:|
| keystroke → compile request sent | 17.7 |
| send → v1 result (old-engine compile) | 33.2 |
| v1 result → display-list sibling decoded | 14.9 |
| decoded → validation starts (main-thread admission) | 24.6 |
| validate + decode + fonts + prepare (0/10 pages reused) | 67.7 |
| pre-raster (10 pages, 2×) | 12.4 |
| delivery to main | 4.9 |
| publish → paint | 10.6 |
| **keystroke → paint** (bench, all keystrokes) | **213** (p95 257) |

The §1.2 target is 16 ms p95. On the app side alone, about 120 ms of the 188 ms
timeline is app work (F2, F3, F9 and F10), not engine time.

## Fixes on this branch (before/after)

The before/after runs used one binary. A temporary environment switch restored
the old code paths for the "old" half, and it was removed before commit
(`scripts/ab-editor.sh`). The runs were editor-only (no producer), 200
keystrokes at 30 ms, with signpost p50s:

| Case | Before | After | Cells |
|---|---:|---:|---|
| Prose, 1000 pages, `editorChange` (F6) | 8.74 / 8.68 ms | **4.51 / 4.48 ms** | `raw/old{1,2}-mid_paragraph_-plain-1000-…`, `new{1,2}-…` |
| Equation line, 1000 pages, `editorChange` (F4) | 95.0 / 95.5 ms | 14.9 / 15.1 ms (scanner) → **9.8 ms** (scanner over the storage) | `raw/old{1,2}-_2__0_-plain-1000-…`, `new{1,2}-…`, `new3-…` |
| Equation line, 120 pages, `editorChange` (F4) | 12.3 / 11.9 ms | **2.2 / 2.3 ms** | `raw/old{1,2}-_2__0_-plain-120-…`, `new{1,2}-…` |
| First page, windowed documents (F1) | never within 60 s (120 and 1000 pages) | **1.66 s** (120), 47.6 s (1000; old-engine startup compiles, F5) | `raw/base-plain-{120,1000}-mid-paragraph`, `raw/fixwin-*` |

The loads during these pairs are in each cell's `load.txt`: 6–10 for the
1000-page pairs, 32–48 for the 120-page pairs, and 41–45 for the F1 runs.

Code:
- `apps/mac/Sources/FlashTeXMac/V2PageWindow.swift`: `fittedPageCount` and
  `servedFrame`. `ShellModel.displayListV2` calls `v2WindowFrameInstalled()`
  when a frame is published.
- `apps/mac/Sources/FlashTeXMac/EditorNavigation.swift`: `environmentPairs`
  uses `forEachEnvironmentUse`. `uses(in:)` is unchanged and still serves the
  other callers.
- `apps/mac/Sources/FlashTeXMac/SourceEditorView.swift`: `autoClose` checks
  `AutoClose.mayClose` first, and the environment-pair highlight scans the
  storage. It also carries the signpost hooks.
- `apps/mac/Sources/FlashTeXEditorCore/AutoClose.swift`: `mayClose`. `closer`
  now starts with the same guard, which changes no result.
- `apps/mac/Sources/FlashTeXMac/PerfSignposts.swift` (new) and hooks in
  `TypingBench.swift`, `ContentView.swift` (the binding) and `ShellModel.swift`
  (compile send).
- Tests: `AppPerfAuditTests` (pairs equal the old construction on 26 cases and
  3,000 random documents; `mayClose` is exact), and
  `V2WindowTests.testNarrowedWindowWithoutTheViewerReRequestsOnce`.

## Not measured (and why)

- **Continuous scroll and pinch-zoom hitches.** They were not run by this lane,
  because the Commander paused benchmark runs at load ~50. #1228 measured them
  in-app from the display link (`FLASHTEX_V2_SCROLL_BENCH`) with tiles:
  - 8 px/pt, 2,400 pt/s on a 2-page document;
  - a 512 px tile costs 0.24–0.32 ms and a new row of 6 tiles 0.68–1.86 ms;
  - after moving colour conversion to the tile queue, `CA::Render::copy_image`
    on main dropped from 148 samples to 3.

  Open item: the same bench on a 120-page windowed document, where scrolling
  across the window edge re-requests a compile (F1/F5).
- **Allocations template.** Not run for the same reason. The memory figures in
  F7 and F12 come from the code and the Time Profiler's `Data.append` samples.
- **"Animation Hitches" and "SwiftUI" templates** were not used: only Time
  Profiler and Allocations were allowed (no permission prompts).

## Reproduce

```sh
python3 docs/evidence/app-perf-2026-09-30/scripts/gen.py /tmp/appperf/docs
(cd apps/mac && swift build -c release)
CARGO_BUILD_JOBS=4 cargo build --release --bin flashtex-render   # in crates/render-pipeline
S=docs/evidence/app-perf-2026-09-30/scripts
$S/matrix.sh base plain-10 plain-120 plain-1000        # typing with preview + Time Profiler
$S/editor-only.sh base plain-1000 mid-paragraph        # editor-only
$S/editor-only.sh base plain-1000 '{2}+0'              # typing inside an equation line
$S/idle-profile.sh plain-10 /tmp/appperf/runs/idle-tp  # idle
python3 $S/startup.py /tmp/appperf/runs/*/app.log      # launch -> first page
```
