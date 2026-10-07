# BOX-MEMO: guarded replay of macro calls that typeset only what they throw away

Lane **P6-INFDESC-PAGE** (Commander, 2026-10-06: "attributes Infinite Descent's 800 M-instruction
page and builds picture/box memoisation"). Status: **design, revision 1, for review.** The prototype
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
1. `big_switch`'s `get_x_token` expands it (D9's `intr_at_switch`), so `main_control` executes all of
   its body;
2. it is balanced: groups, conditionals, `align_state` and the nest are at the end what they were at
   the start;
3. **no node it makes reaches a list outside it**: every list it builds is inside a box it discards
   before the end;
4. its only lasting effects are assignments: global ones to `eqtb` entries and sparse registers, and
   the scalars of §4.3.

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
if bm_on then if intr_at_switch then if bm_cand[warning_index]<>0 then flashtex_bm_call(n);
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
| K6 | the context | `\globaldefs = 0`, no pending `\afterassignment`, the mode, and `interaction`. Tracing parameters are in K2, and a recording that printed anything is abandoned (§4.1) |

**Box registers** are not in the key. A recording that **reads** a box register it did not first
write is abandoned. Reads are `fetch_box` (`\box`, `\copy`, `\unhbox`, `\vsplit`, `\wd`, `\ifvoid`,
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

### 4.3 The effects

The **effects** are what the call leaves behind, taken at the end:
1. **`eqtb` entries assigned at any level.** From the recording's define hooks (`eq_define`,
   `geq_define`, `eq_word_define`, `geq_word_define`). Kept: those whose `(eq_type, equiv,
   eq_level)` (or value and `xeq_level`) now differs from the start.
   - By §4.1 every such change is global (`level_one`).
   - A new token list is stored by its tokens.
   - A pointer to a list or glue spec that existed at the start is stored as *the value of the K2
     or K3 entry that held that pointer at the start*. This is how a `\global\let` or a
     `\skip`-to-`\skip` assignment shares structure.
   - Two effects that share one new list share it in the replay, with the same reference count.
   - Sharing matters: the convergence test (`iso.rs`) pairs nodes one to one, and the verifier
     compares reference counts.
2. **Sparse registers** whose `(value, level)` changed, by (type, number). Values are kept as in 1.
3. **Scalars that persist and are read later.** The prototype starts with `last_badness`, written by
   every `hpack`/`vpack`. A recording that reads `\badness` is refused, so its value at the start
   is never an input. The verifier (§7) adds any other scalar it finds.

## 5. Replay

1. `end_token_list` pops the body (and its parameters).
2. With `rs_on`, the L5 read-set is told about every K3 control sequence and every assigned one, as
   D9's `intr_report_reads` does.
3. The effects are made in recorded order, through `geq_define`/`geq_word_define` and e-TeX's
   `gsa_def`/`gsa_w_def`. That gives the `eq_destroy` of the old values, D9's watch reports
   (`flashtex_intr_touch`) and `\tracingassigns`. Tracing is excluded anyway: a recording made
   with tracing on printed, so it never committed.
4. The recorded scalars are set.

## 6. Storage, checkpoints and restores

- **Where the entries live.** In Rust, outside the word space (`src/boxmemo.rs`), keyed by the
  control sequence and its argument tokens, with at most 8 variants per key (least recently used).
  The total is capped at 64 MB.
- **Why outside the word space.** An edit's restore rolls the word space back to before the edited
  window, which would delete every recording made in it. Out here they survive, and they stay sound
  because the guard checks every value in full.
- **Values, not addresses.** An entry holds no address into `mem`: token lists, glue specs and
  arguments are kept by value.
- **Restores.** Recording state is per call. A restore while a recording is open (a preempted
  compile, the convergence test's own restores) abandons it: `incr.rs` and `checkpoint.rs` call
  `boxmemo::on_restore`.
- **`bm_rec_on` and the version words** are in the word space. Every convergence test happens at a
  page boundary, where no recording is open, so `bm_rec_on` is equal there. The version words are
  state like any other.
- **Checkpoints inside a recording are fine.** The recording is taken on the normal path, so a
  segment or timed checkpoint in the middle of a 10 ms recording is an ordinary checkpoint. A later
  restore to it finds `bm_rec_on` true and the Rust recorder gone, and `on_restore` clears the flag.

## 7. Verification

`FLASHTEX_BOXMEMO=verify` runs both paths for every call the guard admits, the way
`intrinsics_verify.rs` does:
1. checkpoint C;
2. run the normal path, observed by a verifying recording (any refusal is a difference);
3. capture N;
4. restore C and replay;
5. compare the replay's state I with N, over every word either path wrote: `mem` through what
   reaches it, token lists by tokens and reference count, glue specs by fields, the save stack
   entry by entry, and scalars except the verifier's scratch list.

`FLASHTEX_BOXMEMO_VERIFY_FAIL=1` makes any difference exit 3.

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

## 9. Expected gain (to be measured by the prototype)

Typing at a place where a framed environment's frame is measured:
- the windows lose their `\fb@sizeofframe` share: **−14 % to −44 %** (evidence table);
- the first keystroke there records and pays, and every later one replays;
- the cost of a replay is the key check (about 5,000 `eqtb` words, the sparse trees and a few
  thousand control sequences) plus the effects (about 600 assignments): estimated below 0.5 M
  instructions, against 37–40 M.

**A cold pass gains nothing yet.** K2 and K4 include pgf's serial, so every call misses. Phase 2
(below) is where cold compiles gain.

## 10. Later phases (not in this revision)

- **Phase 2, the serial.** A recorded effect may be "a register advanced by one" when the recording
  read it only to advance it and to print it into one assignment. That makes cold-pass hits possible
  (−40 % of a cold pass, by the profile). Proof obligations come first.
- **Phase 3, draw calls.** Calls that append one box to the outer list: frame draws, `tikzcd`. Then
  the box registers they read (`\box\@tempboxa`) become a hole, keyed by the box's dimensions and
  spliced into the recorded box.
- **Automatic candidates.** Calls the macro profiler shows above a cost threshold, instead of names.
