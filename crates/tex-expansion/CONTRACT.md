# `crates/tex-expansion` — adoption contract

**Owner:** kabir-claude (task KC-101, rounds 1-2). **Status:** standalone;
adoption into `crates/compiler` is being done by another agent on
`agent/kabir-claude/compiler-tex-expansion`. This document is the contract
that adoption can rely on.

## What this crate is

A faithful implementation of TeX's *expansion processor* plus the part of
main control that executes assignments: category codes and tokenization
(TeXbook ch. 7-8, `\endlinechar`, `^^` notation), the control-sequence
table with grouping/save-stack semantics (ch. 24), `\def`/`\edef`/`\gdef`/
`\xdef` with delimited/undelimited/`#{` parameters (ch. 20), `\let`/
`\futurelet`, `\expandafter`/`\noexpand`/`\csname`, TeX and e-TeX
conditionals, registers and `\numexpr`/`\dimexpr`, e-TeX/pdfTeX expandables
(`\unexpanded`, `\detokenize`, `\expanded`, `\scantokens`, `\pdfstrcmp`),
`\uppercase`/`\lowercase` with `\uccode`/`\lccode`, `\chardef`/
`\mathchardef`, `\afterassignment`/`\aftergroup`, `\long`/`\outer`/
`\protected` with TeX's errors and recovery, and a LaTeX layer (the kernel
macros real documents and packages use: `\newcommand` and friends built
exactly as `ltdefns` builds them, `\DeclareRobustCommand`/`\protect`/
`\protected@edef`, environments, counters with reset lists, lengths,
`\@ifundefined`, `\@ifnextchar`, `\@for`/`\@tfor`, `\loop`,
`\g@addto@macro`, `\AtBeginDocument`, keyval, `\label` data).

## Public API (`src/lib.rs`)

```rust
pub fn expand_str(source: &str) -> ExpandResult; // { tokens, diagnostics, labels }
pub fn tokens_to_display_string(tokens: &[Token]) -> String;
pub fn is_group_token(t: &Token) -> bool;        // `{`/`}` (explicit or implicit), \begingroup/\endgroup

pub struct Engine;            // new(src), with_limits(src, limits), run(), next_content_token(),
                              // take_diagnostics(), take_labels(), set_mode(Mode),
                              // set_font_metrics(..), set_box_measurer(..),
                              // set_file_reader(..), opened_files(), new_initex(src, limits)
pub struct IncrementalExpander; // new(src), with_options(src, limits, interval), edit(&Edit) -> EditStats,
                                // tokens(), diagnostics(), labels(), source()
pub trait BoxMeasurer;        // \settowidth/\settoheight/\settodepth callback (sp)
pub struct LabelRecord;       // \label{key}: key, expanded \@currentlabel, span
pub fn to_fnsymbol(n: i64) -> Option<String>;
```

Every token carries `Span { source_id, start, end }`: source 0 is the
document, other ids are `\scantokens` pseudo-files and `\input` files
(`Engine::opened_files`). Synthesized tokens use `Span::synthetic()`.

## The output token stream

Assignments execute as they are met, so the stream contains **no**
definitions, register assignments or conditionals. It contains:

- character tokens (content);
- **grouping tokens, with spans**: every `{`/`}` main control processes
  (explicit, or implicit via `\bgroup`/`\egroup`, which appear as the
  brace character with the control sequence's span), and `\begingroup`/
  `\endgroup`. `\aftergroup` tokens follow the closing token. An unbalanced
  `}` ("Too many }'s") is dropped, as TeX does. Consumers that only want
  content filter with `is_group_token`. *(Round 2 change, requested by
  KC-110/PR #96; round 1 swallowed them.)*
- `\relax` for any token whose meaning is `\relax` (`\protect`, an
  undefined `\csname`), and `\par`;
- control sequences this crate does not model (`\section`, `\hskip`,
  `\textwidth`, ...), untouched, for the typesetter. With pending
  `\global`/`\long`/... prefixes, those prefix tokens are passed through
  in front (e.g. `\global\setbox`). An unmodelled control sequence is not
  an error: the crate does not know the whole LaTeX universe.

## Proposed adoption path for `crates/compiler`

1. Depend on the crate; run `Engine` (or `IncrementalExpander`) before the
   compiler's parser, feeding it the token stream instead of raw text.
2. Delete the parser's own bounded `\newcommand` substitution; macro calls
   are gone by the time the parser sees tokens.
3. Map `Diagnostic { severity, message, span }` 1:1; spans share the
   source's byte offsets. Messages are TeX's/LaTeX's own texts (oracle-
   checked), last line being the `! ...` line.
4. Wire `set_font_metrics` (`em`/`ex`) and `set_box_measurer`
   (`\settowidth`) to the font engine; the defaults are 10pt-CM placeholders
   and a zero-width measurer.
5. `set_file_reader` makes `\input` read files (spans get their own
   `source_id`); without it `\input` passes through as today.
6. `\ifvmode`/`\ifhmode`/`\ifmmode`/`\ifinner` read `Engine::set_mode`; a
   real feed needs expansion interleaved with layout.

## Incremental expansion (IDE)

`IncrementalExpander` snapshots the whole assignment state at *safe points*
(input stack = base lexer just past a line break, nothing pending) every
`checkpoint_interval` bytes. An edit restarts from the nearest checkpoint
before it, and stops as soon as the new run reaches an old checkpoint
(shifted by the edit) with an equivalent state, splicing in the old suffix
with shifted spans.

- **Equivalence:** `tests/incremental_tests.rs` checks tokens (kind *and*
  span), labels and diagnostics against a from-scratch run after every one
  of: 12 random edits on each of the 415 oracle documents, 240 on HW1/HW2
  (origin/main `fixtures/real-world/hw{1,2}`), 150 on a macro-heavy
  document. All pass.
- **Latency** (`cargo run --release --example bench_incremental`, 500 KB
  synthetic LaTeX document, Apple M5 Pro, default `Limits`): full expansion
  154 ms; initial incremental run 132 ms with 230 checkpoints; 300 random
  single-character keystrokes each followed by its undo (600 edits):
  **p50 5.9 ms, p95 115 ms, max 379 ms**, 536/600 converged early, final
  tokens identical to a from-scratch run. The p95 is the non-converging
  edits below; the max is an edit that created an infinite macro loop and
  ran to the 2M-step limit.
- An edit that changes state for the rest of the document (e.g. typing into
  `\stepcounter{para}`) cannot converge and costs a re-expansion from the
  checkpoint (≤ one full run, ~130 ms here). An edit that creates an
  infinite macro loop costs up to `Limits::max_expansion_steps`.

## Oracle corpus

`tests/oracle/gen/cases.py` runs real `tex`/`etex`/`pdflatex` (MacTeX 2026;
oracle-only, never a build dependency) and commits `(setup, expr, expected,
mode)` to `tests/oracle/manifest.json`; `tests/oracle_tests.rs` compares
without running TeX. Modes:

| mode | TeX side | compared with |
| --- | --- | --- |
| `tex`, `etex` | plain format, `\immediate\write` of expr | display string |
| `latex-write` | pdflatex, setup + write in the body | display string |
| `latex-doc` | setup in the preamble, write after `\begin{document}` | display string |
| `latex-render` | typeset body, `pdftotext` | content text, whitespace collapsed |
| `tex-err`, `etex-err`, `latex-err` | `! ...` lines of the log | our error messages |

Executed `\relax` and grouping tokens are dropped before comparing (TeX
neither writes nor typesets them). **415 / 415 cases pass** (round 1: 108),
covering everything listed in "What this crate is" plus `\meaning` of every
token kind, `\string` with `\escapechar`, `\csname`'s implicit `\relax`,
dimension/glue arithmetic and units, and ~37 error-message cases.

## `\long` / `\outer`

Enforced as tex.web does (`check_outer_validity`, §392-§399): a `\par` in
the argument of a non-`\long` macro gives "Runaway argument? / ! Paragraph
ended before \foo was complete." and aborts the call; an `\outer` macro
while scanning a definition/argument/`\uppercase`-style text/skipped
conditional gives "Forbidden control sequence found while scanning
definition|use|text of \foo." or "Incomplete \iffalse; all text was ignored
after line N.", with TeX's recovery insertions (`}`, `\par`, `\fi`). Extra
`}` in an argument, "Use of \foo doesn't match its definition", prefix
errors ("You can't use a prefix with ..."), `\newcommand*` vs.
`\newcommand` (`\long` only when the command has parameters, like
`\@yargd@f`) are oracle-checked against the real logs.

## Counters

`\newcounter{c}[within]`, `\@addtoreset`, `\@removefromreset`,
`\counterwithin(*)`, `\counterwithout(*)`, recursive resets on
`\stepcounter`/`\refstepcounter` (LaTeX's `\@stpelt`), `\@currentlabel`
with `\p@c`, and `\label` recording `LabelRecord`s. All oracle-checked.

## `\fnsymbol`

Unicode, matching what pdfLaTeX's text-mode symbols extract to (oracle
`fnsymbol_1`..`fnsymbol_9`): 1 ∗ U+2217, 2 † U+2020, 3 ‡ U+2021,
4 § U+00A7, 5 ¶ U+00B6, 6 ‖ U+2016, 7 ∗∗, 8 ††, 9 ‡‡; other values are
"Counter too large".

## Real-document smoke (`examples/smoke.rs`)

All 44 `.tex` files under `fixtures/` and `tests/` on origin/main
(dbf6ec78): **42 clean, 2 with errors**, both legitimate:
`tests/tex-corpus/cases/extra-closing-group` ("Too many }'s", the case's
point) and `include-scope/local.tex` (a fragment that `\renewcommand`s a
macro its includer defines). Each expands in < 1 ms. Unmodelled primitives
passed through, by documents: `\parindent` 12, `\parskip` 11, `\hfill` 4,
`\input` 4, `\left`/`\right` 3, `\displaystyle` 2, `\hrule` 2, `\kern`,
`\vfill`, `\mathbin`, `\noindent` 1. Top class/package commands passed
through: `\documentclass` 40, `\usepackage` 27, `\pagestyle` 20, `\[`/`\]`
16, `\frac` 11, `\\` 10, page-geometry lengths 8-9, `\section` 7.

## Kernel feasibility probe (`examples/kernel_probe.rs`)

Runs the real `latex.ltx` (read in place via `kpsewhich`, not vendored)
from `Engine::new_initex` (INITEX catcodes, TeX/e-TeX/pdfTeX primitives
only, TeX parameters as registers, I/O stand-ins, `\input` through
kpathsea). Result at this commit: **reaches line 1147 of 22 470 (5.1 %) in
226 ms**, having read `texsys.cfg`, `expl3.ltx` and `expl3-code.tex`. The
first failures, in order:

1. `\sfcode` (and the other code tables `\mathcode`/`\delcode`) — not modelled;
2. the `\.`/`\?`/`\!`/`\:`/`\;`/`\,` spacing commands — defined by code
   that uses the missing primitives;
3. `\batchmode` (interaction modes);
4. the expl3 bootstrap: expl3 renames *every* engine primitive to
   `\tex_...:D` and checks for pdfTeX ones (`\lastnamedcs`,
   `\pdfprimitive`, ...); with them missing it reports "LaTeX requires
   expl3" / "This is one for The LaTeX3 Project: bailing out".

Static gap (every `\name` in the files that is an engine primitive, from
LuaTeX's primitive list): `latex.ltx` references 310 primitives, **180
modelled, 130 missing**; with `expl3-code.tex` 717, 202 modelled, 515
missing, of which 162 are LuaTeX-only. The missing ones are overwhelmingly
typesetting primitives: boxes (`\box`, `\hbox`, `\vbox`, `\setbox`, `\wd`/
`\ht`/`\dp`, `\copy`, `\unhbox`...), glue/kern/penalty/rules, fonts
(`\font`, `\fontdimen`, `\char`), math classes, alignment (`\cr`,
`\noalign`, `\omit`), marks/inserts/`\shipout`, plus the code tables and
pdfTeX utilities.

**Estimate.** Loading the kernel for real is not an expansion-layer task
alone: the expansion side needs roughly 40 more primitives (code tables,
interaction modes, `\lastnamedcs`/`\pdfprimitive`/`\ifpdfprimitive`,
`\show*`, `\numexpr`-family (`\glueexpr`/`\muexpr`...), string/file
utilities, `\currentgrouplevel`/`\currentiftype`, `\everyeof`) — about
1-2 rounds of this task — but ~250 of the missing primitives only make
sense with a typesetter's data structures (boxes with dimensions, fonts
with metrics, math lists, alignments). A feasible plan is either (a) an
executable TeX core that owns boxes/fonts, where this crate is its
expansion front end, or (b) keep the hand-modelled LaTeX layer for
documents and use the probe as a regression gauge. Expanding what the
engine can currently takes < 0.25 s; `expl3-code.tex` alone is 35 k lines.

## Known deviations

- **`\write` vs. main control.** The stream models main control. A let-to-
  `\relax` token is emitted as `\relax`; `\unexpanded` outside an
  expand-only context expands its tokens later (e-TeX main control).
- **No final `\endlinechar`** after the last line of a buffer that does not
  end in a line break (TeX appends one to every line).
- **`\ifvmode`...** use `set_mode`; **`\ifeof`** is true and `\ifvoid` true,
  `\ifhbox`/`\ifvbox` false (no boxes).
- **INITEX-only modelling** of TeX parameters (`\tolerance`, `\parindent`,
  ...) and of `\write`/`\message`/`\read` stand-ins: in the default engine
  these remain pass-through for the typesetter. `\write` text is not
  expanded; `\read` defines the target as empty with a warning.
- **`\settowidth`** uses the host measurer (default: zero).
- **Implicit braces** (`\bgroup`) are emitted as the brace character, so a
  consumer cannot distinguish `{` from `\bgroup`.

## Tests

`cargo test`: 55 unit tests (`tests/expand_tests.rs`), 3 incremental
equivalence tests, the 415-case oracle. Helpers: `examples/probe.rs`
(expand lines of a file), `examples/oracle_scan.rs`, `examples/inc_hunt.rs`,
`examples/dbg.rs`.

## Host integration additions (compiler adoption, kabir-claude)

Added on top of round 2 (e476dbab) for `crates/compiler/src/expansion.rs`; all
additive, default behaviour unchanged, oracle/unit/incremental suites green:

- `Engine::next_content_token_with_origin` -> `(Token, Option<Span>)`: the span of
  the outermost macro invocation whose expansion produced the token (`None`
  for tokens read straight from a source). Tracked per pending token
  (`Pending.origin`), filled by the push functions; `call_macro` tags its
  replacement text.
- `Engine::declare_host_command(name)` (`Primitive::Host`): a host-typeset
  command that is emitted unchanged but counts as defined for
  `\newcommand`/`\renewcommand`; no effect on names the engine defines.
- `Engine::run_host_prelude(text)`: run host TeX definitions (own source id)
  into the checkpointed state before the document.
- `Engine::set_emit_unbalanced_close(bool)` (state option): pass a "Too many }'s"
  brace on so a host parser can report it at its position.
- `Engine::push_input(text)`, `open_input_ids()`, `input_position()`: host-driven
  `\input` with the host's own path rules, and the resume point after a halt.
- `IncrementalExpander::with_host(source, limits, interval, init)` and `origins()`.
- `\newcommand` errors with pdflatex's texts (checked against MacTeX pdflatex):
  `Missing control sequence inserted.` (non-command name, instead of "Command o
  already defined."), `You already have nine parameters.` (`[10]`, was silent),
  `Illegal parameter number in definition of \x.` (undeclared `#n`, was silent).
- `IncrementalExpander::edit`: reused suffix tokens are moved instead of cloned
  and carried checkpoints get lazy span shifts (500 KB keystroke 5.0 -> 0.9 ms
  p50); an edit that changes the line count no longer converges onto old
  diagnostics that embed a line number ("Incomplete \iffalse; all text was
  ignored after line N" kept a stale N).

- Copy-on-write assignment state (adoption round 2): every `Scopes` table is a
  `CowMap` (2^k copy-on-write chunks behind a copy-on-write chunk table,
  FxHash inside), the save stack is a copy-on-write list of copy-on-write
  frames, integer parameters are an array and counter reset lists sit behind
  `Rc`. A checkpoint bumps reference counts; the first write to a chunk after
  it copies that chunk. Each chunk and frame keeps an upper bound on the `end`
  of the document spans it stores; since every span shift is the identity
  below its edit start, convergence (`State::eq_mapped`) skips chunks that are
  the same allocation in both runs and lie below the bound, and compares the
  rest entry by entry without building shifted copies. `map_spans` shares
  chunks a shift cannot change.
- The kernel prelude is lexed under a source id of its own (it was source 0,
  the document's id, so an edit in the first ~4 KB "shifted" prelude macro
  spans and no HW1-sized keystroke ever converged).
- `State::prelude_source_end` marks kernel/host prelude source ids; a
  diagnostic raised at a prelude-body token is reported at the document
  invocation being expanded, or at the last token read from source text
  when look-ahead dropped the invocation origin.


## Package and class files (`src/latex_packages.rs`, S1 of `docs/proposals/packages-fonts-manifest.md`)

`\usepackage`, `\RequirePackage`, `\documentclass` and `\LoadClass` are
primitives that read `[options]{names}[version]` as ltclass.dtx's
`\@fileswith@pti@ns` does and ask the host's **package reader**
(`Engine::set_package_reader(Rc<dyn Fn(name, ext) -> Option<String>>)`) for
each `name.sty`/`name.cls`. The rest of ltclass.dtx runs in TeX, copied
from latex.ltx (`\@onefilewithoptions`, `\@pushfilename`/`\@popfilename`
with the catcode of `@` saved and restored, `\ProvidesPackage/Class/File`,
`\NeedsTeXFormat`, `\DeclareOption(*)`, `\ProcessOptions(*)`,
`\ExecuteOptions`, `\PassOptionsToPackage/Class`, `\CurrentOption`,
`\OptionNotUsed`, `\@ifpackageloaded/with/later` and the `IfPackage…TF`
family, `\AtEndOfPackage/Class`, `\RequirePackageWithOptions`,
`\LoadClassWithOptions`, the option-clash and unknown-option errors,
`\Package…`/`\Class…` messages). `\endinput` ends the file.

- A name the reader **declines** (`None`) is *passed through*: `\ver@`/`\opt@`
  are still recorded, and the original tokens, command included, are
  emitted with their exact spans -- the stream is byte-identical to the
  pre-kernel pass-through when nothing is loaded. Only a list that mixes
  loaded and declined names, or a name with `\PassOptionsToPackage`d
  options, is re-spelt one command per name with the options folded in.
  `\RequirePackage`/`\LoadClass` pass through as `\usepackage`/
  `\documentclass`. Without a reader everything is declined.
- A loaded file's tokens carry a source id of their own
  (`Engine::opened_package_files() -> &[OpenedFile { source_id, name,
  loaded_at }]`; `IncrementalExpander::opened_package_files()` is the union
  over runs). The reader is part of the checkpoint, so restored engines
  read files too (`tests/latex_packages_tests.rs`, incremental case).
- A `.cls` that `\LoadClass`es a declined class emits `\documentclass
  [passed,explicit]{base}`; one that names no base class emits
  `\documentclass[opts]{article}` plus one `LaTeX Warning`.
- LaTeX's errors/warnings from the kernel and from `\PackageError` & co.
  are diagnostics with latex.ltx's texts; `\PackageInfo`/`\typeout` are
  silent. `\@currpath`, `\IfFileExists`, `\InputIfFileExists` and the
  file hooks are not modelled.
