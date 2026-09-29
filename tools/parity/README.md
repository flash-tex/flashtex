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
| **P-T1** | every `\shipout` box dump and the whole `\tracingall` log are identical to the pinned pdfTeX 1.40.29's, in PDF mode | `capture.py` runs one traced pass after convergence. The pass sets `\tracingall` (which includes `\tracingoutput`), `\tracingonline=1`, `\showboxdepth=\showboxbreadth=2147483647` and `\nonstopmode`, with `max_print_line=10000`, `error_line=254` and `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`. Normalised: the banner (lines before the `**` echo) and the work-directory path. Then, per the DESIGN §1.1 ruling, only end-of-run accounting is removed: `\\tracingstats` `Memory usage before: …` lines (whole-line match), the `Here is how much of TeX's memory you used:` and `PDF statistics:` blocks, and the byte count in `Output written on … (N pages, B bytes)`. The page count stays compared. A block continues only through lines that start with a space, so a line appended after one is still compared. The removed lines form a separate **non-gating accounting check** in the report. Box dumps are split at `Completed box being shipped out` |
| **P-T2** | identical embedded font subsets and identical page content streams | Both PDFs go through `qpdf --qdf --normalize-content=y --object-streams=disable` and are parsed with `tools/visual-oracle/pdftext.py`. Fonts: the multiset of (name without subset tag, subtype, hash of the decoded font program with subset tags normalised). Pages: content-stream bytes, resources and media box. Every indirect object is compared by a hash of its content with references resolved, so object numbers never matter |

**P-T1 is n/a for the flashtex CLI.** It isn't a TeX engine and writes no box
dumps or `\tracingall` log, and the report says so instead of printing 0.
Its P-T2 is measured.

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

Third-party sources are **never committed**. `corpus.py fetch` downloads them
into `$FLASHTEX_PARITY_CACHE` (default `~/.cache/flashtex-parity`), verifies
the hashes and unpacks them. It sends at most one arXiv request every 3 s.

## Running it

```sh
cargo build --release --manifest-path crates/flashtex-cli/Cargo.toml --bin flashtex
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
