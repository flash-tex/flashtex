# P4-L5-RESTART: `.aux` read-sets, passes, restart points between pages, barriers (2026-09-29)

Lane P4-L5-RESTART (DESIGN.md §5.2–§5.5, §1.2, §12 P4). Branch
`agent/kabir-claude/p4-l5-restart`, from `agent/kabir-claude/p4-l2-l3` (draft PR #1223; this
branch is stacked on it and does not merge `origin/main`). Raw output in [`raw/`](raw/), the
drivers in [`scripts/`](scripts/) (copies of P4-L2-L3's with the paths moved to `/tmp/p4l5`,
plus the new ones named below).

**Host:** mac-m5pro-kabir, Apple M5 Pro (15 cores), 24 GiB, macOS 26.6.2, release builds. The
machine was shared with other agents all day (1-minute load 4–45 while this lane ran; the
`uptime` lines of each measurement are in `raw/`). Thread-CPU times and instruction counts are
the robust measures; wall times are upper bounds.

__RESULTS__

## What was built

### Passes (DESIGN §5.5: "re-run in the background")

`Session::compile` runs the first pass (the edited page comes out of it: `Report::edited` is the
first page of that pass whose frame changed, with its wall and thread-CPU time); then, while the
last pass changed a file it read (its `\end{document}` rewrote the `.aux`, a `.toc` appeared),
further passes run in the same command — ordinary incremental compiles of what the previous pass
wrote — up to five, stopping early when the files the passes read repeat a state an earlier pass
read (oscillation). A file the run wrote before reading it (beamer's `.vrb`) is not such an input.
The next compile therefore finds the `.aux` it reads already settled: the 0.3–3.9 s "compile
after a label moved" of P4-L2-L3 is gone from the edited page (it is now the second pass's work,
reported in `pass_s`). The soundness drivers compare each compile with from-scratch runs
repeated by the same rule (run again while a run changed a file other than the PDF and the log,
up to five runs, stop on a repeated state).

### L5: `.aux` read-sets (`changes/readset.ch`, `src/readset.rs`)

- **Recording.** From the close of the `.aux` the `.aux` point opened (`Point::AuxDone` is the
  next `big_switch`), `get_next`'s reads of a control sequence's meaning (from a file, a token
  list, an active character, an empty line's `\par`, a `\noexpand`ed token) and every
  `id_lookup` (`\csname`, `\ifcsname`, found or not) log the first read of each name
  (keyed by its bytes). `rs_seen`, in the word space, marks the slots already logged, so the hot
  path is one load and a branch; every checkpoint records the log's length (`ExtRecord::rs`), so
  "before the first read of any of these names" is a prefix of the chain. The log is truncated
  by restores, spliced like the journal at a convergence (with `rs_seen` rebuilt there and on
  the inherited checkpoints' later restores), and put back when a run is abandoned.
- **What a changed `.aux` changes.** When only files the `.aux` read read have changed (the
  `.aux`, an `\include`d part's `.aux`), the pass re-runs that read alone (restore the `.aux`
  point, run to `Point::AuxDone`, stop) and compares the state there with the old run's:
  `readset::aux_delta` finds every control sequence whose meaning differs (by content: token
  lists by their tokens, control sequences by name; candidates are the differing `eqtb` and hash
  slots, the owners of differing token-list cells, and names earlier passes patched). Then it
  **proves** that nothing else differs: it puts the old run's meanings back into the re-read
  state, renumbers the names the read made into the old run's order (a `\bibcite` written before
  a label's page shipped makes the same names in another order), removes names the old read did
  not make (new labels), and runs the convergence test's word-space comparison
  (`same_words`: dead words, free cells, `crate::iso`'s relabelling) against the old run's
  state. Only then is the difference exactly the changed meanings. The engine then returns to
  the old run as if nothing happened (`Arena::reattach`, `Globals::reattach_pending`).
- **Restart.** The pass restarts at the newest checkpoint before the first read of a changed
  entry (and before every other change), with the new meanings put in (`readset::apply_patch`:
  `eq_destroy` the old, build the new body with `get_avail`, insert new names with `id_lookup`),
  and runs on — in the background. Checkpoints from the end of the `.aux` read to the restart
  point still hold the old meanings: the patch is attached to them (`Session::defpatch`) and
  applied after any later restore of one, and no convergence is accepted there. A pass whose
  `.aux` change the proof does not cover falls back to re-reading the `.aux` from the `.aux`
  point (P4-L2-L3's behaviour); the report says why (`Report::l5`).

### Restart points between pages (§5.2, §5.7's cut points)

`changes/checkpoint.ch`: `build_page` requests a checkpoint when it returns (not while the
output routine is active, never over another request); the hook takes it when a line was read
since the last checkpoint and at least `Options::segment_s` of engine time has passed (default
0.5 ms, `FLASHTEX_SEGMENT_S`). A restart then re-runs about one paragraph before the edit
instead of the rest of the previous page. `\end{document}` now re-runs from the last page's
checkpoint (checkpoints after it can lie past its re-read of the `.aux`).

### Barriers (§5.3)

`\pdfelapsedtime` and `\pdfresettimer` read the clock (the run's first read is its start time,
from the pinned clock): each is an external effect, and a convergence is refused where the old
run's later pages made one (`\write18`, the clock), so they re-run. `\pdfuniformdeviate`,
`\pdfnormaldeviate`, `\pdfrandomseed` (seeded from the pinned clock) and `\pdfcreationdate` are
state the checkpoints hold: tested equal to from-scratch runs.
`\pdffilemoddate`/`\pdffilesize`/`\pdfmdfivesum` look the file up through the journal, so a
changed file is a change before its read.

__BUGS__

__DEVIATIONS__

__OPEN__

__REPRO__
