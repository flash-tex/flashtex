# P4-L5-RESTART: `.aux` read-sets, passes, restart points between pages, barriers, preemption (2026-09-29)

Lane P4-L5-RESTART (DESIGN.md §5.2–§5.5, §1.2, §12 P4). Branch
`agent/kabir-claude/p4-l5-restart`: from `agent/kabir-claude/p4-l2-l3` (#1223), with
`agent/kabir-claude/host-unify` (#1235, which supersedes #1215/#1223 and merges `origin/main`)
merged in at the Commander's request. Raw output in [`raw/`](raw/), the drivers in
[`scripts/`](scripts/) (P4-L2-L3's, paths moved to `/tmp/p4l5`, plus `ckseg.py`, `genrefs.py`,
`matrix_table.py`, `keys.sh` and the extended `incr_bench.py`/`soundness.py`).

**Host:** mac-m5pro-kabir, Apple M5 Pro (15 cores), 24 GiB, macOS 26.6.2, release builds; shared
with other agents all day. The final latency matrix ran at load 3–8 (`raw/matrix-uptime.txt`), the
quietest this lane saw; thread-CPU times and instruction counts are the robust measures.

**Engines measured:** final code 1773a1a61 (`fin8`: matrix, keystrokes, interleaved soundness);
f4d50345f (`fin6`: soundness A–C, gates; 1773a1a61 differs only in the host's handling of a run
from scratch after a *stopped* run, which A–C never have); c23e8f92f (before the #1235 merge and
preemption: an earlier matrix at load 10–29, kept in `raw/*-c23e8f92f*`).

## Results

### Against the lane's targets

| Target (DESIGN §1.2, lane) | Measured (verified unless marked) | Status |
|---|---|---|
| Edited page ≤ 16 ms p95, edit inside a paragraph, up to 1,000 pages | p95 over 66 edits per document, wall / thread CPU: plain-10 5.2 / 5.0, plain-100 6.8 / 6.3, plain-300 4.4 / 3.1, plain-1000 13.3 / 10.5; full-10 12.9 / 12.5, full-100 12.1 / 10.4, full-300 13.9 / 9.5, full-1000 **16.3** / 14.1 ms. Over all 528 compiles: CPU p95 11.3, max 14.7 ms; wall p95 14.2, 8 above 16 (max 23) | **met** in CPU for every document; in wall for all but full-1000 (16.3) |
| Reflowing edit: edited page ≤ 16 ms, later pages in the background, stale marked | the edited page is the first pass's; the `.aux` passes after a label move are the background's (`pass_s`: full-300 0.8 s, plain-1000 1.2 s); the next compile finds the `.aux` settled. P4-L2-L3's 0.27–3.9 s "compile after a label moved" (full-300 p95 1,450 ms) is gone | met |
| L5: `.aux` read-sets; re-run only from the first page that read a changed entry; oscillation; cap 5 | implemented (below). full-100, a sentence moving three equations: the second pass restarts at page 47 (the first `\cref` of a moved label), 305 ms instead of 580 ms from the `.aux` point; a label whose page decides the text before it stops on the repeated state (test) | met, with fallbacks (*Open* 1) |
| Restart points between pages; cost and memory within 1 GB | a checkpoint after `build_page` ≥ 0.5 ms of engine time apart: 2.2 a page (full-1000); +3.0 M instructions a page on the 953-page document (+2–3 %), undo logs 222 → 356 MB there, 248 → 436 MB on full-1000; a page checkpoint stays 2.75 M instructions (0.27–0.35 ms CPU). Read-set hooks +1.2 % instructions | met |
| `\pdfelapsedtime` a barrier; random seeds, `\pdfcreationdate` | external effect, no convergence past it (test); random numbers, `\pdfrandomseed`, `\pdfcreationdate` equal scratch runs (test) | met |
| Preemption (Commander, mid-lane): a new keystroke interrupts the previous compile's background | stops at the next page/segment checkpoint; socket, 120-page full document, next key as soon as the edited page is there: edited page p50 / p95 **17.4 / 25.1 ms** (the previous compile's `DONE` came at 444 / 482 ms, which a key had to wait for before); plain 120: 9.9 / 12.1 | met (not yet ≤ 16 ms for the full document: *Open* 5) |
| Soundness: 0 mismatches incl. label moves, citations, toc, multi-pass, restart points, interleaved edits | **13,743 verified compiles, 0 mismatches** (A 8,600; B 1,720; C 1,954; D 1,469 plus 258 interrupted); 200 compiles whose logs differ only in DESIGN §1.1's accounting (C 125, D 75) | met |
| P-T1, P-T2, lockstep, trip/etrip/drift, `gate.sh pr` | on f4d50345f (after the #1235 merge): P-T1 **83/83**, P-T2 **83/83**, lockstep **260/260** (accounting 0), trip, etrip, drift pass, `scripts/gate.sh pr` passed (`raw/`) | met |

### Edited-page latency (`scripts/matrix.py`, `scripts/matrix_table.py`, `raw/latency-matrix.jsonl.gz`)

Engine 1773a1a61, 22:30–22:37 local, load 8 → 3. Per document and region (the first, middle,
last tenth of the prose): a typing session of 8 single-character edits within 400 bytes (each
with its revert: 16 compiles) and 3 sentence insertions with their reverts (6 compiles).
*Edited page* = the host's `edited`: from the `compile` command to the shipout of the first page
of the first pass whose frame changed; *CPU* = the host thread's CPU time over that interval (the
parallel restore's workers are not in it).

| doc | start p50 / p95 ms (CPU) | middle p50 / p95 ms (CPU) | end p50 / p95 ms (CPU) | all 66: p50 / p95 wall | p95 CPU |
|---|---|---|---|---|---|
| plain-10 | 3.8 / 4.2 (3.5 / 3.8) | 2.4 / 3.5 (2.1 / 3.2) | 2.4 / 5.8 (2.1 / 5.6) | 2.6 / 5.2 | 5.0 |
| plain-100 | 6.1 / 7.3 (5.6 / 6.7) | 2.6 / 2.9 (2.1 / 2.3) | 3.6 / 5.0 (3.1 / 4.4) | 3.4 / 6.8 | 6.3 |
| plain-300 | 3.7 / 4.9 (2.6 / 3.9) | 3.2 / 3.5 (2.3 / 2.6) | 3.0 / 3.3 (2.2 / 2.6) | 3.3 / 4.4 | 3.1 |
| plain-1000 | 12.9 / 14.1 (10.1 / 10.7) | 5.2 / 5.4 (2.9 / 3.2) | 4.2 / 7.7 (2.4 / 6.1) | 5.3 / 13.3 | 10.5 |
| full-10 | 9.8 / 10.1 (9.4 / 9.7) | 5.4 / 13.3 (5.0 / 12.7) | 6.7 / 7.4 (6.4 / 6.9) | 7.3 / 12.9 | 12.5 |
| full-100 | 6.7 / 13.4 (5.9 / 12.6) | 7.2 / 9.7 (6.5 / 8.1) | 5.6 / 6.0 (5.0 / 5.2) | 6.7 / 12.1 | 10.4 |
| full-300 | 9.1 / 13.9 (7.2 / 8.8) | 7.8 / 8.9 (6.5 / 7.2) | 7.7 / 14.4 (6.8 / 13.4) | 8.1 / 13.9 | 9.5 |
| full-1000 | 13.6 / 15.5 (8.3 / 10.7) | 11.6 / 13.6 (8.1 / 10.0) | 15.9 / 16.7 (13.9 / 14.7) | 12.9 / 16.3 | 14.1 |

Against P4-L2-L3 (same matrix; its engine at load 4–21): p95 CPU full-10 32.3 → 12.5, full-100
13.8 → 10.4, full-300 1,299 → 9.5, full-1000 16.3 → 14.1; plain within noise. The earlier run of
this lane's engine at load 10–29 (`raw/matrix-table-c23e8f92f.md`) had the same CPU figures
(full-1000 17.1) and wall p95s up to 50 ms, all in far restores waiting for cores.

What the remaining large numbers are:

- **full-1000's end pages** (14–15 ms CPU): the edited paragraph starts on page 986 and ends on
  987, so page 986 ships again before 987 (TeX breaks a page after reading the paragraph that
  crosses it); two output routines with hyperref, siunitx and footnotes. The restart point is
  within 51–385 bytes of the edit: DESIGN §5.6's per-page floor.
- **full-1000's and plain-1000's start region** (wall 13–16 ms, CPU 8–10): the restore from the
  document's end back to page 82 walks ~920 pages of undo logs, three times as many as with page
  checkpoints only, on worker threads (not in the CPU column).
- **Page 1 of full-10** (9.8 ms): the restart is now after the begin-document work (it was the
  `.aux` point: 14–33 ms); page 1's output routine (hyperref's first-page work) remains.

### Checkpoint cost and memory (`scripts/ckseg.py`, `scripts/ckcpu.py`, `raw/checkpoint-cost.txt`)

| | 953 pages (long8) | full-1000 |
|---|---|---|
| page checkpoints (unchanged from P4-L2-L3) | 2.75 M instructions, 0.27 ms CPU each (load 5–10), RSS 101 → 337 MB | — |
| + segment checkpoints (0.5 ms apart) | 971 → 3,033 checkpoints; **+3.0 M instructions a page** (+2–3 %); logs 222 → 356 MB; RSS 353 → 424 MB | 1,043 → 4,197; +4.0 M a page; logs 248 → 436 MB; RSS 382 → 638 MB |
| every `build_page` with a line read (rejected) | — | ~15 a page, +7.4 M instructions and 1.1 ms CPU a page |
| read-set hooks (`readset.ch`) | +1.2 % instructions (full-300) | |

The 1 GB budget did not have to thin anything on these documents.

### Soundness (`scripts/soundness.py` → `scripts/incr_bench.py --verify`)

Every compile's `.pdf`, `.log`, `.aux`, `.out`, `.toc` and terminal output are compared byte for
byte with `flashtex-initex` from scratch (preview mode) on a copy of the directory — **repeated
like the host's passes**: run again while a run changed a file other than the PDF and the log,
up to five runs, stopping when the files repeat a state. After an interrupted compile the next
one is compared with runs on the directory as the last *finished* compile left it plus the new
sources (the previous run's `.aux` is fixed input).

| pass (engine) | edits | documents | verified compiles | mismatches | accounting-only logs |
|---|---|---|---|---|---|
| A (f4d50345f) | 50 single characters + reverts | 83 fixtures + Hello, 120 pages, 953 pages | **8,600** | **0** | 0 |
| B (f4d50345f) | 10 sentence insertions + reverts | 83 fixtures + Hello, 120 pages, full-100 | **1,720** | **0** | 0 |
| C (f4d50345f) | 20 structural: sentence, `\section`, `\label`, `\ref`/`\pageref`, `\cite`, footnote, removed label, removed section + reverts | 83 fixtures + refs-30, refs-120 (toc, bibliography, cleveref), full-100, full-300 | **1,954** | **0** | 125 |
| D (1773a1a61) | 12 interleaved: each compile interrupted in pass 1 or 2 after 1–4 pages, then a second edit (the revert, or another letter nearby) | 83 fixtures + refs-30, refs-120, full-100, full-300 | **1,469** (+ 258 interrupted) | **0** | 75 |

D on f4d50345f found 2 mismatches (`raw/soundness-fin6-D-before-fix.txt`, bug 7 below), fixed in
1773a1a61. `tests/incremental.rs` (8 tests: the three of P4-L2-L3, structural edits, oscillation,
`\pdfelapsedtime`, random numbers and dates, interleaved edits), `tests/host_incremental.rs` (with
`a_newer_compile_preempts_the_running_one` through the socket) and the arena's `reattach` unit
test pass. *Accounting-only*: the log differs only in DESIGN §1.1's end-of-run capacity block,
reported and not compared (an L5 patch that undefines a removed label leaves its name in the
hash: one string and one control sequence more than a from-scratch run).

### Preemption through the socket (`scripts/keys.sh`, `dl3-keys`, `raw/keys.txt`)

40 keystrokes on page 60 of the generated 120-page documents (5 of the 10-page ones), load 3–8.
*wait*: the next key after the previous `DONE` (P3P4-HOST-UNIFY's measurement); *overlap*: the
next key as soon as the previous key's edited page arrives, while its background runs.

| document | wait: edited page p50 / p95 | wait: `DONE` p50 / p95 | overlap: edited page p50 / p95 | compiles preempted |
|---|---|---|---|---|
| plain, 10 pages | 5.1 / 5.3 | 34 / 36 | 4.6 / 5.7 (21 keys) | 20 of 40 |
| plain, 121 pages | 3.7 / 4.0 | 24 / 27 | 9.9 / 12.1 | 39 of 40 |
| full, 11 pages | 15.2 / 15.6 | 60 / 61 | 16.1 / 17.1 (20 keys) | 20 of 40 |
| full, 120 pages | 9.7 / 10.7 | **444 / 482** | **17.4 / 25.1** | 39 of 40 |

Before, a key typed while the 120-page document's background ran waited for it (up to ~0.45 s);
now it waits for the next checkpoint and the stopped run's return (~6 ms: the convergence test at
the edited page's checkpoint runs to its end), then re-types the page.

## What was built

### Passes (DESIGN §5.5)

`Session::compile` runs the first pass — the edited page comes out of it (`Report::edited`) —
then, while the last pass changed a file it read (its `\end{document}` rewrote the `.aux`, a
`.toc` appeared), further passes in the same command: ordinary incremental compiles of what the
previous pass wrote, up to five, stopping early when the files the passes read repeat a state an
earlier pass read (oscillation). A file the run wrote before reading it (beamer's `.vrb`) is not
such an input.

### L5: `.aux` read-sets (`changes/readset.ch`, `src/readset.rs`)

- **Recording.** From the close of the `.aux` the `.aux` point opened (`Point::AuxDone` is the
  next `big_switch`), `get_next`'s reads of a control sequence's meaning (from a file, a token
  list, an active character, an empty line's `\par`, a `\noexpand`ed token) and every
  `id_lookup` (`\csname`, `\ifcsname`, found or not) log each name's first read, keyed by its
  bytes; `rs_seen`, in the word space, marks the slots already logged (the hot path is one load
  and a branch); every checkpoint records the log's length (`ExtRecord::rs`). Restores truncate
  it; a convergence splices it like the journal (`rs_seen` rebuilt there and after restores of the
  inherited checkpoints); `reattach` puts it back.
- **What a changed `.aux` changes.** When the files that changed are read only by the `.aux` read
  (the `.aux`, an `\include`d part's `.aux`), the pass re-runs that read alone (restore the `.aux`
  point, run to `Point::AuxDone`) and compares it with the old run's: `readset::aux_delta` finds
  every control sequence whose meaning differs, by content (token lists by their tokens, control
  sequences by name). Then it **proves** nothing else differs: it puts the old meanings back into
  the re-read state, renumbers the names the read made into the old run's order (a `\bibcite`
  written before a label's page shipped makes the same names in another order), takes out names
  the old read did not make (new labels), and runs the convergence test's own comparison
  (`same_words`: dead words, free cells, `crate::iso`'s relabelling) against the old state.
  `Arena::reattach` then returns to the old run as if nothing had happened.
- **Restart.** At the newest checkpoint before the first read of a changed entry (and before every
  other change), with the new meanings put in (`readset::apply_patch`: `eq_destroy`, a new body
  from `get_avail`, new names through `id_lookup`), in the background. The checkpoints from the end
  of the `.aux` read to the restart point hold the old meanings: the patch is attached to them
  (`Session::defpatch`), applied after any later restore of one, and no convergence is accepted
  there. What the proof does not cover falls back to re-reading the `.aux` from the `.aux` point,
  and `Report::l5` says why.

### Restart points between pages (§5.2, at §5.7's cut points)

`changes/checkpoint.ch`: `build_page` requests a checkpoint when it returns (not while the output
routine is active, never over another request); the hook takes it when a line was read since the
last checkpoint and at least `Options::segment_s` (default 0.5 ms, `FLASHTEX_SEGMENT_S`) of engine
time has passed. A restart re-runs about one paragraph before the edit instead of the previous
page's rest. `\end{document}` re-runs from the last page's checkpoint (later ones can be past its
re-read of the `.aux`). `line_break`'s globals are dead between commands for the convergence test
(with restarts inside pages they held the edited paragraph's values and blocked every later test:
plain-1000 start-region edits now converge five pages on, 80 ms a compile instead of 1.2 s).

### Barriers (§5.3)

`\pdfelapsedtime` and `\pdfresettimer` read the clock (the run's first read is its start, from the
pinned clock): each is an external effect, and a convergence is refused where the old run's later
pages made one (`\write18`, the clock). `\pdfuniformdeviate`, `\pdfnormaldeviate`, `\pdfrandomseed`
(seeded from the pinned clock) and `\pdfcreationdate` are state the checkpoints hold: tested equal
to from-scratch runs. `\pdffilemoddate`/`\pdffilesize`/`\pdfmdfivesum` look the file up through the
journal, so a changed file is a change before the read.

### Preemption (the Commander's addition)

`Session::set_preempt(p)`: at every page and segment checkpoint of a first pass, an `.aux` pass,
or the rest of a document after a convergence, `p(pass, pages)` is asked whether newer work waits
(the socket host: a queued COMPILE from the client, or its CANCEL); a page checkpoint asks before
its convergence test. The run stops there and the compile returns paused (`Report::preempted`;
`DONE` `cancelled`). The next `compile`:

- **keeps the stopped run** (`settle_paused`) when a checkpoint of its own precedes the new change
  (it typeset pages that stay valid): its checkpoints, pages and journal become the document's,
  the old run's future is dropped, and the rest of the document is typeset from its last page once
  the new run converges with it;
- otherwise — typing the same paragraph again, or a change ahead of the stopped run — **goes back
  to the complete run it was replacing** (`abandon_paused`: `Globals::reattach_pending` restores
  that run's checkpoints, state, output files, terminal and read-set, and the session's journal and
  L5 patches as the pass found them), whose later pages can still be converged with; pages the
  stopped run shipped are shipped again;
- continues the stopped run when nothing changed.

The files the stopped run was rewriting after reading them (`.aux`, `.toc`, `.out`) are taken as
the run the next pass stands for read them (DESIGN §5.5: the previous run's `.aux` is fixed
input), written back when the restart, or a run from scratch, reads them again.

## Soundness problems found on the way (all fixed; the final runs have none)

1. **`\end{document}` re-ran from a checkpoint past its own `.aux` re-read.** P4-L2-L3 re-ran it
   from the newest checkpoint; with checkpoints between pages that can be inside
   `\end{document}` → the last page's checkpoint (`Session::end_point`), kept by retention.
2. **The main `.tex` taken for an `.aux`-read file** (a first L5 version: an ordinary edit got
   "0 entries changed"). Caught by a probe before any soundness run.
3. **First reads during the `.aux` read hid the pages' reads** (`\@newl@bel` tests every label) →
   the read-set begins when that `.aux` closes.
4. **Relocated structures** (a parameter token list on the parameter stack): the first `.aux`
   comparison classified words itself → it is now the convergence test's own comparison.
5. **Beamer's `.vrb` made five passes** (written, then read, by the same run): `beamer-fragile`,
   1 mismatch in the first smoke run → `FileRead::written_before`.
6. **A convergence into checkpoints an L5 patch applies to** (found by review) → refused.
7. **After a stopped run, a run from scratch read the stopped run's `.aux`** (soundness pass D:
   `min2-nested-dfrac-bare`, 2 mismatches: a second edit in the preamble went cold) → `cold()`
   writes the fixed inputs back first. Earlier in the preemption work the same pass shape, and the
   new Rust test, found two more: an `.aux` pass stopped after restarting at the `.aux` point was
   kept, not abandoned (the new edit's restart then re-typeset the whole document), and a restart
   inside the `.toc`'s read re-read the stopped run's `.toc` → the keep/abandon rule and the
   write-back test above.

## Deviations from DESIGN.md

- **§5.5 per-page `\r@`/`\b@` reads**: recorded per checkpoint for *every* control sequence and
  name looked up; the changed entries are found by re-running the `.aux` read and comparing
  meanings (proved by the structural comparison), not by parsing the `.aux`. `.toc`, `.lof`,
  `.out` are files: a change restarts before that file's read, then L3 convergence.
- **"In the background"**: the later passes run inside the same `compile`/`finish` call on the
  engine thread; the edited page is reported (and, through the socket, sent) from the first pass,
  and a newer compile preempts them.
- **Logs compared without DESIGN §1.1's accounting** in `tests/incremental.rs` (the Python driver
  always did), and such compiles are counted separately.

## Open

1. **L5 falls back to re-reading the `.aux` from the `.aux` point** when the proof fails: when the
   old `.aux` read made a name the new one did not (a removed label that was defined at the `.aux`
   read), and when a section is inserted or removed in the cross-reference test ("list node paired
   inconsistently": a shared token list, not diagnosed). Sound; slower in the background.
2. **An L5 pass runs from its first reader to the end.** Converging after the readers (patching the
   new meanings into the old run's later checkpoints) is not built; the later passes cost
   0.1–1.2 s in the matrix.
3. **Far restores walk three times as many logs** with segment checkpoints (full-1000's start
   region: 13–16 ms wall); keyframes (§5.2's fallback) or merging the between-page logs far from
   the cursor.
4. **full-1000's end pages** need the output routine faster (§5.6, P6-HYPERREF-INTRINSICS). Note
   for that lane: an intrinsic that replaces macro expansion must report the control sequences it
   reads (`Globals::flashtex_cs_read`), or L5 misses those reads.
5. **Preempted keystrokes on the full 120-page document**: 17.4 / 25.1 ms; the stopped run
   finishes the convergence test it is in (~6 ms) before it returns. A test that checks for newer
   work while it walks, or none at the edited page's own checkpoint, would take that off.

## Reproducing

```
cargo build --release -p flashtex-engine -p flashtex-display-list
scripts/mkeng.sh NAME             # binaries to /tmp/p4l5/NAME, format to /tmp/p4l5/fmt-NAME
python3 scripts/gen.py /tmp/p4l5/docs          # plain-/full-N; then /tmp/p4l5/src-<doc>/<doc>.tex
python3 scripts/genrefs.py /tmp/p4l5           # src-refs-30, src-refs-120
python3 scripts/matrix.py NAME OUTDIR && python3 scripts/matrix_table.py OUTDIR
PYTHONHASHSEED=0 python3 scripts/soundness.py NAME -j 4 --trials 50 --extra DIR:DOC[:FILE] ...
PYTHONHASHSEED=0 python3 scripts/soundness.py NAME -j 4 --trials 20 \
    --kinds sentence,section,label,ref,cite,footnote,unlabel,unsection --extra ...
PYTHONHASHSEED=0 python3 scripts/soundness.py NAME -j 4 --trials 12 --interleave \
    --kinds replace,insert,sentence,section,label,ref,unlabel --extra ...
python3 scripts/ckseg.py NAME DIR DOC REPS off 0.0005     # segment checkpoint cost
python3 scripts/ckcpu.py NAME DIR DOC REPS                # page checkpoint cost
scripts/keys.sh NAME                                      # socket keystrokes, wait / overlap
cargo test --release -p flashtex-engine --test incremental --test host_incremental
```

`FLASHTEX_SEGMENT_S` (`off`, or seconds), `FLASHTEX_NO_L5` and `FLASHTEX_L5_DEBUG` select and trace
the new mechanisms; `iserve` gains `compile-interrupt PASS PAGES`; `Report` gains `edited`,
`passes`, `pass_modes`, `pass_s`, `oscillation`, `l5`, `rs_events`, `ck_stats`, `preempted`.
