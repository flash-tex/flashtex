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
| `mem.py ENGINE DOC --pages P,... --keys N` | the socket host's memory while typing (`FLASHTEX_MEMSTAT=1`: DONE carries `mem`, the process's resident bytes and the checkpoint layer's parts; a `mem-stats` build adds the heap by tag); samples RSS, kills the host above `--limit-gb`, fails above `--gate-gb`; `MEM_HOSTARGS` for the host |
| `mem_table.py FILE...` | mem.py's summaries as a table: peak RSS and where the memory is; pooled latency and peak per engine for rounds tagged `TAG-ENGINE-rN` |
| `mem_gate.sh [--build]` | the memory gate (nightly): peak RSS of plain/full-120 and plain/full-1000 against a limit per size |
| `gates.sh [GATE...]` | the engine gates a lane runs before landing (parity, lockstep, trip, etrip, drift, display-list positions, cargo tests, soundness A/C/D, `gate.sh pr`), for a Linux runner with TeX Live 2026 |

## Machines

Latency figures need a quiet machine and its load recorded (`stages.py` prints the host's thread
CPU next to wall time). On Apple Silicon an engine thread idle for 100+ ms runs its next 10–20 ms
at a half to a third of its speed (`docs/evidence/p4-finish-2026-09-30/raw/probe/`): keystrokes
300 ms apart measure that; back-to-back ones do not. Soundness and convergence rates do not depend
on load and run anywhere.
