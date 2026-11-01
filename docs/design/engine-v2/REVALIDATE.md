# READ-REVALIDATE: a read that left nothing behind does not hold the restart point back

This is the lane's design note. It extends DESIGN.md §5.3 (restart and converge). DESIGN.md
remains the source of truth: where the two disagree, DESIGN.md wins and this note is wrong. The
code is in `crates/flashtex-engine/src/revalidate.rs`, with the hooks in `incr.rs`
(`Session::revalidation`, `Obs::probe_test`, and the second restart in `Session::incremental`).

## 1. The problem (measured by lane FOCUS-CHAPTER, `docs/evidence/focus-chapter-2026-10-09/`)

#1724 made `\pdffilesize` and `\pdffilemoddate` reads of the whole file. That was required:
before it, a size kept in a macro outlived a change of the file. But LaTeX's `\include` and
`\includeonly` name each chapter through expl3's `\file_full_name:n`
(`expl3-code.tex` ~12560–12668), which calls `\pdffilesize` of the chapter for two things only:
- to test that the size is not blank;
- to compare `size(name.tex)` with `size(name)`, which is the same file.

Since #1724, every checkpoint after that read has "consumed" the chapter (§5.3: a restart point
consumed nothing changed). So an edit deep in a chapter restarts at the chapter's start: on a
986-page book, at page 478 for an edit on page 488. The focused job (`\includeonly`) restarts
at S₀, and its keystrokes lost their gain (edited page p50 104 ms).

## 2. The rule

Let **P0** be the restart point the ordinary test finds.

- **P1** is the old run's first page checkpoint after P0. It must lie within
  `revalidate::MAX_PAGES` = 2 pages of P0.
- **The window** is the reads the old run made between P0 and P1. They are the only part of the
  read journal the rule may set aside.
- **P2** is the newest checkpoint after P1 that meets both conditions:
  - it passes `consumed_nothing_changed` with the window left out (`window_journal` blanks
    those entries, keeping every index);
  - every stream it has open on a changed file is still before that file's change
    (`streams_before_edits`).

  P2 must be at least one page past P1; otherwise there is nothing to gain.

The run restarts at P0, as before. At page P1 (its shipout checkpoint) it compares itself with
the old run's checkpoint P1, which the restore at P0 detached into the pending branch. The
comparison has two parts.

1. **Everything outside the engine's state** (`revalidate::same_since`):
   - every input stream at the same offset with the same lookahead, and every stream on a changed
     file still before the change;
   - every output stream (the PDF, the log, the `.aux` and other `\write` files) the same length,
     at its end, with the **same bytes since P0** (`pending_old_bytes`);
   - the terminal the same bytes since P0;
   - the same number of diagnostics notes and of external effects (`\write18`,
     `\pdfelapsedtime`);
   - the same read journal since P0: files (with their closes and the times read), lookups and
     outputs opened. After the second restart the journal is the old run's, so it must be this
     run's as well.
2. **The engine's state**, compared exactly as the convergence test compares it (§5.3 (c), (d)):
   - `cstate.same_as`, for pdfTeX's C parts;
   - `same_words` over the whole word space. It applies the convergence test's dead-word rules,
     the structural comparison (`crate::iso`) and the line shift. This covers the hash table and
     eqtb, so every control sequence l3 defines as it goes (`\__file_seen_...`, `\l__file_*`) is
     compared, along with every macro, register, box and token list.

   One difference from convergence: where `same_words` accepts the new run's extra shipped
   characters (`char_or`, which a jump ORs into the old states), the comparison fails instead. The
   old states after P1 are kept as they are.

If both parts are equal, the run is abandoned (`Globals::reattach_pending`, the pending branch
back in place) and the compile restarts at **P2**, exactly as an ordinary restart there would
(`incremental`, with the old future kept for convergence). If either part differs, the run goes
on from P1 as the ordinary restart at P0. The work done so far is that restart's own, so nothing
is lost.

## 3. Why it is sound

The engine is deterministic: its state plus the bytes it reads next decide everything after.

- **At P1.** If the new run's state equals the old run's there, and so do the outputs and the
  journal, then the new run from P1 on does what the old run did, as long as it reads the same
  bytes.
- **From P1 to P2.** The old run read only what the new text has unchanged:
  - every read entry after P1 passes the ordinary rule;
  - every stream on a changed file is still before the change at P2;
  - lookups whose answer changed are after P2, by the ordinary `bad_lookup` rule. A changed lookup
    inside the window disables revalidation, because whether another one follows is unknown.

  So the old run's state at P2 is the new run's state at P2.
- **Before P1.** The reads in the window were re-executed by the new run against the new files.
  Whatever they did is in the new run's state, outputs, terminal and journal at P1, and those
  were compared.

The size, or a file date, kept in a macro, typeset on a page, or written to the log or the
`.aux` shows up in one of these, so the comparison fails. Where l3 only tested the size and
dropped it, nothing differs.

## 4. Cost and bounds

- **When it holds:** the pages from P0 to P1 (at most 2) are typeset twice. Add the comparison
  (the convergence test's 2–10 ms), the reattach (≤ 3 ms) and the restore at P2.
- **When it fails:** only the comparison is extra.
- **When there is no window**, which is every compile whose restart point is not held back by a
  read in the window, the cost is one journal clone and a binary search. Both are
  sub-millisecond.
- `FLASHTEX_REVALIDATE=off` turns the rule off.

## 5. Tests and evidence

- **The cases:** `tests/incremental.rs::a_size_read_that_left_nothing_does_not_hold_the_restart_back`
  is an `\include` book with edits deep in chapter 3. Every compile equals scratch runs.

  | case | revalidated |
  |---|---|
  | only `\include`'s lookup | true |
  | the size compared with `\ifnum` | true |
  | typeset on the chapter's first page (the state is equal; the page bytes differ) | false |
  | written to the log with `\typeout` (only the log and terminal differ) | false |
  | kept in a macro printed later | false |
  | the chapter's date kept | false |
  | typeset on a page of its own before the chapter (the second read, `\include`'s, is outside the window) | no window |

- **Each comparison is needed.** With the state comparison disabled, "kept" gives a PDF unlike
  scratch. With the output and terminal comparisons disabled, "written to the log" gives a log
  unlike scratch.
- **Evidence:** the measurements and sweeps are in the pull request.
