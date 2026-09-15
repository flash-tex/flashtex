# Typing bench: keystroke → paint latency (2026-09-13T174559Z)

Branch `agent/mac-render-pipeline/perf-1` @ `8465bbd0`; Apple M1 Max; macOS 26.3.1; release build of `FlashTeXMac` (`swift build -c release`).
Script: `tools/typing-bench/run.sh`; raw JSON summaries in `typing-bench-2026-09-13T174559Z/`.

Producers:
- render: crates/render-pipeline/target/release (this checkout)

## Results

Latency is keystroke → paint per typed character (ms); a coalesced keystroke is measured to the first paint that showed it. `compile` is the shell's send → result time on the main thread; `render` is PreviewView body → last page Canvas draw. A cell marked ⚠ ran while the 1-minute load average exceeded the limit (15.0) before or after it; such cells are reported, never used as a gate.

### Route `render`

`render`: direct worker route with `flashtex-render` (render-pipeline branch) as the runtime-v1 worker.

| route | producer | seed | bytes | interval | keys typed | paints | coalesced | k→p p50 | p95 | p99 | max | compile p50 | compile p95 | render p50 | render p95 | unpainted | load before/after |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| render | flashtex-render | body60k | 64103 | 30ms | 200 | 72 | 47 | 229 | 416 | 677 | 941 | 90 | 138 | 84 | 93 | 82 | 7.4 / 10 |
| render | flashtex-render | demo | 6113 | 30ms | 200 | 35 | 165 | 507 | 1306 | 1568 | 1722 | 22 | 37 | — | — | 0 | 10 / 7.4 |

Typed script: `tools/typing-bench/typed-200.txt` (200 characters, inserted before `\end{document}` when present, else at the end).
Seeds: `demo` = `apps/mac/Samples/demo.tex`; `body60k` = the demo's paragraphs repeated to ≥ 60 KB in one document; `fixture` = the entry document of `protocol/fixtures/compile-request.json`.

## Methodology

- The app is launched with `FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_SEED_FILE=<seed> FLASHTEX_TYPING_BENCH=<script>` plus the route's environment (`FLASHTEX_COMPILER=<worker>` for the direct routes; `FLASHTEX_PREVIEW_CONTROLLER=<helper>` with a fresh temporary `FLASHTEX_CONTROLLER_LEDGER_ROOT` per cell for the durable route; `FLASHTEX_PREVIEW_V2=1` for the v2 pane). After the producer attached and its first result was painted, `TypingBenchDriver` (`apps/mac/Sources/FlashTeXMac/TypingBench.swift`) inserts the script one extended grapheme cluster at a time into the real editor `NSTextView` through `insertText(_:replacementRange:)` from a main-run-loop `Timer` at the configured interval (30 ms ≈ a fast typist; 0 ms = one keystroke per run-loop turn, a burst). Each insertion takes the production path: `NSTextViewDelegate.textDidChange` → SwiftUI binding → `ShellModel.updateActiveText` (revision bump, `keystroke:` log line) → auto-compile / `edit` submission (one in flight, newest buffer coalesced) → result → `PreviewView` render.
- Keystroke time is stamped immediately before `insertText` on the monotonic clock (`clock_gettime_nsec_np(CLOCK_UPTIME_RAW)`, i.e. `mach_absolute_time` in ns). For a person typing, the same recorder uses the `NSEvent.timestamp` of the `keyDown` seen by an in-process local event monitor (HID time on the same clock) — no Accessibility permission or event tap is involved, and a key that changes no text is discarded at the end of its dispatch.
- Paint time (`paint:` log line): first main-queue turn after the run-loop iteration whose SwiftUI render pass evaluated PreviewView for the revision (Canvas draw closures of every page ran inside that pass); the CoreAnimation commit has completed, the display's next vsync scan-out is not observed. A revision whose result changed nothing visible (SwiftUI skipped the canvas redraw) is still recorded as painted, flagged `redrawn: false`.
- A paint of revision N makes every unpainted keystroke with revision ≤ N visible; those with revision < N are `coalesced`. p50/p95/p99 are nearest-rank percentiles over the per-keystroke latencies. The run ends when every keystroke is painted (or after the settle timeout), and the app writes the JSON summary and exits.
- Machine load: `sysctl vm.loadavg` (1-minute average) is recorded before and after every cell; run.sh waits for the load to drop below `--quiet-load` before each cell (bounded by `--quiet-wait`) and flags cells whose before/after load exceeded `--load-limit`.

## Limitations

- The bench inserts text programmatically: there is no OS keyboard event, no event-queue wait, no key repeat and no input-method composition; real typing adds the HID → WindowServer → `NSApplication.sendEvent` hop, which the local-monitor path measures but this bench cannot.
- `paint` is the completed CoreAnimation commit, not the display scan-out: the pixels reach the panel at the next vsync (up to one frame, 8–17 ms at 60–120 Hz) after the stamp, and later still if the render server is behind. No IOSurface presentation callback is observed. The window is ordered back (`FLASHTEX_NO_ACTIVATE=1`) and may be occluded during the run; commits still happen, on-screen visibility is not verified.
- Compile time is measured on the main thread from send to result application, so it includes any time the reply waited behind main-thread work; for the durable route it is edit submission → preview `update` applied (the helper's fsync and compile are inside it).
- Typing stops after `FLASHTEX_TYPING_BENCH_MAX_MS` (120 s here); a cell marked 'of 200 (budget)' typed fewer characters because each keystroke waited for main-thread work.
- One machine, one run per cell, no warm-up discard beyond the first compile; numbers are indicative, not a regression gate. Load-affected cells are marked, not excluded.
