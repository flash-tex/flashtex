# P5-KEYSTROKE-MAIN: main-thread work per keystroke (2026-10-03)

Lane P5-KEYSTROKE-MAIN (mac-claude-a, mac-m1max-a: Apple M1 Max, macOS 26.3).
Branch `agent/mac-claude-a/keystroke-main` from `origin/main` `6b6d5edf9`.
The gap is row "Main-thread work per keystroke" of
[app-parity-2026-10-03](../app-parity-2026-10-03/README.md#main-thread-per-keystroke-measured)
(F3 of [app-perf-2026-09-30](../app-perf-2026-09-30/README.md)): 40.7 ms of main-thread
time per keystroke, 91 % busy, measured at load 155–233.

## Result

Engine-v3 pane, 200 keys every 30 ms into `plain-10`, the owner's editor settings
(wrapping off, Menlo 16). Three interleaved before/after pairs; medians, with the
range in brackets. The rows overlap: a stack is counted under every frame it contains.

| Main thread, per keystroke | Before (`origin/main`) | After |
|---|---:|---:|
| **Total** | **41.0 ms** [40.5–41.6], 96 % busy | **11.0 ms** [9.5–14.1], 35 % busy |
| `GraphHost.flushTransactions` (SwiftUI) | 11.7 | 3.5 [2.3–5.6] |
| `CA::Transaction::commit` | 20.6 | 4.8 [4.1–5.7] |
| — `NSHostingView.layout()` | 4.9 | 2.3 |
| — `CompletingTextView.draw` (the editor's text) | 11.8 | 1.2 |
| — `LineNumberGutter` drawing | 1.1 | 0.01 |
| `EngineV3*` frames (the pane's own work, not changed here) | 2.8 | 2.8 |
| `textDidChange` | 0.6 | 0.6 |

The other cells, same method:

| Cell | Before | After |
|---|---:|---:|
| v2 pane, editor only (TypingBench, no compiles), 2 pairs | 36.5 / 36.8 ms, 95 % | 10.4 / 10.4 ms, 33 % |
| v3, 100 keys every 60 ms (both keep up), 2 pairs | 47.0 / 44.8 ms, 75 / 72 % | 10.8 / 11.4 ms, 17 / 18 % |
| v3, SwiftUI changes only (`FLASHTEX_EDIT_TAIL=0 FLASHTEX_LONG_LINE_CLIP=0`), 2 runs | 40.7 (paired run) | 27.6 / 27.5 ms, 86 % |

The ablation splits the gain: the SwiftUI changes take the flush from 11.7 to 2.3 ms
and the hosting-view layout from 4.9 to 1.5 ms; the editor changes take the text
drawing from 12.8 to 1.2 ms and the gutter from 1.2 to 0.

**Keystroke → first changed page committed** (`EngineV3Bench`) is not worse. At
60 ms, where both builds keep up with the typing: p50 26.6 → 26.2 ms, p95 31.5 →
30.7 ms (pair 2; pair 1's "before" ran during a load spike to 60). At 30 ms the
"after" p95 is higher (72 → 102 ms, medians), because the "before" app could not
type at 30 ms: its 200 keys took 8.5 s instead of 6.3 s, so the host compiled less
often. The host, not the main thread, bounds that number now.

**Machine state.** The load average was 19 when the lane started (`uptime` 05:46,
against 155–233 for the parity measurement) and 13–43 during these cells; each
cell's load before and after is in [raw/env.txt](raw/env.txt). Pairs ran back to
back, before then after.

## What re-evaluated per keystroke, and why

Counted with a temporary body counter in every view and a probe that tracked
every observed property of `ShellModel`, `EngineV3Session`, `ProjectDocuments` and
`ShellChrome` (40 keystrokes, v3, before the fixes):

| View | Bodies per 40 keys | Cause |
|---|---:|---|
| the App scene (every menu) | 40 | File ▸ Move To… read `project.entryPath`, which reads `documents` |
| `ContentView`, `ToolRail` + 2 `RailButton`s | 40 / 40 / 80 | the App scene re-created the window's root |
| `WorkspaceSidebar`, `ProjectSection`, its header | 67 each | `WorkspaceSplitPane.updateNSViewController` replaced the sidebar's root on every `ContentView` update; the sidebar's caret-follow `.task(id:)` read `caretUTF16` in its body |
| `StatusBreadcrumb` | 68 | `.task(id: model.caretUTF16)` in its body |
| `WordCountStatusItem` | 66 | `.onChange(of: model.documents.count)` and its help text read `documents` |
| `StatusBar`, `PreviewHUD` | 27 | `ShellChrome.refresh` assigned the old engine's `route`, `routeHelp`, `hasResult`, `compiling`… and then the v3 values: two flips per refresh, each an invalidation |
| `PreviewV3Pane` | 12 | the compile status line and the scroll view were one body |
| `EditorPane` | 40 | the editor's inputs (`editorRevision`, the text binding): expected |

After the fixes, the same count: `EditorPane` 40; the status line under the v3
pages 16 (its text changes with each compile); the tab bar, the preview header and
the Project tree 15–23, at most once per 100 ms chrome refresh, because this bench
types and then deletes a letter, so the document flips between edited and saved,
which those views show; everything else 0 or 1.

**The text view.** TextKit 1 invalidates the display from the edited line to the
end of the document on every keystroke. [scripts/nstextview-tail.swift](scripts/nstextview-tail.swift)
shows a plain `NSTextView` does the same ([raw/nstextview-tail.txt](raw/nstextview-tail.txt):
a 2,812 pt rect per keystroke on a 2,945 pt document; non-contiguous layout makes it
worse). With wrapping off every paragraph is one line fragment of about 1,000
glyphs, and the layout manager draws every glyph of each line fragment the dirty
rect touches, plus every spelling underline (a pattern image each): 11.8 ms per
keystroke to redraw the visible lines below the caret, none of which had changed.

## What changed

| File | Change |
|---|---|
| `FlashTeXMacApp.swift`, `ShellModel.swift` | Move To… reads `menuEntryPath`, a change-only mirror. `documentCount` likewise for the word count. |
| `WorkspaceSplit.swift` | The sidebar's root is replaced only when its inputs (visibility, model, Nearby) change. |
| `IsolatedTask.swift` (new) | `IsolatedTask` / `IsolatedOnChange`: `.task(id:)` / `.onChange(of:)` whose value is read in a zero-size child, so only the child re-evaluates. Used by the sidebar's outline and caret follow, the breadcrumb and the word count. |
| `ShellChrome.swift` | `refresh` assigns the preview fields once, from the engine on screen (`refreshOldEngine` or `refreshEngineV3`). |
| `EngineV3Preview.swift` | `PreviewV3Pane` is two children: the pages (`PreviewV3Scroll`) and the status line (`PreviewV3StatusHUD`). |
| `EditorEditTail.swift` (new), `Completion.swift`, `SourceEditorView.swift`, `EditorIntelligence.swift` | The edit tail: when an edit stays inside one paragraph and the paragraph's last line ends at the same y, the redraw below it and the gutter's redraw are dropped, unless a temporary attribute, glyphs or layout at or after the paragraph's end changed during the edit (`EditTailLayoutManager` records them) or the invalidation did not come from the layout manager. Long line fragments draw only the glyphs near the clip rect (binary search on glyph x; lines with right-to-left text are drawn whole). `FLASHTEX_EDIT_TAIL=0` and `FLASHTEX_LONG_LINE_CLIP=0` turn each off. |
| `ViewBodyProbe.swift` (new) | A body counter for tests (one `Bool` test when off) in the ten large views. |

Nothing that shows on screen was removed or delayed: the breadcrumb, outline and word
count follow the same values with the same debounces; the status bar shows the same
mirrors; the gutter is redrawn whenever an edit moves a line, and its numbers, fold
marks and dots redraw themselves when they change.

## Tests

- `KeystrokeInvalidationTests` (5): typing into the hosted `ContentView` re-evaluates
  `EditorPane` and none of `ContentView`, `ToolRail`, `WorkspaceSidebar`,
  `ProjectSection`, `PreviewPane`, `StatusBreadcrumb`, `WordCountStatusItem`
  (`StatusBar` fewer than once per keystroke); the App menus' mirrors fire no
  observation on a keystroke and follow real changes; an unchanged chrome refresh
  fires nothing; `IsolatedTask` runs once per id without re-evaluating its parent.
  Adding one `model.caretUTF16` read back into `StatusBreadcrumb` fails the first test.
- `EditorEditTailTests` (8): typing and deleting inside a long line invalidates that
  line only; a line break, a temporary attribute added below during the edit, and a
  wrapped paragraph that gains a line all keep the redraw below; an edit that never
  reaches `didChangeText` is flushed at the next display; the gutter is redrawn only
  when an edit moves lines; long lines drawn near the clip rect have the same pixels
  as drawn whole (scrolled into the middle and at the start), and a right-to-left line
  is drawn whole.
- `swift test` over the related suites (completion, editor, folding, conceal, Vim,
  spelling, error lens, line wrapping, outline, print, workspace, TypingBench,
  AppPerfAudit, every `EngineV3*` suite): 547 tests, 0 failures, 12 skipped.
- `scripts/gate.sh pr`: passed.

## What is left

- `EngineV3*` frames, 2.8 ms per keystroke: the v3 pane's own work (the edit
  request, page installs). Not touched here.
- `EditorPane` re-evaluates once per keystroke (about 1–2 ms of flush and hosting
  layout): `SourceEditorView` takes `editorRevision` and the completion metadata is
  bound to it on every update. Removing it needs the editor to read the revision
  lazily; a separate change.
- The edited line itself: about 1.2 ms with wrapping off, a third of it AppKit's
  `NSTextCheckingController` over the drawn range.

## Reproduce

```sh
python3 docs/evidence/app-perf-2026-09-30/scripts/gen.py /tmp/docs
(cd apps/mac && swift build -c release --product FlashTeXMac)
cargo build --release -p flashtex-engine --bin flashtex-host
S=docs/evidence/keystroke-main-2026-10-03/scripts
$S/ab.sh /tmp/km/v3 /tmp/docs/plain-10.tex 3 <before>/FlashTeXMac <after>/FlashTeXMac
$S/ab.sh /tmp/km/v3-swiftui-only /tmp/docs/plain-10.tex 2 <before>/FlashTeXMac <after>/FlashTeXMac swiftui-only "FLASHTEX_EDIT_TAIL=0 FLASHTEX_LONG_LINE_CLIP=0"
CELL=v2type.sh $S/ab.sh /tmp/km/v2 /tmp/docs/plain-10.tex 2 <before>/FlashTeXMac <after>/FlashTeXMac
KEYS=100 MS=60 $S/ab.sh /tmp/km/v3-60ms /tmp/docs/plain-10.tex 2 <before>/FlashTeXMac <after>/FlashTeXMac
```

`v3type.sh` and `v2type.sh` launch the release app under `xctrace record --launch`
(Time Profiler) with `FLASHTEX_NO_ACTIVATE=1`; the app exits when its bench is done,
and a watchdog ends an `xctrace` that never launched it (it happened once: the first
"before" of the ablation set has no trace). `ab.sh` summarises each cell with
[`mainthread.py`](../app-perf-2026-09-30/scripts/mainthread.py). The benches use the
user's editor defaults; do not pass `-FlashTeX.EditorPreferences…` arguments to
change them, because the app writes its preferences back. The `.trace` bundles
(about 2 GB) are not in git; the per-cell summaries are in [raw/](raw/).
