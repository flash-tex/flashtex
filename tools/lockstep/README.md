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
  -ini -etex -interaction=nonstopmode -halt-on-error \
  -no-shell-escape <name>.tex
```

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
  after its box dump (a 2-shipout log carries 2).

The `<...pfb>` font-list line that follows the memory block in logs
using real fonts matches no shape, so it stays compared. Everything
else — tracing, messages, box dumps — must match exactly. This is the
same placeholder replacement rule `tools/parity` uses for its P-T1
comparison (that tool is updated separately by its owner).

Removed originals are still reported as non-gating `accounting`
(see below); the compared log keeps the placeholders.

## Output checks

A log that says `Output written on <file>` claims a PDF was produced:
the job PDF must then exist next to the log, be non-empty, start with
`%PDF-` and end with `%%EOF` (trailing whitespace allowed), or the case
FAILs — a wrapper that runs the real pdftex and then deletes `job.pdf`,
or appends garbage lines to it, FAILs. This check is structural only;
byte-level PDF equality is another tool's job (`tools/parity` P-T2),
not this harness's.

A run must produce its own `job.log`: the stdout fallback (using the
captured stdout as the log) applies only to a run that produced no log
because the engine failed to start (nonzero exit). It never applies to
a run that exits 0, so a wrapper that deletes `job.log` and prints the
reference transcript on stdout with exit 0 FAILs (`exit 0, no log`).

## Shell escape

DESIGN §4.5 keeps shell escape OFF by default. `run.py` runs both
engines with `-no-shell-escape`, from the single module-level setting
`ENGINE_SHELL_FLAGS = ['-no-shell-escape']`, which `capture()` appends
for every run (reference and candidate, `-ini` and `-fmt` modes) — so
the CLI, which only runs engines through `capture()`, inherits it.
That is the one place to change it. Without the flag every case would
differ on the log status line ` restricted \write18 enabled.`, and
`\pdfshellescape` traces as 2 instead of 0 (both verified against
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
cap = capture(tex_path, engine_bin, workdir, *, fmt=None, extra_env=None)
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
