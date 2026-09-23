# Parity scoreboard: how close is FlashTeX to pdfLaTeX, measured

Owner: lane PARITY-SCOREBOARD (kabir-claude, mac-m5pro-kabir). Created
2026-09-21. Python 3 standard library only. **pdflatex (MacTeX) is the oracle
and never runs in the product path.** The producer under test is the shipped
command line, `flashtex build` (the same exact-route PDF the Mac app exports),
run on the untouched source tree.

The question this answers: "can we measurably know that we have full parity
with pdflatex?" The answer is one number per corpus tier, **the share of
documents at L3**, plus the breakdown that says what stops the rest.

## Levels (per document, cumulative, strictest last)

| level | passes when | how it is measured |
|---|---|---|
| L0 | it compiles | pdflatex compiles it with `-halt-on-error` (only then is it in the denominator) **and** `flashtex build --json` exits 0 with `summary.errors == 0` and writes a PDF and a display list |
| L1 | same page count | reference PDF page tree vs display-list pages |
| L2 | same glyphs | per page, the multiset of glyph characters is equal. Reference glyph names (from `/Differences`, or the embedded Type 1 program's built-in encoding for `cm*`/`msbm`/`cmex`) and candidate cluster text are both reduced to NFKD Unicode (`glyphkeys.py`) |
| L3 | every glyph in place | a one-to-one matching of same-character glyphs with \|dx\| ≤ 0.5 bp and \|dy\| ≤ 0.5 bp covers every glyph on every page. Math-extension glyphs (`cmex`, `lmex`) are held on x only, because a cmex glyph hangs from its origin and an OpenType MATH variant sits on the axis. Their vertical allowance is 2 em, plus the stack height for extensible delimiters |
| L4 | pixels match | both PDFs rasterised at 144 dpi grey (`pdftoppm`, else Ghostscript). A pixel differs when \|Δ\| > 64 grey levels, and each page may have at most 0.02 % of its pixels differing |

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
python3 tools/parity/parity.py --tier fixtures                      # ~3 s
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
- Text inside Form XObjects (included PDF figures) is read on neither side.
- L1–L3 blockers are heuristics: the first-divergence label comes from the
  nearest candidate glyph's source span.
- The fixtures tier uses the committed references. Twelve real-world fixtures
  and all divergence probes were made with TeX Live 2025, the rest with
  MacTeX 2026.
- `LaTeX kernel` attribution uses a regex index of definitions, not
  expansion, so a name built with `\csname` is not found and keeps its own
  key.
