# incr-bench: latency, convergence and soundness of the incremental engine

One harness for DESIGN.md §1.2's latency targets and §5's incremental soundness, for any lane
(P4, T7-LATENCY-GATE) to run on any machine. It measures the engine (`flashtex-host iserve`,
the L2–L5 session) and the host's socket (`flashtex-host --socket`, what the app talks to).
It grew out of the P4-L2-L3, P4-L5 and P4-FINISH evidence scripts
(`docs/evidence/p4-*/scripts/`), which it replaces; those stay as the record of those runs.

## Layout

Everything lives under `INCR_BENCH_DIR` (default `/tmp/incr-bench`):

| path | what |
|---|---|
| `$INCR_BENCH_DIR/NAME/` | an engine: `flashtex-host`, `flashtex-initex` (+ `pdftex` link), `pdftex.pool`, `dl3-keys`, `dl3-client`, `HEAD` |
| `$INCR_BENCH_DIR/fmt-NAME/pdflatex.fmt` | its pdflatex format |
| `$INCR_BENCH_DIR/src-DOC/DOC.tex` | a document for `incr_bench.py` (iserve) |
| `$INCR_BENCH_DIR/docs/DOC/main.tex` | the same document for the socket drivers |

Every engine run has a time limit (`to.sh`, `signal.alarm`, `timeout=`), and a host's stderr is
kept next to its output (`*.host-stderr`): the host says there why it ended
(`src/host/crash.rs`).

## Scripts

| script | measures |
|---|---|
| `mkeng.sh NAME [--keep-fmt]` | copies this checkout's `target/release` binaries to `$INCR_BENCH_DIR/NAME` and builds the format |
| `mkdocs.py` | the generated documents: `plain-N`, `full-N` (N = 10, 100, 120, 300, 1000; `gen.py`), `refs-30`, `refs-120` (`genrefs.py`) |
| `incr_bench.py ENGINE SRCDIR DOC ...` | one iserve session: random edits (`--kinds`: letters, sentences, structural), each compiled and, with `--verify`, compared byte for byte (PDF, log, aux, out, toc, terminal) with from-scratch runs |
| `matrix.py ENGINE OUTDIR [DOC...]` | the §1.2 matrix: plain/full × 10/100/300/1000 × start/middle/end × (8 letters, 3 sentences, each reverted); `matrix_table.py`/`matrix_sum.py` summarise the edited-page latency |
| `convergence.py OUTDIR` | the matrix's convergence rate per document and why the last test of each unconverged compile failed (review 2026-09-30, track 1) |
| `soundness.py ENGINE ...` | the soundness sweep (every parity fixture plus `--extra DIR:DOC`), `--kinds`, `--interleave` (preempted compiles) |
| `preamble.py ENGINE DOC [REPS]` | §1.2 "preamble edit ≤ 400 ms to the first visible page": a preamble line added, `compile 1`, the wall time |
| `keys.sh ENGINE DOC AT [TAG]` | keystrokes through the socket (`dl3-keys`), as the app sends them: `KEYARGS="--no-viewport --gap-ms 300 --page P --where start|middle|end [--sentence]"`, `HOSTARGS` for the host |
| `ab_engines.sh DOC PAGE GAP ROUNDS NAME=ENGINE:HOSTARGS...` | interleaved socket runs of several engines/options; `stages.py` prints the host's per-stage times (DONE's `stages`) |
| `keys_at.sh ENGINE TAG FILE PAGE WHERE [--sentence]`, `keys_matrix.sh ENGINE TAG FILE`, `keys_sum.py TAG` | a given document (e.g. a user's 1,000-page book): pages 4, 129, 539, 999 × start/middle/end × letter/sentence; the restart check (`restart_next_gap`: the restart point is the newest checkpoint before the edit) |
| `t7.py [--build] [--docs ...] [--quick]` | **T7, the latency gate** (DESIGN.md §8, §12's P4 exit gate): over the host's socket, per document (plain/full × 10/100/300/1000), letter edits at the start, middle and end, a reflowing sentence, a newline, a paragraph split/join, a preamble edit, and a reopen from the persisted S₀ (pre-warmed and cold); p50/p95/max against §1.2's targets, the convergence rate and the pages re-typeset per edit (held against `t7-baseline.json`), whether later pages were marked stale and all current at `DONE`, and the host's peak RSS. Exit 1 on a miss. See below |
| `gates.sh [GATE...]` | the engine gates a lane runs before landing (parity, lockstep, trip, etrip, drift, display-list positions, cargo tests, soundness A/C/D, `gate.sh pr`), for a Linux runner with TeX Live 2026 |

## Machines

Latency figures need a quiet machine and its load recorded (`stages.py` prints the host's thread
CPU next to wall time). On Apple Silicon an engine thread idle for 100+ ms runs its next 10–20 ms
at a half to a third of its speed (`docs/evidence/p4-finish-2026-09-30/raw/probe/`): keystrokes
300 ms apart measure that; back-to-back ones do not. Soundness and convergence rates do not depend
on load and run anywhere.

## T7: the latency gate

```sh
INCR_BENCH_DIR=/tmp/ib-t7 tools/incr-bench/t7.py --build --wait-load 20   # all 8 documents, ~1 h on a quiet M1 Max
tools/incr-bench/t7.py --quick                                             # plain-10, full-100, fewer keys
tools/incr-bench/t7.py --check OUT/summary.json                            # re-evaluate a run
```

It drives `flashtex-host --socket` (started as a separate process, one per document, with
`--s0-cache`) with `dl3-keys`, the socket client, so every number is the client side of the socket
(§1.2's "socket client time"). What each row means, the targets and the exit codes are in
`t7.py`'s docstring. Choices that a reader of the table needs:

- **Edited page**: COMPILE to the watched page's `PAGE` frame, the viewport set to it, keystrokes
  `--gap-ms` (300) apart after each `DONE`; the host's keep-warm default (owner decision 10A) is on.
- **Preamble**: page 1 is the viewport, and the time is to its `PAGE` frame.
- **Reopen**: the document is edited on disk and a new host opens it from the S₀ the previous host
  persisted. `pre-warmed` starts the clock once the new host listens (gated by default: the
  §1.2 row is met today only pre-warmed, owner decision O8 open); `cold` adds spawn to `listening`
  and is reported, gated only with `--reopen-gate cold`.
- **Held rates**: `t7-baseline.json` is the convergence rate and median re-typeset pages per row of
  a reference run (`--write-baseline`); a row fails when its rate falls by more than 0.15 or its
  median pages grow by more than 25 % + 2. Regenerate it only from a run of `t7.py`, and say so.
- **Noise**: `--margin` (0.10) is for noise only; load1 is recorded per row, and `--wait-load L`
  waits (at most `--wait-max` s) for the 1-minute load to fall below L first.
- **Power (macOS)**: the run records the power source and Low Power Mode at start and end, warns
  when Low Power Mode is on (it lowers the clocks: such a run's latencies are pessimistic) and holds
  `caffeinate -i` so the Mac does not idle-sleep (a sleeping Mac stops the monotonic clock that
  times the keystrokes, and its wake-up load lands on the next ones). A reference run is on AC
  power with Low Power Mode off and the lid open.
- **A harness error** (a host that never listens, a `dl3-keys` that fails) still writes the summary
  of the documents before it, and exits 2.
