# Lockstep harness (slice 4)

Reference pin: pdfTeX 1.40.29. `run.py` refuses any other reference:
`<reference> --version` must report "1.40.29" in its first line, else it
exits 2; `--allow-any-reference` overrides with a warning. `capture()`
enforces the same pin only with `require_reference_version=True` (cached
per binary path via `check_reference_version()`).

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

# regenerate expected/<name>.log from the reference (never hand-written)
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

with `SOURCE_DATE_EPOCH=0`, `FORCE_SOURCE_DATE=1`, `TZ=UTC`, cwd set to
the run dir, and stdin from DEVNULL. The prelude boots INITEX catcodes,
sets `\pdfoutput=1`, turns all behaviour tracing to maximum — including
the e-TeX set (`\tracingassigns`, `\tracinggroups`, `\tracingifs`,
`\tracingscantokens`, `\tracingnesting`), which needs `-etex` —
sets `\showboxdepth=\showboxbreadth=10000`, and defines `\lsshipbox#1`,
which numbers the ship, writes a `LOCKSTEP-BOX <n>` marker to the log via
`\message`, then ships box register `#1`; every shipped box is dumped to
the log by `\tracingoutput=1`. `capture()` splits the normalised log on
that marker to recover one string per shipped box dump (`boxes`).

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

The byte count is replaced by the fixed token `<BYTES>`; the page count,
every glue value and every trace line stay strictly compared. Everything
else — tracing, messages, box dumps — must match exactly. This is the
same normalised set `tools/parity` uses for its P-T1 comparison.

Removed lines are kept as `accounting`: `capture()` returns them (before
replacing, so the `Output written on` entry keeps the real byte count),
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
`str`), `boxes` (list of normalised strings, one per shipout box dump),
`pdf_path` (produced PDF path, or `None`), `returncode`, and `accounting`
(the §1.1 lines the comparison normalises away, before replacing; `log`
itself stays the full normalised transcript). The run uses
cwd=`workdir` and never wipes or cleans files already there: `tex_path`
may be a file inside `workdir` (stage a source tree, run convergence
passes for `.aux`/`.toc`, then call `capture` for the one traced pass).
`fmt=None` keeps the default `-ini -etex` primitive mode; `fmt="pdflatex"`
runs `-fmt=<fmt>` instead (no `-ini`). `extra_env` adds variables on top
of the pinned environment. Missing binary raises `FileNotFoundError`;
timeout raises `subprocess.TimeoutExpired`.

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
