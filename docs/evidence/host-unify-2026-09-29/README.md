# One engine host: the socket protocol drives the incremental engine (2026-09-29)

Lane **P3P4-HOST-UNIFY** (kabir-claude, mac-m5pro-kabir), DESIGN.md §3, §5.1–§5.4,
§6.1, §12 P3/P4. Branch `agent/kabir-claude/host-unify`, from
`agent/kabir-claude/p4-l2-l3` (PR #1223, on #1215) with
`agent/kabir-claude/p3-displaylist` (PR #1217, since landed) and origin/main merged
in. It is the landing vehicle for P4-L1 (#1215) and P4-L2/L3 (#1223), which it
contains and supersedes, and replaces #1217's host (landed on main meanwhile): that
server is kept as the `export` path.

## What was built

Two lanes had each built a `flashtex-host`: the incremental one (P4-L1/L2/L3:
`src/host.rs`, `src/host_main.rs`, a line protocol on stdin) and the socket one
(P3-DISPLAYLIST: `src/host/main.rs`, display-list-v3 over a Unix socket, one engine
**process** per compile). Now there is one binary and one module:

| file | what |
|---|---|
| `src/host/mod.rs` | S₀, its key and its persistence (P4-L1, unchanged but for its place) |
| `src/host/server.rs` | the socket: HELLO, COMPILE/CANCEL/BYE, format preparation (`ensure_format`), warm-up, `export` (P3's child-process engine, kept) |
| `src/host/resident.rs` | the engine thread: one `incr::Session` per document, edits, page cache, per-client delivery, PAGES, diagnostics, S₀ cache |
| `src/host/tools.rs` | P4's `serve`/`iserve`/`bench`/`open`/`selftest`/`layout` |
| `src/host/main.rs` | the bin: subcommands → tools, `--socket` → server, invoked as `pdftex` → the engine |
| `src/displaylist/mod.rs` | the writer made checkpoint-safe and servable (below) |
| `crates/display-list-v3` | protocol 3.1: `PAGES`, `CompileRequest` fields, `dl3-keys` |
| `docs/protocol/display-list-v3.md` | the spec, now 3.1 |

**The unified design in three lines.** A COMPILE's edits are written to the
project, and the resident engine restarts at the last checkpoint before them and
stops when its state converges with the previous run (DESIGN.md §5.3). Every page it
ships goes straight to the socket through the display-list sink, so the edited page
arrives first and pages it did not re-typeset come from the host's cache, in page
order, unless the client (3.1, `"incremental": true`) already holds them. `PAGES`
marks the client's later pages stale until they are current again, and `DONE`
reports the mode, restart page and convergence.

### The display-list writer under checkpoints

The P3 writer assumed one engine run per process. In a resident engine that
restores checkpoints, four things break, and each is fixed:

1. **The side table** (every node's source position) is engine state outside the
   word space. After a restore, cells freed and reused after the checkpoint held the
   old future's positions, so the edited page's carried-over lines would have had
   wrong spans. It is now copy-on-write chunks (16 KB) whose snapshot travels in the
   C-part state (`CState.dl`, taken and put back by `snapshot_state`/`restore_state`,
   ignored by `same_as`: it is not what TeX computes, and line numbers shift with
   every edit, which would defeat convergence if it were compared).
2. **The page index** was a per-process counter; it is now `total_pages` at
   ship-out, which the checkpoint restores.
3. **Font, image and form ids** are engine numbers that a restore can rebind. The
   writer describes each resource once per *key*, recomputes id → key after every
   restore, and a `Peer` (one per client) sends a FONT/IMAGE when an id is new *or
   now stands for another key*. The writer's own file lookups no longer enter the
   engine's read journal (which the convergence test compares).
4. **Spans** are names that outlive restores and compiles: side-table entries are
   span ids, and when an edit moves lines the host moves the spans with them
   (`move_lines`) and re-declares moved spans to clients that hold them
   (`SOURCES`), so kept pages still point at their source. The standalone path
   (`FLASHTEX_DISPLAY_LIST`) is one `StreamSink` with one peer: the same frames in
   the same order as before.

### Two engine fixes the socket host needed (texmfmp.c parity)

With `-output-directory`, which every socket compile uses, LaTeX never found the
`.aux` a previous run wrote there: `\IfFileExists` asks `\pdffilesize`, whose
`find_input_file` (texmfmp.c) tries the output directory first, and ours did not.
Measured on hyperref-toc with `-output-directory`: pdflatex (TeX Live 2026) second
run 0 undefined references; ours 18 before, **0 after**. The incremental journal
also missed that a file appeared there (the lookup that found nothing now names the
output directory, and `lookup_again` tries it first, as `open_input` does).

## Verified results (mac-m5pro-kabir, M5 Pro; loads in `raw/environment.txt`)

Engine binaries for the final gate set (P-T1/P-T2, positions, lockstep):
`b85040c6d`, the tree of this PR without this evidence (origin/main merged at
`300daf2bd`). trip and drift ran on `a15ef631a`, etrip on `6c42484a8`, the
soundness harness on `6c42484a8`; the commits after them change no engine code
(tests, docs, a doc comment, the S₀ save line).

| gate | result |
|---|---|
| P-T1 / P-T2, parity fixtures, display list off | **83/83, 83/83** (`raw/parity-fixtures.txt`) |
| P-T1 / P-T2, parity fixtures, display list on (`FLASHTEX_DISPLAY_LIST=/dev/null`) | **83/83, 83/83** (`raw/parity-fixtures-display-list.txt`) |
| display-list positions checker, all fixtures | **83/83 exact (0 sp): 230 pages, 118,899 glyphs, 1,513 rules** (`raw/positions.txt`, `raw/positions-fixtures.json`) |
| the same in preview mode (`FLASHTEX_PREVIEW=1`, the resident host's mode) | **83/83 exact**, same counts (`raw/positions-preview.txt`) |
| lockstep | **260/260**, accounting 0 differ (`raw/lockstep.txt`) |
| trip, etrip, web2rust drift | pass (`raw/trip.txt`, `raw/etrip.txt`, `raw/drift.txt`; etrip needed `scripts/flashtex-etrip.sh` to copy `src/host/`) |
| licence boundary | clean (`scripts/check-license-boundary.sh`) |
| `tests/host_incremental.rs` (new, through the MIT client) | 3/3 pass, and the `--ignored` sweep of every fixture: **83 documents, 172 compiles compared, 0 differences** (`raw/host-sweep.txt`; 24 small probes had no prose line to edit) |
| incremental soundness harness (P4-L2-L3's `soundness.py` on `flashtex-host iserve`: all 83 fixtures + plain-100 + full-100, 25 random single-letter edits and their reverts each, every compile's PDF/log/aux/out/toc/terminal byte-compared with a from-scratch run) | **4,250 compiles, 0 mismatches**, 264 converged (`raw/soundness.txt`, `raw/soundness.jsonl.gz`) |
| `scripts/gate.sh pr` | passed (`raw/gate-pr.txt`) |
| `tests/display_list_host.rs` (lane P3) | passes **unchanged** against the unified host |
| `tests/incremental.rs`, `tests/checkpoint.rs` (lanes P4) | 3/3, 3/3 |
| clippy `--all-targets -D warnings`, rustfmt (changed crates) | clean |

### The end-to-end test (`tests/host_incremental.rs`)

Through `flashtex_display_list::client` against a real `flashtex-host --socket`:
open a document (compile until the `.aux` is stable), then for each edit send a
COMPILE with the edit and the edited page as `viewport`. Checked every time: the
first PAGE is the edited page or the one before (the restart point), pages arrive in
order, the edited page is re-sent, `PAGES` goes stale-then-complete, and **every
page the client holds equals the page a from-scratch compile gives** (a second host
that never saw the document, on a copy of the directory as the compile found it):
content hash, items, links, destinations, glyph fonts by key, and every source span
resolved to (file relative to the project, line). Each compile that settles the
`.aux` after an edit is compared the same way.

- generated 12-page article: a word replaced, a comment line inserted (the output is
  the same, every later line moves), a paragraph inserted (reflow, 14 → 15 pages,
  then an `.aux`-driven settle compile), a word replaced near the end;
- hyperref-toc fixture (links, destinations, TOC): a word replaced (converged after
  page 4), a comment line inserted;
- `export: true`: the host runs itself as `pdftex` in a child, frames in page order,
  a compressed PDF.

### Keystroke → page through the socket (`dl3-keys`, client side)

`scripts/bench.sh` (`raw/bench.txt`, `raw/*.keys.jsonl`): P4-L2-L3's `gen.py`
documents (plain: article, amsmath; full: plus hyperref, siunitx, cleveref, xcolor,
footnotes, labels), one host per document, 40 keystrokes each on a prose line in the
middle of the document (alternately a letter inserted into a word and deleted), each
sent as a COMPILE with the edit and the viewport, the next after the previous DONE.
Times from sending COMPILE to receiving the frame, measured in the client. Load
average 10–24 during the run (other lanes' builds and tests).

| document | pages | first PAGE p50 / p95 | edited page p50 / p95 | DONE p50 / p95 |
|---|---|---|---|---|
| plain-10 | 10 | 3.8 / 6.4 ms | **5.9 / 8.7 ms** | 35.6 / 38.5 ms |
| plain-120 | 121 | 4.5 / 5.7 ms | **4.5 / 5.7 ms** | 23.9 / 32.1 ms |
| full-10 | 11 | 11.4 / 12.9 ms | **20.5 / 22.9 ms** | 65.7 / 68.4 ms |
| full-120 | 120 | 10.2 / 11.2 ms | **10.2 / 11.2 ms** | 480 / 502 ms |

"First PAGE" is the restart page; on the 10-page documents the restart point is the
page before the edited one. DONE includes the background pass to convergence or the
end: plain-120 converges; full-120 does not (the hyperref documents' known P4 case)
and re-typesets the second half. An earlier run at load 6–9 gave 5.1/5.5, 4.0/4.2,
19.3/22.2 and 9.3/10.2 ms for the edited page.

**Reopen** (§1.2, ≤ 100 ms): a new host (warmed up, then listening) whose S₀ cache
has the document, first PAGE of its first compile: plain-10 **16.0–18.3 ms**,
plain-120 **16.7–19.8 ms**, full-10 **45.2–50.3 ms**, full-120 **44.1–48.1 ms**
(3 runs each, mode `open`).

**What the display list costs a cold compile** (`scripts/dlcost.sh`,
`raw/dlcost.txt`, resident engine, preview mode, 3 runs, load ~10): plain-120
268–294 ms without it (`iserve`), 359–381 ms with it (socket host, 6.46 MB of frames
including fonts); full-120 1012–1030 ms vs 1201–1234 ms. A 6-second sample of six
cold plain-120 compiles (`scripts/prof.sh`) puts the content-stream interpreter at
~38 ms per compile, the allocation hooks at ~13 ms and the side table's
copy-on-write at ~8 ms (belief from sampling, not a controlled measurement).

## The merge-queue failures of #1217 (run 36644076827)

| job | cause | evidence | disposition |
|---|---|---|---|
| `quick (touched crates)`, `rust workspace (ubuntu-latest)` | `tests/format_cache.rs` killed by **SIGSEGV** on Linux. Two of its three tests set kpathsea up in-process (`KpathseaResolver`, configured by `putenv`) while the third spawns processes on another thread: glibc's environment is not safe against that. | The same crash in #1216's own queue run 36639150284 (before #1217 existed) and in 36644470023 (#1226, queued behind #1217); #1217 does not touch the format cache. Not reproduced on macOS (0/30 runs). | **Fixed**: the binary's tests take one lock (`serial()`), commit a15ef631a. |
| `mac app (swift build + test)` | `CompletionTests.testCandidateComputationOnDemoTexStaysUnderTwoMilliseconds`: 2.70 ms averaged against a 2 ms bound on a GitHub-hosted runner | artifact `mac-swift-test-log`, line 14160; #1217 touches nothing under `apps/` | **Pre-existing timing flake**, not ours; the retry that landed #1217 (run 36646277853) passed. |

## Limits and beliefs (not verified here)

- **No preemption inside a run.** A newer COMPILE waits for the running compile's
  background pass (it goes on silently and its DONE says `cancelled`); on full-120
  that pass is ~0.4 s, so fast typing there queues. `incr` has no "compile while
  paused" that keeps the paused run as the old one (it goes cold), so the host
  always finishes. Belief: the fix belongs in `incr` (L5 lane), after which the host
  can stop at the viewport and skip the rest when a newer compile waits.
- **Line insertions never converge** (`line` is in the state), so kept pages after
  a line-moving edit exist only when convergence happens in another file; the span
  re-declaration covers them (unit test `spans_follow_moved_lines`) but no fixture
  exercises it end to end.
- **Memory:** side-table chunks copied since each checkpoint are held with its
  C-part record (outside the undo-log budget). Not measured separately.
- Persisted S₀ carries no side table: nodes made before `\begin{document}` have no
  span after a reopen.
- One document per host process; the app runs one per open document.

## Reproducing

```
cargo build --release -p flashtex-engine -p flashtex-display-list
bash scripts/gates-setup.sh && bash scripts/gates.sh      # P-T1/P-T2, positions, lockstep, trip, etrip, drift
bash scripts/bench.sh                                      # keystroke latency, reopen
bash scripts/dlcost.sh                                     # display-list cost of a cold compile
cargo test --release -p flashtex-engine --test host_incremental
cargo test --release -p flashtex-engine --test host_incremental -- --ignored   # every fixture
bash scripts/soundness-setup.sh && (cd /tmp/hu-sound && PYTHONHASHSEED=0 python3 soundness.py u -j 4 --trials 25 ...)
```
