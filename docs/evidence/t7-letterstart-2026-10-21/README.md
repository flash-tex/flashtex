# T7-LETTERSTART: the two rows the 2026-10-21 reference run missed

Lane T7-LETTERSTART (Commander kabir-claude). The M5 Pro reference run on main `5439b97ea` (PR #1863,
`docs/evidence/t7-reference-2026-10-21-Kabirs-MacBook-Pro/`) passed 118 of 120 gated rows. The two misses:

- full-10 letter@start: 11.5 / 12.2 / 12.4 ms (p50 / p95 / max);
- beamer-30 letter@start: 9.7 / 11.5 / 16.8 ms.

Every new measurement here is from the NixOS PC (Ryzen 7 7800X3D, shared, `ftx-run`), so every time is
non-reference. Load-independent figures are the engine thread's instructions to the edited page
(`DONE.stages.first_page_instr_k`) and callgrind's. The Mac figures quoted come from #1863's
`raw.tgz`. What the Mac will measure after this change is a belief until the next reference run.

- **Before:** main `0995a0676` (engine `main1`). **After:** this branch at `4cc5c7026` (engine
  `m5`; later commits change only docs and formatting).
- **A/B:** `t7.py --docs D --phases P --preamble 0 --reopen 0`, two rounds, main1 then m5 each round,
  19 measured keys per row per round (38 per row in the table).
- **Files:** `raw.tgz`: `ab{1,2}-{main1,m5}-*` (t7's raw records and tables), `cg/` (the callgrind
  sessions' key records), `t7ls-ev/` (the logs of the checks below).

## The causes, from the reference run's raw records (VERIFIED from #1863's `raw.tgz`)

| | full-10 letter@start | beamer-30 letter@start |
|---|---|---|
| queue wait p50 | 0.02 ms | 0.02 ms |
| off-CPU p50 (first page wall minus thread CPU) | **2.06 ms**; every other letter@ row 0.45-0.77, except plain-10 letter@start 1.42 | 0.45 ms |
| restore p50 | **1.97 ms**; full-10 letter@middle 0.65 | 0.83 ms |
| instructions to the page, M5 against the PC | **126.6 M against 89.5 M** (letter@middle: 95.7 against 92.7; full-100 letter@start 73.0 against 69.2) | 130.6 M against 112.5 M (the usual ARM/x86 ratio) |
| the keys | all alike | key 1 **258 M** (16.8 ms), keys 2-19 about 130 M |

**full-10 letter@start is not compute the PC sees.** On the PC it costs what letter@middle costs (89.5 M
against 92.7 M instructions); the M5 spends 37 M more instructions, 2.2 ms more CPU and 1.4 ms more
off-CPU time on it than the ARM/x86 ratio predicts. Only the two page-0 rows show this (full-10 and
plain-10 letter@start, both 1.4 MB PDFs). A restart on page 0 restores to before the first shipout,
where the PDF (and hyperref's `.out`) is open but still empty. A restore kept every other output tail
in the file itself (P4-PAGE-COST), but not one from length 0: it took that file whole, which on
Linux is a `read`, and on APFS a `clonefile` to the temporary directory, an `unlink` of the last
key's clone, and then the new run writing over a file whose blocks the clone shares. The restore
is 1.3 ms slower on full-10 (two such files) and 0.5 ms on plain-10 (one), which fits a fixed
cost per clone. That the window's extra instructions and CPU are APFS's copy-on-write of those
writes is a **belief**: this lane runs nothing on the Mac.

**beamer-30 letter@start is compute.** The macro profile of the edited page (PC, `FLASHTEX_MACRO_PROFILE`,
instructions) against letter@middle: `\beamer@typesetheadorfoot` 22.2 M against 0.6 M, of which
`\beamer@@tmpl@footline` 18.3 M against none. BOX-MEMO's debug log says why: the footline's
recording ended "Objects" on that frame, so it ran for real at every key. Its one object is its
navigation link's (`\pdfstartlink ... goto name{Navigation1}`), which a replay makes again. But the
shipout before the frame had begun an object stream, whose number pdftex.web takes from
`sys_obj_ptr` while leaving `obj_ptr` behind it, so the call seemed to make two objects
(BOX-MEMO.md §13.1). The refusal also explains key 1: its inner calls are only offered after the outer
call's refusal, so key 1 recorded 87 of them.

## The fixes

- **A tail kept from length 0** (`checkpoint.rs`, `Globals::restore`): an output file open but empty
  at the restore target, which the old run did not open again later, keeps its tail in the file, as
  every other open file's tail already did (`system::keep_tail(path, 0)`): nothing is read, cloned or
  deleted at the restore, and the new run writes on in place. An abandon puts it back with
  `put_back_kept` as for any kept tail. Files the old run opened again after the target are read
  (or cloned) whole as before. Test: `system::output_restore_tests::a_tail_from_an_empty_file_stays_in_the_file`
  (no bytes read at the restore; abandon after short, long and no writes; jump; run's end).
- **BOX-MEMO counts a call's objects by `sys_obj_ptr`** (BOX-MEMO.md §13.1; test
  `tests/boxmemo.rs` `links_recorded_after_an_object_stream_began`).
- **BOX-MEMO compares K2's runs of equivalents and integers 16 words at a time** (§13.2; 6 % of the
  slide's typesetting; test `low_halves_compare_as_the_word_by_word_loop`).
- The host writes a `d` perf mark when it delivers the first page (`FLASHTEX_PERF_MARKS`), so a
  profile can end where the gated time ends, before the shipout's checkpoint.

## Before and after (VERIFIED, NixOS PC; times non-reference)

| row | engine | ms p50 / p95 / max (PC, wall) | instructions to the page p50 | key 1 | thread CPU p50 | restore p50 | converged |
|---|---|---|---|---|---|---|---|
| full-10 letter@start | main | 9.3 / 10.8 / 10.9 | 89.6 M | 90 M | 9.08 ms | 0.50 ms | 38/38 |
| | branch | 9.1 / 10.3 / 12.5 | 89.5 M | 90 M | 8.91 ms | 0.47 ms | 38/38 |
| beamer-30 letter@start | main | 11.1 / 20.2 / 21.1 | **112.6 M** | **226 M** | 11.01 ms | 0.55 ms | 38/38 |
| | branch | 9.4 / 13.9 / 14.9 | **90.5 M (−20 %)** | **142 M** | 9.31 ms | 0.55 ms | 38/38 |
| full-10 letter@middle | main | 8.8 / 10.1 / 10.3 | 92.7 M | 93 M | 8.81 ms | 0.44 ms | 38/38 |
| | branch | 8.6 / 10.2 / 10.6 | 92.6 M | 93 M | 8.59 ms | 0.36 ms | 38/38 |
| plain-10 letter@start | main | 4.9 / 5.5 / 5.7 | 42.0 M | 42 M | 4.70 ms | 0.36 ms | 38/38 |
| | branch | 4.9 / 5.9 / 6.0 | 42.0 M | 42 M | 4.74 ms | 0.35 ms | 38/38 |
| beamer-30 letter@middle | main | 9.3 / 13.1 / 14.8 | 93.5 M | 144 M | 8.86 ms | 0.52 ms | 38/38 |
| | branch | 9.2 / 13.8 / 13.8 | 89.8 M | 141 M | 8.90 ms | 0.50 ms | 38/38 |
| full-100 letter@start | main | 6.6 / 7.9 / 8.5 | 66.5 M | 66 M | 6.39 ms | 0.43 ms | 38/38 |
| | branch | 6.5 / 8.2 / 8.2 | 66.4 M | 66 M | 6.33 ms | 0.41 ms | 38/38 |

(Instructions: `edited_instr_k` for letter@middle, whose first page is not the edited one. Key 1 is
the first measured key of each round. Re-typeset pages unchanged: 2 on letter@start, 3 on full-10
letter@middle.)

Steps on the way (PC, beamer-30 letter@start, instructions to the page p50): main 112.5 M; §13.1
94.2 M; plus §13.2 90.5 M.

**Callgrind** (PC, `valgrind --tool=callgrind`, instrumentation on while dl3-keys typed 4 keys after a
2-key warm-up, the whole host instrumented, background work included):

| session | main | branch |
|---|---:|---:|
| beamer-30 letter@start | 3,102.7 M | 2,983.6 M (−119 M, −30 M a key) |
| full-10 letter@start | 2,047.4 M | 2,040.5 M |

On the PC the kept tail changes no user-space instructions: Linux's `read` of the 1.4 MB tail
(0.1-0.2 ms) was kernel work, and the PC's counters count user space only.

## Belief: the Mac after this change

- **full-10 letter@start:** the page-0 restore should cost what letter@middle's does (0.65 ms
  against 1.97), and without the clone the window should cost what the PC's ratio predicts. The row
  would then look like full-10 letter@middle on the same run (8.0 / 9.1 ms), whose per-key compute
  on the PC is the same (92.7 M against 89.5 M). Not measured.
- **beamer-30 letter@start:** −20 % instructions to the page on the PC; at the M5's 9.7 ms p50 that
  is about 7.8 ms. Key 1 stays the slowest key (142 M on the PC, about 1.6 times a later key), so the
  row's p95 (the second-slowest of 19 keys) is the slowest later key, about 9.3 ms at the old spread.
  Not measured.
- If full-10 letter@start still misses, the next step is the mid-paragraph restart point after the
  first inline math (the T7-CONV estimate: about 24 M of the page's instructions); it was not built
  here, since on the PC the row already costs what letter@middle does.

## Checks (VERIFIED, NixOS PC)

- `tests/incremental.rs` (80), `host_incremental.rs` (27, 1 ignored), `intrinsics.rs`, `checkpoint.rs`,
  `boxmemo.rs` and the lib's `iso::` tests, with BOX-MEMO at the hosts' default, `on` and `off`:
  all pass, except `checkpoint.rs`'s `selftest` with `FLASHTEX_BOXMEMO=on`, which fails the same
  way on main (BOX-MEMO.md §11.4 keeps BOX-MEMO off there; T7-LAST12 recorded the same).
- T1 lockstep 1,468 / 1,468 equal; P-T1 88/88 and P-T2 88/88; with BOX-MEMO off and on.
- Command line, BOX-MEMO off, on and `verify` (the whole engine state compared after each replay),
  two passes each: beamer-30 6,590 replays, full-10 8, full-100 163, 0 differences; the PDFs of
  the three modes byte-identical, and identical to main's.
- `cargo test --lib boxmemo` and `system::output_restore_tests`: pass.
- Sweeps and `scripts/gate.sh quick`: in the PR.
