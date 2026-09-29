# T0 pdfTeX regression harness (MIT, stdlib only)

Runs pdfTeX's own regression tests — the `TESTS` from
`texk/web2c/pdftexdir/am/pdftex.am` (`pdftex`, `expanded`, `pdfimage`,
`wprob`, `cnfline`, `partoken`, `wcfname`) plus `ttf2afm` and `pdftosrc` —
against any engine binary. Each test runs in a fresh temp dir with the
environment its upstream `.test` script expects, stdin from `/dev/null`,
a per-test timeout with process-group kill, and the comparison (log / afm /
xref) after upstream's own normalisation.

Upstream files are never committed here: `fetch.sh` materialises the pinned
texlive-source checkout into `.cache/` (git-ignored); `run.py` reads inputs
from there at run time.

## Pins

See `PINS.txt`: texlive-source at
`3d3a4b2b66ed37e1fc16078b26451e1d9402f86b` (no `texlive-2026` tag exists
upstream, so the full SHA is the pin), carrying pdfTeX 1.40.29. Reference
engine: `/Library/TeX/texbin/pdftex` (pdfTeX 1.40.29, TeX Live 2026).

## Usage

```sh
sh tools/pdftex-regress/fetch.sh
python3 tools/pdftex-regress/run.py --engine /Library/TeX/texbin/pdftex
python3 tools/pdftex-regress/run.py --engine <bin> --tests pdftex,expanded
python3 tools/pdftex-regress/run.py --engine <bin> --list
python3 tools/pdftex-regress/run.py --engine <bin> --timeout 60 --allow-any-engine
python3 tools/pdftex-regress/test_run.py   # unit tests
```

`run.py` prints per-test `PASS/FAIL/SKIP` lines, then
`PASS n / FAIL m / SKIP k` and the failing names. Exit 0 only when every
failure is listed in `EXPECTED-FAILURES.txt`; usage/config errors exit 2.
The engine must report pdfTeX 1.40.29 unless `--allow-any-engine` is given.
Each engine call may use a full `--timeout`, so a hanging engine is bounded
by the whole-run `--budget SECONDS` (default 1800): once exhausted, the
remaining tests report `FAIL (budget exhausted)` and the gate exits 1.
An `EXPECTED-FAILURES.txt` entry for a test that now passes is stale and
fails the gate (exit 1) unless `--allow-stale` is given, so the list cannot
rot; entries for tests outside `--tests` are never stale.

## Deviations from upstream

Every engine invocation is checked for timeout first, then for a crash
(`None` after the timeout kill, or a negative return code = signal
death), which always FAILs. The remaining tolerated codes follow each
upstream `.test` script:

| test | engine exit required | upstream source |
| ---- | -------------------- | --------------- |
| `expanded` | 0 or 1 (log must still match) | `expanded.test` never checks the exit; 1 is what the reference engine reports (`No pages of output.`, INITEX run, no `\dump`, nothing shipped), so the gate tolerates 0/1 and FAILs crashes and all other codes |
| `cnfline` | 0 | `cnfline.test`: `... cnfline.tex \|\| exit $?` |
| `pdfimage` | 0 (fmt build) and 0 (fmt run) | `pdfimage.test`: `... \|\| exit 1`, `... \|\| exit 2` |
| `partoken` | ok run 0; xfail run nonzero | `partoken.test`: `if ... partoken-ok.tex; then :; else exit 1`, `if ... partoken-xfail.tex; then exit 1` |
| `wprob` | nonzero (must fail) | `wprob.test`: `... pwprob.tex && exit 1`, then the log grep |

- `expanded`: see the table above. Upstream `expanded.test` never checks
  the engine exit status; the gate hardens it to 0/1 (the reference
  engine exits 1) while still requiring the log to match.
- `cnfline`: upstream `cnfline.test` has no missing-log guard (a missing
  log falls into the `else` branch and `cat`s a nonexistent file); the gate
  instead reports `FAIL no cnfline.log written`, like the other log tests.
- Whole-run `--budget` and the stale-entry gate have no upstream
  counterpart; they bound and protect the gate itself, not the engine.

## Isolation: the engine cannot reach the expected files

Each test copies only its inputs into the fresh temp work dir — never the
expected files — and `TEXINPUTS`/`TEXFORMATS` point at that dir (with a
trailing `:` preserving the engine's own TeX Live tree defaults), never at
the upstream checkout. The `ttf2afm`/`pdftosrc` inputs are invoked by
relative name. The expected files (`expanded.txt`, `*.afm`, `*.xref`,
`tests/fn-utf8.txt`) are read by the runner only, after the run. Leak
shims — one that symlinks `expanded.log` at the expected file found via
`$TEXINPUTS`, one that reads and wraps it, and a fake helper that cats a
sibling of an absolute argv path — all FAIL (`test_run.py` covers each).

## Artifact checks beyond upstream's exit codes

- `wcfname`: upstream requires the job files to exist (`mv ... || rc=14`)
  while its own content diff stays commented out ("does not work": a byte
  compare fails because pdfTeX's `\write` escapes non-ASCII bytes as
  printable `^^XX`). The gate additionally requires every artifact
  non-empty (a stub touching empty files FAILs), decodes the `^^XX`
  escapes in each `job.txt` and byte-compares against `tests/fn-utf8.txt`,
  and requires each per-document term log to carry its `JOB[<job>]` marker
  (in the same `^^XX`-escaped form the engine prints for non-ASCII names;
  log lines are joined first since the engine wraps terminal output at 79
  columns).
- `pdfimage`: upstream checks only the two exit codes, so a stub that
  touches an empty `pdfimage.fmt` and exits 0 twice would pass. The gate
  requires a non-empty `pdfimage.fmt` after the build and non-empty
  `pdfimage.pdf`/`pdfimage.log` after the run.

## Upstream-faithful weak oracles

Where upstream's oracle really is only a substring grep, the gate keeps it
and documents it rather than inventing a stronger check (beyond requiring
the log to exist and be non-empty): `cnfline` passes on one log line
containing `those hyphens are` (`cnfline.test`'s grep); `wprob` passes on
the anchored `Could not open file NoSuchFile.eps.` line (`wprob.test`'s
grep). `expanded` compares the full `START..END` block, and
`ttf2afm`/`pdftosrc` compare full outputs, exactly as upstream does.

## Empty runs fail

Exit is nonzero (with the reason printed) when no test produced `PASS` or
`FAIL` — e.g. `--tests ttf2afm,pdftosrc` with no helper binaries reports
`PASS 0 / FAIL 0 / SKIP 2` and exits 1 — and when any test explicitly named
with `--tests` was skipped.

## Totals (reference engine, 2026-09-29)

`PASS 9 / FAIL 0 / SKIP 0`, exit 0 — identical on two consecutive runs.
`wcfname` passes in every `locale -a` matrix locale present here
(`C.UTF-8`, `en_US.UTF-8`, `ja_JP.UTF-8`, 4 docs each); the absent matrix
locales (`C.utf8`, `en_US.utf8`, `ja_JP.utf8`) are named in its PASS detail
instead of being run. Without kpsewhich/perl/`locale`, or with none of the
matrix locales installed, `wcfname` SKIP­s with the precise reason.

## Gate-script call

```sh
python3 tools/pdftex-regress/run.py --engine "$ENGINE_BIN" --timeout 300 \
  || exit 1
```
