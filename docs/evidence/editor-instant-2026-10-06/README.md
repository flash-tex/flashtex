# APP-EDITOR-INSTANT: the editor under a streaming cold compile (2026-10-06/07)

The owner typed into a 1,000-page document (`book.tex`, 4.2 MB) while its first compile streamed pages
in: "editing the text in the editor portion of the window updates as slowly as the preview."
Gate: key event → glyph drawn ≤ 1 frame (8 ms at 120 Hz) p99, whatever the preview is doing.

## How it is measured

`EditorInstantTests.testTypingStaysInstantWhileAColdThousandPageCompileStreams` (apps/mac/Tests):

- A hosted window (ContentView, engine-v3 pane) holds a 4.2 MB owner-shaped book.
- A background thread feeds the session synthetic host events through the reader thread's own path
  (`EngineV3Delivery`).
- Keys are posted to the main run loop from another thread every 50 ms, as an input source arrives, so
  a key waits behind whatever main is doing.
- `MainThreadProbe` times named main-thread sections and stamps the editor's draws.

The four phases:

| Phase | What runs while typing |
|---|---|
| idle | nothing |
| cold | a 1,142-page cold compile (Infinite Descent ×2's page count) streamed at a page every 2 ms, with PROGRESS, PAGES and a warning every 10 pages |
| steady | a keystroke compile answering each key: its page, PAGES, 100 warnings, DONE |
| bursts | 3 keys, then 400 ms still (what a pause in typing starts) |

Measured per key:
- **key → drawn**: posted → the editor's next draw;
- **queue**: posted → handled;
- **handler**: the key's own main-thread time;
- **ping**: how long any event waits for main (a ping every 2 ms).

**These are GitHub-hosted macos-26 runs of `swift test`, a debug build.** Swift loops run 10–50× slower in
debug, so the absolute numbers are not the shipped app's. The before/after ratios and the attribution are what they
show. Release numbers on the M1 Max reference come from mac-claude-a, recipe on #1319, and are added here
when posted. Raw lines: [raw/](raw/) (one `EditorInstantPhase` line per phase; `raw/phases.py` prints the table).

## Result (CI, debug; key → drawn p50 / p95 ms)

| Phase | Before (main + harness) | #1664 inbox | #1665 DONE | #1666 keys | #1669 text | All six |
|---|---:|---:|---:|---:|---:|---:|
| idle | 180 / 430 | 158 / 277 | 136 / 298 | 48 / 197 | 91 / 204 | **28 / 50** |
| cold compile streaming | 250 / 498 | 126 / 200 | 155 / 371 | 48 / 71 | 96 / 210 | **44 / 77** |
| keystroke compiles | 9,909 / 217,617 | 5,854 / 9,770 | 159 / 241 | 6,686 / – | 4,672 / 7,413 | **125 / 372** |
| bursts | 149 / 341 | 104 / 256 | 240 / 543 | 50 / 69 | 128 / 254 | **40 / 55** |

Queueing (posted → handled) p50 during the cold compile: 229 ms before, 4.4 ms with all six.

## What held the main thread (attribution, same runs)

| Cost | Before | After | Fix |
|---|---|---|---|
| One block per host event; a PAGE past the laid-out pages laid out every page | cold: relayout 194 ms | 16–21 ms | #1664 |
| A DONE walked the 4 MB text from the start for each diagnostic's line | 4.2 s per DONE (debug) | 2–13 ms | #1665 |
| Chrome refresh rescanned every open text for `\input` (every 100 ms while typing or while pages arrive) | 200–400 ms per refresh (debug) | only when the document set changes or after 0.6 s still | #1666 |
| Capture bridge rebased per key with a byte loop over 4 MB (`changedRegion`, also used by the editor marks) | `modelUpdate` 36–51 ms/key (debug) | 5–12 ms | #1666 |
| Whole-buffer UTF-16→UTF-8 transcode per key (`nativeText`) | `bufferCopy` 6–11 ms/key (debug) | 0.0 | #1669 |
| COMPILE JSON-encoded and written to the socket on main (a 4 MB buffer at first compile); copy file written on main | not on the per-key path | off main | #1667 |
| Fold-triangle and outline rescans of the whole document when typing pauses | on main | off main | #1668 |

The capture bridge (`flashtex-bridge`) is attached in these runs, as in the shipped app (it is bundled and
auto-attaches): its per-key `edited` is what the `modelUpdate` section measured.

## Still open

- **Release numbers on the reference Mac.** The 8 ms gate cannot be decided from debug CI.
- **Keystroke compiles with 100 warnings each.** With all six fixes, steady p95 is still 372 ms in debug.
  The time is outside every probed section: SwiftUI's re-render of the editor marks and the Problems
  panel after each DONE. That is the next item.
- **The handler itself.** With all six fixes it is 20–30 ms in debug. `textDidChange` is 6 ms of that; the
  rest is NSTextView's insert and the storage observers (syntax lexing).
