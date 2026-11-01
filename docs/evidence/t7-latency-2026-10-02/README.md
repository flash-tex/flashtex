# T7 latency gate: first runs on mac-m1max-a (2026-10-02)

Lane **T7-LATENCY-GATE** (DESIGN.md §8 T7, §1.2 targets, §12 P4 exit gate). The gate is
`tools/incr-bench/t7.py`, described in `tools/incr-bench/README.md` ("T7: the latency gate").
It replaces closed PR #1275's separate `tools/latency-bench` and runs inside the existing
harness: the host is `flashtex-host --socket`, and the socket client is `dl3-keys`, extended here
with T7's edit kinds and page-state checks.

**Verdict: T7 fails on main `ab9893935`, and both runs are non-reference.** Re-evaluated with
the current gate (`t7.py --check`, after review of #1391): run 2, the quieter run, passes **8 of
56** gated rows (**48 misses**); run 1, on a loaded machine, passes 6 of 56 (50 misses).

What the current gate applies (DESIGN.md §1.2, owner decisions 1 and 8, 2026-09-30):
- **In-body edits: the host's share, ≤ 11 ms p95.** Decision 1 gates key event → preview commit
  ≤ 16 ms p95, split host ≤ 11 ms (COMPILE written → edited-page frame read: exactly what
  `dl3-keys` times) and app ≤ 4 ms. The first version of this PR gated 16 ms; that was wrong.
- **Preamble: ≤ 400 ms** to page 1 (§1.2 gives no host/app split for this row).
- **Reopen: report-only.** Decision 8: a pre-warmed host does not count, and reopen is met by the
  app's stored pages, which a host-socket harness cannot see. The table reports the host's share
  of a cold reopen and, for comparison, page 1 from a host already listening.
- **No noise margin**: a row passes only when it meets its target.
- **Warm-ups**: the first keystroke of every phase and the first reopen are left out. These runs
  had 20 keystrokes per edit row (19 samples), 6 preamble edits (5 samples) and 6 reopens (5): rows
  with fewer than 12 samples are gated on their **maximum**. The current defaults (14 preamble
  edits, 13 reopens) give 13 and 12.

## Conditions (read before the numbers): non-reference

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
  runs and did not change the owner's power settings (power state from `pmset -g log`, not from
  the summaries).
- `t7.py` now records a structured `power` field (source, battery, Low Power Mode, `pmset -g
  therm`, load at start and end) and marks a run **non-reference** on battery, in Low Power Mode,
  under a thermal limit, above half the cores in load1, or with no power recorded. These two
  summaries predate the field, so `--check` reports both as "power state not recorded" and load
  above 5 (run 2: 12.9, run 1: 147).
- The reference host that DESIGN.md §8 asks for (quiet, named, not the shared NixOS PC) does not
  exist yet (owner question Q1).
- Keystrokes 300 ms apart, keep-warm on (the host's default, decision 10).
- **Edit kinds of these runs**: `newline` inserted a line break before a word and `split` a blank
  line there, each deleted again. `dl3-keys` now uses `tools/incr-bench/edits.py`'s definitions
  (#1387): the space after a word becomes a line break (or a blank line) and back, the join being
  edits.py's `join` of that break. Both change only input lines (newline) or split and rejoin one
  paragraph (split), as before; the re-typeset pages should be the same, but the rows were not
  re-measured with the new definitions.
- Phase order in these runs was fixed (letter@start → … → split → preamble → reopen in every
  document); the gate now rotates it per document (`--order rotate`), because each phase leaves
  its checkpoint history and RSS to the next.

## Run 2 (load 4–13), current gate: host share ms p50 / p95; misses in bold

| doc (pages) | letter@start | letter@middle | letter@end | sentence@middle | newline@middle | split@middle | preamble (max, n=5) | reopen cold, host share (report) | reopen, host listening (report) | gated rows passing |
|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 (10) | **17 / 18** | 10 / 11 | 7 / 8 | **11 / 11** | **11 / 12** | 10 / 11 | 98 | 664 / 668 | 29 / 36 | 4/7 |
| full-10 (11) | **32 / 35** | **18 / 22** | **36 / 38** | **19 / 23** | **19 / 23** | **21 / 23** | **428** | 628 / 639 | 103 / 105 | 0/7 |
| plain-100 (100) | **7 / 12** | **13 / 14** | **20 / 22** | **13 / 15** | **13 / 16** | **14 / 16** | 108 | 544 / 554 | 27 / 28 | 1/7 |
| full-100 (101) | **15 / 23** | **14 / 19** | **39 / 40** | **15 / 21** | **16 / 21** | **16 / 21** | **446** | 682 / 704 | 110 / 112 | 0/7 |
| plain-300 (301) | **9 / 12** | **9 / 12** | **18 / 23** | 7 / 8 | **8 / 11** | **9 / 14** | 137 | 581 / 649 | 29 / 35 | 2/7 |
| full-300 (299) | **14 / 22** | **38 / 89** | **57 / 62** | **36 / 42** | **37 / 42** | **37 / 40** | **533** | 634 / 642 | 106 / 109 | 0/7 |
| plain-1000 (1001) | **11 / 16** | **18 / 22** | **18 / 19** | **26 / 46** | **19 / 36** | **25 / 33** | 292 | 563 / 584 | 30 / 33 | 1/7 |
| full-1000 (1002) | **23 / 26** | **47 / 70** | **79 / 122** | **49 / 83** | **50 / 69** | **53 / 59** | **4,745** | 693 / 707 | 157 / 159 | 0/7 |

The 8 passing rows: plain-10 letter@middle, letter@end, split@middle and preamble; plain-100,
plain-300 and plain-1000 preamble; plain-300 sentence@middle. Several misses are within 1 ms of
11 (plain-10 sentence 11.3, newline 11.9; plain-300 letter@middle 11.8, newline 11.4): on AC power
without Low Power Mode they may pass, which only a reference run can say.

**Where the data is.**
- The gate's tables (re-evaluated with `--check`), with p50/p95/max, the gated statistic, first
  changed page, DONE p95, convergence, re-typeset pages, RSS and load per row: `run1/table.md`,
  `run2/table.md`.
- Every keystroke with the host's `DONE` and stages: `run*/summary.json.gz` (`raw`).

## Verified (measured in these runs)

1. **Edited page (host share, gate 11 ms).**
   - Against the 11 ms gate only 4 in-body rows of 48 pass (all on plain-10 and plain-300). plain-*
     rows sit at 8–23 ms p95 at 10–300 pages and 16–46 ms at 1,000.
   - Every full-* row misses: 19–23 ms at 10–100 pages, 22–89 ms at 300, 26–122 ms at 1,000.
   - The host's own edited-page CPU tracks the client time (e.g. full-1000 letter@end: client p95
     122 ms, host CPU p95 72 ms, restore p95 99 ms). The misses on the large hyperref documents are
     therefore mostly engine time, not socket overhead or load.
2. **Preamble.**
   - plain-* meets 400 ms. plain-1000's **first** preamble edit after the typing phases took
     1,820 ms; it is the phase's warm-up, which the gate leaves out. The next five took 170–292 ms.
   - full-* takes 421–446 ms at 10–100 pages, 462–887 ms at 300 pages, and 3,415–4,840 ms at 1,000
     pages, every time.
   - In each case the host spends the time before page 1 on CPU (`first_page_cpu` ≈ `first_page`).
   - full-1000's first compile, at open with no `.aux`, showed page 1 in 436 ms. After a preamble
     edit, with the 2,641-line `.aux`, it takes 3.4–4.8 s.
3. **Reopen (report-only, decision 8).**
   - The host's share of a cold reopen (spawn → listening → page 1 from S₀): 544–721 ms. Most of
     it is the host's start-up (format check and warm-up), before the app's stored pages would
     be replaced by the host's.
   - Page 1 from a host already listening: 27–36 ms on plain-*, 102–159 ms on full-*.
   - All 48 reopens came from S₀ (`mode: open`).
4. **Convergence and background pages: deterministic.**
   - Run 1 (loaded) and run 2 (quiet) give identical convergence counts and median re-typeset pages
     on all 56 rows. This held across the 10× load difference.
   - These counts (run 2, warm-ups left out) are `tools/incr-bench/t7-baseline.json`, which the
     gate holds.
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
These p95s are over all 20 keystrokes (6 for the preamble, maximum), warm-ups included, as the
first version of the gate computed them; the comparison is between the two columns only.

| doc | letter@start | letter@middle | letter@end | sentence@middle | newline@middle | split@middle | preamble | reopen, host listening |
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
- **Low Power Mode.** On AC power with Low Power Mode off, the plain-* rows within a millisecond or
  two of 11 ms might pass; most plain-* rows (12–23 ms, up to 46 ms at 1,000 pages) and every full-*
  row would likely not: on full-300 and full-1000 the host's CPU alone is 40–72 ms.

## Reproduce

```sh
cargo build --release -p flashtex-engine -p flashtex-display-list
INCR_BENCH_DIR=/tmp/ib-t7 tools/incr-bench/mkeng.sh t7 && INCR_BENCH_DIR=/tmp/ib-t7 python3 tools/incr-bench/mkdocs.py
INCR_BENCH_DIR=/tmp/ib-t7 python3 tools/incr-bench/t7.py --wait-load 5 --require-reference --out /tmp/ib-t7/run  # about 40 min at low load, on AC power, Low Power Mode off
gunzip -k docs/evidence/t7-latency-2026-10-02/run2/summary.json.gz
python3 tools/incr-bench/t7.py --check docs/evidence/t7-latency-2026-10-02/run2/summary.json   # 48 misses, 8 of 56 pass
```
