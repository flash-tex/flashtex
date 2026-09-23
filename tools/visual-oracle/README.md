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
5. **Alignment** per page: `difflib.SequenceMatcher` on normalised word text,
   then `reanchor_pairs` repairs repeated-token mispairs — text-only LCS
   alignment has no notion of position, so a page with many identical short
   tokens (a table of contents' section/page numbers, list counters) can pair
   a candidate token with a same-text reference token on another line while
   the two tokens' true, close partners sit unaligned on both sides. For
   every pair whose measured points (`pair_points`) disagree by more than
   50 bp in x or 0.6 line heights in y, `reanchor_pairs` looks for an
   unpaired same-text word within that threshold on each side and, per text,
   greedily re-pairs the nearest ones; a pair with no such neighbour (a
   genuine reflow, or a duplicate the two sides can't reconcile) is left
   exactly as difflib found it, so `aligned`/`unaligned` stay honest and a
   real line-break difference still reports as reflowed. For every aligned
   pair `dx`, `dy` (candidate − reference, bp; 1 bp = 1.00375 TeX pt). Per
   page: aligned / unaligned counts, median shift, mean and max |dx| / |dy|,
   words within 0.01 bp and 0.5 bp, *reflowed* words (|dx| > 50 bp = the word
   sits on another line), and the `--top` largest deltas with word, both
   positions, both fonts, source span and excerpt.
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

## Companion: `cumulative.py` — ranking by blast radius

`rank.py` ranks *pages* by their single largest delta. `cumulative.py` ranks
*divergences* by how many baselines each one displaces, because the two answers
differ: GH-706 was 1.99 bp per list boundary and moved every baseline below it
on 17 fixtures, while GH-717 was 5.06 pt and moved one glyph stack in one
matrix.

```sh
python3 tools/visual-oracle/cumulative.py                       # -> docs/evidence/corpus-fidelity-<UTC>/
python3 tools/visual-oracle/cumulative.py --only hw1 --only cv
python3 tools/visual-oracle/cumulative.py --fixtures <dir-of-probe-fixtures> --out <dir>
```

It reuses this tool's producer route, `pdftext` reference reader, word grouper,
`difflib` alignment and `rank.pair_points` anchoring unchanged — there is still
one definition of a word, one alignment and one measured point in this
directory (it calls `rank.align_words` directly, not `rank.geometry_page`, so
it does not run `rank.reanchor_pairs`'s repeated-token repair). What it adds:

1. aligned word pairs are bucketed into **reference lines**, giving each page a
   `dy` profile;
2. every change in that profile is classified **cumulative** (the median `dy`
   below it differs from the median above: its cost is the step times the lines
   below it) or **local** (the page comes back: its cost is one line);
3. each step is labelled with the LaTeX construct standing between the two
   lines in the source, so one cause groups its witnesses across fixtures
   instead of arriving as one finding per fixture;
4. lines whose own words disagree about `dy` — which is what happens around
   cmex big operators, where the reference PDF has no usable text for `\int`
   and a neighbouring limit can pair across the display — are reported as **low
   confidence** and left out of the ranking;
5. every page's **glue set** is read back from pdfTeX's `\tracingoutput`, and a
   step on a page with a finite-order set is flagged `cause_not_localised` and
   kept out of the ranking's line counts.

### Why (5) exists

TeX scales every shrinkable skip on a page by one page-global ratio to make the
material fit `\textheight`. If the two producers' natural page heights differ
*anywhere* — including below the line being looked at — their ratios differ and
every baseline separates in proportion to the shrinkable glue above it. That
glue is `\abovedisplayskip`, `\topsep`, `\itemsep`, `\parskip` and the
`\@startsection` skips, so such a page manufactures a step at every display,
list and heading, each sized by that construct's own shrink and none of them
caused by it. Ranking those steps by position credited 704 of one sweep's 1031
displaced baselines to the wrong construct and produced two whole findings that
did not exist.

The **reference** ratio is pdfTeX's own, from a re-run under `\tracingoutput`
that is first verified to reproduce the pinned reference PDF (byte-identical, or
every extracted baseline within 0.05 bp). `--texbin` points at another pdflatex;
`--no-glue-set` skips the pass and flags every step, because an unmeasured glue
set is not a zero one. The **candidate** ratio is not reported: `render-pipeline`
computes it in `pagebuild::glue_set` and discards it, and the v2 display list
carries only the fixed page size, so the engine's `overfull_vbox` diagnostic is
reported in its place rather than a number being invented.

First report: `docs/evidence/corpus-fidelity-2026-09-16T1130Z/` (GH-66).
Tests: `python3 -m unittest discover -s tools/visual-oracle -p 'test_*.py'`.
