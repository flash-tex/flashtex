# T7-HEADROOM: headroom on the five near-line rows of the 2026-10-21d reference run

Lane T7-HEADROOM (Commander kabir-claude). The M5 Pro reference run on main `8b3f9ec62`
(`docs/evidence/t7-reference-2026-10-21d-Kabirs-MacBook-Pro/`) passed 119 of 120 rows, and each
run tips another near-line row over 11 ms. The aim: p95 about 9.5 ms or less on the Mac for
full-1000 split@middle and letter@start, beamer-30 letter@end and letter@start, and full-10
letter@start, with the same output.

**Kinds of numbers.** VERIFIED: read from the records (`raw.tgz`). The Mac rows come from four
non-reference runs on the M5 Pro (`tools/incr-bench/t7.py --docs full-1000,beamer-30,full-10
--phases split@middle,letter@start,letter@end --order shuffle --seed 2`, release build;
owner-approved for this lane only). **The Mac was on battery from run 2 on** (62-77 %; run 1 was on
AC), so runs 3 and 4 are a pair under the same conditions: run 3 is main's engine, run 4 the
branch's. The PC figures come from the NixOS PC (Ryzen 7 7800X3D, shared, `ftx-run`; often loaded
by CI), non-reference too. A **belief** is marked as one.

| Mac run | engine | power | what it was for |
|---|---|---|---|
| reference 2026-10-21d | main `8b3f9ec62` | AC | the rows to fix |
| 1 | `2cf1f468b`: main + measurement + a flush experiment (A/B by key pairs) | AC | phases; the flush |
| 2 | `b9c828a1b`: + changes 2-5 below + a cut experiment (3 ways, by key pairs) | battery | the cut |
| 3 | main `8b3f9ec62` | battery | **before** |
| 4 | `2a1da168b` (tree = the branch head) | battery | **after** |

## 1. Where the time went (VERIFIED, Mac runs 1-2)

New measurement (kept): `DONE.stages.marks`, the wall, user and system time (µs) of each phase to
the first page (`os::phase_mark`: `p` edits applied, `m` spans moved, `f` restart point found, `r`
checkpoint restored, `k` restore done, `b` engine resumed, `d` first page); `restore_us`, the
restore's steps; `io_us`, calls and µs by kind of file work (`os::io_time`).

- **Every row's slowest keys were the deleted letters.** In run 1 the top four keys of each of the
  five rows were deletes but one. Their `apply` cut the source shorter (`ftruncate`): 0.33-0.83 ms,
  of which 0.4-0.6 ms off-CPU (blocked), against 0.04-0.2 ms for an insert's whole `apply`; and the
  restore right after was slower too (run 2: 1.01 against 0.82 ms on average; up to 1.7 ms blocked).
- **Not dirty pages.** Run 1 wrote the source's pages out (`fsync`, on a thread, after the
  compile) before every other pair of keys and cut before writing: the cut still took 360-790 µs.
- **The cut's length.** Run 2 cut three ways by key pair. Cutting to a 16 KB boundary, or to 0, took
  38-66 µs on full-10 and beamer-30; cutting inside a block 330-830 µs; cutting at the edit (inside a
  block, more bytes after it) on full-1000 up to 1.4 ms.
- **full-1000:** moving the spans (`m`) 0.62 ms (the line count reads the 4 MB source four times
  over), `SOURCES` for a split 0.36-0.9 ms (built as a tree of JSON values), the change check's
  diff of the 4 MB source from both ends 0.1-0.2 ms, and copying every old page's line shifts at
  each restore 0.2 ms on a split (PC, perf in `pc/`).
- **beamer-30:** the restore's output files (`restore_us[1]`) 0.41-0.79 ms, mostly system time: each
  file was stat'ed, opened, sought, read, written and closed by path, ten calls a file.
- **full-10 letter@start:** 0.5 ms of system time and 0.56 ms off-CPU in typesetting page 0, on
  every key, cut or not (run 2 by cut mode). Run 4's `io_us` (medians of 19 keys, wall time,
  nested kinds): 37 lookups 0.97 ms (26 of them TFM opens, 0.75 ms); within them 51 memo checks
  0.78 ms; and 54 drains of the watch queue (`kevent`, `fswatch::sync`) 0.73 ms, 13 µs each. The
  drains are most of page 0's kernel and off-CPU time. Not changed (§5).

## 2. Changes (`agent/kabir-claude/t7-headroom`)

1. **A file that gets shorter is cut at the 16 KB boundary at or before its new end** and written
   from there, or from the edit if that comes first (`host::resident::apply_changes`, `CUT_ALIGN`).
   Test: `host::resident::tests::edits_leave_the_file_as_the_text` (400 edits near and across the
   boundaries; the file and the host's copy are the text).
2. **The span move counts only near the edit** (`line_change_edit`, `Ends` kept with each text in
   `Doc::texts`, the edit from `apply_changes`'s `EDITED`). Test: `line_change_of_an_edit` (40,000
   random edits, with and without CRs, equal to `line_change`).
3. **The change check diffs from the host's edit** (`system::note_known_edit`/`known_edit`,
   `incr::diff_edit_at`), when the journal's copy is the very one the host edited. Test: in
   `diff_edit_matches_the_definition`.
4. **`SOURCES` written directly** (`Sources::to_json_string`, byte for byte `to_json().to_string()`;
   test `sources_written_directly_as_the_tree_writes_them`), and the moved spans found in one pass
   (`Peer::moved_spans`).
5. **Line shifts shared, not copied** (`Reloc::lines` an `Arc`; the observer's `old_lines`).
6. **`keep_tail` by one descriptor** (open, `fstat`, `pread`, `pwrite`, `fstat`), and a kept tail
   reads through it (`Keep::file`: `save_to`, `kept_tail`). `os::read_exact_at` added.
7. Measurement: the marks, `restore_us` and `io_us` above. The experiments (flush, cut modes) and
   temporary marks were removed.

## 3. Mac before and after (VERIFIED, M5 Pro, on battery; p50 / p95 / max ms, 19 keys)

| row | reference 21d (AC, main) | run 3 (main) | run 4 (branch) |
|---|---|---|---|
| full-1000 split@middle | 9.2 / 11.3 / 11.5 | 8.5 / 10.5 / 12.9 | **7.7 / 8.7 / 9.6** |
| full-1000 letter@start | 9.5 / 10.6 / 10.7 | 8.6 / 9.8 / 10.0 | **7.6 / 8.8 / 10.4** |
| beamer-30 letter@end | 8.3 / 10.3 / 10.9 | 8.9 / 9.8 / 9.9 | 8.4 / 10.1 / 12.4 |
| beamer-30 letter@start | 9.2 / 10.0 / 10.0 | 9.5 / 11.1 / 11.1 | 9.4 / 10.4 / 11.6 |
| full-10 letter@start | 9.1 / 10.0 / 10.1 | 8.6 / 9.3 / 9.3 | 8.9 / 9.5 / 10.7 |

Run 4 passed all nine rows it ran (run 3: eight, beamer-30 letter@start 11.1 ms). By stage, run 3
against run 4 (means over the keys; `cmp.py`):

| row | the cut, deletes (µs) | move_spans | restore |
|---|---|---|---|
| full-1000 split@middle | 259 → 51 | 0.53 → 0.17 | 0.90 → 0.73 |
| full-1000 letter@start | 526 → 265 (one key 2,224) | 0.59 → 0.12 | 0.83 → 0.66 |
| beamer-30 letter@end | 494 → 187 (one key 1,448) | | 1.30 → 1.23 |
| beamer-30 letter@start | 476 → 42 | | 1.81 → 1.77 |
| full-10 letter@start | 423 → 221 (one key 1,639) | | 0.62 → 0.62 |

- **full-1000 (both rows): met**, p95 8.7 and 8.8 ms.
- **beamer-30: not met.** On battery both engines had restores of 1.5-2.6 ms on most letter@start
  keys (run 3: 15 of 19; run 4: 13 of 19, all of them in `restore_us[1]`, the output files), which
  run 1 on AC did not have (0.32-0.49 ms). The keys without it restored in 0.66-0.79 ms (run 4)
  against 0.99-1.06 ms (run 3). **Belief:** on AC, with the cut fixed, both beamer rows lose the
  0.4-0.8 ms the deletes paid and their p95 falls to about 9.5-9.8 ms; the steady keys' typesetting
  (6.1-6.5 ms) is the rest.
- **full-10 letter@start: not met** on AC terms. The cut is fixed (three outliers aside), but its
  page-0 system and off-CPU time (1.07 ms, §1) remains: the watch queue's 54 drains, 0.73 ms.
- The rare slow cuts that remain (3 keys in 95, 1.4-2.2 ms) are cuts to a 16 KB boundary too; not
  explained.

Commit `c59e7bdc2` reverted change 6 on reading run 4's slow beamer restores as its doing; run 3
showed them on main too, and `af9071d41` put it back. The branch head's tree is run 4's.

## 4. Checks (VERIFIED, NixOS PC, at `af9071d41`, whose engine tree is run 4's)

- T1 lockstep 1,468 / 1,468 equal; P-T1 88/88 and P-T2 88/88; with BOX-MEMO off and on
  (`scripts/engine-parity.sh`).
- `tests/incremental.rs` (80), `host_incremental.rs` (27, 1 ignored), `intrinsics.rs` (7),
  `checkpoint.rs` (3), `boxmemo.rs` (10) and the lib's `iso::` tests pass with BOX-MEMO at the
  hosts' default, `on` and `off`, except `checkpoint.rs`'s `selftest` with `FLASHTEX_BOXMEMO=on`,
  which fails on main too (BOX-MEMO.md §11.4).
- The lib tests `arena::`, `system::`, `boxmemo`, `checkpoint`, `hashcache`, `host::resident` and
  `diff_edit` (74 pass), the runtime's `os::`, `logalloc`, `resolver`, `statsig`, `fswatch` and
  `lookupproof` (14), flashtex-display-list's (15); `tests/lookup_memo.rs`, `lookup_racy.rs`,
  `lookup_memo_db.rs`.
- `scripts/gate.sh quick` (rustup 1.98.1, fresh target directory) at `827dce9ff`: passed. Two
  earlier runs failed: one on rustfmt and clippy's `type_complexity` (fixed in `2a1da168b`), one
  because a second gate run deleted the shared target directory under it.
- `sweeps.yml` with `ref=agent/kabir-claude/t7-headroom` at `af9071d41`: default (run 39627470545)
  success; `boxmemo=verify` (run 39627476644) success. At `b82e507ad` both passed (39618674314,
  39618678779).

## 5. Next steps, by expected gain

1. beamer-30 and full-10 on AC: the Commander's next reference run says what the cut fix gives.
2. full-10 page 0: the watch queue is drained at every lookup's memo check (54 `kevent` calls,
   13 µs each on the Mac, 0.73 ms a key). Draining once for the checks a compile makes within a
   short window would save most of it, if the watches' protocol (`fswatch`) allows a check to use a
   drain made just before it: a soundness question for review, not taken here.
3. The S₀ key and the change check redo every lookup on each key: the output directory's lookups
   (`main.aux`, `.toc`, `.nav`, `.out`) have no known dependencies (`DEP_UNKNOWN`: the kpathsea
   shim's `search_dirs` finds them under no path element; `pc/ftrace-beamer30.txt`); 0.4-0.6 ms of
   beamer-30's find on the Mac.
4. Output files are still cut inside a block at a stream's close (`cut_to_logical`); off the
   keystroke's path, but a belief says APFS's aftermath can slow the next restore.

## Files

`raw.tgz`: `mac/run{1,2,3,4}/` (t7's raw records, tables and environment), `pc/` (t7 runs with the
marks, strace and perf attributions by window), `scripts/` (record readers: `ph.py` phases,
`cmp.py` stages, `top.py` slowest keys, `cut.py`/`ab.py` the experiments, `tails.py`, `chain.py`
perf by window).
