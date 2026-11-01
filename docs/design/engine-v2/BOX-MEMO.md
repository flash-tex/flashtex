# BOX-MEMO: guarded replay of macro calls that typeset only what they throw away

Lane **P6-INFDESC-PAGE** (Commander, 2026-10-06: "attributes Infinite Descent's 800 M-instruction
page and builds picture/box memoisation"). Status: **revision 2 (T7-BEAMER, 2026-10-20): on in the engine host** (§11): it records in an
edit's window and replays; the command-line engine keeps it off unless `FLASHTEX_BOXMEMO=on`.
Revision 1 (the framed.sty prototype, off by default) is §2-§10.

It is DESIGN.md §12 P6's "picture memoisation" and §5.5's "tikz/pgfplots picture memoisation", in
its first, narrowest form. DESIGN.md stays the source of truth: where the two disagree, DESIGN.md
wins and this document is wrong.

**The governing rule** is MACRO-REPLAY's, unchanged: a replay must leave the engine in a state no
later computation can tell from the state the real expansion leaves, except for memory addresses and
memory-usage statistics (DESIGN.md §1.1). When anything the guard cannot vouch for is different, the
call is expanded for real. A replay is an optimisation, never a second implementation of TeX.

## 1. Why (measured)

`docs/evidence/p6-infdesc-page-2026-10-06/` has the measurements. Instructions per keystroke window,
from the restore to the edited page's shipout, on *Infinite Descent*:

- **The windows.** They are 125–850 M instructions. TikZ pictures are 45–75 % of them, and the
  output routine is 8–46 %.
- **The largest single item is framed.sty's `\fb@sizeofframe`.** It is 14–44 % of each window and
  40.5 % of all macro time in a cold pass. To learn how much a frame adds, it draws the whole frame
  (here a TikZ picture) around a 5 in rule in a box. It keeps two `\global` dimensions and throws
  the box away. It runs at least twice per framed environment, at 37–40 M instructions per call.
- **One call (pdflatex `\tracingassigns`).** It makes 623 global assignments, and no node leaves the
  call. Two of them depend on where the call is: pgf's picture serial (`\count367`, N → N+1) and
  `\pgf@sh@pi@box` = `pgfid`N+1. While a user types, the call at the same place sees the same N.

MACRO-REPLAY cannot take it, since its recordings refuse boxes, dimensions and typesetting. The
boundary agreed on #1319 (2026-10-06) splits the work:
- MACRO-REPLAY replays expansion-only macros;
- BOX-MEMO takes calls that build nodes;
- while a BOX-MEMO recording runs, no macro replay or intrinsics recording starts inside it, and a
  BOX-MEMO replay skips the inner calls entirely;
- BOX-MEMO never starts inside an intrinsics recording.

## 2. The unit: a *measure call*

A **measure call** is a call of a registered macro, with or without parameters. It qualifies when:
1. `big_switch`'s own `get_x_token` calls it, with no `expand` in progress (`bm_at_switch` and
   `expand_depth_count=0`), so `main_control` executes all of its body. D9's `intr_at_switch` is
   stricter than needed here: it is cleared by any expansion in the same `get_x_token`, such as the
   `\fi` just before `\fb@sizeofframe` in `\MakeFramed`;
2. it is balanced: groups, conditionals, `align_state` and the nest are at the end what they were at
   the start;
3. **no node it makes reaches a list outside it**: every list it builds is inside a box it discards
   before the end;
4. its only lasting effects are its assignments and groups (§4.3) and `last_badness`.

`\fb@sizeofframe#1` is the case that matters:

```tex
\begingroup \setbox\z@\vbox{... #1{\hbox{\vrule ...}} ...}%
\global\fb@frw\wd\z@ \global\fb@frh\ht\z@ \endgroup
```

**Registration.** Calls are offered by name, like D9's `\pdfstringdefPreHook`.
- The default list is `fb@sizeofframe`; `FLASHTEX_BOXMEMO_NAMES` overrides it.
- A registered macro is marked in `bm_cand[p]` (word space) when the format is loaded, or when the
  name is entered into the hash.
- Naming a candidate only offers it: the guard and the recording decide. No LaTeX is re-implemented.

**The call site.** `macro_call`, right after `@<Feed the macro body and its parameters to the
scanner@>` (tex.web §389):

```
@<Feed the macro body and its parameters to the scanner@>;
if bm_on then if bm_at_switch then if expand_depth_count=0 then
  if bm_cand[warning_index] then flashtex_bm_call(n,save_scanner_status);
exit: ...
```

At that point the body is the top input level, and its `n` arguments are `param_stack[param_ptr-n ..
param_ptr)`. Everything before the hook runs the same on both paths:
- the parameter scan and its errors;
- `\tracingmacros`;
- the "conserve stack space" pops;
- D9's and MACRO-REPLAY's own call sites (which come earlier, before `begin_token_list`).

A **replay** pops the body level with `end_token_list`, exactly as `get_next` does once the body is
used up (it also flushes the parameters and reports the level to the profiler). It then makes the
recorded effects. MACRO-REPLAY's call site, before `begin_token_list`, is a different line from this
one, so the two change files do not collide.

## 3. The guard: what a recording read must hold again

A call is replayed only if the **key** holds now. The key is checked by value, in full, on every
offer, with no hashes, so it cannot collide (D8's lesson).

| part | what | how it is captured |
|---|---|---|
| K1 | the macro's meaning and its arguments | `eqtb[warning_index]` by content, and each argument's tokens |
| K2 | every `eqtb` entry from `glue_base` to `eqtb_size` (glue, local, int and dimen regions) **except the box registers** | **conservatively**: all of them, at the call. Pointer values are taken by content: glue specs by their four fields, token lists by their tokens, `\parshape` by its words. Typesetting reads parameters directly (`hpack` reads `\hbadness`, `append_to_vlist` reads `\baselineskip`, ...); the conservative snapshot covers every such implicit read without a hook per site |
| K3 | the meaning of every control sequence the recording read before writing it | precisely: `get_next`'s delivered `cur_cs`, `\csname`/`\ifcsname` look-ups (the D9 read points, with BOX-MEMO's own hooks). By content: macros by their tokens, the rest by `(eq_type, equiv)` |
| K4 | e-TeX's sparse registers (`\count`, `\dimen`, `\skip`, `\muskip` and `\toks` above 255) | conservatively: every entry of the sparse trees, by (type, number, value), values by content as in K2 |
| K5 | the fonts and the hyphenation exceptions | two **version words** in the word space, `bm_font_version` and `bm_hyph_version`. Each gets a fresh, never-reused number at every `\fontdimen`, `\hyphenchar`, `\skewchar`, pdfTeX font-code (`\efcode`, `\lpcode`, `\rpcode`, ...), `\pdfcopyfont`/`\letterspacefont` and `\hyphenation`. A restore takes them back with the arrays they describe, and since a number is never reused, a version names exactly one state |
| K7 | names `\ifcsname` looked up and did not find; box registers read while void | each is still absent or undefined, and still void |
| K6 | the context | `\globaldefs = 0`, no pending `\afterassignment`, the mode, and `interaction`. Tracing parameters are in K2, and a recording that printed anything is abandoned (§4.1) |

**Box registers** are not in the key. A box register the recording reads must be one of two kinds:
- one it wrote first;
- one that is void when read, such as LaTeX's `\voidb@x`. Its voidness becomes part of the key
  (K7).

Any other box read abandons the recording. Reads are `fetch_box` (`\box`, `\copy`, `\unhbox`, `\vsplit`, `\wd`, `\ifvoid`,
...), `\leftmarginkern`/`\rightmarginkern` and `\showbox`. So `\fb@sizeofframe`'s `\@tempboxa`,
which holds the edited theorem, never invalidates it.

**Why this is enough.** A TeX run is a deterministic function of its state and its input. The
recorded call reads input only from its own levels: the body and the argument lists, in K1. It
reads state only through:
- `eqtb` (K2 and K3);
- the sparse registers (K4);
- the fonts and the hyphenation exceptions (K5);
- the box registers it wrote;
- the nest and the page builder, which it may not touch (§4.1);
- the scalars and C-part state, which §4.1 forbids or §4.3 lists.

What it builds in `mem` is reachable only from those. Anything else it could read is refused in §4.1.

## 4. Recording

A recording is the normal expansion, observed. It starts at the hook when no valid entry's key holds,
and runs with `bm_rec_on`.

### 4.1 Refusals

Any of these abandons the recording. The call then stays an ordinary call, and is offered again
later.
- **Input.** Reading a token from below the body's level (D9's `Why::Level`). A file or pseudo-file
  level at or above it (`\input`, `\scantokens`, `\read`, `\endinput`).
- **Box registers.** Reading one it did not write. Writing one at the call's own group level, or
  with `\global`.
- **The outer list.** Touching it at all: a node appended at the call's nest level, `\unskip`,
  `\unkern`, `\unpenalty` or `\lastbox` there, `\lastskip`, `\lastpenalty`, `\lastkern` or
  `\lastnodetype` read there, `\prevdepth`, `\spacefactor` or `\prevgraf` read or set at or below
  it, or a mode change of that level.
- **The page builder.** `\pagegoal` and the other `\page...` dimensions, `\deadcycles`,
  `\insertpenalties`, `\interactionmode`. At the end, the page and contribution lists, `page_so_far`,
  `last_glue`/`last_penalty`/`last_kern`, `output_active` and `insert_penalties` must be as at the
  start.
- **Marks.** `\topmark` and its kind (D9's `Why::Mark`). A `\vsplit` that changes the split marks or
  e-TeX's `disc_ptr` (checked at the end).
- **`last_item` reads outside an allowlist.** The allowlist: `\eTeXversion`, `\pdftexversion`, the
  `\...expr` scanners, `\fontchar..`, `\gluestretch` and its kind, `\pdfstrcmp`,
  `\pdfshellescape`. Refused, among others: `\badness`, `\inputlineno`, `\lastxpos`, the
  `\pdflast...` family, `\pdfrandomseed`, `\pdfelapsedtime`, `\currentgrouplevel` and the
  `\currentif...` family.
- **Expandable primitives outside an allowlist** (D9's list plus `\fontname` and `\jobname`).
  Refused: the file primitives (`\pdffilesize`, `\pdffilemoddate`, `\pdfmdfivesum`, `\pdffiledump`),
  the random and time ones, `\pdfpageref`, `\pdfximagebbox`, `\pdfcolorstackinit`, `\ifeof`.
- **Extensions outside an allowlist.** The allowlist is the non-immediate whatsits, which stay in
  the discarded box: `\write`, `\openout`, `\closeout`, `\special`, `\pdfliteral`,
  `\pdfcolorstack` (not `init`), `\pdfsetmatrix`, `\pdfsave`, `\pdfrestore` and `\pdfsavepos`.
  `\immediate` and every PDF object, annotation, link, destination, outline or catalog command are
  refused.
- **Assignments that change K5's arrays** (`\fontdimen`, `\hyphenchar`, `\skewchar`, the pdfTeX font
  codes, `\hyphenation`). A new font, string or control sequence (`font_ptr`, `fmem_ptr`, `str_ptr`,
  `pool_ptr`, `hash_used` changed).
- **A local assignment at the call's own group level.** The save stack below the call must be
  untouched; inside its groups anything goes.
- **Output of any kind** (log and terminal length, `file_offset`, `term_offset`). Errors (`history`,
  `error_count`). `\shipout`, `\end`, `\dump`, interaction-mode changes.
- **pdfTeX's C-part state changed** (`cstate`).
- **A restore while the recording runs** (§6).

### 4.2 The end

A recording ends at `big_switch` once every input level from the body's on is used up (D9's
`rec_exhausted`). It commits only if all of these are as they were at the start:
- `cur_level`, `save_ptr` and the save stack below it;
- `cond_ptr`/`if_limit`/`cur_if`/`if_line`;
- `align_state`, `nest_ptr`, the mode, `tail`, `scanner_status` and `after_token`;
- the string, hash and font counters;
- the output counters and the page-builder state of §4.1.

### 4.3 The effects: an operation log, as D9 keeps one

A recording keeps every `eqtb` and sparse-register assignment and every group, in order, at every
level, exactly as D9 does: `Begin(c)` (`new_save_level`), `End` (`unsave`), and `Def`/`Sa` with
the routine (`eq_define`, `eq_word_define`, `geq_define`, `geq_word_define`, `sa_def`, `sa_w_def`,
`gsa_def`, `gsa_w_def`) and the value. Replaying the whole log, local assignments inside the call's
own groups included, reproduces the `eqtb`, the save stack and the sparse registers the normal path
leaves, sharing included, because the same routines run on the same values.

A value is kept **by value**, never by address:
- an integer or dimension, or an `equiv` that is not a pointer the assignment made (a font, a
  `\chardef`, `\relax`);
- a glue specification made for the assignment (reference count null, as `new_spec` makes it), by
  its five fields, or one of the static specifications (`zero_glue` and kin), whose reference
  count the assignment takes;
- a token list made for the assignment (`\def`, `\edef`, `\toks`), by its tokens;
- `\let` and `\futurelet`: the meaning of the control sequence just read (`cur_cs`, D9's
  `K_LETCS`), *as it is at the replay*. A `\global\let` therefore shares the very list the normal
  path would share, so `iso.rs` pairs nodes one to one and the verifier's reference counts agree;
- a `\parshape` made for the assignment, by its words.

Anything else abandons the recording: a shared glue or token list from another source, e-TeX's
penalty shapes, a sparse register's `\countdef`, or a box value. Box assignments inside the call
are not logged: by §4.1 they are local to the call's groups, and so are undone before it ends.

**Scalars.** `last_badness` is written by every `hpack`/`vpack` and read by `\badness`, which a
recording refuses. Its final value is part of the entry, and the replay sets it. The verifier (§7)
reports any other scalar that differs.

## 5. Replay

1. `end_token_list` pops the body and its parameters.
2. With `rs_on`, the L5 read-set is told about the macro, every K3 control sequence, every assigned
   control sequence and every `\let` source, as D9's `intr_report_reads` does.
3. The log is replayed in order through `new_save_level`, `unsave`, `eq_define` and the rest, and
   e-TeX's `find_sa_element(t,n,true)` with the `sa_def` family. That gives the `eq_destroy` of
   old values, D9's watch reports (`flashtex_intr_touch`) and the save-stack entries. A recording
   made with tracing on printed something, so it never committed.
4. `last_badness` is set.

The log of a `\fb@sizeofframe` around a TikZ frame holds a few thousand operations. Its replay
costs a few M instructions, against 37–40 M for the call. A replay skips the groups whose
operations are all local (§11.2, MACRO-REPLAY revision 5's §11.5).

## 6. Storage, checkpoints and restores

- **Where the entries live.** In Rust, outside the word space (`src/boxmemo.rs`), per macro.
  - There are up to 1,024 variants per macro, least recently used dropped first, within a total
    cap of 64 MB (revision 1: 64; a beamer slide makes about 360-710 `\colorlet` calls, §11).
  - Each variant carries a 64-bit hash of K1 and the context, through which an index finds the
    variants to compare (§11.2). The hash only filters candidates; the full comparison decides.
  - Of the variants whose K1, context, K2 and K4 match, the most recent that also passes K3, K7
    and the object and font keys (§11.3) is replayed.
  - Why so many: a page of *Infinite Descent* measures a dozen frames, each with its own key, and a
    typed letter alternates between two states of each; a beamer slide's calls are hundreds.
- **Why outside the word space.** An edit's restore rolls the word space back to before the edited
  window, which would delete every recording made in it. Out here they survive, and they stay sound
  because the guard checks every value in full.
- **Values, not addresses.** An entry holds no address into `mem`. Token lists, glue specs and
  arguments are kept by value.
- **Where recordings are made.** Replays are allowed everywhere. Where recording is allowed depends
  on the caller:
  - On the command line, anywhere.
  - In the host, only inside an edit's window, from the engine's resumption after the restore to
    the edited page's shipout (`boxmemo::record_window`, set by `incr.rs`). Those are the calls the
    next keystroke runs again.
  - Never in a cold pass. Its calls never hit, because pgf's picture serial is in their key (§9).
    Recording one costs about 1.7× the call.
- **Restores.** A restore abandons a recording that is open (a preempted compile, the convergence
  test's own restores). `Globals::fill_scalars` calls `boxmemo::after_restore`.
- **`bm_rec_on`, `bm_at_switch` and the version words** are in the word space.
  - A convergence test at a checkpoint taken while a recording is open sees `bm_rec_on` differ
    from an old run that was not recording, so it does not converge there. That is conservative.
  - The version words are fresh numbers. The convergence test treats them as dead (`incr.rs`
    `dead_word`): the arrays they name are compared themselves.
- **Checkpoints inside a recording are fine.** A segment or timed checkpoint in the middle of a
  recording is an ordinary checkpoint. A later restore to it finds `bm_rec_on` true and the Rust
  recorder gone, and `after_restore` clears the flag.
- **Known gap: diagnostics.** A replayed `\xdef` makes a token list without a definition site
  (`diag::dg_def`), so go-to-definition finds nothing for `\fb@frw`-like macros defined by a
  replay. That changes no output.

## 7. Verification

`FLASHTEX_BOXMEMO=verify` runs both paths for every call the guard admits and compares the whole
engine state. As built (`src/boxmemo.rs` `bm_verify_full`):

1. Where the call is admitted, take a checkpoint C.
2. Run the normal path under a verifying recording. If the recording is abandoned, the guard let
   through an impure call: that counts as a difference.
3. When the body is used up (`big_switch`), the op-log check runs: the normal path's operation log
   and `last_badness` against the entry's. Then the used-up input levels are popped and a
   checkpoint N is taken.
4. Restore C, which keeps N in the pending branch. Replay, and do what `macro_call`'s exit did on
   the normal path (`scanner_status`, `warning_index`).
5. Compare the replay's state I with N using **the convergence test's own comparison**
   (`incr::same_state`, which is `same_words` with the structural comparison `iso.rs`):
   - every word of the word space either path wrote;
   - dead words and free cells aside;
   - nodes allocated in other places compared structurally;
   - pdfTeX's C-part state.
6. The run goes on from I. C is dropped.

Before comparing, the replay's state takes the normal path's values of what each command writes
before it reads. Each item is cited:
- D9's scratch list: the scanner's result registers, `def_ref`, `long_state`, the
  pseudo-printing state, `dig`, `trick_buf`, `pstack`, and `big_switch`'s two flags.
- `remainder`. `x_over_n` and `xn_over_d` set it (tex.web §106, §107), and it is read only right
  after (§458, §461, §716, §717).
- `hpack` and `vpack`'s `total_stretch` and `total_shrink`. Each pack zeroes all four orders first
  (§650, §668).
- `link(hold_head)`, made null on both paths. The alignment preamble scan (§779–§784),
  `init_span` (§789), `fin_col` (§808), `reconstitute` (§905) and the hyphenation loop
  (§913–§918) each set it before reading it. Between two commands it is a dangling pointer to cells
  either path may have reused. The convergence test already treats `temp_head` and `backup_head`
  the same way.

Each of these was found by the verifier as a false positive on *Infinite Descent*. Once covered, 0
differences remained.

- **In the host** (revision 2) the full-state check is off. Each admitted call's operation log,
  output and `last_badness` are still compared with the normal path's (`bm_verified`, steps 2–3).
  - The admission checkpoint C stays the base of the state the run continues from, so it cannot be
    dropped after the comparison. In the host every verification left one among the restart
    points, with no page count, and a compile interrupted by the next edit restarted there.
  - Its restore also replaced the edit's pending branch, so a verified compile never converged.
- Inside an alignment, the comparison takes the preamble (`link(align_head)`: tabskip glue and
  alignrecords, which the walk does not know) by its words. Neither path made it
  (`iso::align_by_words`).
- `FLASHTEX_BOXMEMO_VERIFY_FAIL=1` exits with status 3 at the first difference. The sweeps run with
  both settings (`tools/incr-bench/sweeps.py`).

## 8. Gates (adopted unless the review says otherwise)

1. **Both paths, diffed.** `verify` over the parity fixtures, the lockstep corpus, the soundness
   sweeps' documents and *Infinite Descent*, with **0 differences**.
2. **P-T1/P-T2 unchanged with BOX-MEMO on.** Every engine PDF is byte-identical to pdflatex's.
3. **Soundness sweeps A, C and D** with BOX-MEMO on, plus the typing sweep on *Infinite Descent*
   (letters, sentences, newline, split/join): 0 mismatches against from-scratch runs.
4. **Faults** (`FLASHTEX_BOXMEMO_FAULT`): drop an effect, skip the K2 check, skip a K3 read, forget
   sharing, keep a recording across a restore. **Each must be caught by gate 1 or gate 3.**
5. **Measured.** Instructions per edited-page window on *Infinite Descent ×2*, typing@50ms, with
   BOX-MEMO off against on, and the convergence rate unchanged (T7's held rates).

## 9. Gain (measured by the prototype)

Evidence: `docs/evidence/p6-infdesc-page-2026-10-06/boxmemo/`. It records instructions per
keystroke window on *Infinite Descent*, with the same engine run off and on.

| site | off | on | Δ |
|---|---:|---:|---:|
| equivalence-relations:430 (theorem body) | 857 M | 749 M | −13 % |
| equivalence-relations:426 (prose) | 126 M | 86 M | −32 % |
| sets:195 | 665 M | 375 M | −44 % |
| discrete-probability-spaces:13 | 246 M | 167 M | −32 % |

- **Replay cost.** A replay costs 0.5–2 M instructions against 37–40 M for the call (the
  `\fb@sizeofframe` share that remains). That is the K2/K4 snapshot, the comparison and the
  operation log, a few thousand `eq_define`s.
- **The first keystroke at a place** records, which costs about +30 % per call (+11 to +32 % for
  that window). Every later keystroke there replays.

**A cold pass gains nothing yet.** K2 and K4 include pgf's serial, so every call misses. That is
why the host records only in an edit's window. Phase 2 (below) is where cold compiles gain.

## 10. Later phases

### 10.1 Registers keyed precisely (built)

Primitives never read the `\count`, `\dimen`, `\skip`, `\muskip` and `\toks` registers behind the
scenes. Every read is explicit:
- `scan_something_internal`: `\the`, `\number`, `\ifnum`, `\ifdim`, and every scanned quantity;
- `\advance`, `\multiply`, `\divide`;
- the `\toks` copy, which the recording refuses.

The exception is box registers, which §3 handles.

So K2 now leaves registers out, e-TeX's sparse registers included (K4 is empty). The recording keeps
**KR**: each register it read, by value, at its first read. Hooks: `Fetch a register`, `Fetch a
token list`, `assign_int` & co. naming a register, and `do_register_command`.

A register the call first **assigned globally** is not an input when it is read later.
- A *local* assignment is undone when its group ends, so a later read sees the old value. That old
  value is keyed when the assignment is made.
- Control sequences follow the same rule. The old meaning that a global assignment overwrites is
  not an input, because the replay's own `geq_define` handles whatever is there, as the normal path
  did.

Effect: calls hit wherever the registers they actually read agree. In-window hits went from 6 to 54
on `framed-tikz-theorems`, with 0 full-verify differences.

### 10.2 Phase 2, cold-pass hits: measured, and not reachable by keying

The debug run (`FLASHTEX_BOXMEMO_DEBUG`, `key differs` lines) looked at consecutive
`\fb@sizeofframe` calls in one pass. With precise registers they still differ:

- **pgf's picture serial.** A sparse `\count`, read by its `\advance` and printed into
  `\pgfpictureid`.
- **pgf's leftovers from the previous picture, read before they are written:**
  - `\pgfsyssoftpath@lastmoveto`, the last move-to of the previous path;
  - `\pgf@sh@pi@box`, the picture of the previous `box` node;
  - `\font@name`;
  - `\if@endpe`;
  - `\dimen0`.
- **The environment kind** (`\FrameCommand`, `\@currenvir`). This only multiplies the variants,
  which is fine.

A key built from reads therefore cannot match across calls in one pass. Making it match would mean
proving that these reads do not affect the result: taint tracking of the values from the read to
every use (comparisons, `\csname`, arithmetic), through token lists copied by `\edef` and by macro
arguments. That is research-scale, with a soundness argument of its own.

**Recommendation:** do not pursue phase 2 for now. Typing, the P4 target, already hits, because the
same call at the same place reads the same leftovers keystroke after keystroke. For cold passes,
COLD-OPEN's levers (the anchor, pass-2 reuse) and P6-ENGINE-SPEED's interpreter work are worth more.

### 10.3 Phase 3: draw calls (next)

These are calls that append nodes to the current list in restricted horizontal mode:
- the frame command inside `\centerline`'s `\hbox` (`\fb@putboxa`);
- `tikzcd` and `tikzpicture` bodies.

How a draw call is recorded and replayed:
- **The recorded fragment.** The nodes between the list's tail at the start and at the end are the
  output. They are serialised by value, following `copy_node_list`'s node formats (tex.web §206,
  pdfTeX's whatsits), together with each node's display-list source position (`dl_side`), so the
  preview maps them the same way.
- **The box register the call moves** (`\box\@tempboxa`, the theorem's content) **is a hole.**
  - It is keyed by its type, width, height, depth and shift.
  - At replay, that node is spliced into the rebuilt fragment where the recording found it.
  - `\copy`, `\unhbox` and `\unvbox` of a box not made by the call abandon the recording, because
    they read its contents.
- **The rest of the outer list is untouched.** These are also recorded and replayed: `space_factor`
  (box_end sets it to 1000), the voided register, and the mode (unchanged).
- **The page builder never runs.** In restricted horizontal mode it cannot.
- **Gates** are those of §8, plus the full verifier, which compares the appended nodes structurally.

Expected gain, from the Step 1 profiles: frame draws are 9–25 % of a window and `tikzcd` 24–30 %
where present.

- **Automatic candidates.** Calls the macro profiler shows above a cost threshold, instead of names.

## 11. Revision 2: beamer (lane T7-BEAMER, 2026-10-20)

Every beamer row of T7 missed §1.2 (PR #1850's evidence). A frame's body is a macro argument,
typeset at `\end{frame}`, so an edit in it restarts at the frame's start and the whole slide is
typeset again: about 200 M instructions per key on the default theme (beamer-5) and 457 M on
Madrid (beamer-30), against pdfTeX's 134 M and 312 M per such slide (x86 callgrind, NixOS PC). No
interpreter speed-up reaches 11 ms there; the slide's templates are the cost.

### 11.1 Where the slide's instructions go (measured, the edit window)

`FLASHTEX_MACRO_PROFILE_CLOCK=instr` with roots (each instruction charged to the innermost listed
macro), letter@middle, NixOS PC:

| | beamer-5 | beamer-30 |
|---|---:|---:|
| xcolor's `\XC@col@rlet` (beamer's `\usebeamercolor` calls `\colorlet` ~8 times) | 60 % (363 calls) | 49 % (709 calls) |
| pgfmath | 13 % | 16 % |
| the output routine, inclusive (headline, sidebars, navigation symbols, footline) | 90 % | 54 % |
| Madrid's block (`\beamerboxesrounded` and its end), inclusive | - | 38 % |

### 11.2 Cheap hits

At revision 1 a hit cost about as much as a `\colorlet` (100 K instructions): every offer built
K2 (2,250 values), hashed it, and copied K3's macros to compare them. Now:
- **An index.** The variants of a macro are found by the hash of K1 and the context.
- **K2 in place.** K2 is compared against each kept K2 as it is walked, run by run (a fixed plan of
  `eqtb` runs of one kind), stopping at the first difference; the variants recorded in the same
  state share one K2 (an `Rc`), compared once per offer. Nothing is built for a hit.
- **K3 in place.** A macro's meaning is compared token by token in `mem`.
- **Local groups elided.** A replay skips every group whose operations are all local
  (`elide_local_groups`): its `unsave` restores each location to the word it held (tex.web §283),
  and e-TeX's "reassigning" saves nothing, so the group's net effect on `eqtb`, the sparse
  registers and the save stack is nothing. It leaves only free-list order and the usage statistics,
  which DESIGN.md §1.1 counts as accounting and the convergence test treats as dead. The full
  operation log is kept for the verifier.

### 11.3 Draw calls that beamer's templates make (phase 3, §10.3, built for these)

`\@oddhead` (headline, sidebars, navigation symbols) and `\@oddfoot` (footline) are draw calls in
`\hb@xt@\textwidth{...}`. They needed:
- **Links.** `\pdfstartlink` makes an object (`pdf_new_objnum`). That object is a *relative effect*:
  the recording notes the objects the call made (they must be consecutive, and nothing else may
  make one), the fragment refers to them by index, and a replay makes its own in the same order
  (`pdf_new_objnum` again, so pdfTeX's tables and its C part see the same calls) and writes them
  into the rebuilt nodes; `\pdflastlink` follows. A link's action structure is kept by value, its
  token lists as `delete_action_ref` owns them. `\pdflastlink` read inside a call still refuses it.
- **Forms and images.** `\pdfrefxform` and `\pdfrefximage` copy the object's dimensions into the
  node. Each object the call referred to joins the key: it must still be in its type's list
  (`pdf_check_obj`'s walk) with the same dimensions.
- **Shared glue.** A specification shared with a sparse skip register (`Spec::Sparse`) or with a
  font's interword glue (`font_glue[f]`: an interword space, or beamer's `\beamer@lastskip=\lastskip`
  after one; the font glue's fields join the key), and `\skip=\lastskip` of a glue the call made
  itself (beamer's `\reset@color`: only the call's own nodes share it, so a replay makes it fresh and
  shares it from then on as the normal path does).
- **The current list's `aux`** at the call is part of the context: a draw call's first space reads
  `\spacefactor`.
- **The verifier** compares fragments in the bits the program reads (`boxfrag::meaningful`: a word
  holding one `scaled` leaves its other half as `get_node` found it; an action's unused fields), and
  reports the first difference.

### 11.4 On by default where edits are served, and what is offered

BOX-MEMO is on in the hosts that serve edits (`flashtex-host --socket`, `serve`, `iserve`;
`set_default_on`) unless `FLASHTEX_BOXMEMO=off`. The command-line engine and the host's other tools
(`selftest`, which compares resumed runs word for word, `bench`, `open`, `layout`) keep it off. In
the host it records only in an edit's window (§6) and replays anywhere; the store is at most 32 MB,
the macro bodies of the keys interned once (a 58-page Madrid deck: +57 MB peak RSS over a mixed
T7 session, 245 MB against 188 MB).

`DEFAULT_NAMES`: framed.sty's and ntheorem's (revision 1), `XC@col@rlet`, `pgfmathsetlength`,
`pgf@process`, `pgfpoint`, `@oddhead`, `@oddfoot`, `beamer@typesetheadorfoot` and
`beamer@@tmpl@frametitle` (revision 3 adds five, §12). Naming a macro only offers it; the guard decides. What still refuses
(measured): Madrid's block ends (`\unskip` of the outer list, and the shadow's
`\immediate\pdfobj`), `\beamer@calculateheadfoot` (a box register set at the call's level), the
first frame's font loads.

### 11.5 Font versions named by their changes, and abandoned keys

- **Font versions.** Revision 1 gave the fonts a fresh version at every `\fontdimen` or
  `\hyphenchar` & co. An edit's window that loads a font (NFSS sets the new font's `\hyphenchar`)
  then reached a new version at every keystroke, and no call after it ever hit (beamer-30's first
  block frame: 667 M instructions a key). Now a version is named by its parent and the change
  (`next_version`, an interned table): the same changes from the same version reach the same
  version, and equal versions still name equal font arrays. Each engine start takes a fresh base.
- **Doomed entries.** An abandoned recording's key, as far as it read, is kept: the same reads take
  the call to the same refusal, so a call matching it runs for real, unrecorded, and the calls
  inside it are offered (a recording would have refused them). beamer's
  `\beamer@typesetheadorfoot`, which makes a shading object on a deck's first block frame, no
  longer hides its `\colorlet` calls.

### 11.6 Two holes the verify sweep found (fixed in revision 1's code too, #1692)

- **A `\csname`'s location.** K3 kept the meaning at the location a `\csname` led to, not that
  the name leads there. A name's place in the hash depends on the order names were made in
  (tex.web §260: a colliding name takes the next free slot), so after an edit that makes names
  in another order (sweep C's labels and sections) the same `\csname` leads elsewhere. Now the
  name and its location are part of the key (`Meaning::Name`). Found: framed-tikz-theorems,
  `\fb@sizeofframe`'s operation 1380 defined location 65499 where the normal path defined 65500.
- **`last_badness` of a call that packs nothing.** It was always set by a replay, to the value
  the recording ended with, though a call that packs no box leaves it as the state had it. Now it
  is kept only when the call packed (a hook in `hpack` and `vpackage`). Found: `\XC@col@rlet`
  replayed 19 where the normal path left 0.

### 11.7 Measured

`docs/evidence/t7-beamer-2026-10-20/`: T7's beamer rows, instructions per key to the edited page
(the gated quantity's engine share, load-independent), main against BOX-MEMO on, NixOS PC:
beamer-5 200 M → 37.5 M (−81 %) on every in-body and typing row, beamer-30 470 M → 132 M (−72 %),
its first block frame 486 M → 168 M (−65 %). Convergence is unchanged. 0 differences in verify
mode (host and command line).


## 12. Revision 3: more names, the intrinsics' first page, and new control sequences (lane T7-LAST12, 2026-10-20)

After revision 2, nine of beamer-30's rows and full-10 letter@start still missed §1.2 on the
reference Mac (PR #1855). Evidence: `docs/evidence/t7-last12-2026-10-20/`.

### 12.1 Names (measured, NixOS PC, `FLASHTEX_BOXMEMO_NAMES`)

Added to `DEFAULT_NAMES`, each measured alone and together on beamer-30 letter@middle (typesetting
instructions to the edited page, revision 2: 125 M):
- `pgf@parsefunc`, pgf's parse of a shading's colour function, which Madrid's block shadow runs
  four times a block (`\pgfdeclareradialshading` and kin declare their shading at every block,
  `pgfcoreshade.code.tex`): 107 M. The `\immediate\pdfxform` after the parse still runs for real;
  only the parse is replayed.
- `beamer@thc@docolor` and `beamer@thc@usecolor` (beamer's colour templates) and `pgfusepath`
  (a path's use: its PDF literals are a draw call's fragment, §11.3): together 101 M (−19 %).
- Tried and left out, no gain or a loss: `pgftransformcm` and `pgf@transformcm`, `pgf@sys@bp`,
  `pgfmathparse`, `XC@definec@lor`, `XC@mcolor`, `extractcolorspec`, `pgf@getrgbtuplewithmixin`,
  `\beamer@usebeamercolor`.

On beamer-30's first block frame (letter@start) the gain is smaller (161 M → 146 M): there the calls
make control sequences for the first time (`NewCs`, §4.1), as every keystroke on that frame does
again after its restore (§12.3), and the transition shading of the block's colours is made there
(`\pgfuseshading`, an `\immediate\pdfxform`).

### 12.2 `\pdfstringdefPreHook` taken from the intrinsics (adopted)

hyperref's hook, which siunitx fills (about 25 M instructions a call), is the guarded intrinsics'
first target (DESIGN.md §5.6 item 4): recorded at its first call and replayed at every later one.
Those recordings live in the word space (`intrinsics.rs`), so a keystroke on the page that makes
the first calls (page 0's output routine: full-10 letter@start) restores to before them and runs
the hook twice for real, recording it again each time: 146 M instructions to the edited page.

**First try, not adopted.** `pdfstringdefPreHook` in BOX-MEMO's `DEFAULT_NAMES`, with the
intrinsics leaving it unrecorded only in the host's edit window: 95 M (−35 %), but the run's
intrinsics state then differed from the old run's (one recording fewer). The convergence test
compares the intrinsics' recordings word for word (`Iso::intrinsics`), so a run with such an edit
no longer converged (`tests/incremental.rs`, `intrinsics_recorded_after_an_edit_converge`).

**Adopted: one owner per macro for the whole session.** Wherever BOX-MEMO is on and offers the
hook (`boxmemo::owns`: the hosts, or `FLASHTEX_BOXMEMO=on|verify`), the intrinsics never record it
(`intrinsics::config` leaves it out of their names), and BOX-MEMO records it anywhere, not only in
an edit's window (`RECORD_ANYWHERE`). Every run of the session then leaves the intrinsics' state
alike (no recording of the hook in any of them), so the convergence test is unchanged; BOX-MEMO's
entries live outside the word space and survive the restore. With BOX-MEMO off (the command line)
the intrinsics keep the hook as before. Measured (NixOS PC, T7 full-10, 19 keys each):
letter@start 145.7 M → 95.0 M instructions to the edited page (13.3 → 9.5 ms thread CPU), 19/19
converged; letter@middle unchanged (92.6 M, 93.5 M). `intrinsics_recorded_after_an_edit_converge`
and the rest of `tests/incremental.rs` and `tests/host_incremental.rs` pass with BOX-MEMO on (the
hosts' default) and off.

**A hole the verify sweep found** (sweep A, full-100, run 39509942631): on the command line's
verify mode (the whole state after each replay compared with the normal path's), a replay of the
hook on a page with a pending footnote "differed" in a page insertion record's `broken_ptr`. The
structural comparison (`iso.rs`, used by the convergence test too) followed `broken_ptr` and
`broken_ins` in every record, but pdfTeX reads them only when the record's type is `split_up`
(tex.web §986, §1021); an `inserting` record holds there what `get_node` left, which the
allocation order decides (and a replay's elided groups change it). They are now compared only for
`split_up` records. The case reproduced on the PC (its edited source, run twice) and passes after.

### 12.3 Calls that enter control sequences (`Op::NewCs`)

§4.1 refused a call that left a new string or control sequence: `\csname` of a name never made
before enters it (tex.web §259-260), and where it lands depends on the hash table as the run left it.
Such calls are the first use of something in the document (a colour's `\\color@...` name, a
shading's names, a block's colours on beamer-30's first block frame), and every keystroke on that
page restores to before it and makes them again, so they never hit (`NewCs`, 40 abandoned recordings
over a letter@start session).

- **Recording.** `id_lookup`'s hook for a new control sequence (`flashtex_bm_new_cs`) appends
  `Op::NewCs { p, name }` to the log, in order with the other operations, if no string was being
  built (`id_lookup` moves one up past the new name; else the recording is abandoned as before).
  The name's own lookups are not part of the key (the operation stands for them); its meaning at
  its first read (undefined) is.
- **The end.** The string and hash counters may have moved only for those: the call's strings are
  exactly the new names, one each, in order, with none being built at its start or end
  (`bm_strings_are_new_names`); anything else is still `NewCs`.
- **The offer.** Part of the key: each name is absent, and `id_lookup`'s search for a free
  location, made for each name in order without changing anything, ends where the recording's did
  (`bm_new_cs_placeable`: the bucket's chain, an occupied bucket's overflow to web2c's
  `hash_extra` (`hash_high`) or down from `hash_used`, room in the buffer). Then the replay's
  `id_lookup` makes the same string, the same hash entry and the same link as the normal path.
- **The replay** enters each name where the log has it, as `\csname` does (the name in the buffer
  above `first`, `id_lookup` with `no_new_control_sequence` false, tex.web §374), so the L5
  read-set, the intrinsics and BOX-MEMO hear of it from `id_lookup`'s own hooks; a location other
  than the one worked out stops the program (`assert`). A group holding a `NewCs` is never elided
  (§11.2): no `unsave` takes an entered name away.
- **Measured** (NixOS PC, instructions to the edited page): beamer-30 letter@start 149 M → 126 M
  (from 164 M before this lane); letter@middle unchanged (105 M). Verify mode in the host: 2,179
  calls verified on letter@start, 1,708 on letter@middle, 536 on beamer-5, 15 on full-10, no
  difference. The verifier compares the normal path's log, its `NewCs` operations included, with
  the entry's. On the command line no entry with a `NewCs` ever hits (the names stay made for the
  rest of the run); `tests/host_incremental.rs`
  (`a_replayed_call_enters_its_new_control_sequences_again`) runs the replay in the host and
  compares every page with a scratch compile.

### 12.4 Cheaper offers: meanings found equal, names at their location

After §12.1–§12.3, BOX-MEMO itself was 39 % of beamer-30's edit window (perf, frame pointers,
letter@middle): the key checks 21 % (K3's macro bodies compared token by token 15 %, K2 3 %, the
names' lookups 3 %) and the replays 15 %. Every offer compared every body its variants read, and a
slide's ~300 offers read the same few hundred xcolor, pgf and beamer macros.

- **Meanings found equal** (`Seen`). A body K3 found equal is kept with the location, its type and
  the list's reference count `e`: while `eqtb[p]` still holds that type and `e`, the tokens are
  still that body, and the walk is not made again. A macro's list does not change while it is
  referenced (TeX makes a new list for every definition); `delete_token_ref` freeing a list (its
  count reaching zero, the one place a referenced list is freed) takes the entries naming it with
  it (`flashtex_bm_freed`, a hook in changes/boxmemo.ch behind `bm_on`); every restore of the word
  space, a format load, and another engine on the thread empty them all. 93 % of K3's comparisons
  are answered so on beamer-30.
- **Names at their location.** A `Meaning::Name` (§11.6) is the name `id_lookup` finds at `p`:
  for a location of the hash's chains that is the text there, compared in place (`id_lookup`
  enters a name only where it found none, in that name's chain; a location's text stays). The
  frozen locations, whose texts repeat names, are looked up as before.

### 12.5 Replays make each group's net effect

A replay made every operation of the recorded log but the groups whose operations are all local
(§11.2). Beamer's templates and pgf's `\pgf@process` (`{#1\global\pgf@x=\pgf@x...}`) do their
work in groups that end with global assignments, so their local assignments were made and undone
again: `\@oddhead` replayed 13,667 operations (0.55 ms), `\pgf@process` about 180 a call.

At its `unsave` a group puts back every location it assigned locally and keeps every location it
assigned globally, with the value of its *last* global assignment (`unsave` keeps a location of
level one, tex.web §283; a local assignment after the global one saved the global value, which
`unsave` puts back). So a group stands for one global assignment per location, the last one,
made where the group was (`compress_groups`); a group with none leaves nothing, as before. A
global value taken from the state (a `\let`'s meaning, a shared glue) is the one where the group
was when nothing in the group assigned what it reads before it, and is made there, before what the
group assigns after it. Otherwise the group is replayed as recorded, less its local assignments to
every location no such value in it reads (its `unsave` takes them back unread); a group that
assigns a sparse register globally or enters a control sequence stays whole. What changes besides
is what §11.2's elision changes: free-list order and the usage statistics.

`\@oddhead` now replays 3,051 operations, `\pgf@process` 3 or 4.

**Measured** (NixOS PC, T7 beamer-30, 19 keys; instructions to the edited page, p50; thread CPU):

| | letter@middle | letter@start |
|---|---:|---:|
| main (`9697ee948`) | 141.2 M | 164.2 M |
| §12.1–§12.3 | 104.8 M, 11.1 ms | 126.2 M |
| + §12.4's meanings | 104.1 M, 10.7 ms | — |
| + §12.5 and §12.4's names | 93.4 M, 9.7 ms | 113.7 M, 11.3 ms |

BOX-MEMO's share of the window fell from 39 % to 25 %.

**Correctness.** `cargo test --lib boxmemo` (the group rules, by case); `tests/boxmemo.rs` with
BOX-MEMO off, on and verify; on the command line (`FLASHTEX_BOXMEMO=verify`, which compares the
whole engine state after each replay with the normal path's) beamer-30: 6,133 replays, 0
differences, and the PDFs with BOX-MEMO off, on and verify byte-identical. The host's verify mode
compares the normal path's operation log with the entry's, not the replay, so §12.5 is checked by
the command line's full verifier and the sweeps' PDF comparisons.

## 13. Revision 4: a call's objects counted by `sys_obj_ptr`, K2 compared in blocks (lane T7-LETTERSTART, 2026-10-21)

The 2026-10-21 reference run (PR #1863) still missed beamer-30 letter@start: p50 9.7, p95 11.5 ms.
Its raw records show compute, not waiting: queue p50 0.02 ms, off-CPU 0.3 ms; 130 M instructions
to the page on the M5 against 108 M on letter@middle, and a second key at 258 M. Evidence:
`docs/evidence/t7-letterstart-2026-10-21/`.

### 13.1 A call's objects are counted by `sys_obj_ptr`

The first block frame's footline (`\beamer@typesetheadorfoot`, 22 M instructions a key in the
macro profile against 0.6 M on letter@middle's slide) was refused for good: its recording ended
with "Objects", though the only object it made was its navigation link's, which a replay makes
again (§11.3). The end of a recording counted the call's objects as `obj_ptr`'s rise. But
pdftex.web takes an object stream's number from `sys_obj_ptr` and leaves `obj_ptr` one behind
("object stream is not accessible to user", `pdf_os_prepare_obj`), until the next object made sets
`obj_ptr := sys_obj_ptr`. The shipout before that frame began an object stream, so the call's one
link moved `obj_ptr` by two, and the call looked as if it had made an object of its own. Every key
on the slide then ran the footline for real, and its inner calls were offered only from the second
key on, which is why that key recorded 87 of them (`\XC@col@rlet` and kin) and took twice as long.

A recording's frame now holds both counters. It commits when the objects made are exactly its
links' (`sys_obj_ptr` moved by their number, in order from its value at the start), and
`obj_ptr` ends where the normal path leaves it: unchanged with no object, equal to `sys_obj_ptr`
otherwise, which is also where a replay's `pdf_new_objnum` calls leave it, lag or none. Test:
`tests/boxmemo.rs` `links_recorded_after_an_object_stream_began` (refused before the change,
three hits after it, the PDF and log byte-identical with BOX-MEMO off, on and verify).

### 13.2 K2's runs of equivalents and integers compared 16 words at a time

`bm_k2_is` compared each word of K2's runs of `eqtb` (a few thousand at every offer) through an
iterator with an exit at each word: 6 % of the slide's typesetting. Both kinds compare a word's low
half; `low_halves_eq` XORs 16 words into one value before it tests, the same answer
(`low_halves_compare_as_the_word_by_word_loop` compares it with the loop).

### 13.3 What remains

The second key after the first edit on the first block frame still records a few calls
(`\pgf@parsefunc`, 4 of them, about 50 M instructions): the shading declarations around them make
a PDF object and are refused, and a call is never offered inside another call's recording (§1),
so the inner calls are first offered at the key after the refusal. Recording inner calls while an
outer one records would need a stack of recordings; not done.

## 14. Revision 5: `\pgf@parsefunc` recorded anywhere (lane T7-MARGIN, 2026-10-21)

§13.3's second key: on beamer-30's first block frame the key after the first edit still recorded
the block's four `\pgf@parsefunc` calls (about 50 M instructions, the row's slowest key and so its
p95 on the 2026-10-21b reference run). In the window they could not be recorded earlier: the
shading declarations around them are refused (their `\immediate\pdfxform`), and a call is never
offered inside another call's recording.

`\pgf@parsefunc` is now one of `RECORD_ANYWHERE` (§12.2), so the host records it wherever it is
offered outside a recording, the open's cold run included: there each function is recorded where
it is first parsed, and every later key that parses it again, the second on a frame included, hits.
A cold run's calls of the same function hit too (Madrid declares the same four shadings at every
block). The intrinsics have no `\pgf@parsefunc`, so `owns` changes nothing for them.

Measured (NixOS PC, T7 beamer-30, two rounds of 19 keys, instructions to the edited page): letter@start
key 1 143 M → 93 M (the steady keys 92 M); the open's preamble row (a run from the format) 24.5 G →
24.1 G instructions for the whole compile, 5.29 G → 5.24 G to its first page, wall unchanged.
Evidence: `docs/evidence/t7-margin-2026-10-21/`.
