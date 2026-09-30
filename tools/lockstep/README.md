# Lockstep harness (slice 4)

Reference pin: pdfTeX 1.40.29. `run.py` refuses any other reference:
the first `<reference> --version` line must contain the whole version
token `1.40.29` (matched with `(?<![\d.])1\.40\.29(?![\d.])`, so
`1.40.290` or `11.40.29` do not match), else it exits 2;
`--allow-any-reference` overrides with a warning. `capture()` enforces
the same pin only with `require_reference_version=True` (cached per
binary path via `check_reference_version()`).

Differential test harness for the FlashTeX engine port (design §8, tier T1;
parity definition §1.1 P-T1). It runs the same `.tex` input through the
reference pdfTeX and a candidate engine binary and compares the trace log
and box dumps. MIT-licensed; it only executes external binaries.

## Usage

```sh
# candidate vs reference (exit 0 iff all equal)
python3 tools/lockstep/run.py --engine <path-to-binary>

# run a subset (globs or names, e.g. '001-*' '008-ifx')
python3 tools/lockstep/run.py --engine <bin> --cases '001-*' 008-ifx

# reference against itself: requires 100% equality (also checks determinism)
python3 tools/lockstep/run.py --self-test

# regenerate expected/<name>.log from the reference (never hand-written;
# a reference run that exits nonzero or ships no box is reported as an
# error and never written)
python3 tools/lockstep/run.py --update-expected [--cases ...]

# unit test for the importable capture() entry point
python3 -m unittest tools.lockstep.test_capture
# (or: python3 tools/lockstep/test_capture.py)

# keep per-run temp dirs for debugging
python3 tools/lockstep/run.py --engine <bin> --keep
```

A gate script calls it as `run.py --engine <bin>` and fails the build on a
non-zero exit. Output ends with `N cases, M equal, K differ`; each differing
case prints the first differing log line with 3 lines of context before it.
Every run also prints one `accounting: <case> differs (<kinds>)` line per
case whose accounting lines differ, plus a final `accounting: N case(s)
differ` total. The accounting check never gates: it changes neither the
exit code nor the PASS/FAIL count.

## How it works

Each run stages `cases/<name>.tex` plus the shared `prelude.tex` in a fresh
temp dir and invokes both engines identically:

```
<binary> -cnf-line='max_print_line = 1000' -cnf-line='error_line = 254' \
  -ini -etex -interaction=nonstopmode -halt-on-error <name>.tex
```

plus the single module-level setting `ENGINE_SHELL_FLAGS` (empty by
default, so both engines run in their default mode; see "Shell escape"
below).

with `SOURCE_DATE_EPOCH=0`, `FORCE_SOURCE_DATE=1`, `TZ=UTC`, cwd set to
the run dir, and stdin from DEVNULL. `<binary>` is always a symlink
literally named `pdftex` inside the run dir (see "Program name" below),
so argv[0]-derived log text prints identically for both engines.
The prelude boots INITEX catcodes,
sets `\pdfoutput=1`, turns all behaviour tracing to maximum — including
the e-TeX set (`\tracingassigns`, `\tracinggroups`, `\tracingifs`,
`\tracingscantokens`, `\tracingnesting`), which needs `-etex` —
sets `\showboxdepth=\showboxbreadth=10000`, and defines `\lsshipbox#1`,
which numbers the ship, writes a `LOCKSTEP-BOX <n>` marker to the log via
`\message`, then ships box register `#1`; every shipped box is dumped to
the log by `\tracingoutput=1`. `capture()` splits the normalised log
on the real shipout header to recover one string per shipped box dump
(`boxes`): a box starts at a line beginning with `Completed box being
shipped out` (anchored — a trace line merely mentioning the text does
not start a box). Trace text printed between two shipouts belongs to
the preceding box; the closing trailer (a `Memory usage before:` line,
the memory block, `Output written on`, `PDF statistics:`) is excluded
from the last box.

## Normalisation

Only what legitimately differs is normalised: the temp-dir path, the
`This is ...` banner line (version/date), and calendar dates — plus the
P-T1 accounting set from the design ruling (DESIGN §1.1, ruled
2026-09-29), quoted verbatim:

- "`\tracingstats` memory-usage lines ("Memory usage before/after",
  "still untouched")";
- "the end-of-run "Here is how much of TeX's memory you used" block";
- "the "PDF statistics" block";
- "the **byte count** in "Output written on … (N pages, B bytes)". The
  page count stays compared."

Apart from those normalisations the comparison is byte-exact: CR
bytes and the presence or absence of the final newline are compared,
not normalised (`normalise()` splits on `"\n"` only and forces no
trailing newline; the `.log` file is read with `newline=""` so text
mode never translates CRLF away first). A candidate that writes CRLF
line endings, or drops the final newline, FAILs. Real pdfTeX 1.40.29
logs contain no CR (verified by scanning every reference log: 260
cases, none contains `\r`) and end with a newline, so the strictness
costs no false failures.

Matched accounting lines are replaced by fixed placeholders in the
compared log — each `Memory usage before:` line becomes `Memory usage
<ACCOUNTING>`, the memory block becomes one `<ACCOUNTING memory block>`
line, the PDF-statistics block one `<ACCOUNTING pdf statistics>` line,
and the byte count becomes `<BYTES>` — so presence, position and count
stay compared and only the numbers are normalised; the page count,
every glue value and every trace line stay strictly compared. Each
accounting block is matched by exact line shape (verified against real
pdfTeX 1.40.29 logs: `-ini` `\tracingstats=2` runs including singular
`1 font` / `1 hyphenation exception`, a `pdflatex` article including
`7 compressed objects within 1 object stream`, and a rich `-ini`
document with 3 fonts and 2 hyphenation exceptions — every line of
every real block is matched, no real line is left over): the memory
header plus only its seven shapes (`strings?`, `string characters?`,
`words of memory`, `multiletter control sequences?`, `words of font
info for N fonts?`, `hyphenation exceptions?`, and the exact
`Ni,Nn,Np,Nb,Ns stack positions out of …` shape), the `PDF
statistics:` header plus only its four shapes (`PDF objects?`,
`compressed objects? within N object streams?`, `named destinations?`,
`words of extra memory`), and `Memory usage before:` lines matching
`Memory usage before: A&B; after: C&D; still untouched: E` exactly.
The rule is the same one `tools/parity` uses (its `ACCOUNTING_BLOCKS`
table and `split_accounting` logic, checked against tex.web and
pdftex.web; plurals follow `print_char("s")`), adapted in `run.py`
so the harness stays stdlib-only and self-contained:

- a block's body lines must come in pdfTeX's order, each shape at most
  once (shapes may be missing, e.g. no `compressed objects` line
  without object streams). The first line that fits no remaining shape
  ends the block and is compared, even if it starts with a space
  (` junk`, an `Overfull \hbox` line);
- each block header, and `Output written on`, counts only once, and
  only in the end-of-run trailer after the last `Completed box being
  shipped out`. A header anywhere else (a mid-log `PDF statistics:`
  injection, a duplicate) stays compared. In real logs the trailer
  reads: memory block, font-list `<...pfb>` lines, `Output written
  on`, `PDF statistics:`; `Transcript written on` goes to stdout,
  never into the `.log` file `capture()` reads;
- a `Memory usage before:` line counts only when an earlier shipout
  still owes its one line — real logs print one per shipout, right
  after its box dump (a 2-shipout log carries 2). A shipout is a line
  *beginning with* `Completed box being shipped out` (the same
  anchored rule `split_boxes` and `check_shipout` use — a trace line
  merely mentioning the text mid-line neither ends the trailer search
  nor owes a usage line), so a forged `Memory usage before:` line
  placed before any real shipout stays compared even when such a
  mention sits nearby.

The `<...pfb>` font-list line that follows the memory block in logs
using real fonts matches no shape, so it stays compared. Everything
else — tracing, messages, box dumps — must match exactly. This is the
same placeholder replacement rule `tools/parity` uses for its P-T1
comparison (that tool is updated separately by its owner).

Removed originals are still reported as non-gating `accounting`
(see below); the compared log keeps the placeholders.

## Output checks

A log that says `Output written on <file>` claims an output file was
produced: the check follows the file the log names (quotes and spaces
allowed, as in the byte-count normalisation). A `.pdf` must exist next
to the log, be non-empty, start with `%PDF-` and end with `%%EOF`
(trailing whitespace allowed); a `.dvi` must exist, be non-empty,
start with the DVI preamble bytes `F7 02` and end with at least four
`DF` post-postamble (trailer) bytes — or the case FAILs. A wrapper
that runs the real pdftex and then deletes `job.pdf`, appends garbage
lines to it, deletes `job.dvi`, truncates it, or prefixes it with
garbage FAILs. DVI is supported so backend-independent cases can set
`\pdfoutput=0` (DVI mode avoids font-file lines that belong to a later
engine feature) without tripping the gate; any other extension, or a
claim with no parseable name, FAILs with a clear message. This check
is structural only; byte-level output equality is another tool's job
(`tools/parity` P-T2), not this harness's.

A run must produce its own `job.log`: the stdout fallback (using the
captured stdout as the log) applies only to a run that produced no log
because the engine failed to start (nonzero exit). It never applies to
a run that exits 0, so a wrapper that deletes `job.log` and prints the
reference transcript on stdout with exit 0 FAILs (`exit 0, no log`).

## Process isolation and timeout

Every engine runs in its own process group (`start_new_session=True`)
under a per-run timeout (default 300 s, `--timeout`), and after each
run the whole group is SIGKILLed (`ProcessLookupError` ignored) — a
plain `subprocess.run` kill reaches only the direct child, so a
wrapper that starts `sleep 45` in the background and delegates would
otherwise leave it alive after the gate finishes. A wrapper that
ignores SIGTERM and hangs is SIGTERM'd, then unconditionally SIGKILLed
after a 5 s grace, so the case FAILs (`timed out`) within timeout +
grace with no survivor. Stdout is drained by a reader thread and the
pipe is never closed while that thread can still be blocked in
`read()`; if a `setsid`-detached grandchild (outside the group,
unkillable by the group kill, reaped only by the OS) still holds the
pipe, the run returns after a bounded wait with the output collected
so far instead of hanging. Same handling as `tools/latex-suites`
(its `_kill_tree` / `_join_reader_before_close`), adapted here so
`run.py` stays stdlib-only. `capture()` takes the same timeout as a
keyword argument; a hang raises `subprocess.TimeoutExpired` only
after the group is killed.

## Shell escape

DESIGN §4.5 runs shell escape RESTRICTED by default, exactly as in
TeX Live's pdflatex: both engines run in their default mode, so the
single module-level setting is `ENGINE_SHELL_FLAGS = []`.
`capture()` appends that setting for every run (reference and
candidate, `-ini` and `-fmt` modes) — so the CLI, which only runs
engines through `capture()`, inherits it. That is the one place to
change it: setting it to `['-no-shell-escape']` removes the log status
line ` restricted \write18 enabled.` from both engines' logs, and
`\pdfshellescape` traces as 0 instead of 2 (both verified against
pdfTeX 1.40.29).

## Program name

Warnings print argv[0], and outside `-ini` mode the invoked name even
selects the format (`preloaded format=<name>`, `mktexfmt <name>.fmt` —
verified: invoking the reference through a link named `othername`
sends it looking for `othername.fmt`). So both engines must be invoked
through paths that print identically: for each run `capture()` creates
a per-engine bin directory inside the temp workdir
(`.lockstep-bin-<hash of the resolved binary>`) containing a symlink
named `pdftex` that points at the resolved engine binary, and executes
that symlink. The program name is never normalised in the log text.
The reference still finds its configuration when run through such a
symlink — kpathsea resolves symlinks — verified with a real run in
both `-ini` mode and `-fmt=pdflatex` mode (format loads, output
written).

A differing case prints the first differing log line plus a window
around the first differing column, so long lines read from the
differing region rather than only from their start.

Replaced lines are kept as `accounting`: `capture()` returns the
originals (before replacing, so the `Output written on` entry keeps the
real byte count),
and the CLI diffs them per case as the non-gating check above, labelled
`memory usage` / `pdf stats` / `pdf bytes`.

Deliberate deviations from a literal `\tracingall`, verified against
pdfTeX 1.40.29: `\tracingall` is a plain.tex macro, undefined in `-ini`
mode, so the prelude sets the equivalent switches; the e-TeX tracing
switches work in `-ini` mode as long as the engine gets `-etex` (the log
shows `entering extended mode`), so both engines run with `-ini -etex`;
`\tracingstats` stays 0 (it reports engine-internal memory use);
`max_print_line`/`error_line` are texmf.cnf values, raised via
`-cnf-line`; explicit `\showbox` is avoided because its `! OK.`
pseudo-error is fatal under `-halt-on-error`.

## `capture()` API (for other tools)

`run.py` exposes one importable entry point; the CLI calls it too, so
there is exactly one capture path:

```python
from tools.lockstep.run import capture  # or: import run; run.capture(...)
cap = capture(tex_path, engine_bin, workdir, *, fmt=None, extra_env=None,
              timeout=300)
```

`cap` is a `Capture` with `log` (normalised transcript, always a plain
`str`; when the engine leaves a pre-existing log untouched after a
nonzero exit, the captured stdout becomes the log instead of an empty
string — never for an exit-0 run), `boxes` (list of
normalised strings, one per shipout box dump),
`pdf_path` (produced PDF path, or `None`), `returncode`, and `accounting`
(the §1.1 original lines the comparison replaces by placeholders; `log`
itself stays the full normalised transcript). The run uses
cwd=`workdir` and never wipes or cleans files already there: `tex_path`
may be a file inside `workdir` (stage a source tree, run convergence
passes for `.aux`/`.toc`, then call `capture` for the one traced pass).
`fmt=None` keeps the default `-ini -etex` primitive mode; `fmt="pdflatex"`
runs `-fmt=<fmt>` instead (no `-ini`). A `-fmt` run without
`\tracingoutput` has no shipout lines, so Memory usage lines stay in
the compared log there (strict, not a false pass). Every run appends
`ENGINE_SHELL_FLAGS` and executes a per-engine `pdftex` symlink (see
"Shell escape" and "Program name" above). `extra_env` adds variables on
top of the pinned environment. Missing binary raises
`FileNotFoundError`; timeout raises `subprocess.TimeoutExpired`.

`\nonstopmode` warning: LaTeX's `\tracingall` runs `\loggingoutput`,
which sets `\errorstopmode` — so any line injected to turn tracing on
must be followed by `\nonstopmode`, or TeX blocks on stdin at the first
error. `capture()` also passes stdin=DEVNULL, so a run that still lands
in `\errorstopmode` fails fast on EOF instead of hanging to timeout.

## Adding a case

1. Add `cases/NNN-topic.tex`: first line `\input prelude`, last lines a
   `\setbox`/ship via `\lsshipbox0` and `\end`. Observe state with
   `\message` + `\the`/`\meaning` only — never `\show*` (fatal, see above)
   and never stray printable characters (nullfont). Never emit the text
   `LOCKSTEP-BOX` and never touch `\count250` (the prelude's ship
   counter). The case must exit 0.
2. Run `run.py --self-test --cases NNN-topic` twice; outputs must be equal.
3. Regenerate `expected/` if you use it; those files are git-ignored.

## Near-duplicate check (`dupcheck.py`)

A new case that repeats an existing one adds no coverage, and a review of four waves found
duplicates in every one of them. `python3 tools/lockstep/dupcheck.py --new DIR --ref DIR
[--ref DIR ...]` compares every new case with every reference case (main, other open
branches) and with the other new cases, and reports the cases that are close by either of
two signals: CODE (Jaccard of token trigrams after stripping comments, boilerplate and
numbers) or TOPIC (Jaccard of the words of the case name and first-line description). Neither
signal alone finds the duplicates a person finds (measured on eleven known ones: code puts
the partner first for about half, topic for 9 of 11); together they list the pairs to read
side by side. It is a candidate finder, not a verdict: each flagged case must be justified in
the review or replaced, and the tool's output belongs in the PR description. Run it against
ALL other branches, including the largest one, because that is where the collisions come from.
