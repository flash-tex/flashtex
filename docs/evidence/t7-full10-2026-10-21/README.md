# T7-FULL10: the system calls behind full-10 letter@start's first page on the Mac

Lane T7-FULL10 (Commander kabir-claude). The M5 Pro reference run on main `cb0aa96a7`
(`docs/evidence/t7-reference-2026-10-21c-Kabirs-MacBook-Pro/`) passed 119 of 120 rows. The miss was
full-10 letter@start, 10.5 / 11.4 / 11.5 ms (p50 / p95 / max), steady over all 19 keys. The aim
was p95 about 9.5 ms or less on the Mac, with the same output.

**Kinds of numbers.** VERIFIED: read from the records (`raw.tgz`, and the reference run's own).
The Mac rows come from three short, non-reference runs on the M5 Pro (`tools/incr-bench/t7.py
--docs full-10 --phases letter@start,letter@middle`, release build, on AC power; owner-approved
for this lane only). The PC rows come from the NixOS PC (Ryzen 7 7800X3D, shared, `ftx-run`), so
their times are non-reference too. A **belief** is marked as one.

- **Before:** main `cb0aa96a7` (the reference run 2026-10-21c; PC engine `base`).
- **After:** `agent/kabir-claude/t7-full10`. Mac run 2 ran `10693303a`, and run 3 ran `f4e9b638c`
  (run 2's engine, plus `apply`'s step timers and a read-back experiment that changed nothing and
  was removed afterwards). The branch head adds only tests and this evidence to those.

## 1. The cause: about 1,000 system calls a key, all of them font lookups (VERIFIED)

The reference run's new counters, at the p50 of the 19 measured keys:

| Mac, 2026-10-21c | system calls to the first page (`first_page_sys`) | instructions to the first page | typesetting instructions | page faults |
|---|---:|---:|---:|---:|
| full-10 letter@start | **1,315** | 123.8 M | 119.6 M | 0-1 |
| full-10 letter@middle | 340 | 34.4 M | 90.8 M | 0-9 |
| PC, the same row (user space only) | | 89.3 M | 92.6 M | |

Typesetting cost the same on both machines except on letter@start, where the Mac counted 27 M more.
The page faults were 0, so the extra kernel work was system calls (about 975 a key, roughly 28 K
instructions each, counted in the Mac's per-thread instructions and not the PC's). letter@start
restarts before the document's first inline math, so every key loads its 26 TFM files and two `.fd`
files again. letter@middle restarts after that point.

`strace -f -k` on the PC (`pc/strace-*-key4.txt`, the fourth key of a session, the engine thread from
when it resumes to the first page's `d` mark) attributes the calls. Base had **674** calls in the
window. About 17 of them went to each font load:
- the memo's check (`still_holds`): `getcwd`, a drain of the watch queue, a `stat` of `./`;
- `lookup_dirs` on a memo hit: `access` and `stat` of the answer twice (`flashtex_kpse_db_hazard`,
  `kpathsea_db_search`), and `stat(".")`;
- `is_user_file`: `getcwd`, twice;
- `note_file`: a `stat` for the signature, and another for the hash's identity;
- the open itself: `open`, `read`, `close`; and the output directory's `stat` first.

## 2. The fix (`agent/kabir-claude/t7-full10`)

1. **A memo hit answers `lookup_dirs` too** (`resolver::kpse`, `MemoEntry::deps`). The lookup
   right after a hit, or right after the entry was made, keeps what `lookup_dirs` gave for that
   answer, and gives it back. The hit has just found unchanged what that answer is made from: the
   working directory, the search path, the directories searched and the answer's readability. An
   entry kept by listing a changed directory again drops it. Test: `tests/lookup_memo.rs` now compares
   `lookup_dirs` with the memo on and off at each of its 24 steps, and requires some memo answers.
2. **The working directory is asked for once a compile** (`os::cwd`, `os::cwd_scope`). It is held
   in `incr::Session::compile`, `finish` and `prepare_next`, because the hosts change it only between
   compiles. Other threads, and code outside a scope, still call `getcwd`. Test:
   `os::tests::a_cwd_scope_answers_and_nests`.
3. **`note_file` makes one `stat`** for the signature and the hash's identity
   (`StatSig::of_metadata`, `hashcache::hash_file_from`).
4. **A TFM file's bytes are kept with its hash** (`hashcache::keeps_bytes`, `kept_bytes`;
   `system::open_bytes`). The bytes are given back while the file's identity is the one they were
   read under, and is neither racy nor on a non-local volume: the rule the hash cache already
   follows. The identity is the `stat` the same lookup just made (`LAST_STAT`, cleared at every
   `input_path`). Test: `hashcache::tests::a_tfm_files_bytes_are_kept_while_it_is_unchanged`.
5. On every row: `incr::Session::changes` makes one `stat` for each path, not one for each journal
   entry. A file written within the racy window, such as the `.aux` or the edited source, matched no
   signature, so each of its 8-17 entries took a `stat`. `Job::out_is_root` now canonicalizes twice
   a compile instead of three times.
6. Measurement: `DONE.stages.apply_us` gives `apply_changes`' file steps in µs: open, write, set
   length, close and the `stat` after.

PC, the same window, after change 1-4 (`pc/strace-m1-key4.txt`): **267** calls (674 before). What is
left for each font is the output directory's `stat`, the drain and `./`'s `stat` in the memo's check,
and `note_file`'s `stat`.

## 3. Mac before and after (VERIFIED, M5 Pro)

| run | engine | phase order | full-10 letter@start p50 / p95 / max | system calls to the first page | instructions to the first page / typesetting | letter@middle p50 / p95 / max |
|---|---|---|---|---:|---|---|
| reference 2026-10-21c | `cb0aa96a7` | letter@start after five other phases | 10.5 / 11.4 / 11.5 | 1,315 | 123.8 / 119.6 M | 8.4 / 9.4 / 10.5 |
| run 2 | `10693303a` | letter@middle, letter@start | **8.5 / 10.0 / 10.2** | 463 | 96.0 / 92.7 M | 8.0 / 8.9 / 8.9 |
| run 3 | `f4e9b638c` | letter@middle, letter@start | **8.9 / 9.2 / 9.3** | 472 | 94.8 / 91.3 M | 8.5 / 11.1 / 11.1 |
| run 1 | `10693303a` | letter@start first | 9.1 / 11.8 / 15.0 | 463 (keys 5-19) | 96.4 / 92.7 M | 7.4 / 8.9 / 9.1 |

- The Mac's page-0 excess is gone. Typesetting is 91-93 M, as on the PC (90.6-92.0 M), and the
  calls to the first page fell from 1,315 to about 465.
- **The p95 met the aim in one run of two** (9.2 ms in run 3, 10.0 in run 2). Both runs pass the
  11 ms gate.
- Run 1 put letter@start first after the host opened the document. Its keys 0-4 fell in the 2 s
  racy window after the document's copy and open (`find` and `key` 2.4-3.2 ms each, 1,120-1,250
  calls). The base engine does the same: in the reference run, plain-10 letter@start, its first
  phase, had keys 0-5 at `find`/`key` 2.3-3.5 ms and about 1,470 calls. Runs 2 and 3 therefore
  used `--order shuffle --seed 2`, which puts letter@middle first, as the reference run's
  rotation does for full-10. Run 3's letter@middle shows the same window (keys 1-3, 10.4-11.1 ms).
- What remains is **the `ftruncate` of a deleted letter** (run 3's `apply_us`, every second key).
  Setting the length took 366-779 µs (once 81 µs). Every other step took 1-73 µs, and a key that
  inserts, with no truncation, took 0.07-0.21 ms for the whole `apply`. These keys are the slowest
  of both phases. Restore is 0.5-1.1 ms on the Mac against 0.35 ms on the PC, and its 66 calls
  include the output files' cuts and blanks (`keep_tail`). **Belief:** the same APFS truncation
  cost is in the restore. Reading the file back on another thread after the cut (run 3) changed
  nothing. Run 3 also showed that the page-in counter (`TASK_EVENTS_INFO.pageins`) stayed 0. Both
  were removed. Untested next step: write the file's pages out (`fsync`) on another thread between
  keys, so that the next truncation finds them clean. This applies to every letter, sentence and
  split row.

## 4. PC before and after (VERIFIED, times non-reference)

| full-10 letter@start, PC | ms p50 / p95 / max | instructions to the first page | typesetting |
|---|---|---:|---:|
| base (`q2-base`) | 9.1 / 10.1 / 10.5 | 89.2 M | 92.5 M |
| changes 1-4 (`q2-m1`) | 8.4 / 10.3 / 10.3 | 88.7 M | 92.0 M |
| and 5 (`q2-m2`, run 2's engine) | 8.2 / 9.3 / 9.4 | 87.2 M | 90.6 M |
| with the experiment (`q2-m3`, run 3's engine) | 8.3 / 9.7 / 10.4 | 87.5 M | 90.9 M |

The A/B `ab*` (base and changes 1-4, two rounds each, letter@start and letter@middle) gives the same
picture. In user space the instructions fall by 0.5-2 M. The gain is mostly the kernel work, which
the PC's counters do not see. Re-typeset pages and convergence are unchanged (19/19 converged, 2
pages on letter@start and 3 on letter@middle, in every run).

## 5. Checks (VERIFIED, NixOS PC, at `6e395b9cb`)

- T1 lockstep 1,468 / 1,468 equal; P-T1 88/88 and P-T2 88/88; with BOX-MEMO off and on
  (`scripts/engine-parity.sh`).
- `tests/incremental.rs` (80), `host_incremental.rs` (27, 1 ignored), `intrinsics.rs` (7),
  `checkpoint.rs` (3), `boxmemo.rs` (10) and the lib's `iso::` tests pass with BOX-MEMO at the
  hosts' default, `on` and `off`. One exception, which T7-MARGIN recorded on main too (BOX-MEMO.md §11.4; not re-run on main here): `checkpoint.rs`'s
  `selftest` with `FLASHTEX_BOXMEMO=on`.
- `tests/lookup_memo.rs`, `lookup_racy.rs` and `lookup_memo_db.rs` pass. So do the lib tests
  `arena::`, `system::`, `boxmemo`, `checkpoint` and `hashcache` (66 pass), and the runtime's
  `os::`, `logalloc`, `resolver`, `statsig`, `fswatch` and `lookupproof` (14 pass). In one of seven
  runs, `fswatch::tests::changes_are_seen_at_the_next_check` failed while a build ran beside it:
  its served-count assertion depends on no event arriving for `/tmp`. It passed 6 of 6 times on its
  own, and main's tests passed 4 of 4.
- `scripts/gate.sh quick` (rustup 1.98.1, fresh target directory) at `25ec0a527`: passed (rustfmt, clippy,
  tests of the changed crates, trip, etrip). The first run, at `6e395b9cb`, failed on rustfmt alone.
- `sweeps.yml` with `ref=agent/kabir-claude/t7-full10` at `6e395b9cb`: default (run 39594288291)
  success; `boxmemo=verify` (run 39594293024) success. `25ec0a527` changes only formatting and docs.

## 6. Notes

- The lane allowed only the t7 measurement on the Mac. One `cargo check -p flashtex-engine` also
  ran there by mistake before the PC took over the builds.
- A first Mac attempt measured nothing: its bench directory under the scratch path made the host's
  socket path too long (`SUN_LEN`). The runs used `INCR_BENCH_DIR=/tmp/ftf10`.
- Files in `raw.tgz`: `mac/run{1,2,3}/` (t7's raw records, tables and environment), `pc/` (the A/B
  and `q2` runs, and the strace attributions), and `scripts/` (the record readers:
  `tab.py`/`keys.py` per key, `abtab.py` per row, and `agg.py` for strace windows by mark and
  calling frame).
