# T7 latency gate: first runs on mac-m1max-a (2026-10-02)

Lane **T7-LATENCY-GATE** (DESIGN.md §8 T7, §1.2 targets, §12 P4 exit gate). The gate is
`tools/incr-bench/t7.py`, described in `tools/incr-bench/README.md` ("T7: the latency gate").
It replaces closed PR #1275's separate `tools/latency-bench` and runs inside the existing
harness: the host is `flashtex-host --socket`, and the socket client is `dl3-keys`, extended here
with T7's edit kinds and page-state checks.

**Verdict: T7 fails on main `ab9893935`.** Run 2, the quieter run, has 38 misses out of 64 rows;
its best-case figures are below. Run 1, on a loaded machine, has 50 misses.

## Conditions (read before the numbers)

| | run 1 | run 2 |
|---|---|---|
| engine | main `ab9893935`, `cargo build --release` with rustc 1.100.0-nightly (2026-09-15) | same binaries |
| machine | M1 Max (10 cores, 32 GB), macOS (Darwin 25.3.0) | same |
| `uptime` before | load 97.61 80.34 72.78 | load 4.93 11.33 28.04 |
| `uptime` after | load 5.27 11.50 28.20 | load 7.39 7.34 10.13 |
| load1 per row | 5–147 (other sessions' cargo and swift builds) | 3.9–12.9 |
| power | **battery**, 82 % → 32 % | **battery**, 32 % → 19 % |
| Low Power Mode | **on** | **on** |

- Both runs used battery power with macOS **Low Power Mode on** (`pmset -g custom`: `lowpowermode 1`
  on battery and on AC). Low Power Mode lowers the CPU clocks, so every latency below is
  **pessimistic** next to a Mac on AC power with Low Power Mode off. I found this only after the
  runs and did not change the owner's power settings. `t7.py` now records the power state and warns
  about it.
- The reference host that DESIGN.md §8 asks for (quiet, named, not the shared NixOS PC) does not
  exist yet.
- Times are the client side of the socket (§1.2's "socket client time"), from writing `COMPILE` to
  the decoded `PAGE` frame. This is not the owner's undecided O1 quantity (key event to preview
  commit).
- Each row has 20 keystrokes (6 for the preamble, 6 reopens), 300 ms apart, with keep-warm on (the
  host's default, decision 10A).

## Run 2 (load 4–13): ms p50 / p95; misses in bold

| doc (pages) | letter@start | letter@middle | letter@end | sentence@middle | newline@middle | split@middle | preamble | reopen pre-warmed | reopen cold | converged (in-body) | re-typeset pages p50 (sentence / newline / split) | host RSS peak |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 (10) | **17 / 18** | 10 / 11 | 7 / 8 | 11 / 11 | 11 / 12 | 10 / 11 | 96 / 98 | 29 / 36 | 664 / 721 | 40/120 | 6 / 6 / 6 | 0.3 GB |
| full-10 (11) | **32 / 35** | **18 / 22** | **36 / 38** | **19 / 23** | **19 / 23** | **21 / 23** | 421 / 428 | 102 / 105 | 628 / 639 | 20/120 | 7 / 7 / 7 | 0.5 GB |
| plain-100 (100) | 8 / 12 | 13 / 14 | **20 / 22** | 13 / 15 | 13 / 16 | 14 / 16 | 103 / 117 | 27 / 32 | 544 / 564 | 40/120 | 50 / 50 / 50 | 0.9 GB |
| full-100 (101) | **16 / 23** | **15 / 19** | **39 / 40** | **15 / 21** | **16 / 21** | **16 / 21** | **429 / 446** | **108 / 112** | 670 / 704 | 40/120 | 33 / 51 / 51 | 2.4 GB |
| plain-300 (301) | 9 / 12 | 9 / 13 | **18 / 23** | 8 / 9 | 8 / 11 | 9 / 14 | 124 / 182 | 29 / 35 | 581 / 649 | 60/120 | 65 / 151 / 151 | 2.3 GB |
| full-300 (299) | **14 / 22** | **38 / 89** | **57 / 62** | **36 / 42** | **37 / 42** | **37 / 40** | **479 / 887** | 106 / 109 | 634 / 650 | 80/120 | 33 / 150 / 150 | 6.6 GB |
| plain-1000 (1001) | 11 / 16 | **18 / 22** | **18 / 19** | **26 / 46** | **20 / 36** | **25 / 33** | **199 / 1,820** | 30 / 33 | 561 / 584 | 60/120 | 1,505 / 502 / 1,505 | 8.6 GB |
| full-1000 (1002) | **23 / 30** | **47 / 70** | **80 / 122** | **50 / 83** | **50 / 69** | **53 / 59** | **4,266 / 4,840** | **155 / 159** | 690 / 707 | 80/120 | 33 / 501 / 501 | 8.7 GB |

**Targets.** Every in-body edit: ≤ 16 ms p95. Preamble: ≤ 400 ms. Reopen: ≤ 100 ms.

**How the gate reads the table.**
- A p95 may exceed its target by up to 10 % (the noise margin) before the row misses.
- Reopen is gated on the pre-warmed row. The cold row (spawn → `listening` → page 1) is reported
  but not gated, because O8 is open; it misses on every document (544–721 ms).

**Where the data is.**
- Full tables, with p50/p95/max, first changed page, DONE p95 and load per row:
  `run1/table.md`, `run2/table.md`.
- Every keystroke with the host's `DONE` and stages: `run*/summary.json.gz` (`raw`).

## Verified (measured in these runs)

1. **Edited page.**
   - plain-10, -100 and -300 meet 16 ms except letter edits at the document's start or end
     (p95 18–23 ms).
   - Every full-* document misses: 19–23 ms at 10–100 pages, 22–89 ms at 300, 30–122 ms at 1,000.
   - The host's own edited-page CPU tracks the client time (e.g. full-1000 letter@end: client p95
     122 ms, host CPU p95 72 ms, restore p95 99 ms). The misses on the large hyperref documents are
     therefore mostly engine time, not socket overhead or load.
2. **Preamble.**
   - plain-* meets 400 ms, except plain-1000's **first** preamble edit after the typing phases
     (1,820 ms). The next five took 170–292 ms.
   - full-* takes 421–446 ms at 10–100 pages, 462–887 ms at 300 pages, and 3,415–4,840 ms at 1,000
     pages, every time.
   - In each case the host spends the time before page 1 on CPU (`first_page_cpu` ≈ `first_page`).
   - full-1000's first compile, at open with no `.aux`, showed page 1 in 436 ms. After a preamble
     edit, with the 2,641-line `.aux`, it takes 3.4–4.8 s.
3. **Reopen.**
   - Pre-warmed: 27–36 ms on plain-*, 102–159 ms on full-* (misses on full-100 and full-1000).
   - Cold: 544–721 ms.
   - All 48 reopens came from S₀ (`mode: open`).
4. **Convergence and background pages: deterministic.**
   - Run 1 (loaded) and run 2 (quiet) give identical convergence counts and median re-typeset pages
     on all 56 rows. This held across the 10× load difference.
   - These counts are `tools/incr-bench/t7-baseline.json`, which the gate holds.
   - Letter edits in the middle converge 20/20 on every document.
   - Letter edits at the start converge 20/20, except on **full-10 and full-100 (0/20)**. On
     full-100 an edit 2 % into the document re-typesets 99 of 101 pages (DONE p95 1.1 s).
   - Letter edits at the end converge on the 300- and 1,000-page documents. On the smaller ones
     the edit is on the last page or near it, so there is little or nothing left to converge with.
   - Sentence, newline and split edits never converge on plain-*. Sentence edits on
     full-100/300/1000 do (20/20).
   - newline and split re-typeset to the end of the document (150 of 300 pages; 501 of 1,000).
   - On plain-1000, sentence and split edits run a **second full pass**: `passes: 2`, 1,505 pages,
     DONE p95 9.7–11.2 s. The `.aux` changed (the `toc` lines' page numbers), and the pass restarts
     at page 0.
5. **Stale marking.**
   - 988 of the 1,008 keystrokes refreshed later pages. Every one of them marked those pages stale
     before refreshing them.
   - All 1,008 ended with every page current (`PAGES complete`, count = `DONE.pages`).
   - No keystroke lost its edited page.
6. **Host RSS: over the 1 GB budget (§5.2).**
   - Peaks of 0.3–0.9 GB at 10–100 pages (plain), 2.3–6.6 GB at 100–300, and 8.6–8.7 GB at 1,000
     pages. Main does not have #1300's memory fixes.

## Indicative only: #1300's head (`eac01eae9`) against main, p95 in ms (main → #1300)

This was not interleaved with run 2. It was built with stable rustc 1.98.1, because #1300's
`crash.rs` predates main's Rust 1.99 fix. The battery was at 15 % → 5 %, with Low Power Mode on.
The Mac then **slept** (clamshell sleep) during plain-1000, and woke to load 200–290. The 1,000-page
rows are therefore discarded, and the run was stopped. Raw data: `run3-p4mem-raw.tar.gz`.

| doc | letter@start | letter@middle | letter@end | sentence@middle | newline@middle | split@middle | preamble | reopen pre-warmed |
|---|---|---|---|---|---|---|---|---|
| plain-10 | 18 → 17 | 11 → 11 | 8 → 8 | 11 → 12 | 12 → 12 | 11 → 11 | 98 → 91 | 36 → 26 |
| full-10 | 35 → 38 | 22 → 17 | 38 → 38 | 23 → 17 | 23 → 18 | 23 → 19 | 428 → 446 | 105 → 83 |
| plain-100 | 12 → 8 | 14 → 13 | 22 → 16 | 15 → 13 | 16 → 13 | 16 → 13 | 117 → 91 | 32 → 25 |
| full-100 | 23 → 15 | 19 → 14 | 40 → 38 | 21 → 14 | 21 → 14 | 21 → 14 | 446 → 431 | 112 → 91 |
| plain-300 | 12 → 11 | 13 → 13 | 23 → 20 | 9 → 11 | 11 → 11 | 14 → 10 | 182 → 104 | 35 → 27 |
| full-300 | 22 → 19 | 89 → 39 | 62 → 61 | 42 → 41 | 42 → 39 | 40 → 50 | 887 → 464 | 109 → 89 |

## Beliefs (not verified here)

- **Preamble on full-1000.** The 3–4 s before page 1 is spent reading the large `.aux` at
  `\begin{document}`. The cause may be the per-checkpoint control-sequence read recording (§5.5,
  "as built"), or hyperref/cleveref's `\newlabel` handling. A profile of the cold compile with and
  without the `.aux` would settle it.
- **plain-1000's first preamble edit.** It is slower (1.8 s, against about 0.2 s for the next five),
  most likely from tearing down the large checkpoint history of the typing phases. That would fit
  the 8.6 GB RSS.
- **letter@end on large documents.** The misses are dominated by restore (host restore p95 99 ms on
  full-1000). #1300's restore-from-the-end (`Session::prepare_next`) is aimed at exactly this. The
  run-3 numbers above are too noisy to confirm it.
- **Low Power Mode.** On AC power with Low Power Mode off, the plain-* rows near 16–23 ms would
  likely pass. The full-* rows at 300 and 1,000 pages would likely not: their host CPU alone is
  40–72 ms.

## Reproduce

```sh
cargo build --release -p flashtex-engine -p flashtex-display-list
INCR_BENCH_DIR=/tmp/ib-t7 tools/incr-bench/mkeng.sh t7 && INCR_BENCH_DIR=/tmp/ib-t7 python3 tools/incr-bench/mkdocs.py
INCR_BENCH_DIR=/tmp/ib-t7 python3 tools/incr-bench/t7.py --wait-load 8 --out /tmp/ib-t7/run2   # about 30 min at low load
python3 tools/incr-bench/t7.py --check docs/evidence/t7-latency-2026-10-02/run2/summary.json  # after gunzip
```
