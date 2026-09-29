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

- `expanded`: upstream `expanded.test` never checks the engine exit status,
  and the gate follows it (log match only). This is deliberate: the
  reference engine exits 1 with `No pages of output.` (INITEX run, no
  `\dump`, nothing shipped), so requiring exit 0 would fail every
  conforming engine. A crash is still caught: no log means FAIL, and a
  signal death / timeout is reported as such.
- `cnfline`: upstream `cnfline.test` has no missing-log guard (a missing
  log falls into the `else` branch and `cat`s a nonexistent file); the gate
  instead reports `FAIL no cnfline.log written`, like the other log tests.
- Whole-run `--budget` and the stale-entry gate have no upstream
  counterpart; they bound and protect the gate itself, not the engine.

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
