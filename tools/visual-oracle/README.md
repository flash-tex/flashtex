# Visual oracle: per-page geometry + raster ranking

Owner: lane `mac-visual-oracle-2` (Claude Code subagent, parent `mac-claude-a`,
machine `mac-m1max-a`). Created 2026-09-12. Python 3 standard library only.
Ranks every page of HW1 and the real-world corpus by how far the exact route
(`flashtex-render --v2` at the pinned SHA → `flashtex-pdf-exact from-v2`) is
from the pinned pdfLaTeX reference — first by word geometry, then by pixels —
and names the crate to look at first for every top item. **Nothing here is a
parity claim, whole-PDF byte equality is not a goal, and pdflatex is an
oracle only, never in the product path.**

```sh
# pinned producer (build once; see tools/real-world-corpus/run.sh for the git-archive recipe)
export FLASHTEX_RENDER=tools/real-world-corpus/target/render-pipeline-9aaec57a/crates/render-pipeline/target/release/flashtex-render
python3 tools/visual-oracle/rank.py                      # -> docs/evidence/visual-oracle-<UTC>/{report.md,report.json,thumbs/}
python3 tools/visual-oracle/rank.py --only hw1 --thumbs 3 --out /tmp/vo
# follow-up 1: the 18 visual-corpus harness fixtures under the pdflatex-lm preamble (fresh MacTeX references)
git archive origin/agent/mac-visual-oracle/reference-raster tests/visual-corpus/harness/fixtures | tar -x -C /tmp/hf
python3 tools/visual-oracle/rank.py --harness-fixtures /tmp/hf/tests/visual-corpus/harness/fixtures --out <evidence>/harness-fixtures
python3 -m unittest discover -s tools/visual-oracle -p 'test_*.py' -v   # 8 pure-python tests
```

## What one run does

1. **Fixtures**: every `fixtures/real-world/<id>/` (discovery and reference
   lookup shared with `tools/real-world-corpus/run.py`; HW1 compares against
   the user's byte-immutable `HW1-reference.pdf`). `--harness-fixtures DIR`
   instead takes `*.tex` bodies (everything from the real `\begin{document}`
   on) under the harness's pdflatex-lm preamble and renders the reference now
   with `pdflatex` (`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two passes).
2. **Producer**: one `compile` request per fixture to `flashtex-render --v2`
   (fresh process, `FLASHTEX_FONT_DIRS`/`FLASHTEX_TFM_DIRS` = `apps/mac/Fonts`),
   then `flashtex-pdf-exact from-v2 … --font-dir apps/mac/Fonts` — the same
   invocation as the Mac shell's `ExactPDFExport.swift`. Crashes and from-v2
   refusals are reported with their stderr, never patched here.
3. **Reference geometry** (`pdftext.py`): the reference PDF's content stream
   is replayed (xref/object streams + FlateDecode via `zlib`; `Tf Td TD Tm T*
   TL Tc Tw Tz Ts Tj TJ ' " q Q cm`) with the fonts' own `/Widths`, giving
   every glyph's origin; glyphs become words at gaps wider than 0.16 em, a new
   baseline or a new text object. Text for alignment comes from
   `/Differences`, else the OT1/T1 ligature slots + ASCII (`?` otherwise).
4. **Candidate geometry**: the rendering-v2 display list's glyphs, origin in
   bp (`bp_2pow20`) and baseline from the page top, grouped into words by
   **the same function as step 3** (`pdftext.words_from_glyphs`), with the
   cluster source spans (`path:start-end`) of each word. A `glyph_run` is a
   typesetting artefact, not a word — the pipeline starts a new run at every
   face change — so grouping by run made one siunitx `S` cell three words
   against the reference's one. There is one definition of a word in this
   tool and both sides use it.
5. **Alignment** per page: `difflib.SequenceMatcher` on normalised word text.
   For every aligned pair `dx`, `dy` (candidate − reference, bp; 1 bp =
   1.00375 TeX pt). Per page: aligned / unaligned counts, median shift, mean
   and max |dx| / |dy|, words within 0.01 bp and 0.5 bp, *reflowed* words
   (|dx| > 50 bp = the word sits on another line), and the `--top` largest
   deltas with word, both positions, both fonts, source span and excerpt.
6. **Pixels**: reference and candidate PDFs rasterised at 144 dpi to 8-bit
   grey (`pdftoppm`, else Ghostscript — this Mac has `gs` 10.08.0) and
   compared with the real-world-corpus comparator (differing pixels, max
   grey delta, ink pixels; page-count/size mismatches reported, not compared).
7. **Ranking** (`rank_key`): missing pages first, then pages where no word
   aligns, then by the largest positional delta, then by differing pixels.
8. **Owner per top item** (`owner_for_word`, a documented heuristic): a
   producer diagnostic whose source span overlaps the word's span decides
   (owner table of `tools/real-world-corpus/run.py:owner_for`); else a math
   font → `crates/math-layout`; else a delta equal to the page's median shift
   → render-pipeline page builder / vendored document-style; else |dx| > 50 bp
   → paragraph-layout line breaking; else an isolated layout delta. A page
   whose median shift exceeds 2 bp also gets a page-level owner line.
9. **Thumbnails** (`thumbs.py`, follow-up 2): for the `--thumbs` worst
   compared pages a PNG sheet *reference | candidate | diff* at 48 dpi (red =
   candidate-only ink, blue = reference-only ink), written by a stdlib PNG
   encoder; linked from the report.

## Reading the numbers

- On the 18 harness fixtures (evidence `harness-fixtures/`) the text-only
  fixtures sit at **max |dx| 0.01 bp, |dy| 0.00 bp** for every aligned word
  (the earlier PDFKit measure showed a constant 1.15 pt `dy`; reading the
  baseline from the content stream removes that font-box artefact). The
  residuals are the known ones: `11-nested-lists` (list environment not
  implemented, page-wide shift), `13`/`14` (math parser drops `\left`/`\nu`),
  `10` (`ǅ` has no Latin Modern glyph), `06` (`\sqrt` box).
- On the corpus every compared page has a large median vertical shift and
  dozens of reflowed words because an unimplemented class/environment
  upstream (`letter`, `\maketitle`, `theorem`, `array`, …) changes the text
  that is typeset; the aligned deltas on those pages measure that upstream
  difference, not the line layout. `aligned` and the unaligned counts say how
  much of the page the geometry covers.

## Honest limits

- Word alignment is by text only; `?` glyph names are dropped from the
  alignment key. Math still aligns less often than prose, but not because
  the two sides disagree about word boundaries: both group with
  `pdftext.words_from_glyphs`, so a font change never ends a word on either
  side.
- Grouping is geometric, so it reports stray ink instead of hiding it: a run
  whose glyphs jump backwards (more than half an em) is split, and the
  fragment keeps its own origin. `fixtures/real-world/unicode-accents` has
  one — five glyphs of `ellipsis\dots;` sit at x = -12321 bp — which
  grouping by run concealed inside a word whose `x` came from its first
  glyph.
- The reference reader is not a general PDF parser: XObjects, inline images,
  Type 3 / CID fonts and non-Flate filters are reported in `notes`, not read.
- Ghostscript anti-aliases; pixel counts are the secondary key only.
- Owners are triage pointers, not verdicts.
