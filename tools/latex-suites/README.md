# latex-suites: official LaTeX regression suites as an engine gate

Runs the LaTeX team's suites — latex2e `base` + `required`, latex3
`l3kernel` — against any pdfTeX-compatible engine, through **l3build's own
normalisation** (`l3build check -e pdftex`) versus the committed upstream
`.tlg` references. MIT-licensed; it only runs external programs.

## Quick start

```sh
sh tools/latex-suites/fetch.sh            # pinned checkouts into .cache/
python3 tools/latex-suites/run.py --engine /Library/TeX/texbin/pdftex --suite base
```

A gate script calls one line per engine build:

```sh
python3 tools/latex-suites/run.py --engine "$BIN" --suite all
```

`--tests name,...` runs a subset (names are `testfiles/*.lvt` basenames);
`--list` counts those files. Per-directory `PASS n / FAIL m / SKIP k`
lines plus failing test names are printed. Exit 0 iff all failures are
in `EXPECTED-FAILURES.txt`; 1 on unexpected failures (or crashed/
unresolved tests); 2 on infra errors (including a refused engine).
`--timeout-dir SECONDS` (default 1800) kills the l3build process group
if a directory takes longer and FAILs every unfinished test as
`[UNEXPECTED] (timeout)` (exit 1).

## Reference engine

The default reference is pdfTeX 1.40.29 (TeX Live 2026). `run.py`
refuses any other `<engine> --version` banner (exit 2) unless
`--allow-any-engine` is passed, which prints a warning and proceeds —
that flag is the only exemption for a candidate engine under test.

## How it works

Per suite dir, `run.py` puts a recording shim `pdftex` (runs `--engine`,
logs every exit code + argv) first on `PATH` and runs
`l3build check -e pdftex`. Failure evidence is the `*.pdftex.diff`
files under `<checkout>/build/test*/`. Each run starts with
`l3build clean` so results never depend on earlier installs (stale
`build/local` files flip e.g. `github-1336`) and a shim that writes
nothing can never inherit a previous run's results.

Crash safety: if the engine process dies (crash, kill, any exit other
than pdfTeX's 0/1 — TeX-level errors exit 1 and stay l3build's call),
l3build aborts with a Lua assertion, no `--> failed` line and no diff.
The shim's call log maps the death to its `-jobname=` test, and any
started-but-unresolved test (or, if l3build dies before any test, every
requested test) is reported FAIL, never PASS or SKIP.

Hang safety: l3build's unpack/format steps invoke the engine WITHOUT
`-interaction=nonstopmode`, so a broken engine drops pdftex into an
errorstopmode `Please type another input file name:` prompt on stdin
(a shim that runs the real engine then exits nonzero hung this way for
1.5 h at 0% CPU under a harness holding stdin open). `run.py` therefore
runs l3build with stdin from `/dev/null` (prompts emergency-stop at
once) in its own process group, and enforces the per-directory timeout
above (SIGTERM, then unconditional SIGKILL after a 5 s grace, whole
group, reaped on Ctrl-C too). The SIGKILL fires even when l3build
itself already exited on SIGTERM, so an engine that ignores SIGTERM
(a `trap '' TERM` loop hung the runner forever before this fix) cannot
delay the return past timeout + grace + a few seconds; the stdout pipe
is likewise never closed while the reader thread is still blocked.
l3build has no per-test timeout flag, so the directory timeout is the
enforcement point. A setsid-detached grandchild is outside the process
group: it cannot be killed by the group kill and is only reaped by the
OS; the runner stops waiting on its pipe after a short bounded grace
and returns with the output collected so far.

All dirs run sequentially: every latex2e dir shares `<repo>/build` via
its `maindir` (build root plus `build/local` installs), so overlap
would corrupt results — there is intentionally no `--jobs` flag. SKIP
is always 0: this l3build emits no per-test skip lines (config-level
`Skipping unknown engine` lines are whole-config and never attributed
to a test). A dir that runs 0 tests (graphics targets etex/xetex, so
`-e pdftex` matches nothing) prints a WARNING.

## Totals (reference engine, 2026-09-29)

Per-dir check executions (test × l3build config), `--suite all` exit 0:

| dir | PASS | FAIL | notes |
|---|---|---|---|
| latex2e/base | 799 | 1 | xmarks-009 (expected) |
| latex2e/required/cyrillic | 1 | 0 | |
| latex2e/required/graphics | 0 | 0 | checkengines etex/xetex: pdftex runs nothing (WARNING) |
| latex2e/required/tools | 130 | 2 | github-1814, tlb2914 (expected) |
| latex2e/required/amsmath | 39 | 0 | |
| latex2e/required/firstaid | 23 | 0 | |
| latex2e/required/latex-lab | 324 | 6 | tagging-status, scrartcl-001, table-006/007-longtable, table-012-caption, test-ltugboat (expected) |
| latex3/l3kernel | 206 | 0 | |
| total executed | 1522 | 9 | 1531 executions, 1529 unique names (github-0524, github-1336 run in two dirs) |

Three different "totals" exist; do not mix them: `--list` counts
`testfiles/*.lvt` files only (989), but base/tools/latex-lab/firstaid
also run sibling `testfiles-*` dirs, so executed checks (1531) are
higher; unique test names (1529) are lower by the 2 cross-dir dupes.
The Commander's ~1,789 counts every `.lvt` in both checkouts (~1,712 at
this pin, including non-gated suites such as l3experimental/l3packages
and other-engine TU/luatex dirs); the gated pdfTeX total is smaller
because graphics' 31 tests are etex/xetex-only and TU/luatex/disabled/
broken/local-only dirs don't run under `-e pdftex` (remainder is pin
drift in the estimate).

## Adding an expected failure

Only for failures reproduced with the reference engine that are
upstream/environmental (never for the engine under test):

1. Reproduce with the bare engine (e.g. run the `.lvt` via l3build with
   the system pdfTeX, or a minimal document showing the same engine
   wording) and confirm the committed `.tlg`/sources expect the old
   behaviour.
2. Append one line `<testname>: <one-line reason>` to
   `EXPECTED-FAILURES.txt`.
3. Re-run `--suite all` on the reference; it must exit 0 with the new
   entry listed `[expected]`, and no `stale ... now passing` line may
   name a test you didn't fix (stale = listed test now passes → remove
   it from the file).

`python3 tools/latex-suites/test_run.py` (stdlib only, 22 tests) covers
the transcript parser, crash attribution, the engine gate, and the
timeout path (sleeping fake l3build, process-group kill, timeout FAILs,
SIGTERM-ignoring child and grandchild returning within
timeout + grace + seconds with no survivors).

## Pins

`PINS.txt` holds `<repo> <tag> <sha>` for `latex2e`/`latex3`, matching the
installed TeX Live (now: format `2025-11-01`, expl3 `2026-01-19`). `fetch.sh`
shallow-clones them into `.cache/` (git-ignored; never committed) and aborts
on SHA mismatch. To bump: read the installed `\fmtversion` (`kpsewhich
latex.ltx`) and `\ExplFileDate` (`kpsewhich expl3.sty`), find the matching
tags (`release-YYYY-MM-DD` / `YYYY-MM-DD`), update `PINS.txt`, re-fetch, and
re-baseline `EXPECTED-FAILURES.txt` with the system pdfTeX.

## Limits (slice 2)

- `--tests` selects `testfiles/*.lvt` basenames only; tests living in
  sibling `testfiles-*` dirs (e.g. base `testfiles-ltmarks/xmarks-009`)
  can't be singled out (a full-dir run covers them).
- The shim also serves l3build's unpack step, so the engine under test must
  run `--fmt=pdflatex` unpack jobs; a from-scratch engine needs unpack with
  system pdfTeX first (future work).
