# FT-071 `display-list-v2-compact`: wire size, byte identity, gates — 2026-09-14

Lane `mac-perf-4` (Claude Code subagent of `mac-claude-a`), machine
`mac-m1max-a` (Apple M1 Max, macOS 26.3.1). Branch
`agent/mac-render-pipeline/perf-4-compact`, base `origin/main` `eea975c3`.
Control worktree: a detached checkout of `eea975c3` built with the same
commands. Other agents built on this machine throughout (1-minute load
9.8–10.7 during the IPC cells, recorded in `raw/ipc-*.txt`): every latency
below is load-affected; every byte count and digest is exact.

Proposal: [`protocol/proposals/display-list-v2-compact.md`](../../../protocol/proposals/display-list-v2-compact.md).
Producer: `crates/render-pipeline/src/display_list_compact.rs` (+ `Wire::compact`
in `display.rs`, negotiation in `v1.rs`/`protocol.rs`, compact-aware width
rule and frame in `delta.rs`). Consumer: `apps/mac/Sources/FlashTeXProtocol/DisplayListCompact.swift`,
`RenderingV2Fast.swift` (compact clusters + array glyphs), `RenderingV2.swift`
(`clusterEncoding`), `apps/mac/Sources/FlashTeXMac/DisplayListDelta.swift`
(per-request capability, encoding-keyed `page_bytes`/frame accounting,
`delta_encoding_mismatch`), `DisplayListCompactSupport.swift`
(`FLASHTEX_DISPLAY_LIST_COMPACT=0` kill switch).

## Summary

- **The 60 KB body gets a v2 frame.** Its first compact frame is
  **5 986 729 B** (22 pages, 51 729 glyphs), under the 16 MiB reply cap, where
  the full encoding was declined on every request (estimated > 16 MiB). It now
  runs the same route as HW1: v2 + deltas + elided v1 pages, **306 + 228 992 B**
  per warm keystroke instead of 2 494 207 B of v1 pages and no v2 frame.
- Per-keystroke reply bytes with `--delta --only`: HW1 261 360 → **79 657 B**
  (−69.5 %), HW2 153 490 → **61 515 B** (−59.9 %), demo 808 116 →
  **225 706 B** (−72.1 %). First (full) frames: HW1 1 152 024 → 356 207 B,
  HW2 1 029 984 → 340 583 B, demo 1 941 498 → 540 454 B (27–33 % of full).
- **Byte identity when not requested:** the MD5 of the whole reply stream
  (v1 line + sibling, 21 requests) is identical between the control binary
  and this branch for every seed × mode (`v2`, `v2+delta+only`):
  `raw/md5-control.txt` vs `raw/md5-branch.txt`. HW1/HW2 PDFs from the CLI are
  byte-identical before/after (`raw/hw-gate.txt`).
- **Exactness:** the compact line reads back to the identical model
  (Rust round trip on 11 real-world fixtures, 22 616 clusters, plus an
  adversarial run exercising every override; Swift decode of the compact and
  the full sibling of the same request are `Equatable`-equal over 30 062
  clusters, every caret/hit-rect coordinate within 1e-6 bp). The delta
  chain runs under the compact encoding with exact `page_bytes` (Rust: 60
  edits, 38 deltas, 335 relocated pages verified against serialisation;
  Swift: 5 edits against the real producer, target == fresh line length).

## Producer over stdin/stdout, one request per keystroke (`ipc_bench.py`, 20 steps; `raw/ipc-control.txt`, `raw/ipc-branch.txt`)

`ipc_bench.py` is `perf-mac-2026-09-13T1740Z/ipc_bench.py` plus `--compact`
and a Python reference decoder of the compact encoding (a third
implementation, used to compute the delta acknowledgement from a compact
full line). Seeds as `tools/typing-bench/run.sh` builds them: `hw1`/`hw2` =
the real-world fixtures, `demo` = `apps/mac/Samples/demo.tex`, `body60k` =
the demo's paragraphs repeated to ≥ 60 KB (63 899 B).

| seed | binary | caps | first (cold) v1 + v2 bytes | warm p50 / p95 ms | warm v1 + v2 bytes | deltas / full / declined | stream md5 |
| --- | --- | --- | ---: | ---: | ---: | --- | --- |
| hw1 | control | v2 | 185 184 + 1 152 024 | 5.19 / 5.40 | 185 679 + 1 155 862 | 0 / 21 / 0 | `14f3274a…` |
| hw1 | branch | v2 | 185 184 + 1 152 024 | 5.24 / 5.55 | 185 679 + 1 155 862 | 0 / 21 / 0 | `14f3274a…` (identical) |
| hw1 | control | v2+delta+only | 1 854 + 1 152 024 | 8.07 / 8.34 | 1 879 + 259 481 | 20 / 1 / 0 | `cf052dd1…` |
| hw1 | branch | v2+delta+only | 1 854 + 1 152 024 | 8.00 / 8.22 | 1 879 + 259 481 | 20 / 1 / 0 | `cf052dd1…` (identical) |
| hw1 | branch | v2+compact | 185 210 + **356 207** | 4.30 / 4.42 | 185 705 + 357 316 | 0 / 21 / 0 | `b7705ee0…` |
| hw1 | branch | v2+delta+only+compact | 1 880 + 356 207 | 7.81 / 12.51 | **1 905 + 77 752** | 20 / 1 / 0 | `85ab316b…` |
| hw2 | control | v2 | 194 186 + 1 029 984 | 5.39 / 5.64 | 194 681 + 1 033 822 | 0 / 21 / 0 | `eedc4d9f…` |
| hw2 | branch | v2 | 194 186 + 1 029 984 | 6.83 / 34.84 (load) | 194 681 + 1 033 822 | 0 / 21 / 0 | `eedc4d9f…` (identical) |
| hw2 | control | v2+delta+only | 2 755 + 1 029 984 | 7.75 / 7.94 | 2 780 + 150 710 | 20 / 1 / 0 | `e37771ce…` |
| hw2 | branch | v2+delta+only | 2 755 + 1 029 984 | 7.63 / 7.87 | 2 780 + 150 710 | 20 / 1 / 0 | `e37771ce…` (identical) |
| hw2 | branch | v2+compact | 194 212 + **340 583** | 4.50 / 4.65 | 194 707 + 341 692 | 0 / 21 / 0 | `5dbf55bf…` |
| hw2 | branch | v2+delta+only+compact | 2 781 + 340 583 | 7.52 / 7.88 | **2 806 + 58 709** | 20 / 1 / 0 | `89459cb3…` |
| demo | control | v2 | 222 044 + 1 941 498 | 6.82 / 7.18 | 222 541 + 1 945 350 | 0 / 21 / 0 | `f3733bc5…` |
| demo | branch | v2 | 222 044 + 1 941 498 | 6.73 / 7.22 | 222 541 + 1 945 350 | 0 / 21 / 0 | `f3733bc5…` (identical) |
| demo | control | v2+delta+only | 255 + 1 941 498 | 12.36 / 13.45 | 280 + 807 836 | 20 / 1 / 0 | `e523b6fe…` |
| demo | branch | v2+delta+only | 255 + 1 941 498 | 12.40 / 12.85 | 280 + 807 836 | 20 / 1 / 0 | `e523b6fe…` (identical) |
| demo | branch | v2+compact | 222 070 + **540 454** | 4.87 / 5.09 | 222 567 + 541 567 | 0 / 21 / 0 | `630f7949…` |
| demo | branch | v2+delta+only+compact | 281 + 540 454 | 11.49 / 11.84 | **306 + 225 400** | 20 / 1 / 0 | `b1c82c3d…` |
| body60k | control | v2 | 2 493 699 + 0 (declined) | 55.62 / 57.67 | 2 494 207 + 0 | 0 / 0 / 21 | `a08c2e68…` |
| body60k | branch | v2 | 2 493 699 + 0 (declined) | 55.20 / 57.39 | 2 494 207 + 0 | 0 / 0 / 21 | `a08c2e68…` (identical) |
| body60k | control | v2+delta+only | 2 493 699 + 0 (declined) | 55.95 / 57.15 | 2 494 207 + 0 | 0 / 0 / 21 | `a08c2e68…` |
| body60k | branch | v2+delta+only | 2 493 699 + 0 (declined) | 55.54 / 57.21 | 2 494 207 + 0 | 0 / 0 / 21 | `a08c2e68…` (identical) |
| body60k | branch | v2+compact | 2 493 515 + **5 986 729 (accepted)** | 70.51 / 110.38 | 2 494 023 + 5 987 853 | 0 / 21 / 0 | `efe99027…` |
| body60k | branch | v2+delta+only+compact | 281 + 5 986 729 | 138.45 / 140.60 | **306 + 228 992** | 20 / 1 / 0 | `e9741beb…` |

The v1 line grows by 26 B under compact (the echoed capability name). The
warm p50 with compact alone is lower than full (4.3 vs 5.2 ms on HW1: fewer
bytes to write). The body60k delta path's 138 ms p50 is the r5 per-page
`dl2-canon-1` hashing and lockstep compare over 22 pages (the perf-3
addendum's "per-page digest cache keyed on the relocation is the next
producer step"), not the encoding: the same document's compact full line
serialises in 70 ms including layout.

Full 32-hex digests: `raw/md5-control.txt`, `raw/md5-branch.txt`.

## Compact encoding on the real-world corpus (`raw/compact-corpus-stats.txt`, `tests/display_list_compact.rs -- --nocapture`)

| fixture | full B | compact B | ratio | runs | clusters | overrides `c` / `h` / `hv` / `l` / `s` / `e` / `ts` / explicit |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| article-twocolumn | 1 272 790 | 358 976 | 28.2 % | 633 | 3 078 | 0 / 0 / 548 / 10 / 109 / 167 / 0 / 0 |
| cv | 515 083 | 140 305 | 27.2 % | 234 | 1 254 | 0 / 0 / 239 / 3 / 21 / 46 / 0 / 0 |
| hw1 | 1 152 028 | 356 211 | 30.9 % | 679 | 2 742 | 0 / 0 / 522 / 3 / 282 / 478 / 0 / 0 |
| hw2 | 1 029 988 | 340 587 | 33.1 % | 684 | 2 412 | 0 / 0 / 465 / 8 / 268 / 541 / 0 / 0 |
| inline-math | 1 317 878 | 544 233 | 41.3 % | 1 271 | 2 913 | 0 / 0 / 667 / 7 / 575 / 1 504 / 0 / 0 |
| input-bibliography | 1 012 974 | 290 293 | 28.7 % | 518 | 2 407 | 0 / 0 / 406 / 6 / 87 / 151 / 0 / 0 |
| lecture-notes | 857 391 | 295 799 | 34.5 % | 605 | 1 986 | 0 / 0 / 388 / 2 / 378 / 690 / 0 / 0 |
| letter | 471 196 | 129 767 | 27.5 % | 219 | 1 149 | 0 / 0 / 277 / 0 / 0 / 0 / 0 / 0 |
| math-sheet | 472 667 | 206 053 | 43.6 % | 468 | 1 029 | 0 / 0 / 278 / 1 / 305 / 714 / 0 / 0 |
| unicode-accents | 421 576 | 118 341 | 28.1 % | 195 | 1 013 | 0 / 2 / 116 / 13 / 89 / 120 / 0 / 0 |
| tikz (synthetic) | 12 711 | 5 120 | 40.3 % | 6 | 26 | 0 / 0 / 0 / 0 / 3 / 5 / 0 / 0 |

`c` (explicit carets) is never needed and `h` (explicit x/width) twice on
the whole corpus: PR #232's caret invariants hold on this producer. `hv`
(per-cluster vertical extent) is the one frequent override.

## Gates (verbatim summaries in `raw/`)

| gate | command | control (`eea975c3`) | branch | new failures |
| --- | --- | --- | --- | --- |
| render-pipeline tests | `cargo test -p flashtex-render-pipeline --release --no-fail-fast` (in `crates/render-pipeline`, font env set) | 43 suites, 202 passed, 0 failed, 1 ignored, exit 0 | 44 suites, **208 passed, 0 failed**, 1 ignored, exit 0 (`raw/cargo-test-render-pipeline-*.txt`) | **0** |
| CLI tests | `cargo test -p flashtex-cli --release` (in `crates/flashtex-cli`) | 17 passed, **1 failed** (`multi_file_project_resolves_inputs_from_the_project_root`: expects a diagnostic under `sections/`; none is emitted), exit 101 | 17 passed, 1 failed (the same test, same output), exit 101 (`raw/cargo-test-cli.txt`) | **0** (pre-existing on main) |
| release build | `cargo build --release -p flashtex-render-pipeline` | — | ok (1 pre-existing dead-code warning, `typeset.rs:5176`) | — |
| HW1 / HW2 via the CLI | `flashtex build fixtures/real-world/hw{1,2}/HW{1,2}.tex -o … --json` | 3 pages, 0 errors, 0 overfull, 0 font-failure diagnostics (4 / 7 informational `math_resource_profile`) | identical; PDFs byte-identical (`raw/hw-gate.txt`) | — |
| Swift protocol tests | `swift test --filter FlashTeXProtocolTests` (`FLASHTEX_NO_ACTIVATE=1`) | — | **38 passed, 0 failed** (5 new in `DisplayListCompactTests`) | — |
| full `swift test` | `swift test` with `FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_REVIEW_HISTORY_DIR=off FLASHTEX_RENDER=<that tree's flashtex-render>` | 1 050 tests, 135 skipped, **1 failure**: `PreviewTextCacheTests.testProducerFaceSwitchNeverServesALineBuiltWithTheOldFace` | 1 058 tests, 123 skipped, **1 failure**: the same test (`raw/swift-test.txt`); it passes alone (11/11) — an order-dependent global font-generation counter on the v1 text route, untouched here | **0** (pre-existing on main) |

The 8 extra branch tests are `DisplayListCompactTests` (5) and
`DisplayListCompactProducerTests` (3), all passing. The skip counts differ
by 12 between the two runs (135 vs 123); the skipped set was not diffed
here — the two runs used each tree's own release `flashtex-render` and ran
under different machine load, and several suites skip on timing/quiet-
machine conditions.

## Commands

```
# control worktree
git worktree add --detach <scratch>/control origin/main
# font env (both trees)
F=<root>/apps/mac/Fonts; export FLASHTEX_FONT_DIRS="$F"
export FLASHTEX_TFM_DIRS="$F/texmf/fonts/tfm/public/lm:$F/texmf/fonts/tfm/jknappen/ec:$F/texmf/fonts/tfm/public/amsfonts/symbols"
# gates
(cd crates/render-pipeline && cargo test -p flashtex-render-pipeline --release --no-fail-fast)
(cd crates/flashtex-cli && cargo test -p flashtex-cli --release)
(cd crates/render-pipeline && cargo build --release -p flashtex-render-pipeline)
crates/flashtex-cli/target/release/flashtex build fixtures/real-world/hw1/HW1.tex -o /tmp/HW1.pdf --json
(cd apps/mac && FLASHTEX_NO_ACTIVATE=1 swift test --filter FlashTeXProtocolTests)
(cd apps/mac && FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_REVIEW_HISTORY_DIR=off FLASHTEX_RENDER=$PWD/../../crates/render-pipeline/target/release/flashtex-render swift test)
# bytes (seeds built exactly as tools/typing-bench/run.sh §3)
python3 docs/evidence/perf-mac-2026-09-14T0100Z/ipc_bench.py <flashtex-render> <seed.tex> 20 [--delta --only] [--compact] --md5 <out>
```
