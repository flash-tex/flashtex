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
  A conversion's input that the run wrote itself (grfguide's `filecontents`
  `a.eps`) is kept with it, because epstopdf logs the input's date.
- **The random seed is pinned** (DESIGN §4.5). pdfTeX seeds
  `\pdfuniformdeviate` from the clock, so l3kernel's `\int_rand` and pgf's
  random numbers differ from run to run. Every pass of every TeX engine the
  harness runs (both oracles and a TeX `--engine`) starts its command line
  with `capture.SEED` (`\pdfsetrandomseed 1\relax`), then `\input{<entry>}`.
  Both logs echo that line equally. The P-T oracle cache key includes the
  seed and `tiers.ORACLE_CACHE_V`, which is bumped whenever what an entry
  holds changes.
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

**Traced logs too big to hold: streamed P-T1.** Some e-prints trace to
tens of gigabytes (arXiv 2501.08663v2: 23.7 GiB from pdfTeX). A traced log
over `--pt1-max-log-mb N` (default 1024; 0: no budget) is never read whole.
It is read once, as a stream, into its P-T1 *fingerprint* (`pt1stream.py`,
constant memory: about 20 MiB whatever the log's size), and
`tiers.compare_pt1_streamed` compares two fingerprints:
- **The same verdict as the in-memory compare, on every input.** The rules
  are `capture.py`'s own code (`workdir_subs`, `banner_end`,
  `Accounting.step`, `BoxSplitter`), not a second normaliser. Only the
  driver differs: a stream can't look ahead to the last shipout or the `**`
  line, so it runs those as hypotheses that the stream resolves (see
  `pt1stream.py`). The strict log and each box dump are compared by SHA-256
  of exactly the text `split_accounting` and `split_boxes` produce; the
  accounting lines are kept whole, so the non-gating accounting report is
  unchanged. A failure gives the first differing shipout exactly and the
  strict-log lines (a 4 MiB segment) that hold the first log difference,
  not the line's text.
- **The fast path.** A run of lines where no rule can fire (no shipout, no
  owed `Memory usage`, no block header or `Output written`, no blank line in
  an open box) goes to the hashes as one text. Every line still feeds them.
- **No log reaches the disk.** The oracle's traced pass always writes its
  log into a named pipe the harness reads while pdfTeX runs. The oracle
  cache keeps a small log's normalised text (`log.gz`, as before) or a big
  log's fingerprint (`fingerprint.json`); an entry cached before streaming,
  over the budget with neither, is made again. A candidate's traced pass
  uses a pipe when the oracle's log is over the budget. A candidate log that
  is over the budget unexpectedly is on disk; it is streamed from there and
  deleted. A candidate over the budget that differs fails P-T1.
- **Time limit** (`--pt1-timeout S`, default 1800): a traced pass gets
  `max(S, oracle log bytes / 8 MiB/s)`. A pass it stops is a **harness
  error**, never a pass: a candidate's fails P-T1, an oracle's leaves the
  document not evaluated, and both are counted (`harness_errors`) and
  reported. A cached oracle entry that a shorter limit stopped is made again.

`--pt1-skip [tier/]ID` still reports a document's P-T1 as not evaluated
(its oracle is never traced), but size is no longer a reason to use it.
The 2026-09-29 arXiv scoreboard skipped 2501.07413v3, 2501.08663v2,
2501.08950v2 and 2501.10183v1 for size; they need no `--pt1-skip` now.
The summary counts the documents P-T1 compared as streams (`streamed`).

**Oracle work directories.** `tiers.oracle` runs pdfTeX in
`<cache>/pt-oracle/<key>/work-<pid>` and removes it on every exit path:
success, an exception, the time limit, and SIGTERM (a worker's handler
unwinds the document it is scoring, which kills its engine and runs each
cleanup; parity.py's own SIGTERM handler passes the signal to its workers).
A worker killed with SIGKILL runs no cleanup, so `parity.py` sweeps, at
startup, every `work-<pid>` and `*.<pid>.tmp` whose process is no longer
alive.

**A worker that dies** (killed for memory, say) breaks the pool, and every
unfinished document fails with it. Those documents run again on a fresh
pool. If that breaks too, the rest run one per pool. A document whose own
worker dies is recorded as failed at every level and tier (`worker_died`),
never excluded, so no denominator shrinks. The report is written, but
`parity.py` exits 3 and makes or checks no baseline, so no gate can use the
run.

**A traced pass without the end of its log** is reported as a timeout when
the capture's time limit (`--pt1-timeout`) stopped it (a harness limit, class c in
`engines.py`). Otherwise it is reported as a crash (the engine's, class b).

**Capture adapter.** `capture.py` exposes
`capture(tex_path, engine_bin, workdir, *, fmt=None) -> Capture(log, boxes, pdf_path)`
(plus the harness's `stream=` and `timeout=`, and a Capture's `size`, `complete`,
`fingerprint` and `timed_out` for a streamed log).
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
- **packages**: `corpus/packages-texlive-2026.json`, 92 documents under
  TeX Live 2026's `texmf-dist/doc`, one per package for 92 of the 98
  packages on the M1 list (amsmath, xcolor, geometry, pgfplots, hyperref,
  siunitx, microtype, beamer, minted, …), pinned by hash. Each file loads
  its package directly and compiles with pdflatex alone under TeX Live's
  restricted `\write18`, which is the harness's `default` mode (at most 3
  passes, at most 100 pages). Don't run it with `-no-shell-escape`: minted
  needs `\write18`.
  `copy_dir` copies the file's whole directory and `files` names the
  neighbours it needs. The `skipped` list gives the 6 packages with no such
  file and why (biblatex needs biber, background's only loader is too large,
  …). An entry's `pt1_skip` says why pdfTeX's own `\tracingall` log differs
  between runs, even with the pinned seed (tabu's `\pdfelapsedtime`), so
  its P-T1 is reported as not evaluated, never as passed. P-T2 and L0–L4
  are still measured. The Muse M1 lanes (daniel-muse-lead) drew the tier
  and #2 reviewed it.

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
python3 tools/parity/parity.py --tier packages -j 2                   # traced logs to GBs: keep -j low
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
