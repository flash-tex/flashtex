# L6: macro profile and guarded intrinsics for the per-page output routine (2026-09-29)

Lane P6-HYPERREF-INTRINSICS (DESIGN.md §5.6 L6 items 1, 4 and 6; decision D9; §1.2).
Branch `agent/kabir-claude/l6-hyperref-intrinsics`, from `origin/agent/kabir-claude/p4-l2-l3`.
Raw output is in [`raw/`](raw/); the drivers are in [`scripts/`](scripts/) (see *Reproducing*).

**Host:** mac-m5pro-kabir, Apple M5 Pro, macOS 26.6.2, rustc 1.98.0, release builds. The
machine was shared with other sessions throughout: the 1-minute load average was 4 to 13
during these measurements (each table gives it). Engines are always interleaved and medians
taken, so the comparisons hold, but the absolute times are higher than on a quiet machine
(full-100 compiles in 0.80 s quiet, 0.97 s at load 11).

## Summary

| Item | Result (verified unless marked *belief*) |
|---|---|
| 1. Profile | A macro-level profiler in the engine (`FLASHTEX_MACRO_PROFILE=FILE`, off by default). On full-100, hyperref's `\pdfstringdefPreHook` is **93%** of `\pdfstringdef` and **29%** of all profiled time. siunitx fills it: per `\pdfstringdef` call **1,044 µs with siunitx, 38 µs without** (27×): 758 µs are twelve `\DeclareCommandCopy` calls (`\c__siunitx_bookmark_seq`), ~225 µs a `\cs_set_eq:Nc` for each of ~250 units. Confirmed: the page-label/anchor `\pdfstringdef` path dominates per-page LaTeX work. |
| 2. Guarded intrinsics | Recorded replay of a registered parameterless macro run by `main_control` (`src/intrinsics.rs`, hooks in `changes/intrinsics.ch`). Registered: `\pdfstringdefPreHook`. Guard: O(1) per-recording count of watched `eqtb` entries that no longer hold what the recording read, plus a handful of integer pairs; preconditions `\globaldefs=0`, no pending `\afterassignment`, no expansion-visible tracing. A replay costs **45 µs** (675 operations) instead of ~1.1 ms. |
| Both-paths diff | `FLASHTEX_INTRINSICS=verify` / `verify-all`: every replayable call runs both paths from one checkpoint and the whole word-space change is diffed. **0 differences in 53,401 calls run both ways** (83 parity fixtures, every parameterless macro: 39,016; benchmark documents: 3,149 registered + 11,236 every-macro); logs and PDFs byte-identical to intrinsics off in every document. Lockstep corpus: 343 candidate calls, none replayable. Injected replay faults are caught (4 of 4). |
| Tests | `tests/intrinsics.rs`, 7 tests: redefined dependency (falls back and re-records), `\globaldefs=1`, pending `\afterassignment`, tracing, register arithmetic, impure macros, `verify-all`; logs identical off/on/verify, 0 differences. |
| Per-page CPU | Whole document, full-1000: **6.72 → 4.11 ms/page** (6.73 → 4.12 s; pdflatex 6.33 s). Marginal (full-1000 minus full-100): 6.37 → 3.75 ms/page. Edited page in the resident host (§1.2): full-100 p50/p95 **7.1/17.6 → 5.1/12.4 ms** CPU, full-1000 (end) **8.7/11.3 → 6.0/9.9 ms**. |
| vs pdflatex | full-10 0.38 s vs 0.48 s; full-100 0.74 vs 1.00; full-1000 4.12 vs 6.33 (FlashTeX faster on all three with intrinsics; without them full-1000 was 6% slower). plain-1000 unchanged (1.63-1.70 s, noise), still slower than pdflatex's 1.27 s. |
| Gates | P-T1 **83/83**, P-T2 **83/83**; lockstep **260/260** (accounting 0 differ); trip, etrip, drift pass; `cargo test -p flashtex-engine --test intrinsics` 7/7; clippy clean outside `src/generated/`; `scripts/gate.sh pr`: see *Gates*. |

## 1. Profile (DESIGN §5.6 item 1)

**Profiler** (`src/macroprof.rs`; hooks in `changes/intrinsics.ch`): `macro_call` reports each
macro body it feeds to the scanner and `end_token_list` each macro level it leaves. A shadow
stack of macro input levels attributes the time between two events to the innermost macro
level (*self* time) and the time from entry to exit to the macro (*inclusive*, once when
recursive). The clock is the ARM generic timer. Caveats, all from TeX's structure: a
macro's arguments run while the macro's level is still innermost, so `\use_ii:nn` and
`\use_i:nn` collect the self time of the code they select; TeX leaves a used-up macro before
it expands its last token (§390), so a macro's tail call is not in its inclusive time.
Profiling roughly doubles run time; it changes no output.

**full-100 at the lane's start** (`raw/profile-full-100-base.tsv.gz`, load ~10, profiled
total 1,477 ms, 101 pages). Top 20 by self time:

| # | macro | calls | self ms | self % | incl ms | self µs/call |
|---|---|---|---|---|---|---|
| 1 | `\use_ii:nn` | 67,659 | 200.6 | 13.6 | 709.8 | 2.97 |
| 2 | `\use_i:nn` | 79,224 | 68.4 | 4.6 | 743.5 | 0.86 |
| 3 | `\exp_args:NNc` | 73,373 | 50.7 | 3.4 | 51.4 | 0.69 |
| 4 | `\l_siunitx_unit_symbolic_seq` | 759 | 33.0 | 2.2 | 97.7 | 43.46 |
| 5 | `\token_if_macro:NTF` | 5,955 | 28.8 | 2.0 | 36.6 | 4.84 |
| 6 | `\@swaptwoargs` | 140 | 27.4 | 1.9 | 213.5 | 195.62 |
| 7 | `\str_if_eq:eeTF` | 16,587 | 26.2 | 1.8 | 29.2 | 1.58 |
| 8 | `\__seq_item:n` | 73,645 | 21.3 | 1.4 | 43.2 | 0.29 |
| 9 | `\tl_if_empty:oTF` | 20,465 | 18.8 | 1.3 | 20.6 | 0.92 |
| 10 | `\use:x` | 18,319 | 18.1 | 1.2 | 70.4 | 0.99 |
| 11 | `\pdfstringdef` | 226 | 18.0 | 1.2 | 455.5 | 79.80 |
| 12 | `\exp_args:Ne` | 9,347 | 17.6 | 1.2 | 120.3 | 1.89 |
| 13 | `\robust@command@chk@safe` | 2,736 | 17.5 | 1.2 | 17.5 | 6.40 |
| 14 | `\vbox_set_to_ht:Nnn` | 102 | 16.2 | 1.1 | 19.1 | 158.79 |
| 15 | `\exp_args:Ncc` | 17,579 | 15.1 | 1.0 | 15.2 | 0.86 |
| 16 | `\use:n` | 27,005 | 13.8 | 0.9 | 90.3 | 0.51 |
| 17 | `\hook_use:n` | 6,865 | 12.9 | 0.9 | 21.3 | 1.87 |
| 18 | `\exp_not:c` | 22,604 | 12.1 | 0.8 | 15.6 | 0.54 |
| 19 | `\__cmd_copy_expandable:nnN` | 9,718 | 11.8 | 0.8 | 52.8 | 1.21 |
| 20 | `\vbox_set:Nn` | 101 | 11.7 | 0.8 | 11.9 | 115.41 |

By inclusive time the per-page chain is `\@outputpage` (674 ms) → `\__hook shipout/before`
(418) → `\Hy@EveryPageHook` → `\HyPL@EveryPage` (page label, 214) and
`\Hy@EveryPageAnchor` (anchor, 203) → `\pdfstringdef` (226 calls, 456) →
`\pdfstringdefPreHook` (422) → `\c__siunitx_bookmark_seq` (313). Most of the top self-time
entries (`\exp_args:NNc`, `\__seq_item:n`, `\token_if_macro:NTF`, `\str_if_eq:eeTF`,
`\robust@command@chk@safe`, `\__cmd_copy_expandable:nnN`) are called from inside that hook.

**Why siunitx makes it slower** (`scripts/profpick.py` on `raw/profile-*.tsv.gz`; full-100
and the same document with siunitx removed, intrinsics off, one run each):

| per `\pdfstringdef` call | without siunitx | with siunitx |
|---|---|---|
| `\pdfstringdef` inclusive | 38 µs | 1,044 µs |
| `\pdfstringdefPreHook` | empty | 983 µs |
| of which `\c__siunitx_bookmark_seq` (12× `\DeclareCommandCopy`) | – | 758 µs |
| of which `\siunitx_unit_pdfstring_context:` (a `\cs_set_eq:Nc` per unit) | – | ~225 µs |

siunitx adds two `\pdfstringdefDisableCommands` blocks at `\begin{document}`
(siunitx.sty l.9423-9445): `\DeclareCommandCopy` of the pdfstring-context version onto each
of `\ang`, `\qty`, `\num`, `\unit`, `\numlist`, `\qtylist`, `\numrange`, `\qtyrange`, `\si`,
`\SI`, `\SIlist`, `\SIrange`, and `\siunitx_unit_pdfstring_context:`, which maps over
`\l_siunitx_unit_symbolic_seq`. hyperref runs the hook inside every `\pdfstringdef`, and
every page makes two (page label, page anchor). One call is ~4,200 macro expansions and
~950 assignments (pdfTeX trace: `\tracingmacros` and `\tracingassigns` "changing" lines). The ratio here is 27×
per call; the 7× per page of the galley study includes the rest of the output routine.

## 2. Guarded intrinsics (DESIGN §5.6 items 4 and 6, D9)

What the profile shows is not a hot *leaf* function in the expl3 sense but a pure,
argument-free hook whose effect is a fixed set of assignments. So the intrinsic is a
**recorded replay**, generic over such macros, and the first registered name is
`\pdfstringdefPreHook` (`DEFAULT_NAMES`; `FLASHTEX_INTRINSIC_NAMES` overrides it).

**Recording.** When a registered parameterless macro is about to be expanded by
`big_switch`'s own `get_x_token` (so `main_control` will execute its body; `\expandafter`,
`\edef` or any other expansion context never qualifies), its normal expansion is observed:

- *Reads.* The meaning of every control sequence `get_next` delivers; `\csname` and
  `\ifcsname` look-ups; the internal quantities `scan_something_internal` fetches;
  `\catcode`s via `\catcode`\x`; the register `\advance`/`\multiply`/`\divide` read;
  `\escapechar`; all 256 `\catcode`s once a token list is printed (`print_cs` reads them).
  Tokens that are only scanned, never interpreted (macro arguments, parameter texts,
  unexpanded bodies, the control sequence being defined) record only "not `\outer`, not
  `#`". An entry first *written* by the run is not a dependency (only `eq_define`'s own
  bookkeeping reads it, and a replay calls `eq_define`) unless the run reads it back while
  it again holds its old value (after an inner `\endgroup`).
- *Writes*, in order: `eq_define`, `geq_define`, `eq_word_define`, `geq_word_define`,
  `new_save_level`, `unsave`. A macro or token list value becomes either a template (a list
  just made by `\def`/`\edef`/a braced token assignment; a replay defines a fresh copy, so
  `eq_define` decides exactly as for the fresh list of the normal path) or a reference to
  the control sequence just read (`\let`/`\futurelet`; a replay takes that control
  sequence's meaning as it is then).
- *Purity.* Only an allowlist runs: `\relax`, `\begingroup`/`\endgroup`, `{`/`}`,
  `\ignorespaces`, spaces outside horizontal mode, and the assignments `\let`,
  `\futurelet`, `\def` family, `\chardef` family, codes, integer parameters and registers,
  `\advance`/`\multiply`/`\divide`, braced token assignments; expandable: macros,
  `\expandafter`, `\unless`, `\noexpand`, `\csname`, conditionals that read tokens and
  integers only, `\number`, `\romannumeral`, `\string`, `\meaning`, `\the`, `\unexpanded`,
  `\detokenize`, `\expanded`, `\pdfstrcmp`, `\pdfescape…`, `\numexpr`, version numbers.
  Anything else abandons the recording: typesetting, boxes, glue, dimensions, fonts, files,
  marks, e-TeX sparse registers, token-register copies, `\afterassignment`, `\aftergroup`,
  messages, errors, any log or terminal output, a new control sequence, reading a token
  from outside the macro's own input levels, closing a group or conditional it did not
  open, unbalanced braces. The recording ends at `big_switch` once every input level at or
  above the body's is used up.

**Guard (O(1)).** Each watched entry has a watch record (the `eq_type` and `equiv` read,
not `eq_level`) and a flag saying whether it holds that now; `eq_define`, `geq_define` and
`unsave`'s restore report writes to watched entries (`flashtex_intr_touch`), which keeps a
per-recording count of entries that differ. Token lists compare by their tokens (TeX never
looks at an address). The guard is: that count is 0; the recorded integer entries (1-3 for
the PreHook) still hold; mode, `align_state` and `par_token` as recorded; `\globaldefs=0`;
no pending `\afterassignment`; `\tracingmacros`, `\tracingcommands`, `\tracingassigns`,
`\tracingrestores`, `\tracinggroups`, `\tracingifs`, `\tracingscantokens`,
`\tracingnesting` ≤ 0 (so P-T1's `\tracingall` runs never replay). Other writers of
`eqtb` below `int_base` (listed by `scripts/eqtb_writers.py`) touch only box registers and
font identifiers, which a recording may not read. Up to four recordings are kept per macro
(on full-100 the page-label and page-anchor calls see different `\PU-cmd` meanings);
when none passes, the call runs normally and is recorded again (at most 64 times).

**Replay** calls `eq_define`/`geq_define`/`eq_word_define`/`geq_word_define`/
`new_save_level`/`unsave` with the recorded arguments, so save-stack entries, `eq_level`s,
reference counts, "reassigning" and the e-TeX line saved with each group come out as the
normal path makes them. On full-1000: 2,240 replays of 675 operations each, 101 ms in all
(45 µs per replay, 67 ns per operation).

**State.** Everything lives in the word space (`intr_state`, `intr_data`, `intr_watch`,
`intr_seen`, `intr_pre`, `intr_cand`), so checkpoints and restores (§5.2) carry it with
the `eqtb` and `mem` it describes. Recorded and read token lists are pinned (reference
count raised) while their recording lives; only memory-usage statistics can tell, and P-T1
normalises those out.

**Deviation from DESIGN §5.6's wording, stated plainly.** §5.6 says the dependency set is
computed when the format is built. `\pdfstringdefPreHook` does not exist in the format:
hyperref and siunitx are loaded in the preamble and fill it at `\begin{document}`. Here
the dependency set is computed by the first run of the macro in a document (and again
after any dependency changes), from what that run actually reads; it is exact for that
run and the guard keeps it so. A kernel-level function present in the format could be
recorded the same way (the names are resolved at format load, `flashtex_intr_loaded`), but
none showed up in the profile as a pure parameterless macro worth it.

## 3. Verification: both paths, diffed

`src/intrinsics_verify.rs`. With `FLASHTEX_INTRINSICS=verify` every call the guard would
replay instead (1) takes a checkpoint of the whole engine, (2) expands normally under a
verification recording (the same purity checks; if the normal path is impure, the guard let
through a call it should not have: a difference), (3) at `big_switch` captures that state,
restores the checkpoint, replays, and diffs. `verify-all` makes every parameterless macro
that `big_switch` expands a candidate (a stress test of recording and guard; it replays
macros that the product never would).

**What is compared:** every word of the word space either path wrote since the checkpoint
(the arena's open log). `eqtb` and save-stack entries holding macros or token lists are
compared by tokens *and reference count*; the input and parameter stacks by the tokens left
to read (used-up levels are popped in both paths first, as the next `get_next` would); the
condition stack by its entries; scalars by name; the log length. Excluded, each because no
later computation reads it before writing it: `mem` addresses (compared through what reaches
them), `buffer` beyond `first`, `str_pool` beyond `pool_ptr`, `str_start` beyond `str_ptr`,
`save_stack` beyond `save_ptr`, `dig`, `trick_buf`, `pstack`; the scanner's result registers
(`cur_cmd`, `cur_chr`, `cur_cs`, `cur_tok`, `cur_val`, `cur_val_level`, `radix`,
`cur_order`, `def_ref`, `long_state`); allocation state and high-water marks; pseudo-print
scratch (`tally`, `first_count`, `trick_count`, `base_ptr`); `warning_index` (printed only
while `scanner_status` is not `normal`, and set by whoever sets that status);
`last_tokens_string` (flushed by every caller of `tokens_to_string` at once); `skip_line`
(set by `pass_text` before it skips, read only while skipping); the intrinsics' and
checkpoints' own bookkeeping.

**The verifier catches faults** (`scripts/faults.py`, `raw/faults-full-10.txt`; full-10,
21 verified calls; `FLASHTEX_INTRINSICS_FAULT` breaks the replay on purpose):

| injected fault | calls flagged |
|---|---|
| none (control) | 0 of 21 |
| leave out the last operation | 21 of 21 (`\H` and a reference count differ) |
| leave out the first `\let` | 21 of 21 (`save_ptr`, `eqtb`, reference count, save stack) |
| `\let` without taking a reference | flagged at the first call (339 differences); the corrupted run then hangs, as expected |
| make global integer assignments local | 21 of 21 (`save_ptr`, `xeq_level`, save stack) |

**It found three real problems during development**, all fixed before the numbers below:
`\global\advance` replayed its recorded result although `do_register_command` reads the
register directly (now a read hook); `skip_line` and `warning_index`/`last_tokens_string`
differed (shown to be scratch, excluded with the reasons above); and the used-up body's
input level still held a reference in the normal path (both paths now pop used-up levels).

**Sweeps** (`scripts/sweep.py`; each document compiled to convergence with intrinsics
off, then once in the verification mode from the same `.aux`, and once more off; the
`.log` less memory statistics and the `.pdf` compared byte for byte):

| corpus | mode | documents | candidate calls | calls run both ways | differences | outputs identical |
|---|---|---|---|---|---|---|
| parity fixtures (`fixtures/real-world`, `fixtures/divergence-probes`) | verify-all | 83 | 660,924 | 39,016 | **0** | 83/83 |
| benchmark documents (plain/full 10-1,000 pages) | verify | 8 | 3,165 | 3,149 | **0** | 8/8 |
| benchmark documents | verify-all | 8 | 551,424 | 11,236 | **0** | 8/8 |
| lockstep cases (prelude's tracing switched to 0) | verify-all | 260 | 343 | 0 | 0 | 260/260 |

The parity fixtures have no registered intrinsic in use except where hyperref and
siunitx are loaded, hence `verify-all`. The lockstep cases are primitive-level tests with
almost no parameterless macros; with their own prelude (tracing on) every call falls back,
which the lockstep gate below covers.

**Incremental host.** P4's `incr_bench.py --verify` (every incremental compile compared
byte for byte with a from-scratch run of the CLI): full-100 20 compiles and full-1000 16
compiles, each with intrinsics on and off: **72/72 identical, 0 mismatches**, convergence
unchanged (14/20 and 2/16 both ways). The intrinsics' state survives the host's checkpoint
restores.

**Tests** (`crates/flashtex-engine/tests/intrinsics.rs`, INITEX, no TeX installation):
`replays_and_matches`, `redefined_dependency_falls_back` (`\def\a{AA}`, `\let\b=\a`, the
macro itself redefined: each falls back, re-records, replays again),
`globaldefs_and_afterassignment_fall_back`, `tracing_falls_back`,
`register_arithmetic_is_a_dependency`, `impure_macro_is_not_replayed` (`\message`,
`\setbox`, `\aftergroup`, reading past its end, `\dimen`), `verify_all_on_the_test_file`.
Each checks that the log is byte-identical with intrinsics off, on and in verification, and
that verification finds 0 differences.

## 4. Performance

**Whole document** (`scripts/timing.py`, `raw/timing1.txt`; second run with a settled
`.aux`, user+sys CPU of the child, median of 5, engines interleaved; *base* = the lane's
starting point, *off*/*on* = this branch with `FLASHTEX_INTRINSICS=off`/default):

| document | pages | base | off | **on** | pdflatex | load |
|---|---|---|---|---|---|---|
| full-10 | 11 | 0.392 s | 0.396 s | **0.382 s** | 0.475 s | 11.5 |
| full-100 | 101 | 0.969 s | 0.989 s | **0.742 s** | 0.996 s | 10.7 |
| full-1000 | 1,002 | 6.598 s | 6.731 s | **4.122 s** | 6.333 s | 11.0 |
| plain-100 | 100 | 0.350 s | 0.344 s | 0.353 s | 0.394 s | 12.7 |
| plain-1000 | 1,001 | 1.652 s | 1.630 s | 1.695 s | 1.272 s | 11.4 |

Per page, full documents: full-1000 **6.72 → 4.11 ms** (whole document / pages);
marginal (full-1000 − full-100, 901 pages) **6.37 → 3.75 ms**. On plain documents
(no hyperref) the hooks cost nothing measurable: off/on/base differ by less than the
run-to-run spread (plain-1000 runs 1.53-1.94 s). The quiet-machine figure for
full-1000 with intrinsics on was 3.23 s (one run, `i2`, later in the session; *not* a
controlled measurement).

**Edited page in the resident host** (§1.2 target 16 ms p95; P4's `incr_bench.py`,
single-character edits and reverts, `raw/incr/`; load 8-11):

| document, region | compiles | intrinsics off: CPU p50 / p95 | **on: CPU p50 / p95** | wall p50 / p95 off → on |
|---|---|---|---|---|
| full-100, middle | 20 | 7.1 / 17.6 ms | **5.1 / 12.4 ms** | 8.0 / 18.5 → 5.9 / 13.7 ms |
| full-1000, end | 16 | 8.7 / 11.3 ms | **6.0 / 9.9 ms** | 11.5 / 16.6 → 8.9 / 16.4 ms |

## 5. Gates

On the engine built from c0b3b6526 (`/tmp/l6/i1`; later commits add a timing counter, a
renamed reason and clippy/rustfmt changes only):

- **P-T1 83/83, P-T2 83/83** (`tools/parity/parity.py --tier fixtures --engine
  flashtex-initex`, `raw/parity-fixtures/`). The non-gating accounting check differs on
  83/83 as before (1,428 candidate lines). hw2 at L2 is the committed baseline's level.
- **Lockstep 260/260**, accounting 0 cases differ.
- **trip, etrip** pass (`raw/trip.txt`, `raw/etrip.txt`); web2rust **drift** passes.
- `cargo test --release -p flashtex-engine --test intrinsics`: 7/7. Clippy
  `--all-targets`: no warnings outside `src/generated/`.
- `scripts/gate.sh pr` on 1b50eda78 (`raw/gate-pr-1b50eda78.txt`): clippy, tests (546 s,
  every engine test), licence boundary, parity self-tests, parity fixtures holding their
  baseline, bundled inventory: pass; rustfmt failed on one line this lane added to
  `arena.rs`, fixed in a7099e1de (the generated files' formatting is pre-existing and not
  gating).
- **Re-run on the tip engine** (a7099e1de, `/tmp/l6/i3`): lockstep 260/260 (accounting 0
  differ); fixtures `verify-all` 660,924 calls, 39,016 both ways, 0 differences, 83/83
  outputs identical (`raw/sweep-fixtures-all-i3.txt`); benchmark documents `verify` 3,149
  both ways, 0 differences (`raw/sweep-docs-verify-i3.txt`, which adds the no-siunitx
  document); `tests/intrinsics.rs` 7/7.

## 6. What is not done, and caveats

- **origin/main is not merged in.** Merging it into this P4-based branch collides P4's
  resident host (`src/host.rs`, `src/host_main.rs`, `changes/checkpoint.ch`) with main's P3
  display-list host (`src/host/`, `changes/displaylist.ch`): both define the `host` module
  and the `flashtex-host` binary. No P4 branch (including `p4-l5-restart`) has merged main
  yet. That integration belongs to the P4 lanes and the host code this lane was told not to
  touch; this lane's own files (`intrinsics.ch` in the args files, two module lines in
  `lib.rs`, one line in `main.rs` and `system.rs`) merge trivially once it is done.
- Only one macro is registered. After it, the per-page cost on full documents is spread
  over the shipout itself, marks, footnotes, cleveref and siunitx's `\SI` bodies (see
  `raw/profile-full-100-on.tsv.gz`); none of it is a pure parameterless macro. The
  galley study's 0.15 ms/page for hyperref *without* siunitx is the empty-hook case, which
  the recording abandons (the body is used up before `big_switch`; nothing to save).
- The dependency set is computed at the first run, not at format build (section 2).
- *Belief:* the arXiv tier would show the same zero differences; it was not swept.
- Absolute times are from a loaded machine (section 4).

## Reproducing

```sh
S=docs/evidence/l6-intrinsics-2026-09-29/scripts
bash $S/regen.sh                      # regenerate src/generated, build release
bash $S/mkeng.sh i1                   # engine + format in /tmp/l6/i1, /tmp/l6/fmt-i1
python3 docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py /tmp/l6/docs
bash $S/run.sh i1 full-100 FLASHTEX_MACRO_PROFILE=/tmp/p.tsv; python3 $S/proftop.py /tmp/p.tsv 20
python3 $S/sweep.py i1 fixtures --mode verify-all   # also: docs --mode verify, lockstep
python3 $S/faults.py i1 /tmp/l6/docs/full-10.tex
python3 $S/timing.py full-10 full-100 full-1000 plain-100 plain-1000 --reps 5
```

Environment: `FLASHTEX_INTRINSICS=off|verify|verify-all` (default on),
`FLASHTEX_INTRINSIC_NAMES=a,b`, `FLASHTEX_INTRINSICS_STATS=FILE` (counters and reasons),
`FLASHTEX_INTRINSICS_DEBUG=1`, `FLASHTEX_MACRO_PROFILE=FILE`.
