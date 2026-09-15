# FT-071 Mac producer + IDE hot path: measurements, 2026-09-13

Lane `mac-perf-1` (Claude Code subagent of `mac-claude-a`, Fable), machine
`mac-m1max-a` (Apple M1 Max, 10 cores, macOS 26.3.1). Branch
`agent/mac-render-pipeline/perf-1`, base `origin/main` `8465bbd0`. Time box
90 minutes; other agents were building on this machine throughout (1-minute
load 6–24 during the runs, recorded per cell).

Scope: `crates/render-pipeline` (measured only; no Rust change was needed —
see byte identity), `apps/mac` (the V2 pane, IPC, toolbar), `tools/typing-bench`.

## Summary

1. **The producer is not the keystroke bottleneck.** `flashtex-render` answers a
   warm HW1 keystroke with `display-list-v2` in **3.9 ms in-process** (p50, 40
   steps, under load 24) and **4.7 ms measured over stdin/stdout** from a
   driver process (raw/producer-ipc-base.txt). Target "warm HW1 keystroke < 5 ms
   in-process": met by the current main (FT-065 landed the wins).
2. **The Mac main thread is saturated while typing into the V2 pane.** A 1 ms
   `sample` of the app during the demo bench shows the main thread busy 79–80%
   of wall time, of which **< 2% is app code**; the rest is SwiftUI
   view-graph updates (`NSRunLoop.flushObservers` 54–60%) and AppKit/CoreAnimation
   window layout (`CA::Transaction::commit` → `NSWindow layoutIfNeeded`, 18%).
   Every keystroke costs 3–4 whole-window SwiftUI transactions (editor revision,
   status lines, compile result, v2 publish), each 10–20 ms on this window, so
   at 30 ms typing intervals the v2 deliver and publish blocks wait 40 ms each.
3. **Landed:** the window toolbar no longer re-evaluates per request
   (`ShellModel` change-only mirrors, `ContentView.WorkspaceToolbar`): the
   `AppKitWindowController.updateRootView` preference update fell from 960 to
   289 samples and the `NSToolbarItemViewer` layout (255) disappeared from the
   profile; demo bench p50 at the same paint point 507/514 → 409 ms (fallback
   paint point, see 5). This is a real but partial win; the remaining cost is
   the content hosting view's transactions (handoff below).
4. **Wire size is the 60 KB blocker, not layout time.** The v2 display list is
   ~1 MB per page (demo: 2 pages = 1.94 MB; HW1: 3 pages = 1.15 MB) because
   every cluster carries `carets`, `hit_rects` and `sources`; the 60 KB body
   (~24 pages) would be ~24 MB, over the 16 MB line cap, so the producer
   declines `display-list-v2` (`display_list_declined`) and the bench's body60k
   cell runs on the v1 pane (v1 reply 2.49 MB, 27 ms warm producer round trip).
   No Mac-side optimisation reaches the 60 ms target for that document while
   the whole list is re-sent per keystroke; the delta contract
   (`crates/render-pipeline/docs/proposals/display-list-v2-delta.md`) or a
   compact encoding is required (FT-070 handoff).
5. **The typing bench does not observe the edited page on the V2 route.** The
   script types before `\end{document}`, i.e. on the last page, which the
   pane's `LazyVStack` never materialises at the bench window size; the
   recorded "paint" is the bench's extra-turn fallback on frames that changed
   no visible page (165 of 200 keystrokes coalesced, `redrawn false`). Added
   `FLASHTEX_TYPING_BENCH_AT` / `run.sh --at first-paragraph` and an `hw1`
   seed so a run can type on a visible page; numbers below keep the old
   position for comparability with the 2026-09-12 baseline.

## In-process producer (`examples/perf_bench`, 40 steps, load 24.5, release)

`raw/perf-bench-hw-base.txt`; digests `raw/digests-hw.txt` (HW1/HW2 v2 md5:
`cc8f0092abe2629ad5bdad90be978e3e`, `537ac1a5ab319c66cdf91a8bd93f5e0e`,
`raw/producer-ipc-base.txt`). No Rust source changed in this lane, so the
digests are those of `8465bbd0` by construction; `cargo test --release --test
incremental` passes (1 passed, 1 ignored).

| scenario | p50 ms | p95 ms |
| --- | ---: | ---: |
| HW1 full | 7.40 | 56.66 (load spike; max 347) |
| HW1 type-paragraph | 2.76 | 4.77 |
| HW1 type-paragraph +v2 | 3.86 | 4.04 |
| HW1 type-inline-math +v2 | 3.74 | 3.92 |
| HW2 type-paragraph +v2 | 4.02 | 4.20 |
| HW1 full +v2 | 6.90 | 7.26 |
| HW2 full +v2 | 6.64 | 7.12 |

## Producer over stdin/stdout, one request per keystroke (`ipc_bench.py`, 20 steps)

Send → last reply byte, from a Python driver (adds the driver's own parse of
nothing: it only reads lines), load 5.8.

| seed | caps | cold first request | warm p50 / p95 ms | v1 line | v2 line |
| --- | --- | ---: | ---: | ---: | ---: |
| HW1 (5.1 KB, 3 pages) | +v2 | 66.6 ms | 4.67 / 5.11 | 185 KB | 1.15 MB |
| HW1 | v1 only | 66.1 ms | 2.54 / 2.91 | 185 KB | — |
| demo (5.9 KB, 2 pages) | +v2 | 26.4 ms | 6.23 / 6.76 | 222 KB | 1.94 MB |
| demo | v1 only | 23.6 ms | 3.00 / 3.26 | 222 KB | — |
| body60k (64 KB) | +v2 | 60.3 ms | 27.4 / 29.1 | 2.49 MB | declined (est. > 16 MB) |
| body60k | v1 only | 57.4 ms | 26.3 / 82.0 | 2.49 MB | — |

The cold first request is font loading (HW1 66 ms here under load; FT-065
measured 16–24 ms on a quiet M5).

## End-to-end keystroke → paint (`tools/typing-bench/run.sh --producers render`, 30 ms interval)

Same producer binary (`8465bbd0` render-pipeline, release), release
`FlashTeXMac`, V2 pane (default). `typing-bench-base.md` /
`typing-bench-after.md`, raw JSON under `raw/typing-bench-*`.

| build | seed | keys | painted | coalesced | k→p p50 | p95 | p99 | compile p50 | unpainted | load |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| base 8465bbd0 | demo | 200 | 35 paints | 165 | 507 | 1306 | 1568 | 22 | 0 | 10.4 → 7.4 |
| base 8465bbd0 | body60k (v1 pane: v2 declined) | 200 | 72 | 47 | 229 | 416 | 677 | 90 | 82 | 7.4 → 10 |
| after (this lane) | demo | 200 | 32 paints | 168 | 460 | 1135 | 1359 | 50 | 0 | 8.5 → 18.1 (load-affected) |
| after (this lane), sampled run | demo | 200 | 35 paints | 165 | 409 | — | — | — | 0 | 7.9 |
| after (this lane) | body60k | 0 | 0 | 0 | — | — | — | — | — | 14.5 → 25.2: worker never attached within 60 s (load-affected; the Swift test build ran concurrently) — not re-run inside the time box |

Both after cells ran with the Swift test filter building in parallel (1-min load
18–25 vs 7–10 for base); the demo p50 (460 / 409 vs 507 / 514) is directionally
better and consistent with the profile change, but is not a clean A/B. The
base body60k number (229 ms p50, v1 pane) stands as the current reference.

Per-stage attribution of the demo cells (`timeline.py`, p50 ms over painted
v2 revisions; `raw/main-thread-sample-base-vs-after.txt`):

| build | send→v1 | recv→val (main thread wait) | validate (off-main) | deliver (main thread wait) | pub→paint | key→paint |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base | 13.6 | 19.3 | 5.2 | 44.8 | 37.3 | 131 |
| after | 47.1 (a) | 37.0 | 5.2 | 40.0 | 41.8 | 129 |

(a) the v1 result was decoded 0.7 ms after arrival in both runs; the difference
is where the main thread happened to be in its 30 ms keystroke cycle (the
stages are stamped on different threads and the sums match). The three "wait"
columns are the finding: ~100 ms of every keystroke is the main thread doing
SwiftUI/AppKit work that no lane-owned code calls.

Main thread, 8 s `sample` at 1 ms during the same cells (samples of 8000):

| frame | base | after |
| --- | ---: | ---: |
| main thread busy | 6323 | 6383 |
| `NSRunLoop.flushObservers` (SwiftUI graph updates, all hosts) | 3433 + 1097 | 3823 + 957 |
| `AppKitWindowController.updateRootView` (toolbar/preferences) | 960 + 317 | 289 |
| `NSToolbarItemViewer _layoutSubtreeWithOldSize:` | 255 | 0 |
| `CA::Transaction::commit` → window `layoutIfNeeded` | 1214 / 1041 | 787 + 330 / 662 + 258 |
| `NSHostingView.layout()` (content hosting view) | 312 + 224 | 161 + 133 |
| all `(in FlashTeXMac)` frames except `main` | < 120 | < 120 |

## Changes in this lane

- `apps/mac/Sources/FlashTeXMac/ShellModel.swift`: `toolbarHasResult`,
  `toolbarHasV2Frame`, `toolbarProblemCount`, `producerSummary` — assigned only
  when the value changes (`didSet` on `result`, `displayListV2`,
  `layoutDiagnostics`, `workerStatus`).
- `apps/mac/Sources/FlashTeXMac/ContentView.swift`: `WorkspaceToolbar` reads
  those mirrors (Export menu enablement, Problems count, Producer tooltip);
  the preview header's source badge tooltip reads `producerSummary`.
- `apps/mac/Sources/FlashTeXMac/TypingBench.swift`, `tools/typing-bench/run.sh`:
  `FLASHTEX_TYPING_BENCH_AT=<needle|first-paragraph>` / `--at`, `hw1` seed;
  test in `TypingBenchTests`.

## Handoff: Mac shell (mac-claude-a / the preview-latency lane)

Ordered by measured share of the saturated main thread:

1. **Whole-window SwiftUI transactions per keystroke.** `EditorPane`,
   `StatusBar` (`r\(editorRevision)`), `PreviewHeader` (`editor at r… —
   compiling…`, `ProgressView` toggled by `inFlightRevision`), the v1 result
   apply (`result`, `latenciesMs`, `selection = nil`, `workerStatus` twice per
   request) each invalidate the content hosting view's graph; the sample shows
   3.8k of 8k ms in graph updates with no app frame inside. Candidates: move the
   revision/latency labels and the compiling indicator into a small `@Observable`
   throttled at ~4 Hz; stop assigning `workerStatus` per request (it is also
   logged per assignment); keep `selection = nil` from firing when already nil.
2. **`CA::Transaction::commit` → full window `layoutIfNeeded` (~10%)**: the
   AppKit view tree under the hosting view is re-laid out each transaction
   (`NSHostingView.layout()`); the editor `NSViewRepresentable` with 14 changing
   parameters is the likely trigger (`SourceEditorView.updateNSView` appears in
   the profile every keystroke).
3. **Bench paint point on the V2 route** (item 5 above): rerun with
   `--at first-paragraph` once 1–2 land; expect page reuse 0/N for that
   position (every later page's source offsets shift), which is the realistic
   mid-document case and is where the delta contract's relocations pay.

## For FT-070 (compiler / display-list owners)

Measured on this machine; function names from the FT-065 profiles still hold.

1. **`display::DisplayList::write_json` output size** — ~400 bytes per
   cluster: per-glyph `carets` (`height`,`text_byte`,`top`,`x`), `hit_rects`
   and `sources` objects repeated for every cluster of every glyph run, so a
   2-page demo is 1.94 MB and a 24-page document exceeds the 16 MB line cap
   (declined at `protocol::handle_line`, `estimated_json_bytes`). This, not
   layout, bounds the 60 KB target (27 ms producer, ~24 MB of JSON). Options
   that keep the semantic model: the delta contract (unchanged pages by digest
   plus offset relocations; the Mac consumer `DisplayListDelta.swift` is ready
   behind `FLASHTEX_DISPLAY_DELTA=1`, the producer side does not exist yet), or
   a `carets`/`hit_rects` derivation rule so they are omitted when equal to the
   glyph advance box (needs a contract revision; changes bytes).
2. **`v1::fallback` + `write_envelope`**: the v1 `compile_result` is 2.49 MB
   for the 60 KB body and is always sent alongside the v2 line; when
   `display-list-v2` is accepted the v1 `pages` are unused by the V2 pane.
   A negotiated "v1 pages elided when v2 accepted" would halve the IPC of every
   keystroke (contract change; bytes change).
3. **Cold first request**: 60–67 ms on HW1/body60k here (font set + first
   layout); `FontSet::with_default_dirs` + first `typeset` (FT-065: typeset
   22–29 ms on request 1, 0.2 ms after) — a persisted glyph-metric cache would
   remove it from the app's first paint.
4. **Not a producer problem**: warm HW1/HW2 keystrokes are 3.7–4.1 ms with v2;
   the pipeline's `RenderCache` reuse works (`reused pages` on the Mac is
   N−1 of N when the edit is on the last page).

## Addendum, lane mac-perf-3 (FT-071 IPC size): delta + v2-only landed, compaction measured

Branch `agent/mac-render-pipeline/perf-3-wire` (base `d46a0f63`). Producer:
`crates/render-pipeline/src/delta.rs` (`display-list-v2-delta`, proposal r5
producer side) and `display-list-v2-only`
(`protocol/proposals/display-list-v2-only.md`); consumer: the Mac requests
both whenever the v2 pane is active and holds an installed base
(`ShellModel.compile`, `receiveDisplayListV2`), `FLASHTEX_DISPLAY_DELTA` is
no longer needed (`=0` turns the request off). Raw: `raw/ipc-perf3-base.txt`
(binary of `d46a0f63`), `raw/ipc-perf3-after.txt`, `raw/glyph-bytes-perf3.txt`.

Reply bytes per warm keystroke (`ipc_bench.py`, 20 steps, typing before
`\end{document}`; other agents were building throughout, so latencies are
load-affected — bytes are exact):

| seed | before: v1 + v2 | after `--delta`: v1 + v2 | after `--delta --only`: v1 + v2 | deltas/full |
| --- | ---: | ---: | ---: | --- |
| HW1 (3 pages) | 185 667 + 1 157 718 = 1 343 385 B | 185 691 + 259 481 | 1 879 + 259 481 = **261 360 B (−80.5%)** | 20 / 1 |
| demo (2 pages) | 222 541 + 1 945 350 = 2 167 891 B | 222 565 + 807 836 | 280 + 807 836 = **808 116 B (−62.7%)** | 20 / 1 |
| body60k (~24 pages) | 2 494 207 + declined | 2 494 207 + declined | 2 494 207 + declined | 0 / 0 |

The delta carries only the edited page (HW1: page 3 of 3; demo: page 2 of 2,
the larger page) plus the complete header, digests and `page_bytes`; the
producer's warm round trip with `--delta` was 8.1 ms p50 on HW1 vs 5.2 ms full
(the extra is `dl2-canon-1` hashing of every page and the lockstep relocation
compare; both scale with document size — a per-page digest cache keyed on the
relocation is the next producer step).

**The 60 KB body still gets no v2 frame.** A delta needs an installed base,
and the first full frame (~21 MB estimated for 24 pages) is over the 16 MiB
line cap, so `display-list-v2` is declined on every request and the v1 pane
carries the 2.49 MB v1 line. `-only` cannot help there either (it elides pages
only when a sibling exists). This is the compaction case:

| per cluster (one cluster per glyph) | HW1 | body60k (6-page cut) |
| --- | ---: | ---: |
| `carets` (1–2 objects) | 85.8 B (20.4%) | 87.2 B (20.9%) |
| `hit_rects` (1 object) | 79.1 B (18.8%) | 79.1 B (19.0%) |
| `sources` (1 range, path repeated) | 64.8 B (15.4%) | 69.6 B (16.7%) |
| cluster `text_*_byte` + framing | 41.0 B (9.8%) | 41.0 B (9.8%) |
| glyph object | 101.2 B (24.1%) | 101.1 B (24.2%) |
| total per glyph | ~420 B | ~417 B |

Within the frozen schema there is no additive way to drop these: `clusters`,
`carets`, `hit_rects` and `sources`/`synthetic_reason` are required by
`protocol/rendering-v2.schema.json`, and omitting them "when derivable"
changes the meaning of a valid document. Not implemented; the numbers above
are the input for a contract revision (`display-list-v2-compact`, a separate
`render_format`/capability): (a) a run-level `sources` span with per-cluster
byte offsets (−15%), (b) `carets`/`hit_rects` omitted when equal to the
glyph's advance box (−39%), (c) glyph arrays as parallel integer lists
(−10–15%). (a)+(b) alone bring the 24-page body to ~10 MB — under the cap —
after which the delta path applies to it as to HW1.

Gates on this branch: `cargo test --release` in `crates/render-pipeline`
184 passed / 0 failed (incl. the new `tests/display_list_delta.rs` gate:
every delta over 60 cumulative edits reconstructs byte-identically to the
fresh full line, `incremental` still 1 passed); Mac `swift test --filter
'PreviewV2|RenderingV2|DisplayListDelta|LayoutCapability|V2Path|V2Image'`
with `FLASHTEX_RENDER=<this binary>`: 73 tests, 72 passed, 1 skipped
(`RealCompilerTests…` wants `flashtex-compiler`), 0 failures — the five
`DisplayListDeltaTests` drive the real producer, so Swift and Rust
`dl2-canon-1` digests agree.
