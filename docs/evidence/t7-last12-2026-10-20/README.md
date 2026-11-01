# T7-LAST12: the twelve rows the M5 reference run missed (2026-10-20)

Lane T7-LAST12 (Commander kabir-claude). The reference run on main `fcc32b779` (PR #1855,
`docs/evidence/t7-reference-2026-10-20-Kabirs-MacBook-Pro/`) passed 108 of 120 gated rows; this lane
works on the other twelve: beamer-30's nine (letter@start, @middle, @end, sentence, newline and
split @middle, typing@50, @60 and @150ms), full-10 letter@start, and full-1000 typing@100 and
@150ms.

**Every number here is from the NixOS PC** (AMD Ryzen 7 7800X3D, x86-64, shared with other agents'
jobs inside `flashtex-agents.slice`'s CPU quota; another lane's callgrind ran beside the final run),
so every time is **non-reference**: the quota stopped the slice during these runs (off-CPU p95 up to
71 ms on some rows), and the PC's wall times say nothing about §1.2. What is load-independent, and
reported as the measure, is the engine thread's **instructions** to the edited page (the host's
`DONE.stages.edited_instr_k`, or `first_page_instr_k` where a row has none), with its **thread CPU**
(`edited_cpu`, `first_page_cpu`) beside it. The reference times come from the Commander's next Mac run.

- **Before:** main `9697ee948` (engine `base`; its engine crates are those of `fcc32b779`, the
  reference run's).
- **After:** `agent/kabir-claude/t7-last12` at `b613b19f5` (engine `l12r`).
- **Command:** `tools/incr-bench/t7.py --docs D --phases P --preamble 0 --reopen 0`, the rows'
  phases at their default keys (19 measured, 39 for typing), one host at a time, base then after.
- **Files:** `raw.tgz` (`fin-base-*`, `fin-l12r-*`: t7.py's raw records and tables).

## The rows (VERIFIED: instructions; CPU is p50 thread CPU, load-affected where marked)

| row | cause | fix | instructions to the edited page, before → after | CPU ms p50, before → after |
|---|---|---|---:|---:|
| beamer-30 letter@start | the deck's first block frame re-typeset per key; BOX-MEMO's own checks and replays 39 % of it | §12.4, §12.5 | 164.2 M → 113.7 M (−31 %) | 16.3 → 10.8 |
| beamer-30 letter@middle | the slide re-typeset per key (Madrid block, output routine); BOX-MEMO's overhead | §12.4, §12.5 | 141.2 M → 93.4 M (−34 %) | 14.4 → 10.0 |
| beamer-30 letter@end | as letter@middle | as above | 141.2 M → 94.0 M | 14.7 → 10.1 |
| beamer-30 sentence@middle | as letter@middle | as above | 141.7 M → 94.3 M | 15.0 → 14.0 (load) |
| beamer-30 newline@middle | as letter@middle | as above | 140.5 M → 91.1 M | 25.9 (load) → 10.3 |
| beamer-30 split@middle | as letter@middle | as above | 141.4 M → 93.9 M | 14.3 → 9.6 |
| beamer-30 typing@50ms | as letter@middle | as above, and newer work seen at node allocation | 141.4 M → 93.6 M | 12.9 → 9.0 |
| beamer-30 typing@60ms | as above; 4.7 ms queue wait on the Mac | as above | 141.4 M → 93.6 M | 13.0 → 10.2 |
| beamer-30 typing@150ms | as above | as above | 141.5 M → 93.6 M | 13.5 → 10.0 |
| full-10 letter@start | hyperref's `\pdfstringdefPreHook` (siunitx) run for real twice per key: the intrinsics' recordings of it are in the word space, and the page restores to before them | BOX-MEMO.md §12.2: BOX-MEMO owns the hook wherever it is on | 145.7 M → 96.6 M (−34 %); 95.0 M in a quieter run | 12.9 → 11.0; 13.3 → 9.5 in the quieter run |
| full-1000 typing@100ms | **both**: every other key arrives while the last compile's background still runs; it waited for the run to notice (queue up to 3.8 ms on the Mac), then abandoned that run (2.3 ms, 24 M instructions on the Mac) | newer work seen at node allocation; the abandon's prepared restore reuses its buffer; a convergence jump waits for no newer work | 96.2 M → 95.7 M | 9.7 → 9.3; queue p95 2.5 → 1.1 ms; abandon p50 2.27 → 1.96 ms |
| full-1000 typing@150ms | as typing@100; also a convergence jump (4–5 ms, not stoppable) in front of 2 of 39 keys on the Mac | as above | 98.0 M → 97.5 M | 10.2 → 9.8; abandon p50 2.69 → 2.29 ms |

Convergence and re-typeset pages: unchanged on every row (19/19 converged on all in-body rows,
before and after; beamer-30 newline 3 / 5 pages and split 5 / 5, as before).

The twelve rows' wall times in this run (p50 / p95 / max ms, non-reference):

| row | before | after |
|---|---|---|
| beamer-30 letter@start | 17.5 / 18.6 / 20.6 | 11.3 / 20.1 / 20.5 |
| beamer-30 letter@middle | 15.7 / 90.2 / 96.9 | 10.5 / 11.4 / 13.4 |
| beamer-30 letter@end | 15.9 / 98.2 / 100.1 | 10.5 / 14.7 / 15.8 |
| beamer-30 sentence@middle | 16.6 / 94.0 / 96.3 | 75.8 / 84.9 / 86.5 (off-CPU p95 68.5) |
| beamer-30 newline@middle | 93.6 / 98.4 / 100.8 | 10.5 / 11.8 / 73.8 |
| beamer-30 split@middle | 15.8 / 93.1 / 94.7 | 9.8 / 11.1 / 11.5 |
| beamer-30 typing@50ms | 13.8 / 14.3 / 14.8 | 9.1 / 13.3 / 63.4 |
| beamer-30 typing@60ms | 13.9 / 15.0 / 17.8 | 16.9 / 76.9 / 82.6 (the slice stopped: `idle` queue waits of 46 ms) |
| beamer-30 typing@150ms | 14.6 / 65.4 / 71.6 | 10.4 / 50.6 / 58.5 |
| full-10 letter@start | 13.3 / 13.9 / 14.4 | 11.2 / 78.8 / 82.9 (off-CPU p95 64.2) |
| full-1000 typing@100ms | 11.1 / 13.3 / 13.7 | 10.1 / 11.2 / 15.2 |
| full-1000 typing@150ms | 11.3 / 12.6 / 15.0 | 10.6 / 11.9 / 12.1 |

In an earlier, quieter run of nearly the same engine (`l12o`, PC): beamer-30's in-body rows 9.5–9.9
ms p50 and 10.4–12.9 ms p95, typing 8.8–9.3 ms p50; full-10 letter@start 9.5 / 10.6 ms. The first
six keys of the final typing@60 run, before the slice was stopped, took 9.1–9.7 ms.

## Changes

- **BOX-MEMO.md §12.2, adopted:** `pdfstringdefPreHook` is BOX-MEMO's wherever BOX-MEMO is on
  (`boxmemo::owns`; the intrinsics leave it out of their names, and BOX-MEMO records it anywhere),
  so every run of a session leaves the intrinsics' state alike and the convergence test is
  unchanged. The earlier try (the intrinsics leaving it only in the edit window) stopped
  convergence and was reverted (`11282390c`); this one keeps it (below).
- **§12.4, cheaper offers:** K3's macro bodies found equal are kept per location until their list
  is freed (`delete_token_ref`'s new hook, changes/boxmemo.ch) or the word space restored; a
  `Name` meaning is compared with the text at its location.
- **§12.5, replays make each group's net effect:** a group stands for its last global assignment
  per location (`unsave` keeps level-one locations); a `\let` of what the group has not assigned
  yet is made where the group was; a group kept whole loses the local assignments nothing in it
  reads. `\@oddhead` 13,667 → 3,051 replayed operations, `\pgf@process` ~180 → 3–4.
- **The host** (no change to what is typeset):
  - FS-WATCH (`crates/flashtex-runtime/src/fswatch.rs`): the per-keystroke checks of TeX Live's
    files are vouched for by inotify (Linux) or kqueue (macOS) instead of a `stat` each, with a
    `stat` whenever a watch cannot vouch: a path not seen before (the watch is made before the
    `stat` that is kept: the startup case), a queue overflow or read error (every path unvouched
    until its next `stat`), a watch that could not be made, a volume that is not local, a path in
    the document's or output directory, a racy signature. The kernel queues a change's event in
    the system call that makes it, and each check drains the queue first, so a change made before
    a keystroke's check is seen by that check, before its output is shown. Beamer-30: about 1,000
    signatures served and 45 `stat`s per key (the document's own files) instead of ~600 `stat`s.
  - Newer work is seen where a node is allocated (`checkpoint::PREEMPT_HINT`, polled in
    `dl_note_node`), the output routine included, instead of only at segment checkpoints and
    shipouts: the run stops at a segment checkpoint taken there.
  - A convergence jump whose comparison finished after newer work arrived is not made: the run
    pauses at the convergence point, as when the comparison is stopped.
  - The abandon's prepared restore reuses the last one's buffer (at most 64 MB kept): on
    full-1000 its 3.2 MB were faulted in at every abandoned compile (1.0 → 0.2 ms).
  - The structural comparison (`iso.rs`) follows a page insertion record's `broken_ptr` and
    `broken_ins` only for a `split_up` record, the only kind pdfTeX reads them in (found by the
    verify sweep, below).
  - The NewCs host test turns BOX-MEMO on itself; the intrinsics' verifier leaves BOX-MEMO's
    `bm_at_switch` flag as it leaves its own (it failed `tests/intrinsics.rs` with
    `FLASHTEX_BOXMEMO=on`, also on main).

## Correctness (VERIFIED, NixOS PC unless named)

- **Convergence of the §12.2 commit** (`efb529808`, before any other change here):
  `tests/incremental.rs` 78/78 and `tests/host_incremental.rs` 27/27 (1 ignored) with BOX-MEMO at
  the hosts' default (on), with `FLASHTEX_BOXMEMO=on` and with `off`, including
  `intrinsics_recorded_after_an_edit_converge`; T1 lockstep 1,468 / 1,468 equal and P-T1 88/88,
  P-T2 88/88 with BOX-MEMO off and on (`d0cc2f1f4`). T7 full-10 letter@start: 19/19 converged.
- **Final code** (`ec2deecab`, the same code as `b613b19f5` but for rustfmt):
  - `tests/incremental.rs`, `host_incremental.rs`, `intrinsics.rs`, `checkpoint.rs`, `boxmemo.rs`
    with BOX-MEMO default, on and off: all pass but `checkpoint.rs`'s `selftest` with
    `FLASHTEX_BOXMEMO=on`, which compares resumed runs word for word and fails the same way on
    main `9697ee948` (BOX-MEMO.md §11.4 keeps it off there).
  - T1 lockstep 1,468 / 1,468 equal; P-T1 88/88 and P-T2 88/88; with BOX-MEMO off and on.
  - `scripts/gate.sh quick`: rustfmt (then fixed in `b613b19f5`), workspace check, clippy, the
    changed crates' tests, the web2rust drift test, trip and etrip pass.
  - `cargo test --lib boxmemo`: the §12.5 group rules, by case.
- **BOX-MEMO verify modes** (final engine `l12r`):
  - command line (`FLASHTEX_BOXMEMO=verify`: the whole engine state after each replay compared
    with the normal path's): beamer-30 6,133 replays, beamer-5 414, full-10 6, 0 differences; the
    PDFs with BOX-MEMO off, on and verify byte-identical for all three;
  - host (`FLASHTEX_BOXMEMO=verify`, the normal path's operation log compared with the entry's),
    6 keys each: beamer-30 @middle 2,455 and @start 3,170 calls, beamer-5 716, full-10 letter@start
    26, 0 differences.
- **Sweeps** (`sweeps.yml`, `ref=agent/kabir-claude/t7-last12`):
  - on `b613b19f5`: the product default (run 39509938355) passed every gate, 0 bad, 0 wrong, 0
    aborts. BOX-MEMO verify (run 39509942631) found one bad compile, sweep A on full-100 (trial
    22, an insert on page 39): the reference compile, run on the command line in verify mode, stopped
    at page 88 on a full-state difference after a replay of `\pdfstringdefPreHook`. The difference
    was in a page insertion record's `broken_ptr`, which the structural comparison followed in
    records that pdfTeX never reads it in (BOX-MEMO.md §12.2, "A hole the verify sweep found").
    Bisected on the PC: main and `11282390c` pass, `efb529808` and later fail (the hook is a
    BOX-MEMO name from there); fixed in `506b33c4a` (`iso.rs`), after which the same 100 compiles
    pass and the edited source's command-line verify runs report 159 and 163 replays, 0 differences;
  - after the fix: the product default on `506b33c4a` (run 39520971283) and BOX-MEMO verify on
    `632ff9ed5` (run 39520974683, the same code with these notes) passed every gate: 0 bad, 0
    wrong, 0 aborts.
- **The final code** (`506b33c4a`): the engine tests above in the three modes again (the same
  result), and `scripts/gate.sh quick` on the PC passes every step.

## Remaining (beliefs unless marked)

- **The Mac decides.** On the PC the rows now take 9–11 ms of engine CPU per key at 91–114 M
  instructions. The M5 ran main's beamer-30 slide (133 M of its own instructions) in 9.7–11.6 ms
  of CPU to the page; at the PC's ratio (−34 %) that is about 6.5–7.7 ms, plus the host's fixed
  costs, which FS-WATCH shrinks most on macOS (its `stat` costs more). Belief, not measured.
- **full-1000 typing** keeps half its keys paying for the abandoned background run: 1.96 ms p50
  on the PC (rewinding the abandoned run's nine pages of writes 0.8 ms, comparing it with the
  changes 0.35 ms, the restore after). Not cutting that further here: a restore straight back to
  the abandoned run's target, keeping the old run pending, would avoid the reattach's copies, but
  touches the restore's output-file bookkeeping (#1348's class). The convergence jump itself
  still cannot be stopped once begun.
- **beamer-30's block shadow** (`\pgfdeclarefading`, about 15 % of the slide): a `\setbox` at the
  call's level and a `\pdfxform` whose number goes into a macro (`\pdflastxform`), both outside
  BOX-MEMO's model. The output routine (~20 %) is ordinary typesetting.
- **full-10 letter@start**'s first inline math (`\check@mathfonts`, about 24 M) still re-runs per
  key; the mid-paragraph restart point after an inline math was not built, since §12.2 brought the
  row from 145.7 M to 95–97 M.
