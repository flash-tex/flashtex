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
| `pdftex` | 0 for `--version`, `--help`, and the smoke run | `pdftex.test`: `... --version \|\| exit 1`, `... --help \|\| exit 1`; the smoke run (below) has no upstream counterpart |

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

Every artifact check requires a real regular file (`os.path.isfile and
not os.path.islink`): an engine that leaves symlinks (even to real
files) at artifact paths FAILs. Log reads (`expanded`, `cnfline`,
`wprob`) apply the same rule before reading.

- `pdftex`: beyond `--version`/`--help` exit 0, the gate runs a real
  typeset (`hello` + `\bye` under the engine's default format, written
  by the harness as `smoke.tex`) and requires exit 0 with a non-empty
  `smoke.log`. A stub that only answers the flags writes no log and
  FAILs.
- `wcfname`: upstream requires the job files to exist (`mv ... || rc=14`)
  while its own content diff stays commented out ("does not work": a byte
  compare fails because pdfTeX's `\write` escapes non-ASCII bytes as
  printable `^^XX`). The gate additionally requires every artifact
  non-empty (a stub touching empty files FAILs), decodes the `^^XX`
  escapes in each `job.txt` and byte-compares against `tests/fn-utf8.txt`,
  requires each per-document term log to carry its `JOB[<job>]` marker
  (in the same `^^XX`-escaped form the engine prints for non-ASCII names;
  log lines are joined first since the engine wraps terminal output at 79
  columns), and requires the same marker in each `job.log` — the
  generated inputs `\write16` it, so the reference run records it in
  both the terminal capture and the log file (verified against the
  reference engine).
- `pdfimage`: upstream checks only the two exit codes, so garbage
  non-empty artifacts would pass. The gate requires a non-empty
  `pdfimage.fmt` after the build, then requires `pdfimage.pdf` to start
  with `%PDF-` and end with `%%EOF`, and `pdfimage.log` to carry the
  reference run's marker lines: `Output written on pdfimage.pdf
  (3 pages,` plus the three embedded image names (`1-4.jpg`, `B.pdf`,
  `lily-ledger-broken.png`) — all observed in the reference run's log.
- `partoken`: an argv-sniffing stub that exits 0/1 while writing nothing
  would pass on exits alone. The gate requires the non-empty log the
  reference run writes for both runs, with the reference log's marker
  line in each: `PAR-TOKEN` in `partoken-ok.log` (the test's own text,
  echoed by the engine) and `Runaway argument?` in
  `partoken-xfail.log` (the engine's error on that input).
- `ttf2afm`: upstream pipes helper output through `sed '/Converted
  at/d'` and byte-compares. The gate mirrors exactly that: only the
  dateline is dropped, every other byte — line endings, and a missing
  final newline — is compared exactly (the old splitlines/join + forced
  trailing newline masked both).
- `pdftosrc`: upstream compares with plain `diff`, or `diff
  --strip-trailing-cr` where the platform diff supports it (its own
  probe: pre-generated results are LF, output may be CRLF). The gate
  runs the same probe and mirrors it: byte-exact compare, except CRLF
  maps to LF when the probe succeeds. Lone CR bytes are never masked
  (the old `replace(b'\r', b'')` hid them).

## Containment: the engine runs in a box, and the box is audited

Every engine/helper subprocess (including the `--version` gate probe and
the `locale`/`kpsewhich`/`perl` helpers) runs with `HOME`, `TMPDIR`,
`TEXMFVAR`, `TEXMFCONFIG` and `TEXMFHOME` pointed into per-test dirs
inside its work dir, so an engine cannot read or write outside it
unnoticed. After each test the driver audits two things, and either
violation FAILs the test even when its own checks passed:

- nothing new appeared in the scratch parent outside the test's work
  dir (an engine writing `../escaped.txt` is caught);
- every file in the work dir beyond the pre-run snapshot is on the
  test's documented allowlist: the inputs the harness stages plus the
  outputs the reference engine leaves (containment-dir contents are
  contained by construction and never listed).

Reference-observed allowlist (anything else FAILs):

| test | staged inputs | reference outputs |
| ---- | ------------- | ----------------- |
| `pdftex` | `smoke.tex` (harness-written) | `smoke.log`, `smoke.pdf` |
| `expanded` | `expanded.tex` | `expanded.log` |
| `cnfline` | `cnfline.tex` | `cnfline.log`, `cnfline.dvi` |
| `pdfimage` | `pdfimage.tex`, `basic.tex`, `1-4.jpg`, `B.pdf`, `lily-ledger-broken.png` | `pdfimage.fmt`, `pdfimage.log`, `pdfimage.pdf` |
| `partoken` | `partoken-ok.tex`, `partoken-xfail.tex` | `partoken-ok.log`, `partoken-ok.dvi`, `partoken-xfail.log` |
| `wprob` | `pwprob.tex` | `pwprob.log` |
| `ttf2afm` | `postV3.ttf`, `postV7.ttf` | (helper writes to stdout; nothing) |
| `pdftosrc` | `test-13.pdf`, `test-15.pdf` | `test-13.xref`, `test-15.xref` |
| `wcfname` | `fn-generate.perl` (+ generated `pdftests/`) | `pdftests/` holding `fn*-utf8.tex` inputs, `fn*-utf8-tmp*.tex` (the vir one carries a random numeric suffix from `\rnd`), `fn*-utf8-pdf.{txt,log,fls}` and `fn*-term.log` |

`missfont.log` (kpathsea's byproduct when fonts are missing) is allowed
wherever an engine runs. Residual risk, documented: a name-shaped plant
(e.g. an extra `fn-fake-utf8.tex` under `pdftests/`) matches a pattern
and is not flagged — the content checks are the backstop there.

## Known limit: self-asserted identity

The version gate trusts the engine's own `--version` line: a shell stub
printing a forged `pdfTeX ... 1.40.29` string passes it, and no outside
check can prove which binary answered. This is unavoidable for an
external binary and is documented rather than fixed. The real check is
everything after the gate — the parity tiers (exact log/afm/xref/job
compares) and the lockstep exit-code, marker, and containment rules —
plus the `pdftex` test's smoke run, which forces at least one real
typeset with a log. A stub can forge each of those individually only by
reproducing the reference bytes, which is the point.

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
