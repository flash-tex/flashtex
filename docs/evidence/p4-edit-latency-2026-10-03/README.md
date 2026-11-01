# P4-EDIT-LATENCY: where an in-body edit's 11 ms goes (2026-10-03, mac-m1max-a)

Lane **P4-EDIT-LATENCY** (DESIGN.md §1.2 decision 1: the host's share, COMPILE written → edited
page's `PAGE` frame read, ≤ 11 ms p95; §5; §5.6; §12 P4/P6). Harness: `tools/incr-bench/t7.py`
(the T7 gate) over the host's socket, keystrokes 300 ms apart, keep-warm on (the default), 20
keystrokes per row (19 samples, the warm-up left out).

**Every number here is from a non-reference run.** The Mac (M1 Max, 10 cores, 32 GB) was in
**macOS Low Power Mode** throughout (`pmset -g`: `lowpowermode 1`), switched between battery and
AC power during the runs, and other sessions' builds kept load1 at 6–25. Only the relative
before/after numbers, measured interleaved on the same machine, are meant to be read; the absolute
milliseconds are pessimistic. Each run's power and load: `runs/*/power.txt` and the summaries'
`power` field.

## What was measured

- **Before**: PR #1300's head `1a12cc813` (P4-MEMORY: restore from the end, keep-warm), which
  landed on main as `c17b9b753` while this lane ran.
- **After** ("fix"): the same tree plus this lane's change (commit `bd6c8a576` on main
  `c17b9b753`; the engine diff is identical to the one measured).
- Interleaved: before, after, before, after (`scripts/ab.sh`), rounds 1–2 on full-100,
  full-1000 and plain-100 (letter@start, letter@middle, letter@end, in t7's rotating order),
  rounds 3–4 on full-100 only. Raw: `runs/ab-{p1300,fix}-r{1..4}/summary.json.gz`
  (every keystroke with the host's `DONE` and its stages), `table.md` per run.
- Stage times are the host's (`DONE.stages`: `apply`, `find` (of which `key`, `changes`),
  `restore`, `first_page_dl`, `edited_wall`, `test`), split by `scripts/split.py`; the
  restore was further split with temporary timers (not committed) and `sample(1)`
  (`runs/profile-full-100-letter-end-top.txt`).

## VERIFIED: the breakdown of the edited page (after the fix, round 1, p50 ms)

| doc | edit | edited page (client) | request: queue + apply + find (S0 key, changes) | restore | engine to the edited page's shipout, display lists excluded | display list, first page shipped | pages shipped to the edited one | socket and client |
|---|---|---|---|---|---|---|---|---|
| full-100 | letter@start | 16.4 | 1.6 (0.4, 0.4) | 2.8 | 8.5 | 3.2 | 1 | 0.35 |
| full-100 | letter@middle | 15.9 | 1.7 (0.4, 0.4) | 2.9 | 7.8 | 3.1 | 1 | 0.48 |
| full-100 | letter@end | 22.5 | 1.6 (0.4, 0.4) | 3.3 | 14.1 | 3.1 | 2 | 0.37 |
| full-1000 | letter@start | 21.3 | 3.1 (0.9, 1.0) | 3.9 | 10.2 | 3.3 | 1 | 0.67 |
| full-1000 | letter@middle | 20.1 | 2.9 (0.9, 1.0) | 3.5 | 10.0 | 3.2 | 1 | 0.50 |
| full-1000 | letter@end | 22.7 | 2.8 (1.0, 1.0) | 4.1 | 12.2 | 3.2 | 2 | 0.36 |
| plain-100 | letter@start | 9.5 | 1.2 (0.2, 0.2) | 2.2 | 2.7 | 3.0 | 1 | 0.40 |
| plain-100 | letter@middle | 9.8 | 1.2 (0.2, 0.2) | 2.4 | 2.8 | 3.2 | 1 | 0.43 |
| plain-100 | letter@end | 13.0 | 0.8 (0.2, 0.2) | 1.8 | 7.2 | 3.1 | 2 | 0.12 |

"Engine to the edited page's shipout" is `edited_wall` less the request, the restore and the
first page's display list: re-typesetting from the restart point, the output routine, the PDF
page and the checkpoints on the way. Where two pages ship before the edited one (letter@end: the
edited paragraph starts on the page before, which TeX ships only after reading the whole
paragraph, so the restart point is before that page), it also holds the second page's display
list (2.1–2.3 ms, measured with a temporary timer). The convergence test (`test`, 6–20 ms) runs
after the edited page is out; it does not delay it. Socket and client: under 0.7 ms everywhere.

On the 11 ms budget for full-100 letter@middle (15.9 ms): request 1.7 + restore 2.9 + engine 7.8
+ display list 3.1 + socket 0.5.

## VERIFIED: the cost fixed here

**Restores grew with every converged compile.** Each convergence appends a PDF-position
correction (`Reloc`) to every later checkpoint it keeps (`incr.rs`, `after_run`), and a restore
of such a checkpoint applied them in turn, each one rebuilding `rs_seen` from the whole read-set
(`readset::rebuild_seen`): about 1.0 ms each on full-100 (8,419 read-set events), against
1.4 µs for the position corrections themselves (1,051 PDF objects). So in a typing session the
restore cost rose by about 1 ms per converged keystroke for every checkpoint after the
convergence point: in T7, letter@end (after the letter@middle phase's 20 converged edits)
restored in 20–26 ms instead of 3–4. Measured with temporary timers on full-100 letter@end after
8 converged edits: `Globals::restore` itself took 2.7 ms and the corrections loop 7.3 ms of a
10 ms `restore` stage (8 rebuilds at 0.9–1.1 ms, positions 0.011 ms in all).

**Fix** (`Reloc::apply_all`): the position corrections still run in order, once each, and
`rs_seen` is rebuilt once after the last. `rebuild_seen` reads only the read-set and the live
names (hash, string pool); the corrections write only PDF file positions (`pdf_gone`,
`pdf_save_offset`, `pdf_stream_length_offset`, `obj_tab` offsets and the `overrides`, which are
`obj_offset` words from `new_positions`), so one rebuild after the last correction leaves the state
that a rebuild after each one left. `rebuild_seen` also builds one view for all events and lends
the events instead of copying them (1.04 → 0.93 ms at 8,419 events).

Edited page, client side, ms p50 / p95 (restore stage p50), before → after, interleaved:

| doc | edit | round 1 | round 2 | round 3 | round 4 |
|---|---|---|---|---|---|
| full-100 | letter@end | 39.0 / 40.1 (20.1) → 22.5 / 24.1 (3.3) | 39.6 / 40.8 (20.2) → 30.5 / 104.9 (5.5)\* | 39.6 / 41.4 (20.4) → 22.5 / 26.1 (3.5) | 41.3 / 60.1 (21.0) → 22.3 / 23.8 (3.1) |
| full-1000 | letter@end | 44.9 / 64.1 (26.1) → 22.7 / 25.7 (4.1) | 43.2 / 44.5 (25.1) → 22.9 / 23.7 (4.8) | | |
| plain-100 | letter@middle | 14.7 / 20.5 (7.1) → 9.8 / 10.8 (2.4) | 14.7 / 16.0 (6.9) → 10.2 / 10.9 (2.9) | | |
| full-100 | letter@middle | 15.6 / 16.1 (2.5) → 15.9 / 19.6 (2.9) | 15.6 / 19.7 (2.4) → 18.7 / 54.2 (3.6)\* | 15.6 / 21.3 (2.8) → 16.6 / 18.5 (2.7) | 16.9 / 23.2 (2.7) → 15.6 / 16.1 (2.6) |
| full-100 | letter@start | 15.8 / 16.4 (2.7) → 16.4 / 17.4 (2.8) | 16.2 / 18.6 (2.5) → 16.8 / 22.6 (2.6) | 17.2 / 41.2 (3.5) → 17.1 / 20.9 (3.2) | 16.8 / 19.2 (3.1) → 16.0 / 18.3 (2.6) |
| full-1000 | letter@middle | 24.8 / 34.5 (3.8) → 20.1 / 29.0 (3.5) | 19.3 / 19.8 (2.9) → 20.7 / 22.2 (4.0) | | |
| full-1000 | letter@start | 21.0 / 22.2 (4.1) → 21.3 / 25.7 (3.9) | 21.3 / 22.5 (3.5) → 21.6 / 22.8 (5.0) | | |
| plain-100 | letter@start | 9.3 / 11.5 (2.1) → 9.5 / 11.4 (2.2) | 9.3 / 10.7 (2.2) → 9.4 / 10.8 (2.6) | | |
| plain-100 | letter@end | 14.1 / 18.7 (2.3) → 13.0 / 14.0 (1.8) | 14.1 / 15.6 (2.0) → 13.4 / 15.3 (2.3) | | |

\* round 2's "after" full-100 rows ran while load1 rose from 7 to 26 (another session's build);
its 105 ms and 54 ms p95 are that load (the restore stage stayed at 3–6 ms).

- The rows that follow converged edits (letter@end after letter@middle; plain-100 letter@middle
  after letter@start) lose 16–22 ms p50 at full-100/1000 and ~5 ms at plain-100: restore
  20–26 → 3–5 ms. plain-100 letter@middle's p95 goes from 16.0–20.5 to 10.8–10.9 ms, under 11.
- The other rows do not change beyond noise (no corrections had accumulated on their
  checkpoints).
- Convergence and re-typeset pages: identical before and after on every row (`table.md`,
  `converged`, `re-typeset pages`).
- Without the fix the cost keeps growing for as long as the user types: n converged keystrokes
  add about n ms to every later restore past the convergence point. With it, what still grows
  with n is the position corrections (about 1.4 µs each at 1,051 objects, more on larger
  documents) and the memory of the per-checkpoint `Reloc` lists (see the plan).

## Soundness (main `c17b9b753` + the fix, `bd6c8a576`; Mac, MacTeX 2026 as the oracle where the gate uses one)

VERIFIED on this Mac, outputs in `runs/gates/`:

| gate | result |
|---|---|
| soundness A (subset: 20 single-character edits + reverts, not gates.sh's 50; every fixture + plain-120 + full-100) | 3,520 compiles, 0 mismatches, 286 converged (full-100 33/40, plain-120 33/40) |
| soundness C (subset: 10 structural edits, not 20; fixtures + refs-30 + refs-120 + full-100) | 970 compiles, 0 mismatches |
| `cargo test --release` incremental, host_incremental, display_list_host | 25 passed, 1 ignored (as on main) |
| lockstep (`tools/lockstep/run.py`) | 1,435/1,435 equal (7 differ in accounting only, which §1.1 does not compare) |
| trip, etrip | pass |
| `scripts/gate.sh pr` (load1 up to 159 from other sessions) | every step passes except clippy, which fails on 4 `needless_range_loop` errors in `pdftex/writettf.rs` (not touched here) with the local nightly clippy 1.100; `cargo +stable clippy -p flashtex-engine --all-targets -D warnings` (1.98.1, CI's `stable`) passes (`runs/gates/clippy-stable.txt`) |

The new test `tests/incremental.rs::a_restore_after_several_convergences_equals_scratch_runs`
makes letter edits that converge in the middle, then edits at the end that restore checkpoints
carrying 1–3 corrections (counted with a temporary probe); every compile equals scratch runs.
It guards the multi-correction restore; it is not a fail-before case, since the change keeps the
resulting state (sweep A, whose sessions converge 286 times, covers the same path at scale).

## BELIEVED (not measured): the remaining plan, by expected gain

1. **Engine re-typeset to the edited shipout, 7.8–10 ms on full-\*, 2.7 ms on plain-\*.** Most of
   the full-\*/plain-\* difference is the per-page output routine with hyperref (DESIGN.md §5.6
   item 6: the page label and anchor PDF strings), already the named intrinsics target. Next:
   profile one edited page's span alone (restart → shipout) with the timers used here, split into
   paragraph building, `fire_up` with the output routine, and `pdf_ship_out`. letter@end pays
   for a second page because the edited paragraph starts on the page before; a restart point
   inside that paragraph is not possible (TeX reads the whole paragraph before breaking it), so
   only a cheaper page helps there.
2. **Display list of the first page after a restore: 3.0–3.3 ms**, about 1.1 ms more than the
   next page's 2.1–2.3 ms. `displaylist::restored()` clears the font-key, width, advance and
   font-kind caches on every restore (the next page recomputes them: map lookups, kpathsea
   lookups, encodings). Keeping the entries of fonts the restored state already had loaded, with
   a check of the inputs each key was computed from (name, size, map entry, kind), would save
   about 1 ms per edit on every document; it needs its own soundness case (a `\pdfmapline` edit
   before a font's first use). The remaining ~2.1 ms per page is the content-stream interpreter.
3. **Restore, 2.8–4.1 ms after the fix**: output tails ~1.2 ms (reading the PDF's tail back),
   host state ~0.9 ms, undo ~0.5 ms (the restore prepared by #1300), and `rebuild_seen` ~0.9 ms
   whenever corrections are present. The rebuild could be skipped when the read-set and names
   are unchanged since the last rebuild at that checkpoint, or the corrections composed into one
   per checkpoint at convergence time (which also bounds their memory).
4. **Request, 1.6–3.1 ms**: on full-1000 the S0 key check (0.9 ms) and the change detection
   (1.0 ms) each scale with the inputs; both could be cached between keystrokes when only the
   edited file changed.
5. **The machine.** Every number here is under Low Power Mode at load 6–25. T7's reference run
   (AC, Low Power Mode off, quiet) is still needed before any row's absolute verdict.

Sum of 2–4 at their estimates: about 2–3 ms per edit on full-\*; with 1 the full-\* rows could
approach 11 ms only if the output routine's cost falls by half or more.
