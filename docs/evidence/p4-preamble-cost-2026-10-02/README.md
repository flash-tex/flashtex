# P4-PREAMBLE-COST: why a preamble edit took 4.7 s on full-1000 (2026-10-02, mac-m1max-a)

Lane **P4-PREAMBLE-COST** (DESIGN.md §1.2's preamble row, ≤ 400 ms to the first visible page;
§5.1; §12 P4). T7 (`docs/evidence/t7-latency-2026-10-02/`) measured a preamble edit at
3.4–4.8 s on full-1000 and 462–887 ms on full-300, with the CPU spent before page 1, and a cold
open without an `.aux` at 436 ms. The belief was that the large `.aux` was the cause.

**Verdict: the `.aux` is not the cause.** The time went into *dropping the previous run's engine*
(`Session::cold`: `self.g = None`), and almost all of that was the checkpoint records' copies of
the display-list side table (`CState::dl`, a `displaylist::Snap` per checkpoint). Draft PR #1300
(P4-MEMORY, `eac01eae9`) moves that side table into the word space (`dl_side`), and with it the
drop falls from 3.0–4.6 s of CPU to 21–29 ms on full-1000. So this lane lands no engine change:
#1300 is the fix, measured below. What is left (about 0.42–0.50 s of CPU to page 1 on every
`full-*` document, in Low Power Mode) is the run from the format through the preamble's packages,
which does not depend on the document's length.

## Conditions: non-reference

- M1 Max (10 cores, 32 GB), macOS (Darwin 25.3.0), **Low Power Mode on throughout**
  (`pmset -g`: `lowpowermode 1`). AC power for the first runs, then **battery** (73–78 %) for the
  `rows2` runs (t7.py's `power` field records it). I did not change the owner's power settings.
- Shared machine: load1 **27–120** during `rows`, **8–15** during `rows2` (`--wait-load 12`).
  Wall times are therefore noisy; the host's **thread CPU** (`stages.first_page_cpu`) is the
  robust quantity and is reported beside them.
- Engines: main `e16e906d4` (nightly rustc, release) as **main**; #1300's head `eac01eae9`
  (stable rustc 1.98.1, because its `crash.rs` predates main's Rust 1.99 fix) as **#1300**.
  Same `dl3-keys` (main's) for both. Formats built by `mkeng.sh` per engine.
- `tools/incr-bench/t7.py --phases preamble --preamble 8` per document, the two engines
  interleaved per document (order alternating), `INCR_BENCH_DIR=/tmp/ib-p4pc`. 7 samples per row
  (the warm-up left out). Raw summaries: `raw.tar.gz` (`rows/`, `rows2/`).

## 1. Profile (VERIFIED)

**Where the CPU before page 1 goes.** DONE's `stages.first_page_cpu` counts thread CPU from the
COMPILE; `stages.edited_cpu` counts it from the start of the run's observer, created in
`Session::cold` after the reset. Their difference is the reset. On main, full-1000 (baseline run,
load 7–10): first_page_cpu 2.7–3.8 s, edited_cpu 0.47–0.61 s, so **2.2–3.3 s before the run
starts**.

**Which part of the reset.** A temporary trace in `Session::cold` (not committed; env
`FLASHTEX_COLD_TRACE`, output in `raw.tar.gz` `t7-trace2/raw/full-1000.host-stderr`), timing
each step of the reset, full-1000 on main:

| step | wall ms | thread CPU ms |
|---|---|---|
| drop the checkpoint layer (`arena.extra`: records, pending branch, read-set) | 3,310–4,266 | 2,838–3,409 |
| drop the rest of `Globals` (word space, slab, undo logs) | 32 | 32 |
| `reset_state`, diag, outputs, journal | 3–8 | 3–5 |
| `Globals::new` | < 1 | < 1 |

The layer holds one `ExtRecord` per retained checkpoint (every shipout plus the segment
checkpoints; over a thousand on full-1000), each with a `CState`, whose `dl: displaylist::Snap` is an
`Arc<Vec<Arc<Chunk>>>` of the side table's `(mem_max + 1) / 2,048` = 2,442 chunks. #1300's
description measured that side table at 6.8 GB of plain-1000's 7.2 GB heap; freeing it is the
3 s. #1300 deletes `CState::dl` (checked in its source: the side table is the word-space region
`dl_side`).

**The `.aux` itself costs little.** CLI runs of full-1000's preamble (`split.py`, #1300 engine,
5 runs each, median CPU; the CLI includes process start-up, kpathsea and the font map, about
135 ms, which the resident host has already paid):

| run | CPU ms |
|---|---|
| `\documentclass{article}\begin{document}\end{document}` | 168 |
| full-1000's preamble, `\begin{document}\end{document}`, no `.aux` | 495 |
| the same with full-1000's 2,641-line `.aux` (read twice: `\begin` and `\end{document}`) | 504 |
| preamble + the first 14 paragraphs (2 pages), no `.aux` | 645 |
| the same with the `.aux` | 653 |

So the `.aux` costs about 10 ms per read. Loading the format (`fmtload.py`: `-fmt=pdflatex \end`
against `-ini \end`) costs 8–16 ms. The rest is the packages: under the macro profiler
(`FLASHTEX_MACRO_PROFILE`, `raw.tar.gz` `split/prof-pre.tsv`), `\usepackage` file loading
(`@input@file@exists@with@hooks`) is 825 of the 929 profiled ms, and `\document` 38 ms.

## 2. The preamble rows: main against #1300 (VERIFIED, non-reference)

Page 1 ms is the gate's quantity (COMPILE written → page 1's PAGE frame read on the socket).

`rows2` (load1 8–15, battery, Low Power Mode):

| doc | engine | page 1 ms p50 / max | host CPU to page 1 p50 / max | of it, the reset p50 / max |
|---|---|---|---|---|
| full-300 | main | 473 / 489 | 472 / 488 | 48 / 58 |
| full-300 | #1300 | **415 / 428** | 413 / 420 | 4 / 4 |
| full-1000 | main | 4,867 / 9,137 | 3,553 / 5,082 | 3,043 / 4,562 |
| full-1000 | #1300 | **485 / 554** | 462 / 492 | 21 / 29 |

`rows` (all eight documents, load1 27–120, AC then battery, Low Power Mode; wall is dominated by
load, CPU is not):

| doc | main: page 1 ms p50 / max | #1300: page 1 ms p50 / max | main: CPU p50 (reset) | #1300: CPU p50 (reset) |
|---|---|---|---|---|
| plain-10 | 267 / 349 | 193 / 231 | 108 (3) | 103 (1) |
| full-10 | 916 / 1,100 | 767 / 1,089 | 482 (6) | 463 (1) |
| plain-100 | 228 / 359 | 118 / 171 | 123 (11) | 101 (2) |
| full-100 | 709 / 1,019 | 613 / 1,038 | 513 (22) | 475 (2) |
| plain-300 | 138 / 251 | 129 / 394 | 126 (24) | 105 (3) |
| full-300 | 2,094 / 2,535 | 555 / 604 | 1,294 (769) | 468 (5) |
| plain-1000 | 5,067 / 6,038 | 146 / 263 | 2,100 (1,959) | 116 (8) |
| full-1000 | 10,758 / 11,730 | 541 / 1,167 | 4,600 (3,963) | 498 (25) |

## 3. What is left, and beliefs (UNVERIFIED unless said)

- With #1300, the host's CPU to page 1 is 413–498 ms on every `full-*` document (10 to 1,000
  pages) and 101–116 ms on `plain-*`: a property of the preamble, not of the document's length or
  `.aux`. In Low Power Mode the `full-*` rows still miss 400 ms. Belief: on AC without Low Power
  Mode they are near or under it (T7's main-engine full-10 row took 428 ms in Low Power Mode, and
  #1269 measured 41–281 ms engine side on a quieter Mac); only a reference run can say.
- Further cuts are not in this lane: the packages' macro execution is engine speed (DESIGN.md
  §5.6, L6), and skipping it would need restart points inside the preamble (checkpoints before
  S₀, keyed like S₀), which does not help T7's edit (right after `\documentclass`) anyway.
- If #1300 is delayed, deferring the drop of the old checkpoint layer to after the viewport page
  would take the 3–4 s off page 1 on main (the rest of `Globals` must still drop first: its
  `BufWriter`s flush on drop). With #1300 that saves only the remaining 21–29 ms.

## Soundness

No engine code changes here, so there is nothing new to sweep. The measured fix is #1300's, which
carries its own soundness evidence (`docs/evidence/p4-memory-2026-09-30/`). A preamble edit is a
cold run (`mode: cold`, a full run from the format) on both engines in every sample above.
