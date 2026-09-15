# FT-071 Mac IDE hot path: window invalidation per keystroke, 2026-09-13

Lane `mac-perf-2` (Claude Code subagent of `mac-claude-a`), machine
`mac-m1max-a` (Apple M1 Max, macOS 26.3.1). Branch
`agent/mac-perf-2/window-invalidation`, base `origin/main` `d46a0f63`. Time
box 75 minutes. Other agents built on this machine throughout: the 1-minute
load was 6–51 during the cells (recorded per cell; every cell is
load-affected by the bench's own rule, so treat the numbers as directional,
not as a gate). Follows `perf-mac-2026-09-13T1740Z` (mac-perf-1), whose
handoff this lane worked.

## Summary

- **Keystroke → paint on HW1 (`run.sh --at first-paragraph`, V2 pane, 30 ms
  interval): p50 145 ms → 65–75 ms; demo 108 → 65–76 ms.** Same producer
  binary, same bench, same paint point. Not yet the 30 ms target.
- The single biggest cost was **not** the status bar / header labels the
  handoff listed (fixing those alone moved p50 by ~10 %, step 1): it was the
  App scene's **menu commands** reading `model.result == nil` and
  `model.displayListV2?.frame == nil` for `.disabled()`. Every producer reply
  and every v2 frame re-evaluated the *App* graph, and SwiftUI then re-read
  the window's root preferences (`AppKitWindowController.updateRootView` →
  `GraphHost.preferenceValues` → `ResolvedTextFilter`): 660–880 of ~3 900
  busy main-thread samples in a 5 s window. Reading the change-only
  `toolbarHasResult` / `toolbarHasV2Frame` mirrors instead removed it entirely
  (step 2: 9 samples) and halved keystroke → paint.
- What remains (step 3 sample): AppKit auto-layout of the hosting view's
  subtree — `CA::Transaction::commit` 61 % of busy, `NSHostingView.layout()`
  / `layoutIfNeeded` 22 %, SwiftUI `LayoutEngineBox.sizeThatFits` (stack
  layout of the whole content tree, ~2 600 samples) driven by the editor
  pane's inherent per-keystroke update (`SourceEditorView.updateNSView`,
  the line-number gutter redraw) and the V2 pane's per-keystroke caret
  update plus per-frame publish. The main thread is still ~70 % busy at 30 ms
  typing; see the handoff.
- **body60k** on this branch: the worker attached and answered (118 / 166 ms
  compiles, `recovered`), but the bench never saw a first paint within its
  60 s attach window (load 37 → 51 while `swift test` built in parallel);
  the cell is recorded as not measured, not as a number. The v2 route
  declines that document anyway (perf-1 README, wire size); the 60 ms goal
  for it is blocked on the delta/compact contract (FT-070), not on this lane.

## Keystroke → paint (`tools/typing-bench/run.sh --producers render --intervals 30 --at first-paragraph`)

Release `FlashTeXMac`, `flashtex-render` from this worktree
(`crates/render-pipeline/target/release`, source `d46a0f63`), V2 pane. Files:
`typing-bench-base.md`, `typing-bench-step1.md`, `typing-bench-step2.md`,
`typing-bench-step3.md`; raw JSON under `raw/typing-bench-*`.

| step | commit | seed | keys | painted | coalesced | k→p p50 | p95 | p99 | compile p50 | load |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| base | d46a0f63 | hw1 | 200 | 200 | 0 | **145** | 200 | 263 | 24 | 43 → 25 |
| base | d46a0f63 | demo | 200 | 200 | 0 | **108** | 156 | 174 | 18 | 25 → 18 |
| 1 chrome mirror | 02205529 | hw1 | 200 | 200 | 0 | 132 | 196 | 209 | 11 | 6.8 → 5.8 |
| 1 chrome mirror | 02205529 | demo | 200 | 200 | 0 | 115 | 157 | 186 | 22 | 5.8 → 5.2 |
| 2 + commands mirrors | 8d955312 | hw1 | 200 | 184 | 16 | **65** | 184 | 246 | 19 | 33 → 32 |
| 2 + commands mirrors | 8d955312 | demo | 200 | 185 | 15 | **65** | 147 | 205 | 18 | 32 → 32 |
| 3 + sidebar/diagnostics lists | 9c32aaac | hw1 | 200 | 173 | 27 | 75 | 171 | 195 | 20 | 23 → 29 (a) |
| 3 + sidebar/diagnostics lists | 9c32aaac | demo | 200 | 173 | 27 | 76 | 226 | 321 | 24 | 29 → 37 (a) |
| 3 | 9c32aaac | body60k | 0 | 0 | 0 | — | — | — | 166 | 37 → 51 (a): no first paint within 60 s |

(a) `swift test` (debug build of the package + the test run) ran on the same
machine during step 3; step 2 ran on a quieter machine. Step 3 is not
expected to be slower than step 2 by construction (it removes three more
per-reply/per-keystroke List re-evaluations); the difference is load.

Base ran at load 25–43 too, so the base → step 2 comparison (145 → 65) is
between two loaded runs; step 1 (load 6) versus base shows the chrome mirror
alone is worth ~10 %.

## Main thread while typing HW1 (`sample <pid> 5 1`, 5 000 ms at 1 ms; `raw/main-thread-summary-*.txt`, `raw/sample-hw1-*.txt.gz`, `raw/sample-summary.py`)

"busy" = main-thread samples not in `mach_msg`; the percentages are of busy.
"app code" counts every sample whose outermost app frame is inside
`FlashTeXMac` (so it includes the SwiftUI/AppKit work *called from*
`updateNSView` and the gutter draw), which is why it grows once the pure
SwiftUI churn is gone.

| build | busy % of wall | SwiftUI graph updates (`flushObservers`) | `CA::Transaction::commit` | of which `layoutIfNeeded` / `NSHostingView.layout` | `AppKitWindowController.updateRootView` | app code |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base d46a0f63 | 71 % (3 558) | 73 % (2 611) | 24 % (869) | 16 % (559) | (inside graph updates) | 6 % (212) |
| step 1 02205529 | 79 % (3 940) | 75 % (2 940) | 23 % (891) | 14 % (563) | 883 (22 %) | 5 % (210) |
| step 2 8d955312 | 72 % (3 615) | 36 % (1 292) | 51 % (1 848) | 22 % (791) | 9 | 21 % (764) |
| step 3 9c32aaac | 69 % (3 467) | 27 % (953) | 61 % (2 108) | 22 % (751) | 0 | 22 % (757) |

Step 1's higher busy % is the bench running more keystroke cycles per
second on a quieter machine, not a regression: the per-keystroke p50 fell.

Top app frames after step 3 (outermost, samples): `NSTimer` thunks 162 (the
bench's 30 ms typing timer plus the chrome refresh timer), run-loop block
thunks 160 (v2 deliver / paint hops), `SourceEditorView.updateNSView` 143,
`LineNumberGutter.drawHashMarksAndLabels` 111, `List` row closure 70 (the
sidebar List's rows, re-diffed when the outline staleness flag flips at
≤ 10 Hz), `PageBitmapLayer` 19. No view `body` above 12 samples.

## Changes (all in `apps/mac/Sources/FlashTeXMac`; every hunk listed)

Commit `02205529` (step 1):
- `ShellChrome.swift` (new): `@Observable` mirror of what the chrome shows
  (`editorRevision`, latency label/help, route, problem counts, notes,
  capture note, durable revision, preview source/result help/status,
  `compiling`, historical label, stale text, load error, capability notes,
  accepted capabilities, active text byte/UTF-16 counts, project listing,
  include closure). `refresh(from:)` assigns a field only when it changed;
  `compiling` stays on for one extra interval so a fast reply never flickers
  the indicator. Interval 100 ms (`FLASHTEX_CHROME_MS`; 0 = next turn).
- `ShellModel.swift`: `chrome`, `refreshChrome()` under
  `withObservationTracking` (the first change to anything the refresh read
  arms one run-loop `Timer`), `flushChrome()`; `problemsList` /
  `resultStatus` change-only mirrors (in `refreshToolbarMirrors`); armed at
  the end of `init`; `handle(.result)` writes `previewSource`,
  `historicalPreview` and `selection = nil` only when they change;
  `bindLayout` writes `negotiation` / `fontSubstitutions` /
  `layoutDiagnostics` only when they change.
- `ContentView.swift`: `PreviewHeader` and `StatusBar` read `model.chrome`
  (parent-retained file: it is this lane's hot path); the compiling
  `ProgressView` lives in a fixed 12×12 slot; `EditorPane` skips equal
  `caretLengthUTF16` writes.
- `ProblemsPanel.swift`: reads `problemsList` / `resultStatus`.
- `WorkspaceSidebar.swift`: outline staleness and the rescan task key read
  `chrome.editorRevision`; `ProjectSection` reads `chrome.listing` /
  `chrome.closure`.
- `DocumentTabBar.swift`: tabs from `chrome.listing`, byte counts from
  `chrome`.
- `WordCountStatusView.swift`: rescan trigger on `chrome.editorRevision`.
- `SourceEditorView.swift` (`updateNSView`): `compileResult` /
  `editorRevision` pushed to the completion view only when changed.
- `PreviewV2View.swift`: `stale` removed from `PageV2View ==` (nothing drawn
  depends on it).

Commit `8d955312` (step 2):
- `FlashTeXMacApp.swift` (`.commands`): `.disabled(model.result == nil)` ×2
  → `!model.toolbarHasResult`; `.disabled(model.displayListV2?.frame == nil)`
  → `!model.toolbarHasV2Frame`; comment explaining why.
- `Navigation.swift` (`NavigationCommands`): the three
  `.disabled(model.result == nil)` → `!model.toolbarHasResult`.

Commit `9c32aaac` (step 3):
- `ShellChrome.swift`: `carriedLine`, `entryPath`.
- `WorkspaceSidebar.swift` (`ProblemsSection`): `displayedDiagnostics` →
  `problemsList`.
- `DocumentTabBar.swift` (`ProjectMenu`): `discoverClosure()` /
  `project.entryPath` → `chrome.closure` / `chrome.entryPath`.
- `DiagnosticsPanel.swift` (`DiagnosticsListView`): document order from
  `chrome.listing`, status from `resultStatus`, retention line from
  `chrome.carriedLine`.

Behaviour: every model property, status string and log line is unchanged;
only the chrome's labels lag by ≤ 100 ms and menu enablement follows the
presence (not the identity) of a result / frame, which is what it tested.

## Tests

`swift test --filter 'PreviewV2|RenderingV2|WorkspaceShell|PreviewLatency|TypingBench|CommandTable|EditorDiagnostics|Completion'`
with `FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1
FLASHTEX_REVIEW_HISTORY_DIR=off FLASHTEX_RENDER=<worktree release binary>`
(`raw/swift-test.log`): 155 tests, 15 skipped, **28 failures, all in the
completion vocabulary fixtures** (`CompletionTests` ×6,
`CompletionLatencyTests` ×2, and
`SourceEditorViewTests.testCompositionCancelDropsTheStepsAndClosesTheCompletionList`,
which asserts on the same `\si[options]{units}` vs `\section{...}` ordering
and the `beyondCompiler` lists) — the known stale fixtures another lane is
fixing; not touched here. `PreviewV2Tests`, `RenderingV2*`,
`WorkspaceShellTests`, `PreviewLatencyTests`, `TypingBenchTests`,
`CommandTableTests`, `EditorDiagnostics*` all pass.

## Handoff (what is left between 65–75 ms and 30 ms)

1. **AppKit layout of the hosting view per transaction** (61 % of busy after
   step 3). Two transactions per keystroke remain inherent as the code is
   structured: the keystroke itself (`documents` / `editorRevision` /
   `caretUTF16` → `EditorPane` + the V2 pane's caret highlight) and the v2
   publish (`displayListV2` → `PreviewV2Pane`). Each runs `NSHostingView`
   `layout()` / `minSize()` over the whole content tree
   (`LayoutEngineBox.sizeThatFits` → `StackLayout`, `_FlexFrameLayout`).
   Candidates: give the editor and preview panes fixed frames from the
   `HSplitView` (so a child update cannot propose a new size upward), host the
   editor `NSScrollView` in its own `NSHostingView`-free container, and move
   the caret highlight into the page's `PageBitmapView` (an AppKit overlay
   updated directly) so the caret move never re-evaluates `PreviewV2View`.
2. **`PreviewV2Pane` publish path**: `.loading` then `.loaded` are two writes
   per keystroke; when the v2 line arrives in the same run-loop turn as the
   result they coalesce, otherwise not. Writing `.loading` only when there was
   no previous frame (the stale flag no longer affects the pages) would make
   the pane re-evaluate once per frame.
3. **Sidebar `List` rows** (70 samples): the outline section's `stale` flag
   flips at ≤ 10 Hz and the List re-diffs its rows; compute staleness inside
   `OutlineSection` from a `chrome` field so the List itself is untouched.
4. The `perf-1` items stand: v1 `pages` elision when v2 is accepted, and the
   delta contract for the 60 KB body.
