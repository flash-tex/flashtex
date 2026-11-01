# HW1 through `flashtex-render --v2` → `flashtex-pdf-exact from-v2`: what is the export's, and one fix

Lane `mac-pdf-2` (Claude Code subagent, parent `mac-claude-a`, `mac-m1max-a`),
2026-09-12. Every number below came out of the commands in "Reproduction".
Nothing here claims parity with pdfLaTeX; the corpus report
(`docs/evidence/real-world-corpus-2026-09-12T144246Z/report.md`) already
shows HW1 differing in 7.8 % / 4.9 % / 3.9 % of the pixels per page, and this
note separates the export's share of that from the producers'.

## Pipeline under test

- Producer: `flashtex-render` from `origin/agent/mac-render-pipeline/unified`
  @ `9aaec57a` (the corpus harness's pinned scratch build), request
  `fixtures/real-world/hw1/HW1.tex`, `FLASHTEX_FONT_DIRS=apps/mac/Fonts`,
  `FLASHTEX_TFM_DIRS=apps/mac/Fonts/texmf/fonts/tfm/public/lm`. Display
  list sha256 `b9c5bb0307e99ca9…`: 3 pages, 714 glyph runs, 3 232 glyphs,
  2 rules, fonts LMRoman10-{Bold,Regular,Italic}, LMRoman8-Regular,
  LatinModernMath-Regular.
- Export: `flashtex-pdf-exact from-v2 LIST --out OUT.pdf --font-dir
  apps/mac/Fonts` (the Mac shell's `ExactPDFExport.swift` invocation).
- Reference: the user's `fixtures/real-world/hw1/HW1-reference.pdf`
  (byte-immutable).
- Raster: CoreGraphics `CGContextDrawPDFPage`, 8-bit grey, 144 dpi
  (1224 × 1584 per page); ink = grey < 128; `ref-only` / `ours-only` are
  pixels inked on one side only. Text: PDFKit `PDFPage.string`.

## Where the visible differences come from

| page | differing px | ink ref / ours | ref-only / ours-only | attribution |
|---|---|---|---|---|
| 1 | 185 416 (9.6 %) | 48 268 / 65 032 | 44 173 / 60 937 | producer: `\hrule`, `\vspace`, `\Large\bfseries`, `center`, `\problem`/`\subsection*`, `\hfill`, `\in`, `\mathbb`, `\mid`, `\text`, `\forall`, `\exists`, `array` are all reported unsupported by the compiler and typeset as their source text; the export paints exactly those glyphs at the producer's origins |
| 2 | 119 310 | 37 297 / 31 303 | 36 527 / 30 533 | same (page break falls elsewhere because page 1 is longer) |
| 3 | 94 490 | 29 475 / 29 892 | 27 074 / 27 491 | same |

Every glyph/rule in the export is one the display list asked for, at the
tick it asked for: replaying the written `Tf`/`Tm`/`Tj` operators of all
three pages in exact rational arithmetic (the `/W` widths for the joined
glyphs) puts all 3 232 glyphs on their display-list origins, 0 mismatches
(scratch `replay.py`, same rule as `exact::glyph_positions`). The two rules
(`\sqrt` overbars, 457 548 ticks = 0.436 pt thick) land where the list puts
them. So on HW1 there is no *painting* difference attributable to the export;
the remaining export-owned property is searchable text, and that had a
defect.

## Export defect found and fixed: `/W` widths from hmtx broke word boundaries

PDFKit text of page 1, before:

```
11 PM on W ednesday , the 2nd of September, 2026.
```

after (this change), and in the reference:

```
11 PM on Wednesday, the 2nd of September, 2026.
```

Cause. The producer lays out with TFM metrics; the exact route wrote each
CID's `/W` width from the OpenType hmtx. For most Latin Modern glyphs the two
agree to a thousandth of an em, but not for the glyphs whose TFM width is
narrower than their outline — bold `W` (TFM 1093/1000 em, hmtx 1189), bold
`y` (511 vs 607), regular `F` (569.5 vs 653), `Y` (666.7 vs 750), `A`
(722.2 vs 750), `y` (444.5 with the `y,` kern folded in). With hmtx widths
the viewer's pen after `W` is 96/1000 em past where `e` actually starts;
PDFKit treats that as a word gap on both sides. A `Tc`-based experiment (pen
moved back exactly, glyphs unchanged) did **not** change the extraction —
PDFKit reads the gap from `/W`, not from the pen — while patching the two
`/W` entries in the file did, with no content-stream change.

Fix (`crates/pdf/src/v2.rs`). Each glyph's observed `advance_x / font_size`
(in 1000/em, as a reduced ratio) is counted per font and glyph while the list
is walked; after subsetting, the most frequent value replaces the hmtx `/W`
entry where it differs (ties go to the larger width; kerns to a following
glyph are folded into `advance_x` by the producer and are the minority). The
join test uses the written width, so `exact::glyph_positions` still replays
every glyph to its envelope origin (tests). Painting is untouched: every
glyph keeps its own absolute `Tm` unless the join is exact.

Measured on HW1 (before `e71ef676…`, after `5d409217…`, 280 510 → 280 922
bytes; 31 + 48 + 1 + 4 `/W` entries replaced):

| check | result |
|---|---|
| CoreGraphics 144 dpi, before vs after, page 1/2/3 | **0 / 0 / 0 differing pixels**; ink 65 032 / 31 303 / 29 892 on both sides |
| PDFKit text, before vs after, all pages | one line changed: the `Wednesday,` line above; the other 72 lines byte-identical |
| `flashtex-pdf-exact` self-check (xref, structure) | passes |
| `cargo test` (crates/pdf) | 87 passed (42 unit, 12 exact, 25 render, 3 type1, 5 v2; none skipped, Latin Modern 12 present) |

Not fixed by this (and not the export's): `\f orallm` in the extraction of
the recovered `\forall` text — the producer places the glyph after math
italic `f` at `f` + width + italic correction (107/1000 em), and PDFKit
reads a 0.107 em gap as a space. pdfTeX places it the same way; whether the
reference extracts `f(x)` as one word was not measured.

## Fonts: one bundling gap seen from the export side

`from-v2` resolved `LMRoman8-Regular` (the 8 pt optical face the producer
draws math scripts from since `f762f82`) at
`/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/lm/lmroman8-regular.otf`,
not from `apps/mac/Fonts`, because the bundle carries `lmroman{7,10,12,17}`
only (README: "so the app does not depend on a TeX installation"). On a Mac
without MacTeX the exact export of any 11 pt/12 pt document with math
scripts refuses with "font LMRoman8-Regular (sha256 …, … bytes) was not found in … directories" (the route
never approximates). `lmroman6-regular.otf` is the same for
`\scriptscriptstyle`. The producer needs the same files for its own glyph
ids (`rm-lmr8`, `rm-lmr6` TFMs are already bundled). Reported to the parent
for `apps/mac/Fonts` (font-resources ownership); not changed by this lane.

## Reproduction

```sh
cargo build --release --manifest-path crates/pdf/Cargo.toml
FLASHTEX_FONT_DIRS=$PWD/apps/mac/Fonts FLASHTEX_TFM_DIRS=$PWD/apps/mac/Fonts/texmf/fonts/tfm/public/lm \
  flashtex-render --v2 hw1.v2.json < hw1-req.jsonl      # request as tools/real-world-corpus/run.py builds it
crates/pdf/target/release/flashtex-pdf-exact from-v2 hw1.v2.json --out hw1.pdf --font-dir apps/mac/Fonts
# raster + text: CoreGraphics/PDFKit scripts kept in the lane's scratch
# directory (cgdiff.swift, pdftext.swift; ~60 lines each), see
# coordination/mac-pdf-2.md for their paths on mac-m1max-a.
```
