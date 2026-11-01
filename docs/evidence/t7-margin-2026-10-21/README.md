# T7-MARGIN: headroom on the five rows the 2026-10-21b reference run missed

Lane T7-MARGIN (Commander kabir-claude). The M5 Pro reference run on main `8d23cf520`
(`docs/evidence/t7-reference-2026-10-21b-Kabirs-MacBook-Pro/`, plugged in, all checks passed) passed
115 of 120 gated rows. The five misses, p50 / p95 / max ms: full-10 letter@start 9.7 / 11.2 / 15.4,
full-1000 typing@60ms 9.2 / 13.4 / 14.2, typing@100ms 9.4 / 12.2 / 12.3, beamer-30 sentence@middle
9.1 / 11.2 / 11.8, beamer-30 letter@start 9.4 / 11.6 / 12.2. The aim is headroom on the Mac (p50 about
8 ms or less, p95 about 9.5 ms or less) with output byte-identical.

**Kinds of numbers.** The Mac figures come from the two reference runs' `raw.tgz` (VERIFIED, read from
the records). Every new measurement is from the NixOS PC (Ryzen 7 7800X3D, shared, `ftx-run`), so its
times are non-reference. The load-independent measure is the engine thread's instructions
(`DONE.stages.*_instr_k`). What the Mac will measure after these changes is a **belief** until the
Commander's next reference run.

- **Before:** main `b09b7bb14` (engine `base`; the engine crates are `8d23cf520`'s, the reference run's).
- **After:** `agent/kabir-claude/t7-margin` (engines `m5`: all but §5; `m6`: all).
- **A/B:** `t7.py --docs D --phases P --preamble 0 --reopen 0`, two rounds, base then the branch each
  round, 19 measured keys per row per round (39 on typing rows); the tables pool both rounds.
- **Files:** `raw.tgz`: `pc/` (t7's raw records and tables of every PC run named below), `prof/`
  (perf over the pre-typesetting and typesetting windows, `pwin.py`), `scripts/` (the record readers
  used here).

## 1. Step 1: full-1000 typing@60 is not a regression (VERIFIED, Mac raw records)

typing@60ms went from 8.8 / 9.6 ms (p50 / p95, run a, main `5439b97ea`, on battery) to 9.2 / 13.4 ms
(run b, main `8d23cf520`). Between them #1864 (READ-TRACK-2), #1866 and #1868 (T7-LETTERSTART) merged.

| run b against run a, 40 keys | run a | run b |
|---|---|---|
| instructions to the first page p50 / p95 | 129.5 / 130.0 M | 129.2 / 129.9 M |
| abandoned keys (20): instructions, `paused` (the abandon) | 129.7 M, 1.87 ms (19.6 M) | 129.5 M, 1.92 ms (19.6 M) |
| queue wait p50 / p95 | 0.12 / 0.40 ms | 0.15 / 0.61 ms |
| restore p50 | 0.70 ms | 0.70 ms |
| IPC (instructions / cycles to the first page) min / p50 / max | 2.96 / 3.39 / 3.49 | **2.47** / 3.32 / 3.46 |

The same work, the same preemption and the same queue: the p95 is twelve consecutive keys (4 to 15,
about 0.7 s) that retired the same instructions at IPC 2.5-2.75 instead of 3.3-3.45, at an unchanged
clock (4.2-4.6 GHz of thread cycles per thread-CPU ms), plus key 8 (a preparation not yet done: 0.61 ms
of `prepare` queue and the full rewind, 26.6 M instead of 6.9 M) and key 14 (a 5.25 ms restore, 5.9 ms of
it off-CPU). That is the machine (cache or memory contention from outside the engine thread), not
#1864 or #1868. typing@100ms in run b is different: three keys waited 2.70-2.78 ms in the queue for
the running compile (`queue_by.typeset`), which §3 explains.

## 2. Where the five rows' time goes (VERIFIED, run b; Mac against PC instructions from the PC's `base`)

| row | to the first page, Mac / PC | typesetting, Mac / PC | restore, Mac / PC | what else |
|---|---|---|---|---|
| full-10 letter@start | 125.7 / 89.5 M | 121.2 / 92.8 M | 5.1 / 2.1 M | `apply` 0.45 ms on the Mac, 0.03 on the PC |
| full-1000 typing@60, abandoned keys | 129.5 / 95.5 M | | 6.9 / 3.2 M | the abandon: 1.92 ms, 19.6 M on the Mac, 6.3-6.6 M on the PC |
| full-1000 typing@100 | as typing@60 | | | 3 keys queued 2.7 ms behind the running compile |
| beamer-30 sentence@middle | 104.3 / 88.3 M | 88.6 / 86.3 M | 9.9 / 2.9 M | key 1: 160.8 M; `apply` 0.70 ms |
| beamer-30 letter@start | 108.8 / 91.2 M | 92.9 / 89.2 M | 10.0 / 3.0 M | key 1: 162.9 M (PC 142 M); `apply` 0.75 ms |

Typesetting costs the same on both machines except on page 0 (full-10 letter@start, +28 M on the Mac;
plain-10 letter@start likewise: 71.4 M to its first page on the Mac against 41.8 M on the PC). What
costs more on the Mac only is, most likely, kernel work, which the Mac's per-thread counters count and
the PC's (user space) do not: the restore (+5 to +7 M), the abandon (+13 M) and the page-0 window. The
PC shows no page faults in the first-page window or the restore (`first_page_flt`, `restore_flt` = 0
on every row of `m1`), so which kernel work the Mac does there is not known: §3.6 adds the counters
that will say.

`apply` (writing the typed edit to the file before the compile) is a Mac-only cost too: 0.45-1.6 ms
for every letter, sentence and split row against 0.12-0.34 ms for the newline rows (0.60 ms on
full-1000's 4 MB file), whose edit (a newline for a space) keeps the file's length; 0.03 ms on the PC.

## 3. Changes

1. **The thinning waits while newer work does** (`incr.rs`, `Obs::on_checkpoint`). Every 32nd
   checkpoint a running compile merges logs (`thin`, `thin_pending`) before it asks whether newer
   work waits: milliseconds nothing stops. A key arriving just before it waited for it (typing@100,
   run b: 2.70-2.78 ms on 3 of 40 keys). Now the merge is skipped while newer work waits and made at
   the next checkpoint without it. The look asks the host's callback only where the preemption check
   would, and its answer is acted on at the next check (the soundness driver's callbacks answer once:
   `tests/incremental.rs`' `a_cold_run_stops_no_earlier_than_s0` failed until it was).
2. **An abandoned run's reattach copies nothing when the restore goes back to its target**
   (`arena::Lazy`, `Globals::reattach_pending_lazily`, `incr::Session::compile`). Typing on a long
   document: the next key abandons the running compile, which put the old run back whole (its latest
   state copied in, a prepared restore to the target made: four copies of the redo's 3,000-7,500
   chunks), and the restore went straight back to the same target. Now the reattach leaves the target's
   state live and keeps the redo; a restore to that target takes it back as the branch, copying
   nothing. Anything else first makes the reattach whole: every changing method of the arena, the
   barrier's first write (all flags are clear meanwhile), and every reading one asserts there is none;
   the compile makes it whole if its pass did not restore there. The scalars are read from the state
   (`Arena::read_state`), and the restore stays lazy only while the host-state capture reads nothing
   else (`lineshift::marks_read_mem`). Test: `arena::tests::a_lazy_reattach_is_a_reattach` (the
   target's state, its checkpoints and a branch that reattaches to the old end; a write, a restore
   elsewhere and the scalars under a lazy reattach). The convergence and host tests exercise it
   (`tests/incremental.rs`, `host_incremental.rs`: 10 lazy restores with `FLASHTEX_INCR_DEBUG`).
3. **A journal's files and lookups are shared between its clones** (`system::ReadLog`,
   `files_mut`, `lookups_mut`). The journal was cloned whole at every pass, before every restore and
   for a stopped run's check, and dropped as often: 25-30 % of the host's work before a beamer
   keystroke's restore (perf, PC). They are now copied on change, as `seen` already was.
4. **An edit sets the file's length only when the write leaves it wrong** (`host::resident::apply_changes`):
   a growing file grows by the write; a shrinking one is cut after it. Belief: on the Mac's APFS
   the length change is what `apply` pays (see §2); the PC shows nothing (0.03 ms).
5. **BOX-MEMO records `\pgf@parsefunc` anywhere** (BOX-MEMO.md §14): the second key on a frame hits.
6. **Measurement:** `DONE.stages` gains page faults and, on macOS, system calls (`os::events`:
   `TASK_EVENTS_INFO`, the task's; Linux: the thread's faults): `flt`/`sys` for the compile,
   `first_page_*`, `restore_*`, `paused_*`, and `apply_sys`.

## 4. Before and after (VERIFIED, NixOS PC; times non-reference)

| row | engine | ms p50 / p95 | instructions to the edited page p50 | restore p50 | key 1 | abandoned keys: abandon ms / instructions |
|---|---|---|---|---|---|---|
| full-10 letter@start | base | 9.2 / 10.1 | 95.0 M | 2.08 M | | |
| | m5 | 8.9 / 9.9 | 94.5 M | 1.89 M | | |
| full-1000 typing@60ms | base | 10.4 / 14.4 | 95.5 M | 3.16 M | | 2.00 ms / 6.6 M |
| | m5 | 9.2 / 10.3 | 93.3 M | 2.06 M | | 1.27 ms / 5.4 M |
| full-1000 typing@100ms | base | 10.5 / 11.5 | 96.5 M | 3.16 M | | 2.33 ms / 7.5 M |
| | m5 | 9.6 / 10.2 | 94.1 M | 2.10 M | | 1.54 ms / 6.2 M |
| beamer-30 sentence@middle | base | 9.1 / 10.4 | 90.1 M | 3.00 M | 89 M | |
| | m6 | 9.1 / 10.2 | 89.6 M | 2.45 M | 89 M | |
| beamer-30 letter@start | base | 9.2 / 10.8 | 91.7 M | 3.14 M | **143 M** | |
| | m6 | 9.3 / 11.3 | 92.6 M | 2.57 M | **93 M** | |

The beamer rows' p95 on the PC is the slowest steady key plus the PC's own noise (one 18 ms key
with 7.6 ms off-CPU in the m5 round): the measure there is key 1's instructions. The `ab*` tables
(`raw.tgz`) hold every other row of full-10, full-1000, beamer-30 and plain-10 (letter@start,
letter@middle, sentence@middle, typing@60, typing@100): with `m5`, instructions to the edited page fall
by 0.2-0.8 M on every row but plain-10 letter@start (45.1 M on both), and by 2.2-2.4 M on full-1000's
typing rows. With `m6` (beamer-30, beamer-5) letter@start and letter@middle read 0.8-0.9 M more than
`base`, sentence@middle 0.5 M less: within the counts' run-to-run spread (the same `base` read 92.4 M
in one A/B and 91.7 M in the other on letter@start). Re-typeset pages and convergence are unchanged on
every row.

Each change on its own (PC):
- the thinning: removes the 2.7 ms queue stretch; not reproduced on the PC, whose arrivals fall
  elsewhere (its `typeset` queue waits are 0.3-1.0 ms on both engines);
- the lazy reattach (`typ*`, base `m1` = main + 1 + 6, against `m3`): full-1000 typing, abandoned keys,
  instructions −2.0 to −2.3 M (the abandon −1.0 to −1.4 M, the restore −0.8 M), the abandon
  1.67-2.30 → 1.13-1.86 ms;
- the journal (perf, beamer-30 sentence@middle, 10 windows): samples before the engine resumes
  100 → 73 (`prof/`); every row's restore −0.2 to −0.55 M;
- `\pgf@parsefunc` anywhere (`pf*`, `po*`): beamer-30 letter@start key 1 143 → 93 M; beamer-30's
  preamble row (a run from the format) 24.48 → 24.14 G instructions for the compile, 5.29 → 5.24 G
  to its first page, 406 / 408 against 406 / 409 ms; reopen unchanged (150 / 151 ms).

**Callgrind** (PC, `valgrind --tool=callgrind`, instrumentation on while dl3-keys typed 4 keys after
one warm-up key, the whole host, background work included; base against the branch at `e1e68fc73`):

| session | base | branch |
|---|---:|---:|
| beamer-30 letter@start (39:7) | 2,986.0 M | 2,991.7 M (+0.2 %) |
| full-10 letter@start | 1,967.4 M | 1,955.6 M (−0.6 %) |

Under valgrind the parsefunc gain does not show: the first measured key recorded the four
`\pgf@parsefunc` calls with both engines (BOX-MEMO's commit counts), so those calls' keys differed
from the open's there, which they do not in the native T7 runs (`pf*`: key 1 commits none and hits
323 calls). The likely difference is the time-based segment checkpoints, 50 times denser in work under
valgrind, which move the restart point; not investigated further. The lazy reattach is not in these
sessions (no typing interval).

## 5. Belief: the Mac after these changes (not measured)

- **full-1000 typing@60/@100:** the abandon cost 19.6 M and 1.92 ms there, against 6.6 M on the PC;
  most of the Mac's excess is likely the copies and fresh pages the lazy reattach no longer makes
  (the PC saves 1.4 M of user-space instructions of those). With the 2.7 ms queue stretch gone,
  the abandoned keys should cost about what the idle ones do (7.4-7.8 ms p50), the rows' p50 about
  8 ms; their p95 still depends on the machine's bursts (§1).
- **beamer-30 letter@start and sentence@middle:** key 1 was the slowest key of both rows. Without
  it, run b's other keys put the p95 at 10.6 and 10.3 ms (the second-slowest of the rest), under
  11 ms but not 9.5: the steady keys cost 104-109 M (BOX-MEMO's key checks about 15 % of the
  typesetting: `bm_meaning_is` 8 %, `bm_k2_is` 3 %, perf).
- **full-10 letter@start:** the journal saves about 0.5 M; the edit's write (`apply`, 0.45 ms) may
  shrink by §3.4. The page-0 excess of 28 M is unexplained (§2). Its p95 should be about 10.5-11 ms:
  still no headroom.

## 6. What the next reference run should look at

`DONE.stages`' new `first_page_flt`/`first_page_sys`, `restore_flt`/`restore_sys`,
`paused_flt`/`paused_sys` and `apply_sys` on full-10 letter@start against letter@middle, beamer-30,
and full-1000 typing: they say whether the Mac's excess there is page faults (large blocks have
mappings of their own on macOS, `logalloc`) or system calls, and so which of the next steps pays.

Next steps, by expected gain per effort:
1. From those counters: the Mac-only kernel work in the restore (+5-7 M a key on every row), the
   abandon and page 0.
2. full-10 letter@start: a restart point after the paragraph's first inline math (T7-CONV: the first
   math's font setup, 26 TFM loads a key); a body mid-line restart, like PREAMBLE-MIDLINE's.
3. beamer steady keys: BOX-MEMO's K3 checks (the `Seen` cache is emptied at every restore).

## 7. Checks (VERIFIED, NixOS PC, at the branch)

- `tests/incremental.rs` (80), `host_incremental.rs` (27, 1 ignored), `intrinsics.rs` (7),
  `checkpoint.rs` (3), `boxmemo.rs` (10) and the lib's `iso::` tests, with BOX-MEMO at the hosts'
  default, `on` and `off`: all pass, except `checkpoint.rs`'s `selftest` with `FLASHTEX_BOXMEMO=on`,
  which fails the same way on main (BOX-MEMO.md §11.4).
- The lib's `arena::`, `system::`, `boxmemo` and `checkpoint` tests (61 pass), and the runtime's
  `os::` and `logalloc` tests (6).
- T1 lockstep 1,468 / 1,468 equal; P-T1 88/88 and P-T2 88/88; with BOX-MEMO off and on.
- `scripts/gate.sh quick` (rustup 1.98.1, fresh target dir) at `102e0d988`: passed (rustfmt, clippy,
  tests of the changed crates, trip, etrip). Its first run, at `97d6202b8`, failed: flashtex-xetex's
  journal call sites (fixed) and `a_cold_run_stops_no_earlier_than_s0` (the thinning's look at newer
  work, fixed in §3.1).
- `sweeps.yml` with `ref=` the branch at `e1e68fc73`: default (run 39567122916) and `boxmemo=verify`
  (run 39567127814), both success.
