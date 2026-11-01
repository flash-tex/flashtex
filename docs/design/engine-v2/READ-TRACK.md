# READ-TRACK: a line the engine has read but not used yet does not stop the jump

Lane **READ-TRACK** (Commander `kabir-claude`, 2026-10-20). This note extends DESIGN.md §5.3
(restart and converge). DESIGN.md stays the source of truth: where the two disagree, DESIGN.md
wins and this note is wrong.

The question the lane was given: a keystroke on the last line of the Unicode M3 bench document b4
costs about 545 M instructions to its page, against 83 M elsewhere. The UNICODE-M3 lane believed
that a fix needs read tracking on `eqtb`, which is a web2rust read barrier. This note measures
that barrier, finds that Classic has the same problem, and proposes a design that needs no read
barrier on the hot path.

Figures marked **measured** were taken by this lane on the NixOS PC (x86-64, main `1aa1018ef`,
release builds, TeX Live 2026 at `~/texlive/2026`). Figures marked **belief** are estimates.

## 1. The problem

b4 (`tools/unicode-m3/bench/b4-fontspec-pages.tex`) ends like this:

```latex
\section{Four}
\lipsum[61-100]
\end{document}
```

The M3 latency job types `x` at the start of the last line, three times. The first keystroke
makes it `x\end{document}`, the second `xx\end{document}`, the third `xxx\end{document}`.

`\lipsum` takes a trailing optional argument. Before it typesets anything, its peek for that
argument reads the next token, which is on the last line. That happens on page 11. The line stays
in TeX's input buffer while the 40 paragraphs are typeset on pages 11 to 18.

So the run restarted for the keystroke differs from the previous run on every page from 12 to 18:
- the buffer holds a different line;
- `first`, `last` and the main file level's `limit` differ by the line's change of length;
- after the first keystroke only, the peeked token differs too, and with it `\l_peek_token`.

No page passes the convergence test, and the run typesets pages 11 to 19 again.

## 2. Classic has the same pattern (measured)

Two Classic documents, in this lane's scratch space, show the pattern:
- **c1-lipsum** is b4 without fontspec.
- **c2-ifnextchar** ends in a LaTeX2e command with an optional argument, `\newcommand\tail[1][]`,
  called alone on the line before `\end{document}`. Its `\kernel@ifnextchar` peek reads `\end`,
  and the body then typesets 8 pages.

Each document was compiled cold and then given keystrokes at the start of the `\end{document}`
line (`rt_end.py`: Classic `iserve`, `FLASHTEX_INCR_DEBUG=1`).

| document | pages | pages re-run per keystroke | edited page, wall (PC) | why every test fails |
|---|---:|---:|---|---|
| c1-lipsum | 19 | 8 | 12.2–13.8 ms | "an input file's lookahead differs" |
| c2-ifnextchar | 22 | 13 | 15.1–16.2 ms | "an input file's lookahead differs" |

So `\@ifnextchar` behaves like the l3 peek. A command that peeks at the next line and then
typesets pages is the pattern, whichever kernel defines it. Classic pages are cheap (about 1.2 ms
each on the PC), so the cost is smaller there. Unicode pages cost about 55 M instructions each.

## 3. What differs (measured with a probe build)

A probe build, used for measurement only and never committed, let the Classic test go past its
early checks and print every word that differs. It ran on c1-lipsum and c2-ifnextchar, with three
keystrokes each.

**Keystroke 1** (the old run peeked `\end`, the new one peeks `x`):
- 9 buffer words, `first` and `last`;
- then, in the structural comparison, `eq_type differs (114 vs 11)`. That is `\l_peek_token`:
  a macro in the old run, a letter in the new one. Behind it are the backed-up token and the
  reference count of `\end`'s body.

**Keystrokes 2 and 3** (both runs peek the same `x`):
- 8 buffer words (the line after the peeked `x`), `first` and `last`;
- the main file level's `limit`, and the stale copies of `limit` in the token-list levels above
  it (`begin_token_list` never sets that field for a non-macro list; tex.web §323).

When the probe also set these aside, the test passed:
- c1-lipsum at page 13, re-running 2 pages instead of 8;
- c2-ifnextchar at page 11, re-running 2 pages instead of 13.

(That probe "converged" to the old run's end, which is wrong output, because the last page must
show the new `x`. It shows only that nothing else differs.)

The Unicode engine reports the same words: with `FLASHTEX_UNICODE_CONV_DEBUG=1`, b4's keystrokes
fail at pages 12–17 on `first`, `last` and buffer words.

So after the first keystroke at a place, the difference is **the unread rest of one line**:
buffered, waiting below the macro levels, and not read again until the pages are done.

## 4. Rejected: a read barrier on `eqtb` (measured)

The general fix is to record every read of the state. Convergence could then accept any word that
differs, provided the old run never read it before overwriting it. web2rust would put the barrier
in `Index` of the arrays.

To measure the cheapest version, the probe made every `eqtb[i]` read set bit `i` of a side
bitmap: one load, OR and store per read. Writes were unchanged. This is a lower bound: a real
barrier also needs a per-segment epoch, and would have to cover `buffer`, `input_stack` and `mem`,
because the dormant data lives there. Results are Classic command-line compiles under callgrind
(instructions, Ir); the PDFs were identical:

| document | main | with the barrier | change |
|---|---:|---:|---:|
| plain-100 | 2,277,172,576 | 2,415,474,463 | **+6.07 %** |
| full-100 | 8,605,243,349 | 8,778,969,509 | **+2.02 %** |
| c1-lipsum | 929,392,180 | 1,062,873,910 | **+14.36 %** |

The target is no regression above 1 %, and even this partial barrier is 2–14 times over it. Most
`eqtb` reads are category codes in `get_next` and parameters in the line breaker, not meanings.
**A web2rust read barrier is not acceptable.**

## 5. The design: a dormant line

The rule covers the difference that keystrokes 2 and later leave (§3). It needs no read barrier,
because it only has to know one thing about the old run: did TeX return to that input level, or
print it, before a given page? Both are rare events, and both happen on cold paths.

### 5.1 The rule

Let the new run be at a test checkpoint *k* after its restart, and the old run at its own *k*.

**The dormant-line test.** Every difference between the two states must be one of the following:
- a buffer word of level *j*'s line at or after `loc` (the part not read yet);
- `first` or `last`;
- level *j*'s `limit`;
- a `limit` field of a non-macro token-list level above *j* (dead: §3);
- level *j*'s file stream record in the host: its position and the line it holds.

Here level *j* is the innermost input level that reads a file. Two further conditions hold in
**both** states:
- every level above *j* is a token list, so no other file level uses the buffer;
- `first = limit_j + 1`.

Everything else must pass the ordinary test unchanged: the C parts, the files, the reads, the
words (with its dead words and line shift) and the structural comparison. Level *j*'s `start`,
`loc`, state, index and name must be equal. If the edit moves lines (a line shift is active), the
rule does not apply.

**The jump point *k′*.** This is the old run's last page checkpoint after *k* such that, between
*k* and *k′*, the old run:
- **never made level *j* current.** In the generated code this is `pop_input` taking `input_ptr`
  down to *j* or below;
- **never printed an input level's line.** That means `show_context`, a diagnostics frame or
  `\show` (`ls_print_level` in Classic);
- **never opened an input level.** `begin_file_reading` covers `\input`, `\read`, `\scantokens`
  and the terminal; all of them read `first`;
- **met no existing barrier.** These are the ordinary ones: an external effect, a changed or
  written file read, a changed lookup, and the line journal.

Its \csname scratch never reached `buf_size`: `max_buf_stack` at *k′* plus the growth of
`first` must be below `buf_size`.

**Only where it is worth it** (`dormant::worth`, added in Phase 2). The jump keeps the pages
from *k* to *k′* and typesets the rest of the document again. It is taken only where it keeps
at least as many pages as it typesets again. Between *k* and *k′* the line differs, so no
ordinary test passes there either, and a run without the rule typesets at least to *k′*. So
with this condition the rule never costs more pages than a run without it. Without it, a
dormant line in the middle of a document lost a later convergence: in c1-lipsum,
`\lipsum[16-35]` peeks at `\section{Three}`, and a keystroke in that title jumped from page 4
to page 6 and typeset the 13 pages after it, where main converges at page 8 (§11).

**The jump.** This is the existing jump to a barrier's re-run point (Classic: `rerun_point` /
`rerun_from`; Unicode: `conv_jump` to its target). The old run's pages *k*…*k′* are kept, the
old checkpoint *k′* is restored, and the run goes on live from there. Before it goes on, the new
run's values of the dormant words are written into the restored state, and into every old
checkpoint from *k* to *k′* that the jump keeps. So is every other buffer word of the new
run's line from `loc` up to its `first` (§5.2 says why). `Arena::or_from` already writes a word
into a range of kept states this way; the rule adds a `set_from` beside it, which writes all the
words in one pass over the logs. The host records of those checkpoints get the new stream record.

The run is **not converged**: it re-typesets from *k′* to the end, as a barrier's re-run does.
The test only proves that the pages from *k* to *k′* are the new run's pages.

### 5.2 Why it is sound

The engine is deterministic: its state and the bytes it reads decide everything it does next.

Let *D* be the dormant words. At *k* the two states are equal outside *D*. Take the old run's
steps from *k* to *k′*, one at a time. No step reads a word of *D*:

- **The rest of level *j*'s line**, and level *j*'s `limit`, are read only while level *j* is
  current. That is `get_next` and `firm_up_the_line`, tex.web §343–§362. They are also read by
  what prints the input stack: `show_context` §311–§318, and the diagnostics frames.
  Level *j* becomes current only through `pop_input` from the level above it (§325; every
  `end_token_list` and `end_file_reading` pops through it). The watch sees every such pop, and
  the print counter sees every print. If neither moved, no step read them.
- **`last`** is read only by the line readers of the current file level (§31, §362, §538) and by
  the terminal (§71). The first needs level *j* current. The terminal needs a new input level.
- **`first`** is read by these:
  - `begin_file_reading` (§328), and through it `\input` (§537), `\read` (§483), `\scantokens`
    (e-TeX's `pseudo_start`) and the terminal (§71). All of them are counted.
  - `primitive` and the format line (§264, §1337), which run only at initialisation.
  - `\csname` (§372) and e-TeX's `\ifcsname`. These copy a name to `buffer[first..j)` and
    look it up with `id_lookup`, whose result depends on the bytes, not on where they lie.
    `max_buf_stack` is only accounting (§1.1, already dead in the test). The one dependence on
    the position is the overflow at `buf_size`, which the jump rule checks.
  - The host code was audited the same way. `boxmemo.rs` reads `buffer[j..j+l)`, the same bytes.
    `intrinsics_verify.rs` compares `first` between its two paths.
- **Stale `limit`s of non-macro token lists** are read by nothing: §323 sets `param_start` there
  only for macros, and only `end_token_list` of a macro reads it (§324).
- **The stream record** is read only by `input_ln` for level *j*, which needs level *j* current.

So every step reads the same values in both runs, takes the same branch, writes the same words
and produces the same output. One kind of write lands in different places: a `\csname` in the
old run writes its scratch from the old `first` on, and in the new run from the new `first` on.
Where the new line is longer, the old run's scratch lies over the new line's tail, and a word
there may have been equal at *k*, so it is not in *D*. The jump therefore writes the new run's
values of the whole line from `loc` up to the new `first` (the new run never writes below its
`first`), and the scratch above `first` is dead: every reader of it writes it first.

By induction, the new run's state at *k′* is the old one's with *D*, and the new line, replaced
by the new run's values, up to dead scratch. The pages from *k* to *k′* are the new run's pages,
byte for byte and frame for frame.

A word of the scalars is set aside as `first` or `last` only when every byte that differs is one
of theirs: an 8-byte word may hold another scalar too.

That includes the source map. `dl_here` reads level *j*'s `start` and `loc`, and in Unicode the
bytes `buffer[start..loc)`, which are equal. It never reads past `loc`.

Before *k*, the new run itself re-executed every read of the edited line against the new text.
Its state at *k* is the evidence, so how far `get_next` looked ahead (midline.rs' `loc+3`) does not
matter here.

**Which conditions a case needs (measured, §11).** With each rule disabled in turn, the
soundness test (§10) fails as follows:
- without the return count, the words written in, or the print count: it fails;
- without the open count, or the stream record: no case fails. Each is kept as a guard, at
  no cost:
  - A level opened and closed between two checkpoints moves only positions. Its lines lie at
    `first`, and `end_file_reading` puts `first` back. A level still open at a checkpoint
    changes the watched level anyway.
  - The host reader's lookahead for the line is not read again once TeX holds the line.

### 5.3 Why keystroke 1 is not covered

After the first keystroke the peeked token itself differs. The new run's `\l_peek_token` is a
letter where the old one's is `\end`, and the backed-up token and `\end`'s reference count differ
with it. The source map differs too: `dl_here` gives column 3 (after `\end`) against column 0
(after `x`) to everything typeset on pages 12–18. The Unicode bench shows it: keystroke 1's first
changed frame is page 12.

Covering that case needs three more pieces:
- **Meanings.** A rule that a differing meaning is not read before *k′*. In Classic this is L5's
  read-set, the same pattern PREAMBLE-PATCH uses (`readset.ch`, `meanings_delta`,
  `apply_patch`). The Unicode engine has no read-set.
- **Backed-up tokens.** A dormant class for the backed-up token below the macro levels.
- **The source map.** A rewrite of the columns on the kept pages. The alternative is a change in
  what a column means: "a token TeX looked at and put back is not read".

Each piece is sound in principle. The read-set hooks cost nothing new in Classic; in Unicode they
would be five new tests in `get_next`. But together they double the mechanism, and the source-map
part changes behaviour the app relies on. This lane does not build them (§9).

## 6. Where the barriers go

The only new test on the hot path replaces an existing one:

- **The pop watch.** `checkpoint.ch` already puts a test in `pop_input` for arming S₀:
  `if input_ptr<ckpt_arm_level`. That test becomes `if input_ptr<pop_watch`. `pop_watch` is the
  larger of the S₀ arm level and the dormant watch level (*j*+1).
  - The body of the test stays inline Pascal. It does what it did for S₀, and it counts the
    watched level's return in `dorm_fired`. A host call there made the callers of
    `end_token_list` slower (§11).
  - In the body `ckpt_arm_level` is 0, so today the test never fires. With the watch it fires
    at most once per checkpoint interval: it disarms the watch, and the next checkpoint arms it
    again. Unicode's `checkpoint.ch` has the same test.
  - `dorm_level`, `pop_watch` and `dorm_fired` are dead for the comparisons, because only the
    host reads them.
- **The print counter** goes in the existing cold hooks: `ls_print_level` in Classic, the
  diagnostics frame builder, and the Unicode engine's `show_context` through a new hunk.
- **The open counter** is a new hunk in `begin_file_reading` and `term_input` in both engines'
  change files. Each runs once per opened file, `\read` line or terminal line, so the cost is in
  the noise.
- **The counts are recorded with each checkpoint.** A checkpoint's host record (Classic
  `ExtRecord`, Unicode `PageCk`) holds:
  - the run's epoch, which is new at every restore;
  - the counts: returns, prints and opens;
  - the level armed, and `max_buf_stack`.

  Records are compared only within one epoch, that is, two points of one uninterrupted run.
  At a jump, the new run's checkpoint at the convergence point takes the old run's record
  there, because it stands for the same state. A later test against it can then use the old
  run's later records. Thinned checkpoints need nothing, because the counts are cumulative.

The test that the line differs runs only after the ordinary test has failed. It is a filter over
the word diff and the structural comparison that are computed anyway.

**Generated code versus change files.** Everything above is a change-file hunk plus host Rust.
web2rust needs no new feature, so the barrier lives in the WEB, where every other incremental
hook lives (`checkpoint.ch`, `readset.ch`, `lineshift.ch`). A web2rust read barrier is §4's
rejected design.

**The cost** was measured in Phase 2 (§11): −0.14 % to +0.57 % instructions on Classic cold
compiles, +0.36 % and +0.63 % on in-body keystrokes, and under +0.1 % on Unicode cold compiles,
within the 1 % target.

## 7. How it composes

- **The convergence test.** The rule runs only where the ordinary test has failed, and only for
  that difference. It never marks a run converged, and it never relaxes the test for a
  convergence: the jump is a barrier's re-run, so the run goes on live. A state that would have
  converged still converges.
- **Checkpoints.**
  - The rule needs the old run's page checkpoints between *k* and *k′*, with their records.
    Retention may thin them; the counts are cumulative, so a thinned chain gives a smaller *k′*
    or none.
  - Segment hold (#1614) changes nothing: the watch is armed at each checkpoint taken.
  - The old checkpoints the jump keeps are corrected in place (§5.1). They are then the new
    text's states, so later restores and tests need no correction table. This differs from
    PREAMBLE-PATCH's `defpatch`, and it needs no persistence or retention bookkeeping.
- **The next keystroke.** It restarts at the same point, before the peek. It tests against this
  run's *k*, which is its own, and jumps to the corrected *k′*. So every keystroke after the
  first takes the short path.
- **BOX-MEMO and MACRO-REPLAY.** No soundness interaction:
  - Their replays verify with the ordinary comparison (`same_state`), which this rule does not
    touch.
  - A replay that runs `\csname` writes the same scratch as a normal run.
  - After the jump the run is an ordinary run, so the memos record and replay as usual.

  There is one cost interaction (measured, §11). After a run that recorded memos, the next run's
  replays put nodes in other places. Its dormant test then needs the structural walk:
  82 words on c1-lipsum, 6.7 ms. With `FLASHTEX_BOXMEMO=off` it compares word for word. The
  keystroke after it compares word for word too.
- **Line shifting.** The rule is off when the edit moves lines. A newline typed before
  `\end{document}` therefore still takes the long path. Composing the two is later work.
- **REVALIDATE and MIDLINE.**
  - REVALIDATE ignores a read that left nothing behind, between a restart point and the next
    page. This rule ignores data that is not read before a later page.
  - MIDLINE predicts the new state from the old one (`loc+3`) and restarts in the preamble. This
    rule compares the new run with the old one, in the body.
  - They do not overlap.
- **Preemption and paused runs.** The test is part of the convergence test (preemptible). The
  jump is the existing jump.

## 8. Alternatives considered

- **Compare only the state that is live at the next checkpoint.** Liveness ("read before it is
  written, later") is exactly what read tracking measures, so this is §4 under another name. A
  static liveness analysis is not possible for TeX macros.
- **A later convergence point.** There is none. The last page gets the new `x`, so the states
  never agree again; what this rule saves are the pages before the dormant line is read.
- **Restart directly at the old run's late checkpoint and re-read the line** (MIDLINE in the
  body). This would save even the pages up to *k*. But it has to predict what `get_next` looked
  at, and the category codes in force at the time are not known. MIDLINE's safe bound of `loc+3`
  covers the keystroke position itself, so the rule would never apply here.
- **Make the Unicode pages cheaper.** At about 55 M instructions per page, halving the cost would
  still leave the b4 row near 300 M. It is worth doing for every Unicode edit, but it does not
  close this row.
- **Accept the row** (XETEX-INCR's option (a)). The EDITPOINT keystroke already reaches its page
  in 83 M.

## 9. Expected effect (the Phase 1 belief; §11 has what was measured)

- **b4, keystrokes 2 and 3.** The restart is at page 11. The new run reaches *k* within about a
  page, keeps pages 12–18 and typesets pages 18–19 live. That is about 2 pages of work instead of
  9, roughly 545 M → 150 M to the edited page.
- **b4, keystroke 1.** Unchanged (§5.3).
- **The M3 row.** It takes the median of three keystrokes, so it follows keystroke 2.
- **Classic c1/c2.** 8 and 13 pages re-run become about 2–3.

## 10. Soundness cases

Classic: `tests/incremental.rs::a_dormant_line_is_kept_until_it_is_read`. Each document ends in a
`\newcommand\tail[1][]` called alone before the line it peeks at, and typesets about seven pages.
Three keystrokes go at that line's start, and every compile equals scratch runs (PDF, log,
`.aux`):
1. **`end`.** The line is `\end{document}`'s. Keystrokes 2 and 3 jump, kept to the last page.
2. **`read`.** The line is read before more pages are typeset from it. The jump stops at the
   page before the read.
   **`mid`** (Phase 2) reads it before more pages than the jump would keep: no jump (§5.1).
3. **`late`.** The line opens a file at once after a `\clearpage`. An edit of that file then
   restarts at the kept, corrected checkpoint (measured: `restart_pages` equals `rerun_from`).
4. **`show`.** A `\showthe` in the middle of the pages prints the line. The jump stops before
   it.
5. **`input`.** An `\input` in the middle of the pages. The jump stops before it.
6. **`csname`.** 400 `\csname`s across the pages, while every keystroke moves `first`.
7. **Keystroke 1.** In every case the peeked token differs, so there is no jump.

Unicode: `tests/host.rs::a_dormant_line_is_kept_until_it_is_read` covers `end` and `read` with
fontspec. Each compile's PDF, `.aux` and log equal a fresh process's; the log reports the jump,
and no `UNSOUND` check fires.

`FLASHTEX_DORMANT=off` turns the rule off.

## 11. Phase 2: what was measured

The evidence is in `docs/evidence/read-track-2026-10-20/` (NixOS PC, callgrind, main against
the branch; Classic sessions with their checkpoints pinned to shipouts, because the timed
checkpoints make a session's count depend on the load).

**The Unicode M3 row.** b4's keystroke at the end, to the edited page: keystroke 1 128.7 M
(unchanged), keystrokes 2 and 3 **502.0 M → 322.6 M**. The row takes the median of the three:
502.0 M → 322.6 M. The EDITPOINT row and b2–b3 are unchanged; b1 costs +0.6 M (+0.9 %), the
watch on its last page. The Phase 1 belief of 150 M (§9) left out two costs that every Unicode
convergence pays: the structural walk (107 M here; the Unicode test walks even when every word is
equal, because the glyph handles and the source map live outside the word space), and the
kept pages built again and sent (40 M). The walk is the next lever, in the general test.

**Classic.** Pages re-run per keystroke at c1's and c2's end: 8 and 13 on main; on the branch
8 and 13 at keystroke 1, then 2. Instructions over three keystrokes: c2 −23.9 %, c1 +5.8 %.
c1's keystroke 2 pays a structural walk after BOX-MEMO's replays (§7); from keystroke 3 on, c1
costs 27 % less per keystroke and c2 60 % less.

**The cost.** Cold compiles: plain-100 +0.30 %, full-100 +0.57 %, c1 +0.12 %, c2 −0.14 %.
In-body keystrokes (T7's kind, three in a section title): plain-100 +0.36 %, full-100 +0.63 %.
The watch in `pop_input` is most of it.

**Three corrections the measurements forced:**
- the jump writes the new line's words from `loc` up to `first`, not only those that differ
  (§5.2: the old run's `\csname` scratch);
- the worth rule (§5.1): a dormant line in the middle of a document no longer loses a later
  convergence (c1, a keystroke in `\section{Three}`: +12 % before it, main's pages after it);
- a failed test that the rule let past the edited file's stream turns the rule off for the run,
  as one that set words aside did. In-body edits otherwise paid a diff per page (full-100: 304
  diffs against main's 202).

Rejected after measuring: no structural walk past a dormant line. It saved c1's keystroke 2
but lost c2's (13 pages re-run instead of 2).

## 12. READ-TRACK-2: the walk and the kept pages (measured)

Lane READ-TRACK-2 took the two costs §11 left: the Unicode test's walk of an equal state, and
the kept pages. The evidence is in `docs/evidence/read-track-2-2026-10-20/` (NixOS PC,
callgrind, main `99a06dc01`).

**An equal Unicode state is not walked.** Once the dead and dormant words are set aside and
the free cells dropped, no word may be left for the walk. Then every word the walk reads is
the same in both states, so it would pair each node with itself. All it could still find is a
difference outside the word space:
- a native word's glyphs, read through its handle;
- a node's source position, read from the side table.

So `conv::same_outside` compares those two tables directly, and the walk runs only where they
differ.
- `Handles::same_glyphs` checks that every glyph-info handle has an equal array in both runs.
- `DlSnap::side_diffs` lists the side-table entries that may name other places. It skips the
  leaves both tables share. An entry that differs is then compared by place, as the walk
  compares it.

This holds only where the live run numbers its spans as the old run did at the checkpoint
(`numbered_like`, the jump's own condition). The test never adds "converged": where the tables
differ, or more than 256 entries differ, it walks as before.

**An equal token is not compared.** In the convergence walk of both engines (`Iso::check`), a
token at the same address in both states whose word is the same in both is paired with itself
without a cover. Only differing words are looked up in the covers (`bad_mem`), and this word is
not one. The pairing bits are still set, so a later pointer into the list is checked as before.

**Free lists by chunk table.** `drop_free_mem` reads the old run's free lists through the chunk
table (`ChunkDiff::old_words`), not with a map lookup per word.

**Kept pages reuse their frames.** A page's frame (`out::encode`) is a function of its display
list and of the keys of the fonts and images it names. A kept page built again from its XDV gets
the old run's frame when two things hold:
- its display list is the old one's;
- every resource id both documents have names the same key (`DocKeys`; both tables only grow).

**Measured:**
- **b4's M3 row:** 322.6 M → **194.9 M**.
  - The walk's 107 M became 0.6 M.
  - A kept page costs 4.4 M instead of 7.9 M.
- **Classic c1-lipsum, three keystrokes at the end:** 529.5 M → **482.4 M**. That is 3.6 % less
  than before #1861 (500.7 M); #1861 had left it 5.8 % more.
  - Keystroke 2 costs 230 M → 183 M: the walk 92 M → 54 M, the free lists 17 M → 8 M.
- **c2:** 461.4 M → 429.0 M.
- **In-body keystrokes:** plain-100 −1.4 %, full-100 −2.8 %, c1 `{Thr` −2.6 %.
- **Cold compiles:** +0.03 % at most (code layout).
- **Pages re-run** are main's in every session.

## 13. READ-TRACK-3: the kept pages' documents (measured)

Lane READ-TRACK-3 took b4's end row from 194.9 M to 151.7 M. The evidence is in
`docs/evidence/read-track-3-2026-10-21/` (NixOS PC, callgrind, main `8d23cf520`).

**The old documents are taken over.** A resident run's document is `Doc::new`'s, then each
page built in turn from its inputs: the XDV bytes, the source map's markers, `\mag`, the
pictures found and the fonts the XDV names. So where this run's document at the convergence
point is made from the same inputs as the old run's there (`out::same_doc`: each page the same
one or built from equal `Replay`s, the same forms count, paper, XDV comment and pictures, and no
error in either), building the old pages again would make the old run's later documents. The
jump then takes the old run's document at its target over (`out::adopt_doc`), with this run's
pages up to the convergence point, and sends the old pages with their frames. The carried
checkpoints keep the old run's documents (`PageCk::out` is now shared). Where the documents
differ (an edited page before the convergence point, as at EDITPOINT), the pages are built again
from their XDV as before.

**Two cheaper steps in every compile:**
- the forwarder reads a page's item opcodes without decoding it (`page::item_ops`), and decodes
  only a page that uses an item the client did not accept;
- `Arena::set_from` and `or_from` pack again only the deltas they edit (`Log::edit_chunk`), not
  every delta of each log; the chunk maps of the convergence test hash with a multiply and a
  fold (`ChunkHasher`).

**Measured:** b4 end 194.9 M → 151.7 M; b5 end 339.2 M → 135.8 M; EDITPOINT and Classic rows
equal or lower; cold compiles within ±0.004 % (Classic) and −0.9 % to +0.04 % (Unicode, the
forwarder). The pages re-run are main's in every session.
