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
| `mkbook.py [BOOK ...]` | the book documents: `infdesc` (*Infinite Descent*, 592 pages, the `books.json` entry fetched at its pinned commit and checked by SHA-256 into `$INCR_BENCH_DIR/cache`, never committed) and `infdesc-x2` (the book twice, 1,142 pages: the owner's heavy benchmark, `tools/parity/corpus/infdesc_x2.py`); `docs/BOOK/doc.json` names the chapter in-body keystrokes go into (`docspec.py`) |
| `incr_bench.py ENGINE SRCDIR DOC ...` | one iserve session: random edits (`--kinds`: letters, sentences, structural, and the line/paragraph kinds `newline`, `split`, `join` and the meaning-changing context kinds `math_par`, `verbatim_blank`, `cell_blank` from `edits.py`), each compiled and, with `--verify`, compared byte for byte (PDF, log, aux, out, toc, terminal) with from-scratch runs |
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
| `t7.py [--build] [--docs ...] [--quick]` | **T7, the latency gate** (DESIGN.md §8, §12's P4 exit gate): over the host's socket, per document (plain/full × 10/100/300/1000; the books `infdesc` and `infdesc-x2` when named in `--docs`), letter edits at the start, middle and end, a reflowing sentence, a newline, a paragraph split/join (edits.py's), continuous typing every 50, 60, 80, 100 and 150 ms (each keystroke must get its own page), a preamble edit, and a reopen from the persisted S₀ (report-only); p50/p95/max against §1.2's targets (edited page: the host's ≤ 11 ms share), the convergence rate and the pages re-typeset per edit (held against `t7-baseline.json`), whether later pages were marked stale and all current at `DONE`, and the host's peak RSS. Exit 1 on a miss. See below |
| `test_t7.py`, `test_edits.py` | unit tests (`python3 -m unittest discover -s tools/incr-bench -p 'test_*.py'`); `testdata/t7-summary-trim.json` is a trimmed real summary |
| `gates.sh [GATE...]` | the engine gates a lane runs before landing (parity, lockstep, trip, etrip, drift, display-list positions, cargo tests, soundness A/C/D, `gate.sh pr`), for a Linux runner with TeX Live 2026 |

## Machines

Latency figures need a quiet machine and its load recorded (`stages.py` prints the host's thread
CPU next to wall time). On Apple Silicon an engine thread idle for 100+ ms runs its next 10–20 ms
at a half to a third of its speed (`docs/evidence/p4-finish-2026-09-30/raw/probe/`): keystrokes
300 ms apart measure that; back-to-back ones do not. Soundness and convergence rates do not depend
on load and run anywhere.

On macOS and Linux the host's `DONE.stages` also carry the engine thread's **instruction counts**,
which do not move with load either (`os::thread_counts`: on macOS the kernel's per-thread fixed
counters, P6-HYPEROPT, `docs/evidence/p6-hyperopt-2026-10-04/`; on Linux `perf_event_open`, user
space only, which `perf_event_paranoid` ≤ 2 allows): `instr_k` and `cycles_k` for the whole compile,
`first_page_instr_k`, `restore_instr_k`, `edited_instr_k` (from just before the restore to the
edited page's shipout), `typeset_instr_k` and `typeset_cycles_k` (from the engine's resumption after
the restore to the edited page's shipout: the typesetting alone) and `test_instr_k` (the convergence
tests), all in thousands. Compare engines by these on a loaded machine; quote wall times only from a
quiet one. On Linux, `perf stat` or `perf record` on the host shares the counters, and the host's
counts then fall short. To profile only the typesetting to the edited page, start the host with
`FLASHTEX_PERF_MARKS=FILE`: it appends `b NS` when an edit's engine resumes after the restore and
`e NS` at the edited page's shipout (CLOCK_MONOTONIC), and the samples of
`perf record -k CLOCK_MONOTONIC` between a `b` and the next `e` are that interval. `dl3-keys --edit FILE`
types in another file of the project than `--main` (a book's chapter).

## T7: the latency gate

**The P4 gate's run** (owner decision on Q1, 2026-10-05) is `scripts/t7-reference.sh`. Run it on an
idle, plugged-in Apple-Silicon Mac with Low Power Mode off, TeX Live 2026 on `PATH`, and a clean
checkout of the commit to gate. It refuses to start otherwise (`--check` only checks). It builds
release, waits for load1 < 2 (`--max-load`), and runs every document and phase, typing rows
included, with `--require-reference`. It writes `docs/evidence/t7-reference-<date>-<host>/`
(README with the verdict, table, summary, raw), for a PR. The rest of this section is `t7.py`
itself.

```sh
INCR_BENCH_DIR=/tmp/ib-t7 tools/incr-bench/t7.py --build --wait-load 5 --require-reference  # all 8 documents, ~40 min
tools/incr-bench/t7.py --quick                                 # plain-10, full-100, fewer keys (explicit flags win)
tools/incr-bench/t7.py --check OUT/summary.json                # re-evaluate a run (or a gunzipped evidence summary)
python3 -m unittest discover -s tools/incr-bench -p 'test_*.py'  # t7.py's and edits.py's unit tests
```

It drives `flashtex-host --socket` (started as a separate process, one per document, with
`--s0-cache`) with `dl3-keys`, the socket client. What each row means, the targets and the exit
codes are in `t7.py`'s docstring. Choices that a reader of the table needs:

- **The gated edit quantity** (DESIGN.md §1.2, owner decision 1): COMPILE written → the edited
  page's `PAGE` frame read, the host's share of key event → preview commit, gated at **≤ 11 ms
  p95** (the app's ≤ 4 ms share is the app benchmark's). Viewport set to the edited page,
  keystrokes `--gap-ms` (300) apart after each `DONE`, the host's keep-warm default (decision 10) on.
- **Typing** (`typing@50ms`, `@60ms`, `@80ms`, `@100ms`, `@150ms`; 50 ms ≈ 240 wpm, the owner's
  2026-10-06 bar): letter@middle's edit typed on a clock
  (`dl3-keys --interval-ms`), whatever the host is doing, 40 keystrokes (`--typing-keys`). Most
  arrive while the previous compile's background work runs (re-typesetting to convergence, the
  tests, the jump, the next restore prepared), as when a user types. The in-body rows never meet
  that case, since each of their keystrokes waits for `DONE` and 300 ms more. A keystroke's time
  runs to the watched page from its own compile or a later one. The row is gated at ≤ 11 ms p95
  like the others, and also fails when a keystroke is not painted by its own compile.
- **Edit kinds**: `newline` and `split`/`join` are `edits.py`'s (the soundness sweep's): the space
  after a word becomes a line break, or a blank line, and back; the join is edits.py's `join` of
  the break the split made. dl3-keys picks a space where edits.py's conditions hold in its line.
- **Preamble**: page 1 is the viewport, and the time is to its `PAGE` frame; gated at 400 ms.
- **Reopen: report-only** (decision 8: a pre-warmed host does not count; reopen is met by the
  app's stored pages, which this harness cannot see). Reported: the host's share of a cold reopen
  (spawn → `listening` → page 1 from the persisted S₀), and page 1 from a host already listening.
- **Samples**: the first keystroke of every phase and the first reopen are warm-ups, left out.
  `--keys` and `--preamble` are forced even; defaults give 19 samples per edit row, 13 preamble,
  12 reopen. A row with fewer than 12 samples is gated on its maximum.
- **Order and carry-over**: each phase leaves the host's checkpoint history and RSS to the next.
  `--order rotate` (default) starts document *i* at in-body phase *i* mod 6; the preamble runs
  last (its full runs replace the history), then the reopens. The typing rows run between the
  in-body phases and the preamble. `--order fixed|shuffle` exist.
- **No noise margin**: a row passes only when it meets its target.
- **Held rates**: `t7-baseline.json` is the convergence rate and median re-typeset pages per row
  (`--write-baseline`); a row fails when its rate falls by more than 0.15 or its median pages
  grow by more than 25 % + 2, and a row it lacks is a warning. Regenerate it only from a run of
  `t7.py`, and say so. The current one is run 2 of `docs/evidence/t7-latency-2026-10-02/`
  (warm-ups left out); convergence did not depend on load there.
- **Reference conditions**: the summary's `power` records, at start and end, the power source,
  battery, macOS Low Power Mode, `pmset -g therm` and the load. The run is **non-reference** (in
  the table, the verdict and `--check`) on battery, in Low Power Mode, under a thermal or CPU
  speed limit, with load1 above `--max-load` (default half the cores), or when power was not
  recorded. On Linux it is also non-reference when a cgroup CPU quota over the harness throttled
  during the run (`cpu.stat`'s `nr_throttled` on the harness's cgroup and every ancestor with a
  `cpu.max`; the summary's `power.*.cpu_throttle`). A quota that runs out stops every thread
  under it until the next period (100 ms), so the wall times then measure the quota: on the
  NixOS PC's `flashtex.slice` (800 %, shared by every agent), the same build gave full-1000
  letter@middle 16.9 / 18.8 ms p50 / p95 unthrottled and 25.8 / 70.2 ms with 69 % of periods
  throttled. The **off-CPU** column is how much of each row's time to its page the engine thread
  spent descheduled: the host's time to its first `PAGE`, minus the wait for the engine thread,
  minus that thread's CPU time. Its misses count; `--require-reference` makes a non-reference pass exit 3. The run
  holds `caffeinate -i` (a sleeping Mac stops the clock the keystrokes are timed by), and
  `--wait-load L` waits (at most `--wait-max` s) for load1 below L first.
- **Errors**: an unknown or empty `--docs` or `--phases` value, a host that never listens or a
  `dl3-keys` that fails exits 2, with the summary of what ran written; `--check` exits 2 on a
  summary that records an error.
