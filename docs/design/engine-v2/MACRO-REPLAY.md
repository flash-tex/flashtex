# MACRO-REPLAY: guarded replay of macros with arguments

Sub-lane **P6-MACRO-REPLAY** of P6-HYPEROPT (Commander ruling, 2026-10-04). Status: **design,
revision 4 approved** (#1509), prototype in #1524-#1526; **revision 5 proposed** in §11 (dimensions,
sparse registers, `\afterassignment`, contexts, local-group elision), for review.

**Dependency (process).** Once this design is approved, the Commander amends DESIGN.md §5.6 item 4
(D9: "only pure, non-erroring leaf functions" extended to macros with arguments) and adds the §13
row. No implementation lands before that amendment. It extends DESIGN.md §5.6 item 4 (guarded intrinsics,
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

**Which macro is being called.** At this point `cur_cs`, `cur_chr`, `cur_cmd` and `cur_tok` hold
the *last argument token*, not the macro: `get_token` overwrote them during the scan. The macro is
`warning_index` (set to `cur_cs` on entry to `macro_call`), its body `ref_count` (a local of
`macro_call`), its argument count `n`. The hook therefore passes them explicitly,
`flashtex_intr_call_args(warning_index, ref_count, n)`, and nothing in the call, the guard, the
recording's start (`rec_start` today reads `cur_cs` for the macro's own meaning) or the replay may
read `cur_cs` or `cur_chr` for the macro. A fault (§6.5) that watches `cur_cs` instead of
`warning_index` must be caught by the gates.

**The scanner status.** At the new call site `scanner_status` is `matching` (set for the argument
scan, tex.web §391); `macro_call` restores the caller's value only at `exit`, so the body runs with
`save_scanner_status`. The hook passes `save_scanner_status` into `rec_start`, which records it as
the recording's starting status (the commit compares the status at the end with it, D9's
`Why::State`), and the replay restores it through `macro_call`'s exit as the normal path does.
Recording `matching` instead would make every recording fail its commit.

**The S₀ arming hook.** `changes/checkpoint.ch` arms the begin-document snapshot in `macro_call`
right after `begin_token_list`: `if warning_index=ckpt_arm_cs then ckpt_arm_level:=input_ptr`. A
replay never runs that line, so a replayed `\document` would never take S₀. The call is refused
(expanded for real) when `warning_index=ckpt_arm_cs`. And a recording commits only if
`ckpt_arm_cs`, `ckpt_arm_level` and `ckpt_request` are as they were at its start, which also
rejects a body that arms or triggers a checkpoint inside itself (a nested `\document`, or a body
whose levels include the armed one). **D9 has the same hole** (its call site is
before `begin_token_list` too, with the macro in `cur_cs`): the prototype fixes it there as well
(refuse when `cur_cs=ckpt_arm_cs`).

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

The macro's own meaning is a watched entry, as in D9 (here `warning_index`'s, §3.1), and the
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
- **Eviction is history-dependent.** Which recordings exist at a call depends on the whole history
  of the run (what was called before, in which order, and after an incremental restart, on the
  earlier run's history too). Correctness never rests on which recordings exist: it rests on §5
  (any recording the guard admits replays to the state the expansion reaches). That is why the
  verifier must also run on incremental runs (§6.4), whose caches have histories a cold run never
  has.

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

1. The macro (`warning_index`) is the recording's, called with the same number of arguments
   (implied by the watched meaning).
2. Each argument's tokens equal the recording's.
3. `warning_index` is not the S₀ arming control sequence (`ckpt_arm_cs`, §3.1).
4. **Capacity.** The recording keeps the peak excess over its starting values of every bounded
   resource the normal path uses and the replay does not, and a call is refused when its current
   value plus the recorded peak excess would reach the limit, so a replay never hides an overflow
   the expansion would hit (a fatal `overflow` prints the input stack, which differs on a replay,
   so even its log would differ). The margin depends on the call depth, which is why it is checked
   per call, not recorded. The resources:
   - input levels (`input_ptr` against `stack_size`);
   - parameter-stack entries (`param_ptr` against `param_size`);
   - expansion depth (`expand_depth_count` against `expand_depth`);
   - the buffer (`\csname` builds names in `buffer`: `first`/`last` against `buf_size`);
   - the string pool's temporary use (`str_toks` for `\string`, `\meaning`: `pool_ptr` against
     `pool_size`);
   - the save stack (`save_ptr` against `save_size`: the body's groups and `eq_save`s);
   - main memory: one-word nodes (`dyn_used`, taken from `hi_mem_min` downwards and the `avail`
     list) **and** variable-size nodes (`var_used`, `get_node`: the condition stack's `\if`
     nodes, e-TeX's `\numexpr` expression nodes), taken from the free ring and, when it is
     exhausted, by growing `lo_mem_max` into the gap. The test is conservative: the call is
     admitted only if `peak_dyn + peak_var` fits in the gap `hi_mem_min - lo_mem_max` with room
     for `lo_mem_max`'s growth steps (tex.web §125: at most 1000 words or half the gap each),
     not counting the free lists. So it may refuse a call that would have fit, never admit one
     that would not.

   **How the peaks are observed.** At `rec_start` the recording saves and resets the high-water
   marks the engine already keeps (`max_in_stack`, `max_param_stack`, `max_buf_stack`, and
   `max_save_stack`) to the current values; at commit it reads them (the peak during the body),
   stores the excess, and merges them back (each mark becomes the maximum of its saved and new
   value, which is what it would have been). For the resources without a high-water mark it adds
   increment-site hooks behind `intr_rec_on`: `expand_depth_count` (where `expand` increments
   it), `pool_ptr` (`str_toks` and the other places that append to the pool), `dyn_used`
   (`get_avail` and the fast `fast_get_avail`) and `var_used` (`get_node`), each keeping the
   maximum seen.
5. Everything D9's guard checks (§2), unchanged: zero mismatched watched entries, the integer
   pairs, mode, `align_state`, `par_token`, `\globaldefs=0`, no `\afterassignment`, no tracing.

### 3.6 What a replay does, in addition to D9

1. Raise the high-water marks as the normal path would have: `max_param_stack`, `max_in_stack`,
   `max_buf_stack` (and `pool_ptr`'s peak, which no statistic keeps) to the current value plus
   the recorded peak excess (§3.5, 4). With that, the end-of-run capacity statistics agree except
   the memory-usage lines, which depend on addresses (DESIGN.md §1.1 normalises them); without
   it they would differ, which §1.1 also normalises, but the design does not lean on that.
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
| integer parameters, and integers named by `\countdef` and `\chardef` | value pairs (`F_NRW`), read through `scan_something_internal` (`ASSIGN_INT`, `CHAR_GIVEN`); `\advance`, `\multiply`, `\divide` read their register as a pair | guard fails |
| `\count`*number*, `\dimen`*number* read by number | *reading* one (`REGISTER` in `scan_something_internal`) abandons the recording; *assigning* one (`\count`*n*`=`..., the `REGISTER` command in `prefixed_command`) is allowed and recorded as an operation (`K_WORD`), and `\advance` & co. read the old value as a pair | expanded / recorded |
| `\escapechar` | read at the start of every recording (`rec_start`: printing a control sequence uses it) | guard fails |
| `\newlinechar` | irrelevant: it affects only what is printed to a file or the terminal, which abandons | — |
| `\endlinechar`, `\scantokens` | `\scantokens` (and so `\endlinechar`'s use) is not on the allowlist | expanded |
| `\uppercase`, `\lowercase` (and `\uccode`/`\lccode`) | not on the command allowlist | expanded |
| `\currentgrouplevel`, `\currentiflevel`, `\lastnodetype`, `\inputlineno` | `LAST_ITEM` codes outside the allowlist | expanded |
| `\ifeof`, `\ifvmode`, `\ifinner` (and `\ifhmode`, `\ifmmode`, `\ifvoid`, `\ifhbox`, `\ifvbox`, `\iffontchar`) | `IF_TEST` codes outside the allowlist | expanded |
| `\jobname`, `\fontname`, `\pdfuniformdeviate`, `\pdfnormaldeviate` | `CONVERT` codes outside the allowlist | expanded |
| `eq_level` of entries, `cur_level` | never compared: re-read live by `eq_define` and friends during the replay, which therefore save and restore exactly as the normal path would at the current levels | — |
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
| `\pdfelapsedtime` | a `LAST_ITEM` code outside the allowlist | expanded |
| memory addresses | differ (fresh copies); no TeX computation reads an address. This includes *when* a later main-memory overflow happens: the two paths leave different free lists, so a document that later runs out of main memory may do so at a different point. D9 accepts the same (its replays allocate fresh copies too); §3.5 item 4 only guarantees that the call itself cannot hide an overflow | — |
| `dyn_used`, `var_used` | the *net* change is equal on both paths (the leak check, §6.1); the peak is guarded (§3.5, 4) | — |
| capacity limits and high-water marks | §3.5 item 4 and §3.6 item 1 | refused |

## 5. Why a replay equals the expansion

**Setting.** Let `S` be the engine state at the new call site of a call `c` (after the argument
scan and the level pops), and `T(S)` the state the normal path reaches at the next `big_switch`
once the body's input levels are used up. TeX is deterministic: `T(S)` is a function of the parts
of `S` the normal path reads.

**Claim.** If the guard admits recording `R` for `c`, the replay of `R` from `S` reaches a state
`S'` equal to `T(S)` except for memory addresses (which nodes hold the token lists) and the memory
statistics.

**Proof sketch.**
1. *Same inputs.* The recording run from `S_R` read:
   (a) **the macro's identity** — `warning_index` and the body `ref_count`, passed explicitly
   (§3.1); the macro's meaning is watched and its body pinned, so the same `warning_index` with a
   zero mismatch count means the same parameter text and body;
   (b) the argument tokens — equal by the key;
   (c) `eqtb` entries — by the **first-access rule**: the first access of each entry during the
   run decides. If it was a read, the entry's value is a dependency (watched, or an integer
   pair) and the guard's zero mismatch count says it holds that value now. If it was a write, the
   run's later reads of it see what the run itself wrote (the same in a replay's state by 3),
   unless an `\endgroup` restored the old value, which `rec_read` then treats as a read of the
   pre-write value (`intr_pre`) and records as a dependency;
   (d) `\escapechar` — read and watched at every recording's start;
   (e) token lists through `equiv` pointers — pinned, so an equal pointer is an unchanged list;
   (f) mode, `align_state`, `par_token`, the preconditions, the capacity margins — checked;
   (g) `eq_level` of entries and `cur_level` — not inputs of the *choices* the run makes (no
   allowlisted primitive reads them: `\currentgrouplevel` abandons), only of how `eq_define` and
   `unsave` save and restore, which the replay re-reads live (3).
   The allowlist guarantees that the run read no other state: every command and expandable
   primitive outside it abandons the recording, and those inside read only (a)-(g) (the
   per-primitive tables of `flashtex_intr_expand`, `flashtex_intr_internal`,
   `flashtex_intr_command`, §4). The run reads no input beyond its own levels (`Level`).
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
5. *The input stack.* **Lemma.** At the `big_switch` where the normal path's recording ends, the
   body's level and its parameter levels may still be on the input stack, used up
   (`rec_exhausted`: token lists with `loc=null`); TeX pops them lazily. The next `get_next`
   pops every used-up token-list level (`end_token_list`, flushing the parameters of a macro
   level) before it reads a token, and `back_input` pops them before it pushes one
   (`while (state=token_list)and(loc=null)and(token_type<>v_template) do end_token_list`);
   nothing between `big_switch` and that `get_next` (the checkpoint hook, which only reads)
   looks at them. So the normal path's state, with those levels popped and their parameter lists
   flushed, is observationally the same as the state at `big_switch`; the replay produces the
   former directly (it never pushes the levels, and flushes `pstack[0..n)`, §3.6). Above that
   point the input stack is untouched by both. The high-water marks are raised as the normal
   path raises them (§3.6, 1).
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

**The leak check.** The verifier excludes `avail`, `dyn_used`, `var_used` (allocation state), so a
replay that forgot to free the argument lists would leave nothing in the reachable comparison and
pass. So it also compares the **net change of `dyn_used` and `var_used`**: after the normal path
it first pops its used-up levels (the lemma of §5, 5: what the next `get_next` would do), then
takes `dyn_used - dyn_used(C)`, and requires the replay's delta to be equal. A missing flush (or
any leak) changes the delta. The same pop-normalisation applies to D9's verifier (a
parameterless body's used-up level is popped lazily too). The verification recording takes no
pins and adds no watch records (`S_REC_VERIFY`: it only observes the normal path), so it leaves no
reference counts of its own behind.

### 6.2 The differential test (prototype gate)

- **Random beamer decks.** A generator of decks over the TeX Live beamer themes (inner, outer,
  colour themes, `\setbeamercolor` with `parent=`/`use=`, `\usebeamercolor`, blocks, overlays,
  `\colorlet` with modifiers `!`, `:`, `>` and models `[rgb]`, `[named]`), seeded; each deck
  compiled with `FLASHTEX_INTRINSICS=verify` and `FLASHTEX_INTRINSICS_VERIFY_FAIL=1`: **0
  differences** required.
- **Output identity.** The same decks, the beamer tier (34 decks) and the parity fixtures with
  replay on and off: PDF and untraced log **byte-identical**.
- **Fault injection:** §6.5.

### 6.3 Why lockstep and P-T1 alone are not the gate

P-T1 and lockstep compare logs under `\tracingall`, and the guard refuses every replay while any
tracing is on (a traced expansion prints what a replay would not). So they cannot see replays.
They stay required (the change must not move a traced run), and the gates above carry the
replays: the verifier's state diff, untraced output identity, P-T2.

### 6.4 The gates, in full

| gate | what | required |
|---|---|---|
| (a) stress | `verify-all-args`: every macro with arguments that `big_switch` expands is recorded and, when its guard admits a later call, both paths are run and diffed (the arguments' analogue of D9's `verify-all`), over trip and etrip, the parity fixtures and the T2/T3 suites, with `FLASHTEX_INTRINSICS_VERIFY_FAIL=1` | 0 differences |
| (b) incremental | soundness sweeps A, C, D and the beamer sweep (`docs/evidence/beamer-v3-2026-10-03/`) with `FLASHTEX_INTRINSICS=verify` in the host (so every replay in an incremental run, whose cache has a history no cold run has, is diffed), compared byte for byte with scratch runs; and the same with scratch runs at replay off | 0 differences, 0 mismatches |
| (c) output identity | nightly: replay on against replay off over the corpus (fixtures, beamer tier, arXiv sample, T4 when it runs), PDF and untraced log | byte-identical |
| (d) leak check | §6.1, in every verified call | equal deltas |
| (e) faults | §6.5, each caught by (a) or (b) | every fault caught |
| (f) statistics | hit rate, replays, abandons by reason, refusals by reason (`FLASHTEX_INTRINSICS_STATS`), published with every measurement | published |
| (g) traced gates | lockstep, P-T1, trip, etrip (§6.3) | unchanged |

### 6.5 Faults that the gates must catch

`FLASHTEX_INTRINSICS_FAULT` gains, besides D9's (`drop-last`, `drop-first-let`, `no-ref`,
`local`, `no-readset`):
- `no-flush`: the replay skips the argument flush (caught by the leak check);
- `args-last`: the guard ignores the last token of the last argument (caught by the state diff);
- `const-hash`: every argument list hashes to one bucket (must cost time only: no difference);
- `no-align`: the guard ignores `align_state` (caught on alignments in the corpus);
- `cur-cs`: the call watches `cur_cs` (the last argument token) instead of `warning_index` (§3.1);
- `no-arm`: the call does not refuse `ckpt_arm_cs`. In a normal run this cannot happen: the arm
  is set at the run's first `big_switch` and consumed by the arming control sequence's first
  expansion, while a replay needs an earlier, recorded expansion of the same control sequence. So
  the refusal (and the commit condition of §3.1) is defence in depth, and the fixture makes the
  case happen with a test-only hook that re-arms (`REQ_LOOKUP`, as at the run's start) right
  after a recording of the arming macro commits. The hook is compiled only under `cfg(test)` or
  an off-by-default cargo feature used by the test build, never as a run-time switch of a
  production build. The fixture's preamble calls the registered macro, which has an argument,
  three times: the first call consumes the start-of-run arm (the arming name exists at the
  first `big_switch`, as `\document` does under LaTeX), so its recording fails §3.1's commit
  condition; the second records and commits, and the hook re-arms; the third would replay while
  armed. The test requires the snapshot to be taken at the third call; under `no-arm` it is not
  (the session's next compile is cold).

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

The rule rests on two invariants, stated so they can be checked:

- **(a) Consistency.** In every state the engine runs from, each recording's mismatch count equals
  the number of its watch records whose `eqtb` entry does not hold the watched value, and each
  pinned list's reference count includes exactly one reference per pin.
- **(b) Whole-state adoption.** A restore and a convergence jump adopt *one* run's *entire* word
  space — `eqtb`, `mem` and `intr_*` together, watch counts and pins included — never a cache
  from one run spliced with the document state of another. (The jump's splicing of file outputs
  and `pdf_char_used` touches no `intr_*` word and no reference count.)

(a) holds in a state the engine reached by running, and (b) makes it hold after a restore or a
jump. It is checked in every build, at run time:
- after every restore and every convergence jump, in every run, the engine recomputes every
  recording's mismatch count from `eqtb` and checks, for every pinned list, that its reference
  count is **at least** the number of pins the recordings hold on it (O(watch records + pins),
  well under a millisecond at the budgets of §3.3); this cheap check cannot see a reference count
  that is too high;
- in the verify mode (`FLASHTEX_INTRINSICS=verify`, a run-time switch, not
  `cfg(debug_assertions)`) it is the exact check instead: every reference to every pinned list is
  counted by a walk of `mem` from the roots (`Iso::walk_one`'s reachability) plus the pins from
  `live_words`, and each reference count must equal that sum; a discrepancy is a failure
  (`FLASHTEX_INTRINSICS_VERIFY_FAIL`);
- in every other run it **fails closed**: every recording is forgotten (no replay or recording
  uses them again), the drop is counted in the statistics, and the run continues expanding for
  real. Their pins are **leaked, not released**: when the check has failed, the pin accounting
  may be what is broken, and releasing through `delete_token_ref` could `flush_list` a list an
  `equiv` still points at. A leaked reference only keeps a list alive (memory) and makes the
  convergence test see a reference count the other run lacks, so it does not converge
  (`iso.rs`'s reference-count comparison): safe.

**Pin discounting counts multiplicity exactly**: a list pinned by three recordings has three
references from the cache; the walk subtracts, per list, the number of pins that state's
recordings hold on it (from its own `live_words`, counting duplicates), and compares the rest.

A soundness case ships with it (DESIGN.md §13 R5): a document where the new run records a variant
the old run did not have, converges, and then calls the macro again with those arguments; every
compile equals scratch runs, and the case fails (does not converge) without the rule.

### 7.3 Determinism

Recording, eviction and the decision to replay depend only on the engine state and per-macro call
counts (which are part of the state), never on time or the host. A cold run and its scratch twin make the same decisions; an
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

1. Call site and key: `changes/intrinsics.ch` gains the hook of §3.1 (passing `warning_index`,
   `ref_count`, `n`); `src/intrinsics.rs` gains `flashtex_intr_call_args`, the argument store and
   compare, the flush, the capacity margins, and the `ckpt_arm_cs` refusal (also added to D9's
   call site).
2. Slot heap and argument index (§3.3); `live_words` and the convergence walk learn the new
   layout.
3. Verifier support (§6.1, with the leak check), the faults of §6.5, `verify-all-args`, and the
   invariant assertion of §7.2.
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

## 11. Revision 5: dimensions, `\afterassignment`, sparse registers, contexts, local groups

Status: **proposal** (Commander ruling 2, 2026-10-04: "design revision v5 is authorized"), for
review. The prototype of §9 (#1524-#1526) stays open until this revision shows a measured win of at
least 15 % of the instructions on `long-deck` or *An Infinite Descent*.

### 11.1 Why (measured)

The prototype replays 25 % of `\XC@col@rlet`'s calls on `long-deck` and saves 2.2 % of the
instructions. The refusals and abandons it counts (#1525) have four causes, and a probe engine (not
committed) priced lifting each before any of this is written:

| `long-deck`, settled pass | instructions | vs intrinsics off | `\XC@col@rlet` replays / calls |
|---|---:|---:|---|
| intrinsics off | 49.76 G | | |
| the approved rules | 48.44 G | -2.7 % | 20,863 / 69,961 |
| + dimensions, `\afterassignment`, sparse registers (lifted, not captured) | 50.51 G | +1.5 % | 23,953 / 69,961 |
| + 32 contexts per argument list | 45.92 G | -7.7 % | 55,504 / 69,961 |
| + local-group elision (§11.5) | **42.13 G** | **-15.3 %** | 55,504 / 69,961 |
| the same in verify mode | | | 55,504 verified, **1,757 differ** |

The 1,757 differences (3.2 %) are replays that depended on an input the probe lifted without
capturing it: exactly the inputs §11.3 captures. With them captured, those calls are refused or
replayed correctly, so the estimate for this revision is **-13 % to -15 %** on `long-deck`. On *An
Infinite Descent* (592 pages, #1524's comment), the guard would admit 83 % of `\XC@col@rlet`'s
calls (9.8 % of the book) and 40 % of `\pgf@process`'s (22 %, all of them reading sparse
registers), which estimates **-7 % to -12 %**. The pgfmath parser itself never records: its bodies
read past themselves (`Level`).

### 11.2 Dimensions

Today scanning a dimension abandons a recording (`scan_dimen`'s hook, `Why::Dimension`), and
`\dimen` assignments and `\ifdim` are outside the allowlists. A dimension is computed from tokens,
integers and dimensions held in `eqtb`, and three units that read other state. This revision lets
a recording scan a dimension and captures what it reads:

| input of `scan_dimen` | how it is captured |
|---|---|
| the digits, the sign, the keywords (`pt`, `true`, `plus`, ...) | tokens: read through `get_x_token`, already recorded (meanings watched) |
| `\dimen`*n* (*n* < 256), `\dimendef`'d names, dimension parameters | `eqtb` words in the integer region: value pairs (`F_NRW`), as integers are today. The read through `REGISTER` (`scan_something_internal`, "Fetch a register") becomes a pair read of `dimen_base+n` / `count_base+n` instead of an abandon |
| `true` | reads `\mag` (and `prepare_mag` can error): `\mag` becomes a pair; an error abandons as today (`Error` at commit) |
| `em`, `ex` | read `quad(cur_font)` and `x_height(cur_font)`: **refused** (abandon, `Why::FontUnit`). Capturing them needs the current font's parameters, which `\fontdimen` can change; no measured macro uses them |
| `mu` | math units: refused (abandon) |
| internal glue, `\wd`/`\ht`/`\dp` of a box, `\fontdimen`, `\lastskip` | outside the allowlist as today (`Internal`) |
| the arithmetic (`\advance`, `\multiply`, `\divide` of a dimension, `\dimexpr`) | pure functions of the above; an arithmetic error ("Dimension too large", "Arithmetic overflow") is an error and abandons at commit |

The commands and expandables added to the allowlists: `ASSIGN_DIMEN` (a dimension parameter or a
`\dimendef`'d register as a command), `\dimen` through `REGISTER` (already allowed for
assignment), `\ifdim`, `\dimexpr`. Glue (`\skip`, `\glueexpr`) stays out: its values are pointers to
glue specifications, which no watch record compares.

### 11.3 Sparse registers and `\afterassignment`

**Sparse registers** (e-TeX `\count`, `\dimen` above 255; LaTeX allocates there once 256 are used:
`\pgf@picmaxx`, `\XC@countmixins`) live in nodes of `mem` (`find_sa_element`), not in `eqtb`.
- *Reading one* (`scan_something_internal`'s "Fetch a register", `sa_int`) is recorded as a
  **sparse pair** (type, number, value) in a new part of the slot (`P_SA`). The guard looks each up
  again without changing anything: a hand-written walk of `sa_root`'s index nodes that, unlike
  `find_sa_element`, writes no global (`cur_ptr` included); a missing element reads 0, as TeX's
  does. A different value fails the guard (`Why::SparseDeps`).
- *Assigning one* is recorded as an operation `K_SAWORD` (type, number, value, global) and replayed
  by `find_sa_element(t, n, true)` followed by `sa_w_def` or `gsa_w_def`: TeX's own routines, which
  create the element and save it for `unsave` exactly as the normal path did.
- Token and box registers above 255 keep abandoning (`Sparse`): their values are pointers.

**`\afterassignment`** inside the body (pgf's `\afterassignment\pgf@x\pgf@x=...`) is allowed when
the token it saves is used before the body ends: the commit already requires `after_token = 0`
(§2), and the guard already requires it at the call. The saved token is inserted after the next
assignment by `back_input`, a level above the body's, which the recording reads like any other.

### 11.4 Contexts per argument list

`\XC@col@rlet`'s argument lists recur under different values of beamer's `\color@fg` and
`\color@bg`; each recording watches the values it read, so each context needs its own recording.
`KEY_VARIANTS` becomes 32 (the probe's value), with the least recently replayed evicted, under the
same per-macro budget of 256. Nothing about the guard changes: a recording for another context is
simply another variant.

### 11.5 Local-group elision

A recording's operations come in balanced `K_BEGIN` ... `K_END` pairs for its `\begingroup` /
`\endgroup` (and `{` / `}`) groups. **Claim:** a group whose operations are all local (no
`K_GLOBAL` flag, no sparse global) leaves, when its `K_END` has been replayed, the same state as
not replaying it at all, except for memory addresses and the memory-usage statistics. A replay
therefore skips every such group.

*Proof.* Inside the group every operation is an `eq_define`, `eq_word_define`, local `sa_def` /
`sa_w_def`, or a nested local group. Each saves the old value of its location on the save stack
the first time the location changes at the group's level (`eq_save`, `sa_save`) and `unsave`
restores it at `K_END`, destroying the inner value (`eq_destroy`). A location changed twice is
saved once; its restored value is the one before the group. A location changed by a nested group
is restored by that group's own `unsave` first, then by this one. So after `K_END` every `eqtb`
word and every sparse value written inside holds what it held before `K_BEGIN`, and the save stack
holds what it held before. What else could a replay of the group leave behind?
1. *Memory.* The values made inside (`K_FRESH` copies) are freed by `eq_destroy`; the net change of
   `dyn_used` / `var_used` is zero, which the leak check (§6.1) compares.
2. *Watch records.* Each write and restore reported to `flashtex_intr_touch` updates mismatch counts
   for the new and then for the restored value; after the restore every record holds again what it
   held before the group, so the counts are unchanged.
3. *The read-set.* A replay reports all its reads at its start (§3.6 item 4), independently of
   which operations it then performs.
4. *The high-water marks.* Raised from the recorded peaks (§3.6 item 1), not by the operations.
5. *Values handed out of the group.* `\expandafter\endgroup\@@tmp` expands `\@@tmp` (defined inside)
   before the group ends; what it produces is executed after `\endgroup` and recorded there as its
   own operations: a `K_FRESH` template (the tokens as they were then) or a `K_LETCS` whose source,
   read after the restore, holds its outer value. Neither reads a location the group wrote.
6. *Tracing and `\aftergroup`.* Refused as preconditions (tracing) or outside the allowlist.
So the state after skipping the group equals the state after replaying it. ∎

The commit condition is unchanged: the recording still verifies the group balanced. The verifier
replays with the elision and diffs against the normal path, so a mistake in this argument is a
difference (§11.6).

### 11.6 Gates and faults

The gates of §6.4 are unchanged in kind: the elision, the dimension pairs, the sparse pairs and
`K_SAWORD` are all exercised by `verify` and `verify-all-args`, and the soundness sweeps run with
the switch on. New faults (§6.5), each to be caught by (a) or (b):
- `dims-unwatched`: a dimension register read by number is not recorded as a pair;
- `sparse-unwatched`: a sparse register read is not recorded;
- `sparse-op-local`: a global sparse assignment is replayed as a local one;
- `elide-global`: a group with a global operation is elided too;
- `true-unwatched`: `\mag` is not recorded for `true` units.

The measurement that decides whether the prototype lands (ruling 1): `long-deck` and *An Infinite
Descent*, intrinsics off against this revision on, instructions of a settled pass, with the verify
run of each at 0 differences. The bar is 15 % on either.

### 11.7 The DESIGN.md amendment (text for the Commander)

§5.6 item 4, after the 2026-10-04 extension to macros with parameters: "MACRO-REPLAY.md revision 5
extends the recordable set to dimension arithmetic (`scan_dimen` with `pt`/`pc`/`in`/`bp`/`cm`/
`mm`/`dd`/`cc`/`sp`/`true`; `em`, `ex` and `mu` refused), `\dimen` and `\count` registers read by
number (as value pairs), e-TeX's sparse `\count`/`\dimen` registers (sparse value pairs and
`K_SAWORD` operations through `sa_w_def`/`gsa_w_def`) and `\afterassignment` consumed inside the
body; keeps 32 contexts per argument list; and replays a group whose operations are all local as
nothing (proof in MACRO-REPLAY.md §11.5). The gates and faults are those of §6, plus §11.6's."
And the §13 row: "2026-10-xx | Guarded replay revision 5 (dimensions, sparse registers,
`\afterassignment`, contexts, local-group elision) | Commander, from evidence (#1524's measured
estimate)".
