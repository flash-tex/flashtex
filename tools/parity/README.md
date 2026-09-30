# Parity scoreboard: how close is FlashTeX to pdfLaTeX, measured

Owner: lane PARITY-SCOREBOARD (kabir-claude, mac-m5pro-kabir). Created
2026-09-21; DESIGN §1.1 tiers added by lane P0-PARITY-TIERS (flashtex-2a).
Python 3 standard library, plus `qpdf` (Apache-2.0) run as an external
program for P-T2. **pdflatex (MacTeX) is the oracle and never runs in the
product path.**

The engine under test is `--engine <bin>` (`--flashtex` still works):

- **the shipped command line** (default), `flashtex build`, which makes the
  same exact-route PDF the Mac app exports. It runs on the untouched source
  tree;
- **any pdfTeX-compatible command line**: pdfTeX itself for the self-test,
  later the new engine. It runs as `<bin> -fmt=pdflatex` on a copy of the
  tree, to convergence, like the oracle.

The kind is detected from the first line of `<bin> --version`;
`--engine-kind` overrides it.

The question this answers: "can we measurably know that we have full parity
with pdflatex?" The headline per corpus tier is the two **DESIGN §1.1 gating
tiers, P-T1 and P-T2**. Below them are the older levels L0–L4 (the share of
documents at L3 among them), plus the breakdown that says what stops the rest.

## Gating tiers (headline; DESIGN §1.1)

| tier | passes when | how it is measured |
|---|---|---|
| **P-T1** | every `\shipout` box dump and the whole `\tracingall` log are identical to the pinned pdfTeX 1.40.29's, in PDF mode | `capture.py` runs one traced pass after convergence. The pass sets `\tracingall` (which includes `\tracingoutput`), `\tracingonline=1`, `\showboxdepth=\showboxbreadth=2147483647` and `\nonstopmode`, with `max_print_line=10000`, `error_line=254` and `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`. Normalised: the banner (lines before the `**` echo) and the work-directory path. Then, per the DESIGN §1.1 ruling, only end-of-run accounting is removed: `\\tracingstats` `Memory usage before: …` lines (whole-line match), the `Here is how much of TeX's memory you used:` and `PDF statistics:` blocks, and the byte count in `Output written on … (N pages, B bytes)`. The page count stays compared. Each removed line must have one of the exact shapes pdfTeX 1.40.29 prints, in pdfTeX's order, each at most once (`capture.ACCOUNTING_BLOCKS`, taken from the fixture logs). The headers and `Output written` count only once, in the trailer after the last shipout. A `Memory usage` line counts only when a shipout still owes it. Anything else is compared, including ` junk`, ` Overfull …` and a header in the middle of the log. The removed lines form a separate **non-gating accounting check** in the report. Box dumps are split at `Completed box being shipped out` |
| **P-T2** | identical embedded font subsets and identical page content streams | Both PDFs go through `qpdf --qdf --normalize-content=y --object-streams=disable` and are parsed with `tools/visual-oracle/pdftext.py`. Fonts: the multiset of (name without subset tag, subtype, hash of the decoded font program with subset tags normalised). Pages: content-stream bytes, resources and media box. Every indirect object is compared by a hash of its content with references resolved, so object numbers never matter |

**P-T1 is n/a for the flashtex CLI.** It isn't a TeX engine and writes no box
dumps or `\tracingall` log, and the report says so instead of printing 0.
Its P-T2 is measured.

**How both TeX engines run** (the oracle, and a TeX `--engine`), identically:

- **One `\write18` setting for both.** It is `capture.SHELL_ESCAPE`. By
  default there is no flag, so each engine runs in its own default mode:
  restricted, as in TeX Live's pdflatex (owner decision #1209, DESIGN §4.5).
  That gives ` restricted \write18 enabled.` and `\pdfshellescape` = 2,
  which l3kernel's `\sys_if_shell` reads. `--shell-escape-flag` takes
  `default`, `-shell-restricted`, `-no-shell-escape` or `-shell-escape`. The
  setting is part of the oracle cache key.
  Give the value with `=`, as in `--shell-escape-flag=-shell-restricted`,
  because argparse reads a separate `-shell-restricted` as an option.
- **Converted figures are the oracle's.** Under restricted `\write18`,
  epstopdf converts `x.eps` to `x-eps-converted-to.pdf` during the run.
  Ghostscript stamps each conversion with the time and a fresh id. pdfTeX
  copies those into the including PDF and prints the file's date in the log,
  so two engines that each convert differ in P-T1 and P-T2 for no
  typesetting reason. The oracle's conversions (`tiers.GENERATED`) are kept
  in its cache entry and copied, with their times, into the candidate's tree
  before its first pass, so both see the same files and neither converts
  again. The templates tier copies TeX Live files with their times for the
  same reason.
- **argv[0] is exactly `pdftex`.** Each engine runs through a `pdftex`
  symlink in its own bin directory, and that directory goes first on `PATH`
  so kpathsea resolves the real binary. pdfTeX prints argv[0] in warnings,
  e.g. `pdfTeX warning: pdftex (file ./fig.pdf): …`. Setting the name
  before the run means no program-name token is ever normalised, so nothing
  can hide behind a normaliser.
- **Environment.** `--engine-env KEY=VALUE` (repeatable) goes to the TeX
  `--engine` only, never the oracle; for example
  `FLASHTEX_FORMATS=<dir with pdflatex.fmt>` and `FLASHTEX_POOL=<pdftex.pool>`.
  No `FLASHTEX_*` variable is inherited from the calling shell.

Expected data comes only from the oracle pdfTeX (`--oracle-pdftex`, default
`/Library/TeX/texbin/pdftex`; the run warns if it isn't 1.40.29). It is never
compared against the committed references, which include TeX Live 2025
builds. The oracle run is cached under `<cache>/pt-oracle/`, keyed by the
source-tree hash, the pdfTeX version and the capture settings. The
normalised log is stored gzipped. `--pt pt2` skips the traced pass;
`--pt off` skips both tiers.

**Traced logs that don't fit in memory.** P-T1 holds both traced logs in
memory. Some e-prints trace to several gigabytes (arXiv 2501.08663v2: more
than 25 GB from pdfTeX), and a worker killed for memory takes the whole run
with it. Two options report such documents as P-T1 *not evaluated*, never
as passed, and still measure P-T2 and L0–L4 on them:
- `--pt1-skip [tier/]ID`: the oracle is never traced;
- `--pt1-max-log-mb N` (default 1024; 0 turns it off): skips a document whose
  cached oracle log is larger than N MiB.

The summary counts them as `skipped`. The skip is decided from the oracle
alone. If only the candidate's traced log is over the cap, P-T1 **fails**
(the logs can't be equal), and the oracle's log is not loaded.

**A worker that dies** (killed for memory, say) breaks the pool, and every
unfinished document fails with it. Those documents run again on a fresh
pool. If that breaks too, the rest run one per pool. A document whose own
worker dies is recorded as failed at every level and tier (`worker_died`),
never excluded, so no denominator shrinks. The report is written, but
`parity.py` exits 3 and makes or checks no baseline, so no gate can use the
run.

**A traced pass without the end of its log** is reported as a timeout when
the capture's 600 s limit stopped it (a harness limit, class c in
`engines.py`). Otherwise it is reported as a crash (the engine's, class b).

**Capture adapter.** `capture.py` exposes
`capture(tex_path, engine_bin, workdir, *, fmt=None) -> Capture(log, boxes, pdf_path)`.
That is the signature `tools/lockstep/run.py` will expose (P0-LOCKSTEP-HARNESS;
#2 comments 5885107001 and 5885140240). Until lockstep lands, `capture.py` is a
minimal stand-in marked `TODO(lockstep)`. Then it becomes an import, so the
repository has one capture, not two.

**Self-test:** `--engine /Library/TeX/texbin/pdftex` must score 100% on P-T1
and P-T2 across the fixtures tier.

## Levels L0–L4 (per document, cumulative, strictest last)

| level | passes when | how it is measured |
|---|---|---|
| L0 | it compiles | pdflatex compiles it with `-halt-on-error` (only then is it in the denominator) **and** `flashtex build --json` exits 0 with `summary.errors == 0` and writes a PDF and a display list |
| L1 | same page count | reference PDF page tree vs display-list pages |
| L2 | same glyphs | per page, the multiset of glyph characters is equal. Reference glyph names (from `/Differences`, or the embedded Type 1 program's built-in encoding for `cm*`/`msbm`/`cmex`) and candidate cluster text are both reduced to NFKD Unicode (`glyphkeys.py`) |
| L3 | every glyph in place | a one-to-one matching of same-character glyphs with \|dx\| ≤ 0.5 bp and \|dy\| ≤ 0.5 bp covers every glyph on every page. Math-extension glyphs (`cmex`, `lmex`) are held on x only, because a cmex glyph hangs from its origin and an OpenType MATH variant sits on the axis. Their vertical allowance is 2 em, plus the stack height for extensible delimiters |
| L4 | pixels match | both PDFs rasterised at 144 dpi grey (`pdftoppm`, else Ghostscript). A pixel differs when \|Δ\| > 64 grey levels, and each page may have at most 0.02 % of its pixels differing |

For a TeX engine, L0 means exit 0 with a PDF. Its glyphs are read from that
PDF exactly as the reference's are.

A document "at L3" also passed L0–L2. The report also gives each check's
independent pass rate, the glyph-position error distribution (p50/p95/max of
max(\|dx\|,\|dy\|) over glyphs that `rank.aligned_pairs`, the word alignment
of `tools/visual-oracle/rank.py`, pairs), the first diverging page, and
**blockers**: what stands between each document and its next level.

## Tiers

- **fixtures**: `fixtures/real-world` + `fixtures/divergence-probes` against
  their committed reference PDFs. Deterministic, needs no TeX, and takes about
  3 s. CI runs it on macOS against `baseline-fixtures.json` (no document may
  drop below its recorded level).
- **arxiv**: `corpus/arxiv-2025-01.json`, 149 version-pinned e-prints (10 per
  primary category across math, cs, hep-th, quant-ph, cond-mat, astro-ph and
  gr-qc, submitted 13–17 Jan 2025), each with its SHA-256. `corpus.py
  select-arxiv` records how they were drawn.
- **templates**: `corpus/templates-texlive-2026.json`, 20 sample documents
  shipped in TeX Live 2026 (IEEEtran, acmart, amsart/amsproc/amsbook,
  revtex4-2, elsarticle, llncs, tufte, moderncv, beamer, `sample2e`,
  `testmath`, `amsldoc`), pinned by hash.

Third-party sources are **never committed**. `corpus.py fetch` downloads them
into `$FLASHTEX_PARITY_CACHE` (default `~/.cache/flashtex-parity`), verifies
the hashes and unpacks them. It sends at most one arXiv request every 3 s.

## Running it

```sh
cargo build --release -p flashtex-cli --bin flashtex   # -> target/release/flashtex
python3 tools/parity/parity.py --tier fixtures                      # P-T2 + L0-L4 for the CLI
python3 tools/parity/parity.py --tier fixtures --engine /Library/TeX/texbin/pdftex --raster none   # self-test
python3 tools/parity/corpus.py fetch                                # once; ~450 MB of e-prints
python3 tools/parity/parity.py --tier fixtures --tier arxiv --tier templates -j 10
#   -> docs/evidence/parity-<UTC date>/{report.md,scoreboard.json,documents.json}
python3 tools/parity/parity.py --tier fixtures --raster none --check-baseline tools/parity/baseline-fixtures.json
python3 -m unittest discover -s tools/parity -p 'test_*.py' -v
```

pdflatex references for the fetched tiers are cached under
`<cache>/oracle/`, keyed by the SHA-256 of the source tree, the entry file,
the pdflatex version and argv. Measured on a 15-core M-series Mac: a warm
run of all three tiers takes about 3 min at `-j 10`. A cold run adds 789 s of
serial pdflatex (the longest document takes 79 s), which is about 1.5 min
more at `-j 10`. A document pdflatex cannot compile is cached as such
and excluded.

### Engines side by side (`engines.py`)

`engines.py` puts several runs side by side: the new engine, v1 (the
`flashtex` CLI) and the pdfTeX self-test. For each tier it gives P-T1, P-T2
and L0–L4, and it gives every document of the `--subject` engine that is
not a full pass (P-T1, P-T2 and L4) a root-cause class:

| class | meaning |
|---|---|
| a | a package or font missing from the user's TeX Live (the DESIGN §4.4 bundle fallback) |
| b | an engine difference, with the first differing log, box or content line |
| c | a harness issue in tools/parity |
| d | pdflatex fails too, so the document is excluded |
| e | excluded by the convergence rule: pdflatex compiles it, but its log asks for a rerun on every pass (natbib's `Rerun to get citations correct.`) |

The class comes from the run's records. A notes file adds what a person
found by reading the logs: the pdftex.web section, the owner and the issue.
A note's class overrides the automatic one, and the report shows both. The
output is small and deterministic, and it carries the measuring host, so it
can be committed as evidence (`reports/`):

```sh
python3 tools/parity/engines.py --run new=<out> --run v1=<out> --run pdflatex=<out> --subject new \
    --sha new=<git sha> --sha v1=<git sha> --notes tools/parity/reports/<name>.notes.json \
    --out tools/parity/reports/<name>
```

### The P5 scoreboard (`scoreboard.py`, `scoreboard-run.sh`)

DESIGN §12 P5's gate is "new engine ≥ old on every tier; arXiv L1 ≥ 90%;
retirement complete". `scoreboard.py` puts every tier in one table, new
engine against v1, from what the existing harnesses already wrote. It
measures nothing itself, so there is no second harness:

| tier | harness | metric per engine |
|---|---|---|
| fixtures, arxiv, templates, packages | `parity.py` (`scoreboard.json`, `documents.json`) | P-T1, P-T2, L0–L3 (L4 when rasterised) |
| nightly-5k (T4) | `nightly.py` (`summary.json`, #1276) | P-T1 (its 5% sample), P-T2, L0–L3 |
| T2 LaTeX suites | `tools/latex-suites/run.py` (transcript) | tests agreeing with pdfTeX; target 0 unexpected |
| package-smoke | `tools/package-smoke/run.py` (transcript) | documents equal to pdfTeX |
| fonts | `tools/font-census/census.py` (`census.json`) | fonts identical to pdfTeX |

Each row gets a verdict. **ahead**, **equal** and **ahead (old n/a)** are
green. The others are:
- **behind**;
- **below target**: arXiv L1 < 90%, or an unexpected T2 failure;
- **below bar (old n/a)**: see below;
- **denominators differ**;
- **host mismatch**: two hosts or two oracles in one row (DESIGN §8);
- **invalid**: a worker died, a transcript without its summary line, a zero
  denominator, or a run that is not this board's engine (below);
- **missing**: a tier nobody ran is still a row.

**Fail closed.** A harness output with a missing or renamed field raises
`FormatError` (exit 2); nothing defaults to 0. Every row needs a denominator
above 0. Denominators are the harnesses' own. Exclusions are printed with
their reasons.

A run is **partial** when:
- it was limited (`--limit`, `--only`, `--shard`, `--spread`);
- it saw fewer documents than its tier's manifest lists, or the tier has no
  manifest to count against (fixtures excepted);
- a nightly run has missing shards, documents not returned or unmeasured, or
  mixed fingerprints;
- T2 ran fewer tests than `run.py --suite all --list` counts
  (`--latex-suites-list`; without it T2 is always partial);
- package-smoke ran fewer documents than `tools/package-smoke` holds.

A font census is a **sample** when:
- it ran below its default `--per-family`;
- it ran with `--only` or `--kind`;
- it tested fewer families than it found, or tested no fonts of some kind;
- it does not record its `selection`.

A board with any partial row, any sample, or `--sample-note` is never
all-green.

**One engine, one commit, one oracle.** A run that records its commit
(`nightly.py`) must be at the board's `--sha`. A run that records none, or a
board without `--sha`, is invalid. Every run of one engine that records the
binary's sha256 must match that engine's `parity.py` run. Every run that
records its oracle must name the same one. Any mismatch marks that run's
cells INVALID: a stale T4 cannot count.

Only the T4 tiers are read from a nightly summary. #1276's corpus-t4 job
also runs the T3 tiers, which come from this board's own `parity.py` runs.

**Rows v1 cannot run.** v1 cannot run T2, package-smoke, the font census or
P-T1, because they need a pdfTeX-compatible binary. Its cell there reads
**n/a** with that reason. new >= old is then vacuous, so these rows have a
bar: green only when new passed 100% of what it measured with nothing
skipped, or at least the row's recorded baseline (`--na-baseline FILE`,
`{"rows": {"tier:metric": {"passed": P, "of": N}}}`). Below it the verdict
is **below bar (old n/a)**.

On a TeX Live newer than the suites' pins, pass
`--latex-suites-reference` with the same suites run through that host's
pdfTeX. The T2 baseline is then pdfTeX on the same host, as in
`scripts/engine-parity.sh`.

**T2 failures are (directory, test) pairs.** The same test name can run in two
directories: l3kernel's testfiles-backend runs under etex-dvips and under
etex-dvisvgm. So pdfTeX failing `m3backend01` in one directory while the
engine fails it in the other is a difference. `run.py` prints each
directory's failures (`LABEL: FAILED t1 t2 ...`), and "unexpected" is the set
of pairs the engine fails and pdfTeX passes.

A transcript is INVALID when:
- its per-directory failures do not account for every FAIL count;
- its `failing tests:` block does not name them;
- its `UNEXPECTED failures:` line names a test no directory failed.

The same checks apply to the reference transcript.

**Retirement stages.** The stages of #1236 (`retirement-stages.json`) form a
column and a table. Each row lists the stages it gates: fixtures P-T2 gates
S3 (the P3 exit), and every row gates S5 onward. An all-green board meets
only the scoreboard part of S5's precondition. The status line names what S5
still needs, including "T1 (lockstep) has 0 new differences", which this
board does not measure.

**Issues.** `--issues apply` opens or updates one issue per tier where
new < old, on complete runs only. The body carries the marker
`<!-- p5-scoreboard:tier=NAME -->`. The run closes that issue only when every
row of the tier is green and no cell is partial, a sample or invalid, and the
board has no `--sample-note`. `dry-run` prints the plan.

**T4 v1.** #1276 runs T4 for the new engine only. T4's old column therefore
reads **missing (no v1 leg in nightly: decision 1)** until corpus-t4 gains a
v1 leg that uploads `corpus-t4-v1`.

`scoreboard-run.sh` runs everything but T4 end to end: it builds both
engines and the new engine's formats, then runs each harness for each
engine and aggregates. `.github/workflows/p5-scoreboard.yml` runs it nightly
on the NixOS runners. The T4 rows come from the `corpus-t4` and
`corpus-t4-v1` artifacts of the newest nightly run from the last 36 h, and the
whole board is measured at that run's commit.

The engine is built alone, in its own `cargo build -p flashtex-engine`, as
corpus-t4 builds it, so the two binaries' sha256 can match. Built together
with `flashtex-cli`, shared dependencies unify features and the binary
differs.

The workflow keeps write tokens away from third-party TeX sources:
- the measuring job has read-only permissions and a checkout without
  credentials. Its token is only in the step that downloads the T4
  artifacts, before any TeX runs;
- a separate job that runs no TeX files the issues with
  `scoreboard.py --from-board`, and pushes the summary.

```sh
tools/parity/scoreboard-run.sh --out /tmp/p5 --jobs 2            # every tier but T4
tools/parity/scoreboard-run.sh --out /tmp/p5 --limit 12 --sample-note "local sample"
python3 tools/parity/scoreboard.py --parity new=<dir> --parity old=<dir> \
    --nightly new=<dir> --nightly old=<dir> --latex-suites new=<file> \
    --latex-suites-reference <file> --package-smoke new=<file> --fonts new=<dir> \
    --out <dir> [--summary FILE] [--issues dry-run|apply] [--require-green]
```

## Root causes

A blocker is one of:

- **L0**: every error diagnostic, keyed as `code: cs \name (context) <- file`.
  The context is the reason family (in math mode, in the preamble, dimension
  argument, …). `<- file` is the `.sty`/`.cls` FlashTeX was reading when the
  error fired.
- **L1–L3**: warnings that name an unimplemented construct, plus the source
  construct at the first divergence (`pagination:`, `content:` or
  `placement:`).
- **L3→L4**: font/outline notes.

`definers.py` then **groups constructs by who defines them**, because a
package is what a lane implements. The index is built once (~5 s) from TeX
Live and cached. Precedence:

1. TeX primitive
2. the project's own `.sty`/`.cls`
3. the document class
4. kernel register
5. kernel command, which stays one key per command
6. the document's own macro
7. the loaded package whose `\RequirePackage` closure reaches a definer first

Each cause is ranked by the number of documents whose next level it holds.
`sole` counts the documents that cause alone blocks. `share` is Σ 1/|blockers|.

## Honest limits

- Identity is Unicode, not glyph id. An unmapped reference glyph name is
  reported (`unmapped_reference_glyphs`) and counts as a mismatch.
- Text inside Form XObjects (included PDF figures) is read on neither side
  by L2/L3. P-T2 compares them through the page resources.
- P-T2 does not compare annotations (links), outlines or the document
  catalogue. P-T3 (byte identity) is optional and not implemented.
- A cold P-T1 oracle run is heavy: `\tracingall` logs run from 10 MB to
  430 MB per fixture (beamer is the largest).
- L1–L3 blockers are heuristics: the first-divergence label comes from the
  nearest candidate glyph's source span.
- The fixtures tier uses the committed references. Twelve real-world fixtures
  and all divergence probes were made with TeX Live 2025, the rest with
  MacTeX 2026.
- `LaTeX kernel` attribution uses a regex index of definitions, not
  expansion, so a name built with `\csname` is not found and keeps its own
  key.
