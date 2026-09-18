# Beamer conformance audit — 2026-09-18T172738Z

Program start for full `\documentclass{beamer}` support (issue #841). This is a
measurement and a gap audit only; no engine change was made in this lane.
`report.json` next to this file holds the same numbers machine-readably.

## What was measured

- Fixtures: the five decks committed in `fixtures/real-world/beamer-*` (commit
  `f8616a5f`), references by MacTeX 2026 `pdfTeX 3.141592653-2.6-1.40.29 (TeX
  Live 2026)` under `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, run to
  convergence by `tools/real-world-corpus/run.py:generate_reference`; every
  reference has zero overfull boxes and zero LaTeX warnings. pdflatex is an
  oracle only and is never in the product path.
- Producer: `flashtex-render --v2 --images` + `flashtex-pdf-exact from-v2`
  built from branch `beamer/corpus` = `origin/main bf3ae45e` + PR #855
  (`b7072f9f`, compiler: one page per frame, `\frametitle`, `\alert`) + PR
  #887 (class-geometry: `ClassKind::Beamer`, 128×96 mm, `aspectratio`).
  `crates/render-pipeline` links `crates/class-geometry` directly, so #887 was
  live; the compiler it links is the pinned `vendor/compiler` (`4511166e`), so
  the #855 source diff (`color.rs`, `parser.rs`, `supported.rs`) was applied
  to the vendor mirror in the worktree only (a full `git archive` mirror of the
  merged `crates/compiler` no longer compiles against the pipeline:
  `Inline::PageStyle` and `tabular::Align::Flexible` are newer than the pin).
  Nothing under `crates/` is committed by this lane.
  Render binary sha256 `86316b5371f5755c…`, pdf-exact `927268dfdc38aa03…`.
- Fonts: `FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` = TeX Live
  2026 `jknappen/ec:public/lm:public/amsfonts/symbols`; the harness reported
  zero font-FAILURE diagnostics in every fixture.
- Harness: `tools/visual-oracle/rank.py --only beamer-default --only
  beamer-madrid --only beamer-overlays --only beamer-blocks-columns --only
  beamer-fragile --render <flashtex-render> --pdf-exact <flashtex-pdf-exact>
  --top 5 --thumbs 0` (Ghostscript at 144 dpi, 8-bit grey), plus `flashtex
  check <deck>/main.tex --diagnostics short` per deck. Positions are in bp,
  y from the page top. Word alignment is by text (difflib); when our page
  count is short the pages after the first missing one are compared against
  the wrong reference page, so the aligned-word counts are floors, not a
  parity measure.

## Per-deck results

| deck | pages ours/ref | compared pages | aligned words (ref / cand on compared pages) | max abs dx / dy (bp) | differing px % per page (mean) | diagnostics by code | `flashtex check` |
|---|---|---|---|---|---|---|---|
| beamer-default | **6/7** | 6 | 28 (270 / 286) | 233.4 / 124.2 | 6.8 8.1 6.6 6.7 7.2 5.9 (6.9) | unknown_command 3, math_resource_profile 1 | recovered, 6 pages, 3 errors, 1 warning |
| beamer-madrid | **6/7** | 6 | 34 (311 / 286) | 317.5 / 125.2 | 26.8 20.9 19.2 19.4 19.9 18.8 (20.8) | unknown_command 4, syntax_error 2, math_resource_profile 1 | recovered, 6 pages, 6 errors, 1 warning |
| beamer-overlays | **6/14** | 6 | 40 (165 / 203) | 2069.8 / 1949.6 (see note) | 2.7 3.7 5.2 4.2 3.7 3.3 (3.8) | unknown_command 8, syntax_error 1 | recovered, 6 pages, 9 errors, 0 warnings |
| beamer-blocks-columns | **5/6** | 5 | 10 (156 / 205) | 189.2 / 111.5 | 4.0 7.1 4.9 3.5 5.0 (4.9) | unknown_command 17 | recovered, 5 pages, 10 errors, 7 warnings |
| beamer-fragile | **6/8** | 6 | 16 (148 / 182) | 186.7 / 216.4 | 2.9 4.0 3.3 6.3 3.6 1.9 (3.7) | unsupported_feature 1, unknown_command 5, unsupported_block 1 | recovered, 6 pages, 5 errors, 2 warnings |

Page sizes match: every candidate page rasterised to 726×544 px, the same as
the reference (128×96 mm; #887 works through the pipeline). The differing-px
percentages are low only because both sides are mostly white; the Madrid
number is high because the reference paints a filled frametitle bar and
footline on every page.

Note on beamer-overlays: `pdftext.py` replays beamer's covered text
(`\pause`, `\uncover`, `\onslide` on the slides where it is hidden) at
positions far off the page (about x = 2100, y = −1840 bp), so the top deltas
on that deck are a harness artefact, not a layout error; the page count
(6 of 14) is the real measurement there.

### Diagnostics by construct

| deck | construct | code | count |
|---|---|---|---|
| beamer-default | `\subtitle`, `\institute` in the preamble; `\titlepage` | unknown_command | 1, 1, 1 |
| beamer-madrid | `\usetheme`, `\subtitle`, `\institute`, `\titlepage` | unknown_command | 1 each |
| beamer-madrid | `\title[short]{..}`, `\author[short]{..}` "requires a braced argument" (the optional short form is not accepted and the title is lost) | syntax_error | 2 |
| beamer-overlays | `\pause` ×2, `\only` ×2, `\onslide` ×2, `\uncover`, `\setbeamertemplate` | unknown_command | 8 |
| beamer-overlays | `\alert<2>{..}` "requires a braced argument" (the overlay spec is read as the argument) | syntax_error | 1 |
| beamer-blocks-columns | `\column` ×4, `\textwidth` ×4 (as a length in `\column{.5\textwidth}`), `\setbeamertemplate`, `\titlepage` | unknown_command (error) | 10 |
| beamer-blocks-columns | environments `block` ×3, `columns` ×2, `alertblock`, `exampleblock` "not implemented; body typeset as plain text" | unknown_command (warning) | 7 |
| beamer-fragile | `\note` ×2 (**its argument is typeset as plain text**), `\frame` (command form), `\setbeamertemplate`, `\titlepage` | unknown_command | 5 |
| beamer-fragile | `listings` recognised but not implemented; `lstlisting` block partially typeset | unsupported_feature, unsupported_block | 1, 1 |

### Top 5 positional deltas per deck (aligned words, bp)

beamer-default
1. p4 `The` dx +185.36 dy −94.09 — ref (50.17, 129.41) cand (235.53, 35.31), CMSS10→LMRoman10, `main.tex:1660-1663`
2. p3 `the` dx +233.40 dy −39.18 — ref (50.17, 158.58) cand (283.57, 119.40), `main.tex:1527-1530`
3. p3 `is` dx +226.87 dy −39.18 — `main.tex:1536-1538`
4. p4 `The` dx +158.93 dy −97.08 — `main.tex:1738-1741`
5. p6 `of` dx −107.41 dy −74.57 — `main.tex:2428-2430`

beamer-madrid
1. p1 `1` dx −317.52 dy −104.94 — the reference footline counter `1/7` (CMSS8, 6 pt, baseline 269.47) aligned to our "1 Motivation" section heading, `main.tex:965-973`
2. p4 `The` dx +202.80 dy −92.44 — `main.tex:1768-1771`
3. p5 `the` dx −247.41 dy +43.69 — `main.tex:2401-2404`
4. p3 `the` dx +250.84 dy −40.23 — `main.tex:1635-1638`
5. p3 `is` dx +244.30 dy −40.23 — `main.tex:1644-1646`

beamer-overlays (artefact, see note): p4 `The` (−2069.82, +1943.66) `main.tex:1776-1779`; p4 `the` `main.tex:1715-1718`; p1 `Break`/`lines`/`and` `main.tex:541-556` — every one is a covered word the reference typeset off-page.

beamer-blocks-columns
1. p4 `a` dx +189.19 dy +77.32 — the reference frametitle "A table" (page 5) aligned to our caption text, `main.tex:1999-2000`
2. p3 `the` dx −105.58 dy −111.45 — `main.tex:1604-1607`
3. p3 `of` dx −105.58 dy −111.45 — `main.tex:1601-1603`
4. p2 `of` dx +91.17 dy +108.90 — `main.tex:1360-1362`
5. p3 `The` dx −100.22 dy −65.05 — `main.tex:1567-1570`

beamer-fragile
1. p5 `the` dx +186.73 dy −143.94 — `main.tex:1860-1863`
2. p5 `the` dx −114.75 dy −130.39 — `main.tex:1880-1883`
3. p4 `twelve.` dx +4.73 dy −216.40 — ref (74.28, 227.36) cand (79.01, 10.96): the `[allowframebreaks]` list broke after item 11 in ours and after item 14 in the reference, `main.tex:1334-1341`
4. p4 `Item` dx +3.09 dy −216.40 — `main.tex:1329-1333`
5. p4 `thirteen.` dx +4.73 dy −210.51 — `main.tex:1357-1366`

Every delta above is dominated by page and block placement (missing title
page, top-aligned body, printed `\section`), not by line breaking: on
beamer-default page 2 our first two lines break at the same words as the
reference even though we shape with LMRoman10 metrics and pdflatex with
cmss10.

### What the candidate pages look like (from the rasters and the v2 display list)

- Body: `LMRoman10-Regular` at 10.91 bp (10.95 pt — the size is right, the
  family is wrong: beamer is `cmss`), justified (beamer is `\raggedright`),
  `\parindent` 0 (right), first baseline 35.31 bp below the frametitle
  heading, body starts at the top of the frame (reference [c]-centres it).
- Frametitle: a level-1 heading in `LMRoman12-Bold` at 14.35 bp, black, at
  x = 28.35, baseline 10.96 (reference: `cmss12` at 14.4 pt, regular weight,
  structure colour, x = 8.50, baseline 21.06).
- `\section{Motivation}` between frames is typeset as "1 Motivation" on the
  preceding page; a page number is typeset at (178.7, 272.1) — the paper's
  bottom edge (#887's `foot_baseline = 96mm` with `PageStyle::Plain`).
- No navigation symbols; no title page (an empty frame emits no page); no
  Madrid bars; blocks and columns flattened to prose; `\note` text printed.

## Beamer's defaults, measured (pdflatex, TeX Live 2026)

Probe: a one-frame document with `\typeout` of the lengths, fonts and
`\extractcolorspec` of the beamer colours inside a frame, plus a
`\begin{frame}` ladder of 1, 2, 3 and 7 lines, `[t]`, `[plain]`, no title,
and an itemize, read back with `tools/visual-oracle/pdftext.py`. Values are
pt unless marked bp (1 bp = 1.00375 pt).

Paper and margins (`beamer.cls`, `geometry` with `hmargin=1cm, vmargin=0cm`;
`beamerbaseframecomponents.sty` recomputes head/foot from the templates):

| length | default theme | Madrid |
|---|---|---|
| `\paperwidth` × `\paperheight` | 364.195 × 273.147 (128 × 96 mm) | same |
| `\beamer@leftmargin` = `\beamer@rightmargin` | 28.453 (1 cm) | 10.950 (1 em, infolines `text margin left/right=1em`) |
| `\textwidth` | 307.290 | 342.295 |
| `\headheight` (headline template ht+dp) | 0 | 0 (Madrid keeps the empty default headline unless `[secheader]`) |
| `\footheight` (footline ht+dp + 4 pt) | 4.0 | 12.667 (three boxes `ht=2.25ex dp=1ex` in `\tiny` = 6 pt cmss8) |
| `\textheight` = paperheight − headheight − footheight | 269.147 | 260.480 |
| `\parskip` / `\parindent` | 0 / 0 | same |
| `\rightskip` | `0pt plus 1fil` (`\raggedright` in beamer.cls) | same |
| `\baselineskip` (11 pt: `size11.clo`, cmss10 at 10.95) | 13.6 | same |
| `\leftmargini` = `ii` = `iii`; `\labelsep` | 21.9 (2 em); 5.475 | same |
| list `\topsep` / `\itemsep` / `\parsep` / `\partopsep` (level 1) | 3 ± / 3 ± / 0 / 0 (`beamerbaselocalstructure.sty`) | same |

Fonts (`beamerfontthemedefault.sty`; OT1 `cmss`, i.e. `lmss` under lmodern):
normal text cmss10 at 10.95; frametitle `\Large` = cmss12 at 14.4 with
`\baselineskip` 18; framesubtitle cmss9; title cmss12 at 14.4; subtitle,
author, date 10.95; institute `\scriptsize` cmss8; block title `\large` =
cmss12 at 12; block body 10.95; footnote `\footnotesize` cmss9; caption cmss10
at 10; head/foot (infolines) `\tiny` cmss8 at 6; verbatim cmtt10 at 10.95;
`lstlisting` with `basicstyle=\ttfamily\small` cmtt10 at 10.

Colours (`beamercolorthemedefault.sty`, `whale` + `orchid` for Madrid):

| beamer colour | default fg / bg | Madrid fg / bg |
|---|---|---|
| structure, item, title, frametitle (default) | rgb(0.2,0.2,0.7) / none | frametitle white / rgb(0.2,0.2,0.7); title white / rgb(0.2,0.2,0.7) |
| alerted text | rgb(1,0,0) | same |
| example text | rgb(0,0.5,0) | same |
| block title / block body | rgb(0.2,0.2,0.7) / none; black / none | white / rgb(0.15,0.15,0.525); black / rgb(0.915,0.915,0.9525) |
| block title alerted / example | rgb(1,0,0) / rgb(0,0.5,0) | white on the orchid red / green backgrounds |
| navigation symbols | rgb(0.68,0.68,0.88) | same |
| palette primary / secondary / tertiary bg | — | rgb(0.2,0.2,0.7) / rgb(0.15,0.15,0.525) / rgb(0.1,0.1,0.35) (date / title / author boxes of the footline) |

Frame geometry (measured on the ladder, default theme, bp from the page top):

- Frametitle (`beamerouterthemedefault.sty` `frametitle` template): a
  `beamercolorbox` with `sep=0.3cm` and width `\textwidth + left + right
  margin` = the paper width, so the text starts at x = **8.50** (0.3 cm from
  the paper edge, not from the text edge) with baseline **21.06**; the box
  contributes 22.9 pt (incl. the `\vskip0.25em` after it) to the frame.
  Madrid: same x, baseline 20.06, on a filled bar the full paper width.
- Body placement (`beamerbaseframe.sty` keys `c`/`t`): the frame body is a
  `\vbox to \beamer@frametextheight` = `\textheight` − frametitle box, with
  glue `0pt plus 1fill` above and `0pt plus 1.5fill` below for the default
  `[c]` — the body sits at **40 %** of the free space, not centred. Measured
  first baselines: 1 line 129.06, 2 lines 123.64, 3 lines 118.22, 7 lines
  96.54 (−5.42 bp = 0.4 × 13.55 per extra line); without a frametitle a
  2-line body is at 109.97; `[t]` uses `.2cm plus .5\paperheight` above and
  `1fill` below, first baseline **42.01**; `[plain]` 2 lines at 120.01 (body
  box is `\paperheight` high, no head/foot); `[b]` is 1fill above, 0 below.
- Text column: x = **28.35** (1 cm); Madrid x = 10.91.
- Itemize: label `\blacktriangleright` (MSAM10 char, structure colour) at
  x = 36.23, text at x = 50.17 (28.35 + 2 em); item baselines 16.54 apart
  (13.55 + 3 pt `\itemsep`); enumerate label "1." at 36.23 in the structure
  colour. Madrid (`rounded` inner theme) draws ball items and rounded blocks.
- Blocks (default inner theme, no background): title cmss12 at 12 pt in the
  structure colour at x = 28.35; first body baseline 13.33 below the title
  baseline; next block title 22.5 bp below the last body baseline (block
  titles measured at 84.58, 133.98, 183.37 on beamer-blocks-columns p2).
- Columns (`beamerbaseframecomponents.sty`): an `\hbox to \textwidth` that
  backs up by `\beamer@leftmargin` and lays the columns in a box the paper
  width wide with `\hfill` before, between and after; two `.5\textwidth`
  columns therefore start at x = **18.90** and **190.87** (gaps of 18.97 pt);
  `[T]` = `t` alignment plus `\vskip-1ex\nointerlineskip` at the column head;
  `[onlytextwidth]` keeps the text width instead.
- Title page (default `title page` template, all centred): title baseline
  85.41, subtitle 102.47, author 140.02, institute (cmss8) 162.49, date
  188.55; the whole title page is itself a `[c]` frame body.
- Footnote in a frame: text cmss9 at baseline **268.14** (bottom of the
  frame text area), mark cmss8 at x = 41.26, text x = 44.93; Madrid puts it
  above the footline.
- Navigation symbols (default `sidebar right` template): `\llap` of six pgf
  symbols with `\hskip0.1cm` at the right paper edge, `\vskip2pt` above the
  paper bottom, colour rgb(0.68,0.68,0.88), roughly 125 bp wide on the raster;
  drawn on every page unless `\setbeamertemplate{navigation symbols}{}`.
- Madrid footline (`beamerouterthemeinfolines.sty`): three boxes each
  `.333333\paperwidth` wide, `ht=2.25ex dp=1ex` in `\tiny`, centred text;
  author (`\insertshortauthor` + `(\insertshortinstitute)`), short title,
  short date + `\insertframenumber / \inserttotalframenumber`; baseline at
  **269.47**, texts at x = 31.44 / 165.98 / 280.78 and the counter at 345.87.
- Overlay rule (`beamerbaseoverlay.sty`): a frame is typeset once per slide
  for slide = 1 … N, where N = max(1, the largest number in any overlay
  specification in the frame — `<3->` and `<3>` both count 3, `<+->`
  advances `beamerpauses` — and the number of `\pause` + 1). Each slide is
  its own PDF page with the same `\insertframenumber`; covered material
  (`\pause`, `\uncover`, `\onslide`, `\item<n->`) keeps its space and is
  painted invisible (`\setbeamercovered{invisible}` default); `\only`
  removes its argument entirely on other slides (reflow); `\alert<2>` colours
  on slide 2 only. The corpus deck is 3 + 3 + 2 + 2 + 3 + 1 = 14 pages.
- `[allowframebreaks]`: the body is `\vsplit` at `\beamer@autobreakfactor`
  (0.95) × `\textheight`; each continuation repeats the frametitle with the
  `frametitle continuation` template (` I`, ` II`, roman); the reference
  breaks after item 14 of 20.
- `[fragile]`: the frame body is written to a file and read back so
  `verbatim` / `lstlisting` work; `\end{frame}` must start a line.
- `\frame{...}` is the command form of the environment; `\section` between
  frames typesets nothing in the default theme (only `\AtBeginSection`
  hooks would); `\note{...}` prints nothing unless `\setbeameroption{show
  notes}`.

## Gap audit and tiered plan (draft tracking-issue body)

Owner crates: **compiler** (`crates/compiler`: parsing beamer's commands and
environments into blocks), **class-geometry** (`crates/class-geometry`: page,
text block, fonts and sizes per class and option), **document-style**
(`crates/render-pipeline/vendor/document-style` + `crates/document-style`:
per-class list, heading, caption and footnote parameters),
**render-pipeline** (`crates/render-pipeline`: page builder, frame body
placement, boxes, colour fills, page multiplication).

### Tier 0 — page model (blocks every deck)

| item | pdflatex-measured | owner | #855/#887 give | acceptance |
|---|---|---|---|---|
| One page per frame, **including an empty or `\titlepage`-only frame** | 7 pages for beamer-default | compiler (#855 emits `Block::PageBreak` at `\begin{frame}`; an empty first frame produces no page) | #855: page per non-empty frame | `beamer-default`, `-madrid`, `-blocks-columns`, `-fragile` render 7 / 7 / 6 / 8 pages with `\titlepage` still unknown |
| `\section`/`\subsection` between frames typeset nothing | no "1 Motivation" text anywhere | compiler | — | no word of the section title on any page of beamer-default |
| No page number; `empty` page style; navigation symbols strip bottom-right (rgb 0.68,0.68,0.88) unless `\setbeamertemplate{navigation symbols}{}` | number absent; symbols 0.1 cm from the right edge, 2 pt above the bottom | class-geometry (`PageStyle::Empty` for beamer, drop `foot_baseline = 96mm`), render-pipeline (symbol strip), compiler (`\setbeamertemplate` in the preamble accepted, at least for `navigation symbols`) | #887: `PageStyle::Plain`, foot baseline at the paper bottom (prints "1" at y = 272.1) | no digit glyph at the page bottom on any deck; beamer-default pages show ≥ 100 differing px in the bottom-right 125×12 bp strip and beamer-overlays (symbols off) shows 0 |
| Sans-serif body: `\familydefault = \sfdefault` → cmss10 at 10.95 / `lmsans10-regular.otf` (present in `apps/mac/Fonts`) and `\raggedright` | body CMSS10 at 10.91 bp, `\rightskip 0pt plus 1fil` | class-geometry (`body_font` for `Beamer` = sans family), render-pipeline (`rightskip` fil for the class) | #887: `body_font(Pt11)` = roman; no ragged right | v2 font table of beamer-default lists `LMSans10-Regular`, no `LMRoman10`; every line's right edge < 334.49 except lines the reference also fills |
| `\title[short]{}` / `\author[short]{}` / `\institute[short]{}` / `\date[short]{}` / `\subtitle{}` in the preamble; `\titlepage` | see title page baselines above | compiler (optional short argument; store subtitle/institute), render-pipeline (title page block: centred `\Large` title in structure colour, subtitle, author, `\scriptsize` institute, date; the page is itself a `[c]` body) | — (both `syntax_error` today) | beamer-default p1 aligns all 15 reference words within 0.5 bp |

### Tier 1 — frame geometry (every frame of every deck)

| item | pdflatex-measured | owner | #855/#887 give | acceptance |
|---|---|---|---|---|
| Frametitle box: `\Large` cmss12 at 14.4, regular weight, structure colour rgb(0.2,0.2,0.7), text at x = 8.50 bp (0.3 cm from the paper edge), baseline 21.06; box + `0.25em` = 22.9 pt; `\framesubtitle` cmss9 below | see above | compiler (#855's `Block::Heading{level 1}` is the hook; a dedicated `Block::FrameTitle` avoids heading numbering/spacing), render-pipeline (title box geometry), class-geometry (frametitle font/colour for the class) | #855: `\frametitle` → bold level-1 heading at the text margin, baseline 10.96 | on every titled page of beamer-default the first reference word of the title is within 0.5 bp in x and y and the run's fill is (0.2,0.2,0.7) |
| Body box `\textheight − frametitle box`, `[c]` default with 1fill : 1.5fill glue (40 % of the free space above), `[t]` `.2cm plus .5\paperheight`, `[b]`, `[plain]` uses `\paperheight` and no head/foot | first baselines 129.06 / 123.64 / 118.22 / 96.54 for 1/2/3/7 lines; `[t]` 42.01; no title 109.97; `[plain]` 120.01 | render-pipeline (frame body placement), compiler (keep `[t]`/`[b]`/`[c]`/`[plain]` — #855 skips the options) | #887: `text_top = 0`, `first_baseline = 11pt` (top-aligned) | a ladder document (1/2/3/7 lines, `[t]`, `[plain]`, no title; to be added as a fixture with the fix) matches every first baseline within 0.5 bp; beamer-default p2–p7 first body word within 0.5 bp |
| Text margins 1 cm; Madrid 1 em (10.95 pt) with `\textwidth` 342.30 | x = 28.35 / 10.91 | class-geometry (`\setbeamersize{text margin left/right}` from the theme) | #887: 1 cm | Madrid p2 first body word at x = 10.91 ± 0.5 |
| Lists: `\leftmargini` = 2 em, `\labelsep` .5 em, `\itemsep` 3 pt, `\topsep` 3 pt, `\parsep` 0; itemize label ▶ (MSAM10) in the structure colour, enumerate "1." in the structure colour; Madrid ball items | label x 36.23, text x 50.17, item pitch 16.54 | document-style (beamer list parameters), render-pipeline (label glyph + colour) | — (article lists) | beamer-default p3 and p4: every word within 0.5 bp |
| `\footnote` in a frame: cmss9 text at baseline 268.14, mark cmss8, text x = 44.93 | see above | render-pipeline (footnotes at the bottom of the frame body, above the footline) | — | beamer-blocks-columns p6 footnote words within 0.5 bp |

### Tier 2 — overlays (beamer-overlays: 14 pages)

| item | pdflatex-measured | owner | #855/#887 give | acceptance |
|---|---|---|---|---|
| Slide count per frame = max(1, largest overlay number, `\pause` count + 1); one page per slide, same frame number | 3, 3, 2, 2, 3, 1 | compiler (parse `<…>` specs on `\item`, `\alert`, `\only`, `\uncover`, `\onslide`, `\pause`; emit the frame once per slide with a `visible`/`covered`/`removed` mark per inline or block) | #855: skips `\begin{frame}<…>` specs; `\alert<2>{}` is a `syntax_error` | beamer-overlays renders 14 pages; each frame's pages count equals the comment in `main.tex` |
| Covered text keeps its space and is painted invisible (`\pause`, `\uncover<2->`, `\item<2->`, `\onslide<2->`); `\only` removes and reflows; `\alert<2>` red on slide 2 only | on p4/p5 the "line after" word does not move between slides; p1–p2 of the `\only` frame reflow | render-pipeline (invisible paint or omission of covered runs; the harness must then compare covered words by absence) | — | for every frame, the visible words of slide k are a superset of slide k−1 (except the `\only` frame) and their positions are identical within 0.01 bp; page 14 aligns all 30 reference words |

### Tier 3 — blocks, columns, graphics, tables (beamer-blocks-columns)

| item | pdflatex-measured | owner | #855/#887 give | acceptance |
|---|---|---|---|---|
| `block` / `alertblock` / `exampleblock`: title `\large` cmss12 at 12 in structure / red / green (0,0.5,0), body 13.33 bp below, blocks 22.5 bp apart; Madrid `rounded` inner theme paints title bg rgb(0.15,0.15,0.525) with white text and body bg rgb(0.915,0.915,0.9525) with rounded corners and a shadow | titles at 84.58 / 133.98 / 183.37 | compiler (`Block::BeamerBlock{kind, title, body}`), render-pipeline (box + optional fills) | — (bodies flattened to prose) | p2: all 50 reference words within 0.5 bp |
| `columns[T]` / `\column{.5\textwidth}`: a paper-wide `\hbox` with `\hfill` before/between/after, columns at x = 18.90 and 190.87; `[onlytextwidth]`, `[c]`, `[t]`, `[b]`; `\textwidth` as a length factor | see above | compiler (`\column`, `\textwidth` in a length expression, `columns` options), render-pipeline (side-by-side vboxes with the beamer distribution rule) | — | p3: `Left` at (18.90, 100.36) and `Right` at (190.87, 100.36) within 0.5 bp |
| `\includegraphics[width=.6\textwidth]` in `center`, `table` + `booktabs` + `\caption` in a frame (caption cmss10 at 10 pt, no float placement — beamer floats are in-line) | table words start at x = 120.96, baseline 95.22 | render-pipeline (already supports the pieces; needs `\textwidth` of the frame and no float mechanics) | — | p4 image box within 0.5 bp of the reference's XObject placement; p5 table words within 0.5 bp |

### Tier 4 — frame options and themes (beamer-fragile, beamer-madrid)

| item | pdflatex-measured | owner | #855/#887 give | acceptance |
|---|---|---|---|---|
| `\frame{...}` command form; `\note{...}` typesets nothing; `[plain]` | see above | compiler | — (`\frame` unknown, `\note` argument **printed**) | beamer-fragile renders 8 pages; the strings "presenter" and "hidden" appear on no page |
| `[fragile]` with `verbatim` (cmtt10 at 10.95, x = 28.35) and `lstlisting` (`basicstyle=\ttfamily\small`, cmtt10 at 10) | verbatim first baseline 121.87; listing 113.52 | compiler (fragile bodies are read verbatim until a line starting with `\end{frame}`), render-pipeline | — | p2 and p3 words within 0.5 bp |
| `[allowframebreaks]`: split at 0.95 `\textheight`, continuation titles ` I` / ` II` | break after item 14; p4 title "A long list I" | render-pipeline (split the frame body across pages, repeat the title with the roman suffix) | — | p4 has items 1–14 and p5 items 15–20 with titles "A long list I"/"II" |
| Madrid theme = `whale` + `orchid` colours, `rounded` inner, `infolines` outer: filled frametitle bar (white on 0.2,0.2,0.7), footline of three `.333333\paperwidth` boxes `ht=2.25ex dp=1ex` in `\tiny` with palette tertiary/secondary/primary backgrounds, short author `(institute)`, short title, short date + `frame/total`, baseline 269.47; `\textheight` 260.48; title page in a rounded filled box | see above | compiler (`\usetheme`, short-title arguments, `\inserttotalframenumber` needs a second pass or a pre-count), class-geometry (theme margins and foot height), render-pipeline (bars, footline boxes) | — | beamer-madrid p3: all 45 reference words within 0.5 bp and the frametitle bar / footline boxes each differ by < 1 % px |
| `aspectratio=169` etc. (already in #887) | 160 × 90 mm | class-geometry | #887 | `beamer_geometry.rs` (exists) |

### What the corpus shows is still wrong in #855 / #887

- #855: an empty frame (`\titlepage` unknown → nothing queued) produces no
  page, so four decks are one page short; `\section` between frames is still
  typeset as an article heading on the preceding page; `\frametitle` becomes a
  bold, black, text-margin level-1 heading (needs regular cmss12 at 14.4 in
  the structure colour, 0.3 cm from the paper edge); `\alert<2>{…}` fails to
  parse (`syntax_error`, overlay spec read as the argument); the `\begin{frame}`
  options are skipped, so `[t]`/`[plain]`/`[allowframebreaks]`/`[fragile]` all
  behave as `[c]` non-fragile; `\frame{…}` is not the command form of the
  environment; `\note{…}` prints its argument.
- #887: `text_top = 0` / `first_baseline = 11pt` top-align the body (beamer
  places it at 40 % of the free space under the frametitle box);
  `PageStyle::Plain` with `foot_baseline = 96mm` prints a page number on the
  paper's bottom edge (beamer prints none); `body_font` is roman (beamer is
  sans); no `\raggedright`; theme margins (Madrid 1 em) and foot height
  (12.67 pt) are not modelled. The page size, `aspectratio` table, 1 cm
  margins, 11 pt / 13.6 pt leading and `\parindent = 0` are confirmed by the
  measurement.
