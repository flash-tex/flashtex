# BOX-MEMO: guarded replay of macro calls that typeset only what they throw away

Lane **P6-INFDESC-PAGE** (Commander, 2026-10-06: "attributes Infinite Descent's 800 M-instruction
page and builds picture/box memoisation"). Status: **design, revision 1, for review, with a measured prototype.** The prototype
behind it is off by default.

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
costs a few M instructions, against 37–40 M for the call. Eliding operations whose group closes
inside the call is MACRO-REPLAY revision 5's §11.5 and is not done here.

## 6. Storage, checkpoints and restores

- **Where the entries live.** In Rust, outside the word space (`src/boxmemo.rs`), per macro.
  - There are up to 64 variants per macro, least recently used dropped first, within a total cap
    of 64 MB.
  - Each variant carries a 64-bit hash of K1, the context, K2 and K4. The hash only filters
    candidates; the full comparison decides.
  - All variants whose K1–K4 match are tried, most recent first, until one also passes K3 and K7.
  - Why 64: a page of *Infinite Descent* measures a dozen frames, each with its own key, and a
    typed letter alternates between two states of each.
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

- **In the host,** the verifier's restore replaces the edit's pending branch, so a compile with a
  verified call does not converge. That is acceptable in a gate mode, where the output must still
  equal a full run's.
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
