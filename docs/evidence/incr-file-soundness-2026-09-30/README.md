# INCR-FILE-SOUNDNESS: output files and IMAGE descriptions across restores, 2026-09-30

Lane INCR-FILE-SOUNDNESS (DESIGN.md §5.3: output files are external effects restored and
replayed with the checkpoints; parity first; soundness 0 mismatches). Issues #1294 (an
incremental restart leaves an output file zero-filled) and #1295 (`IMAGE` without a file after
a restore), and the unexplained 2501.07069v1 mismatch of PR #1296's evidence
(`docs/evidence/p5-external-tools-2026-09-30/README.md`). Branch
`agent/kabir-claude/incr-file-soundness` from `origin/main` 296c90197 with
`agent/kabir-claude/p5-external-tools` (PR #1296, not landed) merged in.

**Hosts.** Every run below is on the NixOS PC (16 threads, TeX Live 2026, shared: load 11–76
during these runs) unless it says macOS (mac-m5pro-kabir: unit tests, rustfmt, clippy).
latexmk/pdflatex are the oracle only.

## Root causes

1. **#1294: a checkpoint recorded each output stream's length as the file's length after
   flushing that stream alone.** LaTeX's `\tableofcontents` twice (arXiv 2501.07356v3 has it)
   opens the `.toc` on two `\write` streams; only the second writes it, and its bytes sit in
   its buffer until a checkpoint flushes it. The checkpoint visits the streams in order: the
   idle first stream flushed nothing and recorded the length on disk before the second
   stream's flush (0), the second recorded the whole length (3,340). Restoring that
   checkpoint cut the file to 0 (first stream), then extended it to 3,340 (second): all NUL.
   The next pass read it ("Text line contains an invalid character" ×100, fatal). It needs a
   restart point after the `.toc` was written and before its next flush, which timed
   checkpoints give under load: hence intermittent, "depends on where the timed checkpoints
   fall". Found with `FLASHTEX_FILE_TRACE` (below): `reopen_out NodalMoments.toc len 0 disk
   Some(3340)` then `reopen_out NodalMoments.toc len 3340 disk Some(0)`.
2. The same layer had three more ways to leave a file other than the engine's streams
   recorded it, fixed with it (each has a test that fails before):
   * a stream's position was taken to be the file's length (a stream behind the end, e.g.
     the idle one above, came back at the end);
   * restoring flushed the abandoned state's unwritten buffers into the file just restored
     (`reattach_pending` rewrote the old run's bytes, then closing the abandoned run's streams
     wrote its buffered bytes over them);
   * a file opened for output again (truncated) after a checkpoint: a restore of that
     checkpoint cut the new file to the old length (NUL or foreign bytes); `reattach_pending`
     after the new run truncated a file set its length back with zeros; a convergence jump
     wrote the old run's whole file over one only the new run had written since the restart,
     and spliced a file the old run truncated after the convergence point at the wrong offset.
3. **Another program's writes to the output files** were taken as the run's: the host's
   `"export": true` runs the engine as pdflatex *in the same output directory*, rewriting the
   PDF (compressed), the log and the rest; the next incremental compile restored on top of
   them, keeping the export's first bytes (e.g. `reopen_out main.pdf len 8089 disk 21012`):
   the preview PDF was corrupt until a later pass rewrote it
   (`host_incremental::an_export_in_the_same_directory_leaves_the_next_compile_exact` fails
   without the fix). `xtools.py sound` exports after every trial, so every P5 soundness trial
   ran on such files.
4. **#1295: `delete_image` frees the image entry's `name` (and PNG/JPEG data) once pdfTeX has
   written the XObject.** The display list described the image by `name` and `data`, so a page
   drawing the image again after a restore to such a state, or in a later pass, got `IMAGE`
   `type: none, file: null` (the same key for every such image).
5. **The 2501.07069v1 intermittent mismatch** was #1295 seen through forms: page 11 draws 16
   forms whose content draws images; a form's `IMAGE` changed between compiles (file / null),
   and `xtools.py` digested each page when it arrived, with the `FORM` of that id it held then
   (the previous compile's), while spec §5 lets a `FORM` come after the page that draws it.
   Old engine + old harness: **11 of 12** `.bib` trials fail; old engine + fixed harness: **0
   of 12**; fixed engine + old harness: **0 of 12** (`raw/linux/ifs-xsound-*07069.jsonl`).

## Fix

* `capture_ext` flushes every output stream before it records any, so streams on one file
  record one length; `Stream::Out` gains `at`, the stream's own position (`OutSink::position`),
  restored with `seek`.
* A restore drops the abandoned state's unflushed output (`BufWriter::into_parts`) instead of
  flushing it into the restored file; `reopen_out` never extends a file (a file shorter than
  the checkpoint recorded refuses the restore; the caller compiles from scratch).
* `system::OPENS` logs every output open (restored with the checkpoints; `ExtRecord.opens`):
  a restore of a checkpoint whose open file was opened for output again since is refused
  (cold compile) unless it recorded it empty; `restore` keeps such files whole; the new
  `guard_outputs` keeps the first bytes of a file the new run truncates, for
  `reattach_pending`; `redo_to` takes a file the old run opened again after the convergence
  point whole, keeps one only the new run wrote, splices the rest, and shifts later records
  by generation. The convergence test declines a point where an output stream is behind its
  file's end (the splice could not place its later bytes); on the soundness sets below this
  changes no convergence (baseline P5 head vs this branch, same seeds: A 669 = 669, book
  6 = 6 and 0 = 0).
* Every output file has a stamp (length, modification time, inode) of the engine's last write
  (open, flush, close, restore); a restore that relies on a file another program changed since
  is refused (cold compile; a paused run is settled instead of reattached). `xtools.py` exports
  from a copy of the directories, so its trials stay incremental. *Not changed here* (the
  host's owner): an export in the resident's own directory still costs the next compile a cold
  run; exporting into a private directory would avoid it (a protocol-visible `DONE.pdf` path).
* `ImageEntry.file` keeps the file `read_image` found; the display list describes an image by
  it and `image_type` (not `name`/`data`, which `delete_image` frees).
* `tools/external-tools/xtools.py`: a page's forms are resolved at the next `PAGE`/`DONE`.
* Diagnostics: `FLASHTEX_FILE_TRACE=FILE` logs every change the checkpoint layer makes to an
  output file; `FLASHTEX_TIMED_S` sets the timed checkpoint interval (tests).

## Tests (fail before the fix)

| test | before | after |
|---|---|---|
| `src/system_output_tests.rs`, 8 tests: two streams on one file, an idle stream's position, a file opened again, reattach after a truncation, reattach drops the abandoned buffer, three convergence jumps | 7 of 8 fail; `two_streams_on_one_file` gives the #1294 NUL `.toc` | 8/8 |
| `tests/incremental.rs::a_restart_after_a_toc_write_keeps_the_toc` (`\tableofcontents` twice, a `\write` to the `.toc`, an `\input` file edited; restart points at every line) | fails: the next pass reads the NUL `.toc`, PDF differs from scratch | pass |
| `tests/host_incremental.rs::an_image_drawn_again_after_a_restore_names_its_file` | fails: `IMAGE` `type none, file null` | pass |
| `tests/host_incremental.rs::an_export_in_the_same_directory_leaves_the_next_compile_exact` (numbered items on every page, an export, one more item on page 3: no convergence, no second pass) | fails: the preview PDF differs from the same compiles without the export | pass |
| `system_output_tests::a_file_another_program_rewrote_is_not_restored` | (new check) | pass |
| `tests/host_tools.rs::edits_tools_and_preemption_leave_the_output_files_exact` (stress: two tables of contents, labels, citations, bibtex; 8 rounds of two back-to-back edits, the second preempting the first compile, its `.aux` passes or the tools' follow-up; restart points everywhere; `.toc`/`.aux`/`.bbl` vs a fresh host after every round) | passes: before the fix its trace shows restores extending the `.toc` (`len 180 disk 144`), but a later pass rewrites the file before anything reads it | pass |

## Results (verified)

All on the branch at 9085ee68a (`raw/linux/head.txt`), intrinsics on (the default), on the
NixOS PC; `raw/linux/`.

| gate | result |
|---|---|
| Soundness A (50 single characters + reverts; 83 fixtures, plain-120, full-100) | **8,500 verified, 0 mismatches** (669 converged, 10 logs differing in accounting only) |
| Soundness C (20 structural edits; fixtures, refs-30/120, full-100) | **1,966 verified, 0 mismatches** |
| Soundness D (12 interleaved: each compile interrupted, then a second edit) | **1,409 verified + 225 interrupted, 0 mismatches** (110 converged; 113–116 on other runs: the interruptions are timed) |
| Soundness book.tex (1,072 pages: 8 characters + 4 sentences, + reverts) | **16 + 8 verified, 0 mismatches** |
| External-tools soundness (PR #1296's set: 112 documents, `\cite` added, `.bib` title edited, `\index` added, 2 trials each; external tools on) | **275 of 275 pass** (cite 155, bib 116, index 4; 39 without an edit site), incl. the 5 that failed on #1296 (2501.06980v1 ×2, 2501.07039v1 ×2, 2501.07069v1); compile modes as #1296's run (405 incremental, 116 unchanged, 2 cold); export P-T2 = latexmk in 268 (the other 7: latexmk itself fails) |
| 2501.07069v1 `.bib` edit, 12 trials | old engine + old harness 11 fail; old engine + new harness 0; new engine + old harness 0 (`raw/linux/ifs-xsound-*07069.jsonl`) |
| 2501.07356v3 opened with tools, 6–8 at once (#1294's `flaky.sh`, `scripts/incrfile-stress.sh`) | before (the old code with `FLASHTEX_FILE_TRACE` only): **8 of 24** `.toc` all NUL, all in the first of three rounds (load 43–76; a later run of the old build: 0 of 12, it needs the timed checkpoints to fall so); after: **0 of 24** at 8 at once (the first build of the fix) and **0 of 24** at 6 at once on 9085ee68a, every compile `ok`. (A run with 16 hosts at once ran the PC out of memory: see the note below.) |
| Convergence vs the P5 head (same seeds, `raw/linux/baseline-p5/`) | A 669 = 669; book 6 = 6 and 0 = 0: the convergence rule costs nothing here |
| Tests | host_incremental 6/6 (+1 ignored), incremental 9/9, display_list_host 1/1, intrinsics 7/7, checkpoint 3/3, host_tools 7/7, engine lib 57/57 (+1 ignored), display-list crate: pass |
| P-T1 / P-T2 (83 fixtures) | **83/83, 83/83** |
| lockstep | **1,145 / 1,145** (accounting 1, non-gating) |
| trip / etrip / drift | pass |
| `scripts/gate.sh pr` (Linux) | tests of the changed crates PASS (784 s), licence boundary PASS, parity self-tests PASS; rustfmt and clippy are not installed for the PC's toolchain (the step lists every file), so both ran on macOS: rustfmt 1.9.0 clean on every changed file, `cargo clippy -p flashtex-engine -p flashtex-display-list --all-targets -- -D warnings` clean |

**Note (machine).** One stress run (16 hosts of 2501.07356v3 at once, 2–2.8 GB each, before
vs after side by side) ran the shared PC out of memory at 16:31 local: the kernel killed those
hosts and also other processes (the self-hosted runners' listeners, restarted by systemd, and
other users' session processes). Its results are not used; later runs used at most 6 hosts.

*Beliefs, not measured:* two streams that both write one file (not only one writing and one
idle) give what C stdio's buffering gives in pdfTeX, which this engine's 8 KiB buffers need not
match; no document here does it, and the convergence test declines such points.

## Reproduce

`scripts/ifs-gates.sh` (on the PC; `tools/incr-bench` from PR #1269 copied into the checkout),
`scripts/incrfile-stress.sh` (N parallel opens of 2501.07356v3 with tools, counting NUL
`.toc`s), `scripts/incrfile-xsound.sh TAG LIST TRIALS`.
