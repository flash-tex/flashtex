# latex-suites: official LaTeX regression suites as an engine gate

Runs the LaTeX team's suites — latex2e `base` + `required`, latex3
`l3kernel` — against any pdfTeX-compatible engine, through **l3build's own
normalisation** (`l3build check` with a per-dir `-e` engine) versus the
committed upstream `.tlg` references. Most dirs run `-e pdftex`;
`required/graphics` runs `-e etex` and l3kernel's `testfiles-backend`
additionally runs `-c config-backend` with `-e etex-dvips`/`etex-dvisvgm`
(DVI mode). MIT-licensed; it only runs external programs.

## Quick start

```sh
sh tools/latex-suites/fetch.sh            # pinned checkouts into .cache/
python3 tools/latex-suites/run.py --engine /Library/TeX/texbin/pdftex --suite base
```

A gate script calls one line per engine build:

```sh
python3 tools/latex-suites/run.py --engine "$BIN" --suite all
```

`--tests name,...` runs a subset (names are `testfiles/*.lvt` basenames,
`testfiles-backend` for the two backend rows);
`--list` counts those files. Per-directory `PASS n / FAIL m / SKIP k`
lines plus failing test names are printed. Exit 0 iff every failure is
an ordinary diff mismatch listed in `EXPECTED-FAILURES.txt` (a `.diff`
was produced, with a matching recorded hash when the entry has one);
1 on unexpected failures, crashed/unresolved tests, explicitly requested
tests that never ran, a listed test failing with a *different* diff, or
stale entries (listed tests that now pass; `--allow-stale` downgrades
those to a warning); 2 on infra errors (including a refused engine).
`--timeout-dir SECONDS` (default 1800) kills the l3build process group
if a directory takes longer and FAILs every unfinished test as
`[UNEXPECTED] (timeout)` (exit 1). Timeouts, engine deaths, and
never-ran tests are never excused by a listing: a crash on a listed
test is `[UNEXPECTED]`, not `[expected]`.

## Reference engine

The default reference is pdfTeX 1.40.29 (TeX Live 2026). `run.py`
refuses any other `<engine> --version` banner (exit 2) unless
`--allow-any-engine` is passed, which prints a warning and proceeds —
that flag is the only exemption for a candidate engine under test.
The version must match as a whole token on the first `--version` line
(`(?<![\d.])1\.40\.29(?![\d.])`), so a `1.40.290` engine is refused.

## Candidate engine environment (`--engine-env`)

A candidate engine may need its own files before it can run a suite:
l3build's unpack step runs `pdftex` as a production run with the default
format `pdftex.fmt`, so without the candidate's format every directory
aborts before any check. Repeatable `--engine-env KEY=VALUE` exports
those variables for the engine under test ONLY: the recording `pdftex`
shim `run.py` generates sets them just before it execs the engine, and
they are never present in the environment of l3build itself, of the
reference engine, or of the reference version probe (the version gate
also runs on a candidate engine, so a candidate probe may see them).
Every *other* `FLASHTEX_*` the parent shell exports is scrubbed from
all three environments, so only flagged values can reach the engine.
A pair with no `=`, an empty key, or a key outside
`[A-Za-z_][A-Za-z0-9_]*` is rejected with exit 2 (the key lands
unquoted in the shim's `export KEY=VALUE` line, where a `;` would run
as a shell command); values stay quoted and arrive literally
(`FLAG POOL;$x` stays data).

```sh
python3 tools/latex-suites/run.py --engine "$CANDIDATE" --suite all \
  --allow-any-engine \
  --engine-env FLASHTEX_FORMATS=/tmp/candidate-fmt \
  --engine-env FLASHTEX_POOL="$REPO/crates/flashtex-engine/pdftex.pool"
```

`FLASHTEX_FORMATS` is the directory holding this engine's `pdftex.fmt`;
`FLASHTEX_POOL` is the engine's string pool file. Build the format the
fmtutil way in an output directory (whose `pdftex.fmt` is then the
directory given as `FLASHTEX_FORMATS`):

```sh
mkdir -p /tmp/candidate-fmt && cd /tmp/candidate-fmt
flashtex-initex -ini -jobname=pdftex -progname=pdftex \
  -translate-file=cp227.tcx '*pdfetex.ini'
```

The reference pdfTeX ignores both variables: reference runs never pass
`--engine-env`, so a reference verdict can never depend on them.

## How it works

Per suite dir, `run.py` puts recording shims `pdftex` and `etex`
(each runs `--engine`, logs every exit code + argv) first on `PATH` and
runs `l3build check` with that dir's `-e` engine (and `-c` configs).
The `pdftex` shim serves l3build's unpack step (stdengine default) while
the `etex` shim serves DVI-mode format builds and check runs: it execs
the engine under test with `-progname=etex`, reproducing the real TeX
Live `etex` symlink (verified: the `.ins` unpack runs land on the pdftex
shim, the `-etex -ini` format build and `--fmt=latex` check runs on the
etex shim, and verdicts match the real `etex` binary). Failure evidence
is the `*.<engine>.diff` files under `<checkout>/build/test*/` (the
suffix follows the `-e` engine: `.pdftex.diff`, `.etex.diff`,
`.etex-dvips.diff`, ...). Each run starts with
`l3build clean` so results never depend on earlier installs (stale
`build/local` files flip e.g. `github-1336`) and a shim that writes
nothing can never inherit a previous run's results.

Crash safety: if the engine process dies (crash, kill, any exit other
than pdfTeX's 0/1 — TeX-level errors exit 1 and stay l3build's call),
l3build aborts with a Lua assertion, no `--> failed` line and no diff.
The shim's call log maps the death to its `-jobname=` test, and any
started-but-unresolved test (or, if l3build dies before any test, every
requested test) is reported FAIL, never PASS or SKIP. Such deaths are
never excused by `EXPECTED-FAILURES.txt`, even when the test is listed.

Expected-failure matching is by verdict, not by name alone: each entry
may read `<testname>: sha256=<hex> <one-line reason>`, where `<hex>` is
the SHA-256 of the normalised reference diff the reference engine
produces (file-header lines with run dates/paths dropped, whitespace
normalised). A listed test is `[expected]` only if it failed with an
ordinary diff mismatch *and* the hash matches; a different diff for the
same test is `[UNEXPECTED]`. Old `<testname>: <reason>` lines without a
hash still excuse any produced diff. Regenerate hashes with the
reference engine via `--update-baseline` (only existing entries with an
observed diff are refreshed; new failures need a human reason line
first). Only `.diff` files created or rewritten during the run count:
a directory's `l3build clean` does not wipe the whole shared
`<repo>/build`, so older runs' and other directories' `.diffs` linger
and are ignored via a before/after snapshot.

Sanity: explicitly `--tests`-requested tests that never appear in the
transcript FAIL (even when l3build exits 0), so a silent l3build can
never report `PASS 0 / FAIL 0` for requested tests. Unfiltered runs are
exempt (an `-e`/`-c` combination matching nothing runs 0 tests).

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
to a test). A dir that runs 0 tests prints a WARNING (no INCOMPLETE
configs remain: `dvips`/`dvisvgm` are installed, so both backend
variants run to completion).

Because the build root is shared, a later directory's `l3build clean`
deletes earlier directories' `.pdftex.diff` files — after `--suite all`
only the last failing dir's diffs survive on disk. `--keep-diffs DIR`
copies each failing test's `.diff` into DIR (as
`<repo>_<dir>__<test>.pdftex.diff`, with a config-dir infix when one
test fails in several configs) right after its directory runs, before
the next clean.

## Totals (reference engine, 2026-09-29)

Per-dir check results, `--suite all` exit 0 (PASS/FAIL count unique test
names per dir; executions count test × l3build config):

| dir | PASS | FAIL | notes |
|---|---|---|---|
| latex2e/base | 799 | 1 | xmarks-009 (expected) |
| latex2e/required/cyrillic | 1 | 0 | |
| latex2e/required/graphics@etex | 31 | 0 | `-e etex` DVI mode, all pass |
| latex2e/required/tools | 130 | 2 | github-1814, tlb2914 (expected) |
| latex2e/required/amsmath | 39 | 0 | |
| latex2e/required/firstaid | 23 | 0 | |
| latex2e/required/latex-lab | 324 | 6 | tagging-status, scrartcl-001, table-006/007-longtable, table-012-caption, test-ltugboat (expected) |
| latex3/l3kernel | 206 | 0 | |
| latex3/l3kernel[config-backend]@etex-dvips | 14 | 0 | DVI mode, all pass |
| latex3/l3kernel[config-backend]@etex-dvisvgm | 14 | 0 | DVI mode, all pass |
| total | 1581 | 9 | 1591 executions, 1560 unique names (github-0524, github-1336 run in two dirs; footmisc-005 runs in two latex-lab configs; the 14 backend names run under three engines) |

Reconciliation of executions (test × config) per dir — every number
below was reproduced from the pinned checkouts (`.lvt`/`.pvt`/`.lit`
files per `testfiles*` dir against each `build.lua`'s `checkconfigs`
and each config's `checkengines`) and the reference transcripts
(`--list` plus the per-config `name (i/n)` counts; each config's
ran-set equals its dir's files exactly):

| dir | executions |
|---|---|
| latex2e/base | 800 |
| latex2e/required/cyrillic | 1 |
| latex2e/required/graphics@etex | 31 |
| latex2e/required/tools | 132 |
| latex2e/required/amsmath | 39 |
| latex2e/required/firstaid | 23 |
| latex2e/required/latex-lab | 331 |
| latex3/l3kernel | 206 |
| latex3/l3kernel[config-backend]@etex-dvips | 14 |
| latex3/l3kernel[config-backend]@etex-dvisvgm | 14 |
| total | 1591 |

Derivation (`.lvt` + `.pvt` PDF tests per config dir; l3build runs both):

- base 800 = testfiles 583 + 1run 3 + doc 29 + legacy 23 + ltcmd 16
  + lthooks 98 + lthooks2 28 + ltmarks 13 + lttemplates 7
  (config-TU runs 0 under `-e pdftex`).
- tools 132 = testfiles 110 + legacy 1 + search 21 (TU 0).
- amsmath 39, firstaid 23, cyrillic 1 — build config only (TU 0;
  cyrillic has no second config).
- latex-lab 331 = build 18 + OR 7 + math 38 + sec 14 + toc 12
  + block 53 + graphic 23 + minipage 7 + float 16 + footnote 69
  + bib 10 + LM 5 + table-pdftex 37 + title 10 + firstaid 12
  (the three `*-luatex` configs run 0). Mixed dirs explain the
  non-obvious ones: build is 17 `.lvt` + `standard-a4f.pvt`,
  table-pdftex 24 + 13, toc 2 + 10, title 2 + 8, sec 7 + 7,
  bib 4 + 6, graphic 15 + 8, float 12 + 4, math 35 + 3,
  OR 4 + 3, minipage 6 + 1, firstaid 10 + 2.
- l3kernel 206 = build 185 + backend 14 + l3doc 5 + plain 2
  (l3doc is 4 `.lvt` + the `test-index.lit` index test;
  ptex/context run 0).
- graphics 31 = testfiles 31 under `-e etex` (the xetex target is not
  run; the old `-e pdftex` runs executed 0).
- backend DVI 14 + 14 = the same 14 `testfiles-backend` tests under
  `-c config-backend -e etex-dvips` and `-e etex-dvisvgm` (their
  `.etex-dvips`/`.etex-dvisvgm` reference variants select the engine).

Test dirs upstream leaves out of `checkconfigs` (on disk and/or as an
unused `config-*.lua`, never run): base `testfiles-search` (34),
`testfiles-disabled` (14, no config file at all), `testfiles-broken`
(no such dir; `config-broken.lua` exists but is unlisted),
`testfiles-filename` (2; `config-filenames.lua` names the nonexistent
`testfiles-filenames` and is unlisted); firstaid `testfiles-pdf` and
`testfiles-local` (`config-pdf.lua` / `config-local.lua` exist but are
unlisted); latex-lab `testfiles-OR-local` (`config-OR-local.lua`
exists but is unlisted), `testfiles-only-local`, `testfiles-broken`,
the `testfiles-math/BROKEN` marker, and `math-tagging-examples/`
(luatex-only `config-math-tagging-examples.lua`, unlisted).
Listed configs whose `checkengines` exclude pdftex, hence 0 executions
under `-e pdftex`: every `config-TU` (xetex/luatex), latex-lab
`config-math-luatex` / `config-OR-luatex` / `config-table-luatex`
(luatex), l3kernel `config-ptex` (ptex) and `config-context`
(luametatex, luatex).

Three different "totals" exist; do not mix them: `--list` counts
`testfiles/*.lvt` files per entry (1017: the 989 `testfiles/` files
plus the 14 `testfiles-backend/` files listed twice, once per DVI
variant; `.pvt` files are not counted, so latex-lab lists 17 while
its build config executes 18), but base/tools/latex-lab/l3kernel also
run sibling `testfiles-*` dirs (plus l3doc's `.lit` test), so executed
checks (1591) are higher; unique test names (1560) are lower by the 2
cross-dir dupes (github-0524, github-1336, each in base and tools
`testfiles/`) while footmisc-005 runs in two latex-lab configs —
`footmisc-005 (1/7)` in the OR config and `footmisc-005 (7/69)` in
the footnote config — adding one execution without a new name, and
the 14 backend names each run under three engines (pdftex plus the two
DVI variants).
l3kernel's `testfiles-backend` tests each run once under pdftex (their
`.xetex`/`.luatex`/`.uptex` reference variants select other engines
and contribute nothing under `-e pdftex`) and once more under each of
`etex-dvips`/`etex-dvisvgm`; required/graphics runs its 31 tests under
`-e etex` (the xetex target is not run).
Upstream main holds 1,522 latex2e `.lvt` (base+required) plus 267 for
the whole latex3 repo = 1,789; at these pins the same count is
1,448 + 263 = 1,711 (`git ls-files '*.lvt'` in each checkout, all of
latex2e's inside base/ + required/). The gated total is
smaller (1,591 executions) because of the exclusions above.

## Adding an expected failure

Only for failures reproduced with the reference engine that are
upstream/environmental (never for the engine under test):

1. Reproduce with the bare engine (e.g. run the `.lvt` via l3build with
   the system pdfTeX, or a minimal document showing the same engine
   wording) and confirm the committed `.tlg`/sources expect the old
   behaviour.
2. Append one line `<testname>: <one-line reason>` to
   `EXPECTED-FAILURES.txt` (no hash yet: a legacy line excuses the
   produced diff).
3. Re-run the failing suite on the reference with `--update-baseline`;
   it fills in the entry's `sha256=` token from the reference diff, and
   the run must exit 0 with the entry listed `[expected]` and no
   `stale ... now passing` line naming a test you didn't fix (stale =
   listed test now passes → remove it from the file, or pass
   `--allow-stale` to keep the run green while you investigate).

`python3 tools/latex-suites/test_run.py` (stdlib only, 51 tests) covers
the transcript parser, crash attribution, the engine gate (including
the whole-token version match and listed-crash/mismatched-diff/stale/
never-ran UNEXPECTED verdicts), the expected-failure file format and
diff hashing, and the timeout path (sleeping fake l3build,
process-group kill, timeout FAILs, SIGTERM-ignoring child and
grandchild returning within timeout + grace + seconds with no
survivors), `--engine-env` (candidate-only delivery through the
shim, scrub of unflagged `FLASHTEX_*` from l3build/engine/probe,
key-validation exit 2, literal metachar values, repeatability), and
the `etex` shim (generation alongside `pdftex`, `-progname=etex`
first arg, candidate-only env through it, death detection and
per-engine diff scoping through it, `-c`/`-e` command shape).

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
