# MACRO-REPLAY: guarded replay of macros with arguments

Sub-lane **P6-MACRO-REPLAY** of P6-HYPEROPT (Commander ruling, 2026-10-04). Status: **design, for
review before the implementation lands**. It extends DESIGN.md §5.6 item 4 (guarded intrinsics,
decision D9, as built in `src/intrinsics.rs`, `src/intrinsics_verify.rs`, `changes/intrinsics.ch`)
from macros *without* parameters to macros *with* them. DESIGN.md stays the source of truth;
where this document and DESIGN.md disagree, DESIGN.md wins and this document is wrong.

**The rule that governs everything below:** a replay must leave the engine in a state that no
later computation can tell from the state the real expansion leaves, except for memory addresses
and memory-usage statistics (which P-T1 normalises, DESIGN.md §1.1). When anything a guard cannot
vouch for is different, the macro is expanded for real. A replay is an optimisation, never a
second implementation of TeX.

## 1. Why (measured)

A beamer slide costs the engine about **420 M instructions** (the edited slide of the 118-slide
`long-deck`, `docs/evidence/p6-hyperopt-2026-10-04/`); pdfTeX spends about the same, so the
11 ms edit target (DESIGN.md §1.2) cannot be met by a faster interpreter alone. The macro profile
of the deck (`raw/macro-profile-long-deck.tsv`, inclusive time):

| what | share of the deck |
|---|---|
| the output routine (`\@outputpage`) | 59 % |
| its headline (`\@oddhead`) | 38 % |
| xcolor's `\colorlet` body, `\XC@col@rlet` | **47 %** |
| beamer's `\beamer@usebeamercolor` (calls `\colorlet` ~8 times) | 44 % |

`\XC@col@rlet` is called **69,743 times** in one pass of the deck, 593 times a slide, with only
**104 distinct argument lists**; the 50 most frequent cover 97.7 % of the calls (a traced pdflatex
run, `\tracingmacros=2`). Each call is a pure computation over a handful of macros (the colour
definitions `\\color@NAME`, `\ifglobalcolors`, the catcodes `\XC@edef` tests): the textbook case
for a memo keyed on the macro and its arguments.

The parameterless intrinsics cannot take it: `flashtex_intr_call` refuses any macro whose
parameter text is not empty (`Why::Parameters`).

## 2. The parameterless intrinsics, as built (D9)

The design extends this machinery; it does not replace it.

- **Where a call is intercepted.** At the start of `macro_call` for a *registered* macro that
  `big_switch`'s `get_x_token` is about to expand (`intr_at_switch`): the macro is the first thing
  `main_control` meets, so its whole effect is "the body has been executed by `main_control`".
- **Recording.** The normal expansion runs, observed by hooks (`changes/intrinsics.ch`):
  - every *read* of state: the meaning (`eq_type`, `equiv`) of every control sequence `get_next`
    delivers, `\csname`/`\ifcsname` look-ups, the internal quantities `scan_something_internal`
    fetches (integer and token parameters, codes), all 256 catcodes when a token list is printed,
    the value every assignment overwrites;
  - every *write*: `eq_define`, `geq_define`, `eq_word_define`, `geq_word_define`,
    `new_save_level`, `unsave` — the operations a replay makes again;
  - *purity*: only an allowlist of commands and expandable primitives; anything else (typesetting,
    boxes, glue, dimensions, fonts, files, marks, sparse registers, `\afterassignment`,
    `\aftergroup`, messages, errors, any output, a new control sequence, reading a token from
    outside the macro's own input levels, closing a group or conditional it did not open,
    unbalanced braces) abandons the recording.
  - It ends at `big_switch` once every input level from the body's on is used up, and commits
    only if the group level, the conditional stack, `align_state`, the string pool, the hash,
    errors, mode, nest, tail, scanner status and the log/terminal are as they were.
- **The guard (O(1)).** Each watched `eqtb` entry carries a watch record; `eq_define`,
  `geq_define` and `unsave` report writes to watched entries (`flashtex_intr_touch`), which keeps
  per recording a count of entries not holding the recorded value. The guard is: that count is
  zero; the recorded integer pairs still hold; mode, `align_state`, `par_token` as recorded; and
  the preconditions `\globaldefs=0`, no pending `\afterassignment`, no tracing that would show
  the expansion.
- **Pins.** Token lists the recording read or created are pinned (reference count raised), so an
  equal pointer is always the same, unchanged list.
- **Replay.** The recorded operations, through TeX's own `eq_define` & co.; a `\def`'s body is a
  fresh copy (`K_FRESH`), a `\let` takes the meaning from its source as it is now (`K_LETCS`).
- **Verification.** `FLASHTEX_INTRINSICS=verify` runs both paths for every call the guard admits
  and diffs the complete state change (`intrinsics_verify.rs`: every word either path wrote,
  `mem` through what reaches it); `verify-all` does it for every parameterless macro.
- **State.** Everything lives in the word space (`intr_state`, `intr_data`, `intr_watch`,
  `intr_cand`, `intr_seen`, `intr_pre`), so checkpoints carry it.

## 3. What changes for a macro with arguments

### 3.1 The call site: after the arguments are scanned

A macro with parameters reads its arguments from the input that *follows* it, which is not the
macro's own and differs from call to call. So the interception moves to the point where
`macro_call` has scanned every argument and is about to feed the body
(tex.web §390, `@<Feed the macro body and its parameters to the scanner@>`), after the loop that pops used-up
input levels ("conserve stack space"), immediately before `begin_token_list(ref_count,macro)`:

```
while (state=token_list)and(loc=null)and(token_type<>v_template) do
  end_token_list; {conserve stack space}
if intr_args_ok then if flashtex_intr_call_args(n) then goto exit;   {new}
begin_token_list(ref_count,macro); ...
```

Everything before it — the parameter scan with its delimiter matching, `\par` checks, runaway
errors, `\tracingmacros` output, and the level pops — runs unchanged on both paths, so it is
identical by construction. `intr_args_ok` is set exactly as `intr_at_switch` is (the macro is
expanded by `big_switch`'s `get_x_token`) and the macro is a registered candidate. `goto exit`
leaves through `macro_call`'s own exit (restoring `scanner_status` and `warning_index`).

The parameterless call site stays where it is, and leaves macros with parameters to the new one
(today it refuses them and disables the macro, `Why::Parameters`; that refusal becomes "not here").
A macro is offered at one site or the other, never both. `intr_at_switch` is still true at the
new site: the argument scan uses `get_token`, which never reaches `expand`, where the flag is
cleared.

### 3.2 The key: the macro's meaning and its argument token lists

A recording of a macro with `n` parameters stores, besides the D9 record, the **token lists of its
`n` arguments** (`pstack[0..n)`), token by token, by value. A call may be replayed only if its
arguments are equal to a recording's, token for token (a token is `cmd*256+chr` or
`cs_token_flag+p`: a control sequence is compared by its `eqtb` location, not its meaning).

The macro's own meaning is a watched entry, as in D9 (`rec_start` reads `cur_cs`), and the
recording pins the macro's body; so the parameter text and body are the recorded ones whenever
the guard passes.

Argument tokens are values: whatever the body later does with them (expands them, `\edef`s with
them, prints them) reads state through the ordinary recorded paths (`get_next` delivering a
control sequence from a parameter level reads its meaning, `rec_read`).

### 3.3 Variants: an argument-keyed index

`\XC@col@rlet` needs about a hundred recordings; D9 keeps four variants per macro in fixed
regions of 90 k integers each. The index becomes:

- **A hash of the argument tokens** (FNV-1a over `n` and the tokens, with a separator per
  argument) selects a bucket in a per-macro open-addressing table in `intr_data`; each entry names
  a slot. A hash match only *selects* a candidate: the argument tokens are always compared in full,
  so a collision costs time, never correctness.
- **Variable-size slot regions.** A slot's region is sized by what it recorded (watch records,
  integer pairs, pins, operations, argument tokens), bump-allocated from a slot heap in
  `intr_data`, with free-list reuse and a compaction when the heap is fragmented. Recordings of
  `\XC@col@rlet` measured in the prototype set the default sizes.
- **Budgets.** At most `MAX_VARIANTS_ARGS` (default 256) valid recordings per macro, evicting the
  least recently replayed; a recording budget per macro (`RECORD_BUDGET`) and an abandon budget
  (`ABORT_BUDGET`, retrying only for `NewCs` and `Checkpoint`, as in D9). Every decision depends
  only on the engine state and call counts, never on time, so a run and its re-run make the same
  decisions (§7.3).

### 3.4 What a recording records, in addition to D9

| input | how it is captured |
|---|---|
| the argument token lists | the key (§3.2), compared token by token |
| the macro's parameter text and body | the macro's meaning is watched and its body pinned (D9) |
| meanings of control sequences in the arguments | read when the body delivers them (`get_next` from a parameter level: D9's `rec_read`), or *weakly* when it only passes them on (`intr_weak`: only "not `\outer`, not `#`") |
| the parameter stack | the body reads only its own parameters (`param_start ..`); a token list read from outside the macro's input levels abandons the recording (D9's `Level`) |

Nothing else is new: the arguments arrive as input levels at or above the body's base
(`begin_token_list(param_stack[...], parameter)` in `get_next`), which the D9 recording already
treats as the macro's own.

### 3.5 What the guard checks, in addition to D9

1. The macro is called with the same number of arguments (implied by the watched meaning).
2. Each argument's tokens equal the recording's.
3. Everything D9's guard checks (§2), unchanged: zero mismatched watched entries, the integer
   pairs, mode, `align_state`, `par_token`, `\globaldefs=0`, no `\afterassignment`, no tracing.

### 3.6 What a replay does, in addition to D9

1. The same `max_param_stack` update the normal path makes (`param_ptr+n`), so even the
   end-of-run capacity statistics agree.
2. The recorded operations (D9's replay).
3. **Flush the argument lists** `pstack[0..n)`, which the normal path flushes when the macro's
   input level ends (`end_token_list`: `flush_list(param_stack[p])` for its parameters).
4. Report the recording's reads to the L5 read-set (D9's `intr_report_reads`), plus the control
   sequences of the argument tokens that the recording read.

## 4. The inputs a replay depends on, by kind

The Commander's list, and every other kind of state, with how each is captured or excluded:

| state | captured how | if it differs |
|---|---|---|
| catcodes | read by `\ifnum\catcode` and friends (`scan_char_num` hook: `flashtex_intr_read(m+cur_val)`); all 256 when a token list is printed (`\string`, `\meaning`, `\detokenize`, `\pdfstrcmp`, ...: `rec_catcodes`); the catcodes of the argument tokens themselves are part of the tokens | watched: guard fails |
| meanings of tokens it expands or executes | every control sequence `get_next` delivers (`rec_read`), `\csname` and `\ifcsname` look-ups; `\let` sources (`K_LETCS`) | watched: guard fails |
| bodies of macros it expands | pinned by the recording (same pointer ⇒ same unchanged list) | the pointer differs ⇒ the watched meaning differs |
| integer parameters, `\count` registers | value pairs (`F_NRW`) | guard fails |
| dimensions, glue, `\dimen`/`\skip` | not captured: scanning a dimension abandons the recording | expanded for real |
| token registers (`\toks`) | meaning watched (`ASSIGN_TOKS`, `TOKS_REGISTER` reads); copying one register into another abandons | guard fails / expanded |
| box registers, glue/shape parameters, font identifiers | outside the recordable region (`rec_region_ok`) | expanded |
| the current font (`eqtb[cur_font_loc]`) | never read: `\the\font` reaches `scan_something_internal` with a font command, which abandons (`Internal`); selecting a font (`\font`, a font identifier) is not on the command allowlist | expanded |
| colour state | pdfTeX's colour stacks are C state reached only by `\pdfcolorstack` (an extension command: abandons) and `\color`'s whatsits (typesetting: abandons); xcolor's `\current@color` and `\\color@NAME` are macros (watched) | expanded / guard fails |
| mode, nest, `align_state`, `par_token` | recorded; guarded | guard fails |
| `\globaldefs`, `\afterassignment`, tracing | preconditions | expanded |
| the input stack outside the macro's levels | never read (lookahead past the body abandons: `Level`) | — |
| conditionals, groups | must balance within the body (`Cond`, `Group`) | — |
| the string pool, the hash (new control sequences) | a recording that makes one is abandoned (`NewCs`; retried once the names exist) | — |
| files, `\write`, `\message`, errors, the log | not on the allowlist; the log and terminal offsets are compared at commit | — |
| `\pdfelapsedtime`, `\pdfuniformdeviate`, `\jobname`, `\inputlineno` | not on the allowlist (`LAST_ITEM`, `CONVERT` codes outside it) | — |
| memory addresses, `dyn_used` & co. | differ (fresh copies); no TeX computation reads an address | statistics only (DESIGN.md §1.1) |

## 5. Why a replay equals the expansion

**Setting.** Let `S` be the engine state at the new call site of a call `c` (after the argument
scan and the level pops), and `T(S)` the state the normal path reaches at the next `big_switch`
once the body's input levels are used up. TeX is deterministic: `T(S)` is a function of the parts
of `S` the normal path reads.

**Claim.** If the guard admits recording `R` for `c`, the replay of `R` from `S` reaches a state
`S'` equal to `T(S)` except for memory addresses (which nodes hold the token lists) and the memory
statistics.

**Proof sketch.**
1. *Same inputs.* The recording run from `S_R` read: (a) the argument tokens — equal by the key;
   (b) `eqtb` entries — every read is watched (or a value pair), and the guard's mismatch count is
   zero, so each holds the value read; (c) token lists through `equiv` pointers — pinned, so an
   equal pointer is an unchanged list; (d) mode, `align_state`, `par_token`, the preconditions —
   checked. The allowlist guarantees that the run read no other state: every command and
   expandable primitive outside it abandons the recording, and those inside read only (a)-(d)
   (the per-primitive table of `flashtex_intr_expand`, `flashtex_intr_internal`,
   `flashtex_intr_command`). The run reads no input beyond its own levels (`Level`).
2. *Same path.* By determinism, the normal path from `S` takes the same sequence of commands and
   expansions as the recording run from `S_R` (induction over the steps of `main_control` and
   `expand`: each step's choice depends only on inputs equal by 1).
3. *Same writes.* Every write the path makes to `eqtb` and the save stack goes through
   `eq_define`, `geq_define`, `eq_word_define`, `geq_word_define`, `new_save_level`, `unsave` —
   recorded in order, with values equal by 2 (a `\def`'s body is a list with the same tokens, made
   fresh: `K_FRESH`; a `\let` copies the source's meaning, which is the same by 1: `K_LETCS`). A
   replay calls the same routines in the same order with the same arguments, so it makes the same
   changes, including what `eq_define` saves on the save stack and destroys.
4. *No other writes.* The commit conditions guarantee that the path left the conditional stack,
   group level, `align_state`, string pool, hash, errors, mode, nest, tail, scanner status, log
   and terminal as it found them; the allowlist excludes every other writer (typesetting, files,
   marks, sparse arrays, the box and font registers).
5. *The input stack.* The normal path ends with the body's levels and its parameter levels popped
   and the argument lists flushed; the replay never pushes them and flushes the lists itself
   (§3.6). Above that point the input stack is untouched by both. `max_param_stack` is updated as
   the normal path updates it.
6. *Memory.* The two paths allocate and free nodes differently (fresh copies, no body levels), so
   `avail`, `rover`, `dyn_used` and node addresses differ; no TeX computation's result depends on
   an address, and the statistics are normalised (DESIGN.md §1.1). This is the same argument D9
   rests on, measured by its verifier (53,401 calls, 0 differences, DESIGN.md §5.6).

The verifier (§6.1) checks this claim on every admitted call in CI, which is what makes the proof
sketch more than a sketch.

## 6. Verification and gates

### 6.1 Both paths, diffed

`FLASHTEX_INTRINSICS=verify` extends to macros with arguments unchanged in kind: at a call the
guard admits, take a checkpoint, run the normal path under a verification recording, capture its
state `N`, restore, replay, capture `I`, compare (`intrinsics_verify.rs`'s rules: every word
either path wrote; `mem` through what reaches it; the input and parameter stacks by their tokens).
New for this extension: the argument lists are freed in both, so the comparison of the
parameter stack and of `mem` reachable from it must find nothing left of them in either.

### 6.2 The differential test (prototype gate)

- **Random beamer decks.** A generator of decks over the TeX Live beamer themes (inner, outer,
  colour themes, `\setbeamercolor` with `parent=`/`use=`, `\usebeamercolor`, blocks, overlays,
  `\colorlet` with modifiers `!`, `:`, `>` and models `[rgb]`, `[named]`), seeded; each deck
  compiled with `FLASHTEX_INTRINSICS=verify` and `FLASHTEX_INTRINSICS_VERIFY_FAIL=1`: **0
  differences** required.
- **Output identity.** The same decks, the beamer tier (34 decks) and the parity fixtures with
  replay on and off: PDF and untraced log **byte-identical**.
- **Fault injection.** The existing `FLASHTEX_INTRINSICS_FAULT` faults, plus two new ones (skip the
  argument flush; accept a call whose arguments differ in their last token), must each be caught.

### 6.3 Why lockstep and P-T1 alone are not the gate

P-T1 and lockstep compare logs under `\tracingall`, and the guard refuses every replay while any
tracing is on (a traced expansion prints what a replay would not). So they cannot see replays.
They stay required (the change must not move a traced run), and the gates above carry the
replays: the verifier's state diff, untraced output identity, P-T2.

### 6.4 Incremental soundness

The soundness sweeps A, C, D and the beamer sweep of `docs/evidence/beamer-v3-2026-10-03/`, with
replay on, compared byte for byte with scratch runs (which replay too, independently); and a
sweep where scratch runs have replay off. A difference between the last two is a replay bug the
incremental machinery did not cause.

## 7. Checkpoints, restores and convergence

### 7.1 Checkpoints and restores

All replay state is in the word space (D9's arrays, the slot heap and the argument index in
`intr_data`), so a checkpoint captures it and a restore brings back the recordings, watch counts
and pins that were valid in the restored state. A checkpoint is taken only at `big_switch`; a
replay is atomic within one command, and a recording in progress at a checkpoint is abandoned
(`Why::Checkpoint`, retried).

### 7.2 Convergence

Two runs that reach the same document state may hold different caches (one recorded a variant the
other did not, or evicted another). D9's structural comparison walks the recordings and compares
them (`Iso::intrinsics`), so with argument-keyed variants, caches would block convergence often.
DESIGN.md §5.3's adopted rule (2026-09-30, §13 (c)) already says "the intrinsics' caches are
treated as derived state". This design implements it:

- the convergence test does not compare `intr_*` (beyond what it checks today for a recording
  in progress, which must be none);
- **pins are discounted**: each state's pin count per token list (from its own `live_words`) is
  subtracted from the reference count before reference counts are compared;
- the convergence jump adopts the old run's whole state, caches included. Those caches are
  consistent with the old state (their watch counts and pins were maintained in it), and the old
  state equals the new one in everything else; so after the jump every guard answers as it
  would have in the old run, which is sound by §5.

A soundness case ships with it (DESIGN.md §13 R5): a document where the new run records a variant
the old run did not have, converges, and then calls the macro again with those arguments; every
compile equals scratch runs, and the case fails (does not converge) without the rule.

### 7.3 Determinism

Recording, eviction and the decision to replay depend only on the engine state and per-macro call
counts, never on time or the host. A cold run and its scratch twin make the same decisions; an
incremental run may make different ones (its cache came from another history), which §5 makes
invisible in the output.

## 8. Registration

The prototype registers `\XC@col@rlet` by name (`DEFAULT_NAMES`, overridable as today with
`FLASHTEX_INTRINSIC_NAMES`). Later candidates, by the same profile: `\beamer@usebeamercolor`
(5,178 calls; it ends in `\color`, a whatsit, so only its colour computation can be a replay:
measure whether its inner calls cover it), `\XC@definecolor`, `\pgfmathparse`'s inner loop
(`\pgfmath@parse@next`, 156,773 calls). Automatic registration from a profile is out of scope
until the verifier has run over T4.

## 9. Prototype plan

1. Call site and key: `changes/intrinsics.ch` gains the hook of §3.1; `src/intrinsics.rs` gains
   `flashtex_intr_call_args`, the argument store and compare, the flush.
2. Slot heap and argument index (§3.3); `live_words` and the convergence walk learn the new
   layout.
3. Verifier support (§6.1) and the two new faults.
4. The differential test (§6.2) as `tests/intrinsics.rs` cases plus a deck generator under
   `tools/`.
5. Measure on `long-deck`: instructions per slide, replays per slide, hit rate, abandon reasons;
   then the gates of §6.

**Expected gain (belief, to be measured):** `\XC@col@rlet` is 47 % of the deck inclusive. If 95 %
of its calls replay at a tenth of their cost, a slide drops by roughly 40 %, to about 250 M
instructions. Not the 11 ms target by itself; `\beamer@usebeamercolor` and the templates' boxes
are the next layer.

## 10. Risks and open questions

- **Recordability of `\XC@col@rlet`.** Its body ends `\expandafter\endgroup\@@tmp\xglobal@stop`,
  and `\@@tmp` may be `\XC@definecolor[...]...` (a further macro with arguments) or a `\let` of
  `\\color@NAME` built with `\csname`. First recordings make new control sequences (`NewCs`,
  retried). The prototype's abandon statistics will say which variants record.
- **Variant explosion** for macros with free-form arguments (`\beamer@usebeamercolor{...}` with
  user colour names): the budgets bound the memory; the hit rate decides whether a macro stays
  registered.
- **Pin-adjusted reference counts** in the convergence walk are a new rule in `iso.rs`; it needs
  its own proof note (a pin is the only reference the cache adds) and test.
- **`\aftergroup` in colour code** (`\color` uses `\aftergroup\reset@color`): out of scope here
  (`\color` typesets); listed so nobody adds `\aftergroup` to the allowlist without a design.
