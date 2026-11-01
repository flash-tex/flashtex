# flashtex-class-geometry

Original Rust model of what the LaTeX2e standard classes (`article`,
`report`, `book`) and the `geometry` package decide about the page before any
text is set: paper and text block, side/top margins for odd and even pages,
header/footer baselines, columns, sectioning skips and fonts, `\chapter`
layout, page-style header/footer slots and mark rules.

Every length is an `Sp` (TeX scaled point) computed with TeX's own integer
arithmetic (`round_decimals`, `xn_over_d`, `\divide` truncation,
`\@settopoint`), so results match pdflatex to the scaled point, not "within
rounding". No TeX engine is linked or run; zero dependencies.

```sh
cd crates/class-geometry
cargo test            # unit tests + the pdflatex oracle comparison (no TeX needed)
python3 oracle/generate.py   # regenerate tests/data/oracle.txt (needs pdflatex)
```

```rust
use flashtex_class_geometry::*;
let setup = DocumentSetup::from_preamble(r"\documentclass[11pt,twoside]{report}
\usepackage[margin=2cm,includehead]{geometry}")?;
let doc = resolve(&setup);
let x = doc.frame.text_left(2);           // even page, sp from paper left
let y = doc.frame.first_baseline;         // sp from paper top
let sec = doc.heading("section").unwrap();
```

Relation to `crates/document-style`: that crate (owned by
mac-document-style on mac-m1max-a, consumed by render-pipeline) models
article-only page geometry, lists and a style tree in floating-point pt.
This crate does not modify it; it covers report/book, all class options,
two-sided/two-column frames, the complete geometry algorithm, page styles
and chapters, in exact sp. See `CONTRACT.md` for integration.

## Oracle

`oracle/generate.py` writes one probe document per fixture, runs pdflatex
(MacTeX 2026, `pdfTeX 3.141592653-2.6-1.40.29`), and records
`\the<length>` for up to 31 lengths (incl. `\skip\footins`, `\mathindent`, body em/ex), `\c@secnumdepth`/`\c@tocdepth`, and
`\pdfsavepos` positions (exact sp) of: the first body line, an indented
second paragraph, `\section`/`\subsection`/`\subsubsection` headings and the
lines after them, a run-in `\paragraph`, column two, the first lines of
pages 2 and 3, the page number in the header or footer (via a `\thepage`
that carries a mark), and for report/book `\chapter`, `\chapter*`, and a
chapter after text. `pdftotext -bbox-layout` was not installed; `\pdfsavepos`
is exact instead of 0.01bp.

Result (`cargo test -- --nocapture`): see the `oracle:` line —
`oracle: 96/96 fixtures pass; lengths 2881/2881; positions 2112/2112; worst position error 0sp` (every `\the` length exact to the sp; every `\pdfsavepos` mark exact).

Fixture matrix (96): article/report/book × 10/11/12pt; article ×
a4/a5/b5/legal/executive × 10/11/12pt; landscape (letter 10/12, a4 11);
twoside (article ×3, report); book oneside; twocolumn (article ×4, book);
titlepage, notitlepage, openright, openany, fleqn, leqno, draft; page styles
headings (oneside, twoside, report), myheadings, empty; geometry: margin
(×4), left only, right only, textwidth, textwidth+left, hmargin={a,b},
vmargin, top only, bottom only, textheight, lines, scale, hscale, ratio,
hmarginratio+textwidth, includehead, includefoot, includeheadfoot,
head/foot lengths, includemp+marginpar, bindingoffset+twoside, centering,
landscape, a5paper, papersize, twoside inner/outer, heightrounded, nohead,
no options, twocolumn+columnsep, over-specification, twoside defaults,
width/height, book margins, three sequential `\geometry{}` sequences;
report/book chapters.

## Findings worth knowing

- **PDF page size without geometry is the engine default**, not
  `\paperwidth`: `\documentclass[a4paper]{article}` alone produces a US
  Letter MediaBox (MacTeX `pdftexconfig.tex`) with the A4 text block placed
  from the top. geometry's pdftex driver sets `\pdfpagewidth/height`.
- `\ProcessOptions` applies class options in declaration order
  (`landscape,a4paper` = landscape A4; `twoside,oneside` = two-sided).
- geometry's `\Gm@detall` case "only top (or left) margin given" assigns the
  *first* ratio share to the bottom/right margin (`\Gm@detiiandiii{#2}{#4}{#3}`),
  so `top=1in` alone leaves a bottom margin of 2/5 of the 30 % spare height.
- Chapter title baseline = `\topskip + 50pt + \baselineskip(\huge) + 20pt +
  \baselineskip(\Huge)` below the text top, also after preceding text
  (`\@vspacer` restores `\prevdepth`).
- One-sided `\ps@headings` redefines only the odd head/foot; the even foot
  of the previous style survives (irrelevant unless geometry turns
  `twoside` on later).

## Provenance

TeX Live 2026, `texmf-dist/tex/latex/base/` v1.4n (2025/01/22) and
`texmf-dist/tex/latex/geometry/geometry.sty` v6.0 (2026/03/07). Source line
numbers are cited next to each computation in `src/`.

| Value | Source |
|---|---|
| paper sizes, `landscape` swap | article.cls 53–74 (report/book identical) |
| option processing, defaults | article.cls 79–112; report.cls 96–117 (openright/openany, titlepage default); book.cls 98, 119 (twoside, openright) |
| `\baselineskip` 12/13.6/14.5pt, size table | size10/11/12.clo 47–86; `\@xpt`… latex.ltx |
| `\parindent` 15pt/17pt/1.5em, 1em twocolumn | size1x.clo 87–91 |
| `\headheight` 12pt, `\headsep` 25pt (bk10 .25in, bk11/12 .275in), `\topskip`, `\footskip` 30pt (bk10 .35in, bk11 .38in) | size1x.clo / bk1x.clo 95–100 |
| `\textwidth` min(paper−2in, 345/360/390pt), ×2 twocolumn, `\@settopoint` | size1x.clo 107–127 (bk1x: same lines) |
| `\textheight` whole `\baselineskip`s in paper−3.5in, + `\topskip` | size1x.clo 130–138 |
| `\marginparsep` 11/10pt (10pt twocolumn, bk 7pt), `\marginparpush` 5/5/7pt | size1x.clo 139–144 |
| `\oddsidemargin`, `\evensidemargin`, `\marginparwidth` (.4/.6 twoside, .5/.5 oneside, cap 2in) | size1x.clo 161–188 |
| `\topmargin` | size1x.clo 193–200 |
| `\footnotesep`, `\skip\footins` | size1x.clo 202–203 |
| `\parskip` 0pt plus 1pt | article.cls 117 |
| `\leftmargini` 2.5em (2em twocolumn), `\labelsep` .5em | article.cls 322–340 |
| `\columnsep` 10pt, `\columnseprule` 0pt, `\pagestyle{plain}` | article.cls 627–629; book.cls 734 `headings` |
| `\@startsection` parameters | article.cls 302–321 |
| `\@startsection`/`\@sect`/`\@xsect` semantics | latex.ltx 17231–17302 |
| `\part` | article.cls 268–301; report.cls `\part`, `\@endpart` |
| `\chapter`, `\@makechapterhead`, `\@makeschapterhead` | report.cls (book.cls identical + `\if@mainmatter`) |
| `secnumdepth`/`tocdepth` 3 (article) / 2 (report, book) | article.cls 255, 502; report.cls |
| `\ps@empty`, `\ps@plain` | latex.ltx 18305–18311 |
| `\ps@headings`, `\ps@myheadings`, mark rules | article.cls 131–168; report.cls/book.cls same block with `\chaptermark` |
| `\@outputpage` head/text/foot placement | latex.ltx 20880–20960 |
| `\columnwidth` | latex.ltx 9479–9483 |
| `\mathindent` | fleqn.clo `\AtEndOfClass{\mathindent\leftmargini}` |
| geometry keys, `\Gm@detall`, `\Gm@@process`, defaults 0.7 scale, 1:1 / 2:3 (twoside h) / 2:3 (v) | geometry.sty 67–71, 208–312, 421–618, 700–814, 1011–1018, 1125–1128 |
| body font em/ex (cmr10@10, cmr10@10.95, cmr12@12) | measured `\fontdimen6/5` with pdflatex |

## Not modelled

LaTeX 2.09 compatibility mode; `\newgeometry`/`\restoregeometry`/`\savegeometry`;
geometry `mag`, `truedimen`, `reset`, `layout` offsets beyond the basic keys
(`layoutwidth/height` are implemented but not oracle-tested); lengths
written as expressions (`0.1\paperwidth`, `\baselineskip`); non-CM body
fonts (em/ex); floats, footnotes, `\marginpar` stacking; title pages and
`\maketitle` (daniel-parent/maketitle, daniel-title/title-layout), list
spacing (daniel-parent-b/setlist-spacing), page-control commands
(daniel-parent/page-control). `part`, `mark_rules`, and `myheadings` mark
text are transcribed but not position-tested.
