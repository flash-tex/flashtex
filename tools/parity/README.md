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

- **nightly-5k** (DESIGN §8 T4): `corpus/nightly-5k.json`, about 5,000
  version-pinned e-prints, see below.

Third-party sources are **never committed**. `corpus.py fetch` downloads them
into `$FLASHTEX_PARITY_CACHE` (default `~/.cache/flashtex-parity`), verifies
the hashes and unpacks them. It sends at most one arXiv request every 3 s.

## Nightly corpus (T4)

DESIGN §8 T4 runs every night on the owner's NixOS PC (`.github/workflows/nightly.yml`,
job `corpus-t4`). `nightly.py` drives `parity.py`; it is not a second harness.

- **Corpus.** `corpus/nightly-5k.json` holds 20 primary categories × 10 years
  (2016–2025) × 25 e-prints. It was drawn by `corpus.py select-arxiv-grid`.
  Each (category, year) cell draws from a 14-day window that starts on a day
  of that year chosen by SHA-256 of the seed, category and year. Within the
  window it takes the oldest e-prints that are TeX source with a top-level
  file. The rule, the seed and every cell's query and skip counts are in the
  manifest's `selection`. Each entry is pinned by versioned id and SHA-256.
  The manifest is `on_demand`, so a bare `corpus.py fetch` skips it and does
  not start hours of polite downloading. Each shard fetches only its own
  documents, at most one arXiv request every 3 s.
- **Tiers.** `nightly-5k` plus the T3 tiers `arxiv`, `templates` and
  `packages`. A tier with no manifest yet is skipped with a notice.
- **Shards.** `nightly.py run --shards 50` runs `parity.py --shard K/50` one
  shard after another. Shard K takes every 50th document from the K-th, so
  each shard is a cross-section of the corpus. The runner wipes the job's
  work directory every job, so state lives in `$FLASHTEX_NIGHTLY_HOME`
  (default `~/.cache/flashtex-nightly`). A finished shard's `scoreboard.json`
  and `documents.json` are kept under the SHA-256 of the engine binary, the
  harness sources, the manifests and the settings. A stopped job therefore
  resumes at its first unfinished shard. `--deadline-minutes` stops starting
  shards in time to upload the artifact.
- **Cost.** P-T2 and L0–L4 run on every document. P-T1 needs a traced pass
  whose log can run to GBs, so it runs on a fixed pseudo-random sample:
  `--pt1-sample nightly-5k=0.05`, which is about 250 documents. A document is
  in the sample when SHA-256 of `flashtex-pt1-sample/1/<tier>/<id>`, read as a
  fraction, is below 0.05 (`parity.in_pt1_sample`). The sample is the same
  every night, so the traced oracle logs stay cached. The other tiers get
  P-T1 on every document, as in T3.
- **Memory bound.** A P-T1 comparison holds both traced logs in memory. Its
  peak resident size was measured at 7.5 and 7.6 times the log size, on
  logs of 210 and 252 MiB (`parity.PT1_MEMORY_FACTOR` = 9 allows for the
  candidate's larger cap). Three limits bound the total:
  - `--pt1-max-log-mb 512`: `capture` measures the traced log every 0.2 s
    and kills the engine once the log passes the cap. It deletes the log and
    never reads it. When the oracle's log passes the cap, that document's
    P-T1 is not evaluated. When only the candidate's log passes its cap,
    which is 1.25 times the oracle's to allow for path lengths, P-T1 fails.
  - `--pt1-jobs 2`: at most two documents are traced at once across all
    workers, whatever `-j` is. A semaphore enforces this.
  - `nightly.py run` refuses to start when the bound is over
    `--memory-budget-gib`: `-j` × 1 GiB (an allowance per untraced worker,
    not a measurement), plus `--pt1-jobs` × 9 × the cap. The workflow sets
    the budget to 70% of the PC's `MemAvailable` at the start of the job.
    Its bound is 8 × 1 + 2 × 9 × 0.5 = 17 GiB.
  - A run without a cap is refused. Among 917 traced oracle logs cached on
    mac-m5pro-dq222, the median is about 60 MiB and the 90th percentile about
    400 MiB, so the 512 MiB cap leaves P-T1 not evaluated on a few per cent of
    the sample. That share is an estimate from the gzip sizes (about 10:1),
    not a measurement.
- **Oracle on the same host.** The references are made by the runner's own
  pdfTeX 1.40.29 and pdflatex (TeX Live 2026) and cached by source hash.
  Nothing expected is committed.
- **Ratchet.** `nightly.py ratchet` fails the job when a document drops
  below its recorded level, or when its P-T2 (or P-T1, where it is evaluated
  both times) goes from pass to fail. The baseline is
  `$FLASHTEX_NIGHTLY_HOME/baseline/<host label>.json`. It is recorded **on
  that host** by `--record`, which only a manual `workflow_dispatch` of
  `main` with `corpus_record_baseline` runs. It records the host label and
  the oracle fingerprint, and the check refuses a baseline or results from
  another host, or from another oracle. So a Mac run can never seed it.
  Improvements are reported but never recorded automatically. A document
  whose source could not be fetched is listed as unmeasured.
- **Artifact** `corpus-t4`: `summary.md` has the per-tier P-T1/P-T2/L0–L4
  table, the classification (a)–(e) of every document that is not a full
  pass (as in `reports/arxiv-scoreboard-*`) and the top root causes, with
  numbers folded so that one difference in many documents ranks once.
  `summary.json`, and `documents.json` with one small record per document.
  `ratchet.json`. Never logs.

```sh
python3 tools/parity/nightly.py make-formats --engine target/release/flashtex-initex \
    --pool crates/flashtex-engine/pdftex.pool --out "$FMT"
python3 tools/parity/nightly.py run --host-label "$LABEL" --shards 50 --tier nightly-5k --tier arxiv \
    --tier templates --tier packages --pt1-sample nightly-5k=0.05 --pt1-max-log-mb 512 --pt1-jobs 2 -j 8 \
    --memory-budget-gib 24 --engine target/release/flashtex-initex \
    --engine-env FLASHTEX_FORMATS="$FMT" --engine-env FLASHTEX_POOL="$PWD/crates/flashtex-engine/pdftex.pool" \
    --out "$OUT" -- --texbin "$TEXBIN" --oracle-pdftex "$TEXBIN/pdftex" --texmf "$TEXMF"
python3 tools/parity/nightly.py ratchet --host-label "$LABEL" --results "$OUT"            # check
python3 tools/parity/nightly.py ratchet --host-label "$LABEL" --results "$OUT" --record   # by hand only
```

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
