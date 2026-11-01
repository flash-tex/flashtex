# flashtex-document-style

Original Rust model of what LaTeX's `article` class decides about a document
before any text is set: page geometry, the font size table, paragraph and
sectioning spacing, list measurements, a small block style tree with measured
inheritance, and a user overlay. Zero external crates; edition 2024; the JSON
form is hand-written.

No TeX engine is linked, shelled out to, or required at build or run time.
BasicTeX was used **only** as a reference oracle: the numbers below were
transcribed from its class files and then cross-checked against a running
pdflatex. Product constraints in `AGENTS.md` are unchanged.

```sh
cd crates/document-style
cargo test          # 18 exact-number tests + 1 oracle-gap test + doctest
```

```rust
use flashtex_document_style::*;

let sheet = Stylesheet::article(ClassOptions { paper: Paper::Letter, size: BaseSize::Pt12 })
    .with_geometry(Geometry::margin(Pt::inches(1.0)));
let page = sheet.page_layout();            // text_area in TeX pt, top-left origin
let (x, y, w, h) = page.text_area_bp();    // same in PDF big points
let body = sheet.resolve(&[Block::Document, Block::Section(1), Block::Paragraph]);
let heading = sheet.resolve(&[Block::Document, Block::Heading(1)]);
let json = sheet.to_json();                // inputs round-trip; `export` for adapters
```

## Units

Everything is a **TeX point** (`pt`, 72.27/in) because that is what the class
files compute in; `letterpaper` is 614.295 x 794.97 pt. PDF user space is big
points (`bp`, 72/in): 612 x 792. Convert at the adapter boundary with
`Pt::to_bp`, `PageLayout::text_area_bp`, or `PageLayout::paper_bp`. The
compiler's current constants (`crates/compiler/src/layout.rs`: 612 x 792,
72 pt margins) are bp values; they correspond to `geometry` `margin=1in`,
not to the article defaults.

## Provenance of every default

Reference distribution: BasicTeX at `/Library/TeX/texbin` (TeX Live 2026,
`pdfTeX 3.141592653-2.6-1.40.29`). Files (all v1.4n, 2025/01/22, LaTeX
Project Public License):

| Value | File | Source line(s) |
|---|---|---|
| Paper sizes | `/usr/local/texlive/2026basic/texmf-dist/tex/latex/base/article.cls` | `\DeclareOption{letterpaper}` 8.5in x 11in; `a4paper` 210mm x 297mm; `a5paper` 148mm x 210mm; `legalpaper` 8.5in x 14in |
| `\parskip` | `article.cls` | `\setlength\parskip{0\p@ \@plus \p@}` -> 0pt plus 1pt |
| `\section` .. `\subparagraph` | `article.cls` | `\@startsection` args: section `-3.5ex plus -1ex minus -.2ex` / `2.3ex plus .2ex` / `\Large\bfseries`; subsection `-3.25ex plus -1ex minus -.2ex` / `1.5ex plus .2ex` / `\large\bfseries`; subsubsection same skips / `\normalsize\bfseries`; paragraph `3.25ex plus 1ex minus .2ex` / `-1em` (run-in); subparagraph same, indented by `\parindent` |
| `\leftmargini..iv` | `article.cls` | 2.5em, 2.2em, 1.87em, 1.7em (onecolumn); `\leftmarginv/vi` 1em |
| `\labelsep`, `\labelwidth` | `article.cls` | `.5em`; `\leftmargini - \labelsep` |
| Size table | `size10.clo`, `size11.clo`, `size12.clo` | `\@setfontsize` calls; the size macros `\@xpt`=10, `\@xipt`=10.95, `\@xiipt`=12, `\@xivpt`=14.4, `\@xviipt`=17.28, `\@xxpt`=20.74, `\@xxvpt`=24.88 are in `latex.ltx` |
| `\parindent` | `size1x.clo` | 10pt: `15\p@`; 11pt: `17\p@`; 12pt: `1.5em` (= 17.62482pt with cmr12) |
| `\headheight`, `\headsep`, `\footskip` | `size1x.clo` | 12pt, 25pt, 30pt at every size |
| `\topskip` | `size1x.clo` | 10pt / 11pt / 12pt |
| nominal `\textwidth` | `size1x.clo` | `\@tempdimb` 345pt / 360pt / 390pt |
| `\marginparsep` | `size1x.clo` | 11pt (10pt class) / 10pt (11pt, 12pt) |
| `\partopsep` | `size1x.clo` | 2pt+1-1 / 3pt+1-1 / 3pt+2-2 |
| `\@listi..iii` | `size1x.clo` | see the list table below |
| `\@settopoint` | `latex.ltx` | `\divide#1\p@\multiply#1\p@` (truncate to whole points) |
| Computer Modern ex/em | pdflatex `\fontdimen5\font`, `\fontdimen6\font` | cmr10: 4.30554 / 10.00002; cmr10 at 10.95pt: 4.71457 / 10.95003; cmr12: 5.16667 / 11.74988 |
| geometry rules | `/usr/local/texlive/2026basic/texmf-dist/tex/latex/geometry/geometry.sty` v5.9 (2020/01/02) | `\Gm@detall`, `\Gm@detiiandiii`, `\Gm@detiv`, `\Gm@adjustbody`, `\Gm@@process`; defaults `\Gm@Dhscale`/`\Gm@Dvscale` 0.7, `\Gm@Dhratio` 1:1, `\Gm@Dvratio` 2:3 |

### Size table (`size`/`baselineskip`, pt)

| command | 10pt | 11pt | 12pt |
|---|---|---|---|
| `\tiny` | 5/6 | 6/7 | 6/7 |
| `\scriptsize` | 7/8 | 8/9.5 | 8/9.5 |
| `\footnotesize` | 8/9.5 | 9/11 | 10/12 |
| `\small` | 9/11 | 10/12 | 10.95/13.6 |
| `\normalsize` | 10/12 | 10.95/13.6 | 12/14.5 |
| `\large` | 12/14 | 12/14 | 14.4/18 |
| `\Large` | 14.4/18 | 14.4/18 | 17.28/22 |
| `\LARGE` | 17.28/22 | 17.28/22 | 20.74/25 |
| `\huge` | 20.74/25 | 20.74/25 | 24.88/30 |
| `\Huge` | 24.88/30 | 24.88/30 | 24.88/30 (`\let\Huge=\huge`) |

Correction to the dispatch brief: `\Large` at the 12pt class is 17.28pt with
**22pt** leading (`\@setfontsize\Large\@xviipt{22}`), not 20.74; 20.74 is the
`\LARGE` size. `\section` therefore uses 17.28/22 at 12pt and 14.4/18 at 10pt
and 11pt.

### Article page defaults, as the class computes them

```text
textwidth      = settopoint(min(paperwidth - 2in, nominal))
textheight     = trunc((paperheight - 2in - 1.5in) / baselineskip) * baselineskip + topskip
oddsidemargin  = settopoint(.5 (paperwidth - textwidth) - 1in)          (oneside)
evensidemargin = settopoint(paperwidth - 2in - textwidth - oddsidemargin)
marginparwidth = settopoint(min(2in, .5 (paperwidth - textwidth) - marginparsep - .8in))
topmargin      = settopoint(.5 (paperheight - 2in - headheight - headsep - textheight - footskip))
text area x    = 1in + oddsidemargin
text area y    = 1in + topmargin + headheight + headsep
first baseline = text area y + topskip
```

Values printed by pdflatex (`\the\textwidth` etc.) and asserted in
`tests/article.rs` for all twelve combinations:

| size | paper | textwidth | textheight | oddside | evenside | topmargin | marginparwidth |
|---|---|---|---|---|---|---|---|
| 10pt | letter | 345 | 550 | 62 | 62 | 16 | 65 |
| 10pt | a4 | 345 | 598 | 53 | 54 | 17 | 57 |
| 10pt | a5 | 276 | 346 | 0 | 0 | 19 | 3 |
| 10pt | legal | 345 | 766 | 62 | 62 | 17 | 65 |
| 11pt | letter | 360 | 541.40024 | 54 | 55 | 21 | 59 |
| 11pt | a4 | 360 | 595.80026 | 46 | 46 | 18 | 50 |
| 11pt | a5 | 276 | 351.00015 | 0 | 0 | 17 | 4 |
| 11pt | legal | 360 | 759.00034 | 54 | 55 | 20 | 59 |
| 12pt | letter | 390 | 548.5 | 39 | 40 | 17 | 44 |
| 12pt | a4 | 390 | 592 | 31 | 31 | 20 | 35 |
| 12pt | a5 | 276 | 345.5 | 0 | 0 | 20 | 4 |
| 12pt | legal | 390 | 766 | 39 | 40 | 17 | 44 |

So `article` 12pt letter has a **390pt** text width (not 345, which is the
10pt value), a 548.5pt text height, and its text area starts at
(111.27pt, 126.27pt) from the top-left of the paper.

### `geometry` overrides (verified against pdflatex, 12pt letter unless noted)

| options | textwidth | textheight | oddsidemargin | topmargin |
|---|---|---|---|---|
| `margin=1in` | 469.75502 (= 468bp) | 650.43001 | 0 | -37 |
| `top=1in,bottom=1in,left=1.25in,right=1in` | 451.68752 | 650.43001 | 18.0675 | -37 |
| `textwidth=400pt` | 400 | 556.47656 | 34.8775 | -13.87262 |
| `textwidth=400pt,left=1in` | 400 | 556.47656 | 0 | -13.87262 |
| `textheight=600pt` | 430.00462 | 600 | 19.8752 | -31.28201 |
| `left=1in` | 449.87982 | 556.47656 | 0 | -13.87262 |
| `top=1in` | 430.00462 | 627.30263 | 19.8752 | -37 |
| (none) | 430.00462 | 556.47656 | 19.8752 | -13.87262 |
| `margin=1in,includehead` | 469.75502 | 613.43001 | 0 | 0 |
| a4 `margin=2cm` | 483.69687 | 731.23584 | -15.36449 | -52.36449 |

Rules (per axis, "first" = left/top, "second" = right/bottom): with nothing
given the body is 0.7 of the paper and the margins split the rest 1:1
(horizontal) or 2:3 (vertical); a body alone is centred by the same ratio;
two of {first, body, second} determine the third; a single margin keeps the
*other* margin at its default-body share (geometry swaps the names in the
`first`-only case, so `top=1in` leaves the bottom at the 2/5 share) and the
body fills the rest. Without `includehead` the header sits inside the top
margin, hence the negative `\topmargin`.

### Lists (`\@listi..iii`; level 4+ inherit level-3 skips)

| size | level | leftmargin | labelwidth | topsep | parsep | itemsep | partopsep |
|---|---|---|---|---|---|---|---|
| 10pt | 1 | 25.00003 (2.5em) | 20.00003 | 8+2-4 | 4+2-1 | 4+2-1 | 2+1-1 |
| 10pt | 2 | 22.0 | 17.0 | 4+2-1 | 2+1-1 | 2+1-1 | 2+1-1 |
| 10pt | 3 | 18.69997 | 13.69997 | 2+1-1 | 0 | 2+1-1 | 1+0-1 |
| 11pt | 1 | 27.37506 | 21.90005 | 9+3-5 | 4.5+2-1 | 4.5+2-1 | 3+1-1 |
| 11pt | 2 | 24.09003 | 18.61502 | 4.5+2-1 | 2+1-1 | 2+1-1 | 3+1-1 |
| 11pt | 3 | 20.47649 | 15.00149 | 2+1-1 | 0 | 2+1-1 | 1+0-1 |
| 12pt | 1 | 29.3747 | 23.49976 | 10+4-6 | 5+2.5-1 | 5+2.5-1 | 3+2-2 |
| 12pt | 2 | 25.84969 | 19.97475 | 5+2.5-1 | 2.5+1-1 | 2.5+1-1 | 3+2-2 |
| 12pt | 3 | 21.97221 | 16.09727 | 2.5+1-1 | 0 | 2.5+1-1 | 1+0-1 |
| 12pt | 4 | 19.97475 | 14.09981 | (level 3) | (level 3) | (level 3) | (level 3) |

`labelsep` is 5pt / 5.475pt / 5.87494pt. Em multiples are computed from the
Computer Modern quad and agree with pdflatex to < 0.0001pt (TeX rounds to
scaled points).

## Style tree and inheritance

`Stylesheet::resolve(path)` walks a path of `Block`s from the document root:

- `Document`, `Section(n)`: containers; inherit body style.
- `Heading(n)`: font and leading from the size table; bold; `space_before`
  is the `\@startsection` before-skip and `space_after` the after-skip,
  **both evaluated in the body font's ex** (they are read before the heading
  font group opens); `Heading(4)`/`(5)` are run-in with `run_in_after = 1em`.
- `Paragraph`: inherits size/leading/indent/alignment/weight/shape;
  `space_before = \parskip` (or the list's `\parsep` inside an item);
  `first_line_indent = true` except inside items.
- `ParagraphAfterHeading`: the paragraph after `\section`..`\subsubsection`
  (`\@afterindentfalse`): no first-line indent.
- `List(kind)`: nesting depth counts `List` nodes; `left_margin`
  accumulates `\leftmargin` per level; `space_before/after = \topsep +
  \parskip` (the enclosing list's `\parsep` when nested); `parindent = 0`.
- `Item`: `space_before = \itemsep + \parsep`.
- `Align(..)`: `center`/`flushleft`/`flushright` (`\trivlist`): `\topsep +
  \parskip` around, alignment set.
- `Inline(..)`: `Emph` toggles italic, `Bold`/`Italic` set, `Size(name)`
  changes size and leading.

Inherited: font size, baselineskip, bold, italic, parindent, alignment,
left/right margin, list context. **Not inherited:** `space_before`,
`space_after`, `first_line_indent`, `run_in_after` (reset at every node).

`StyleDelta` is a CSS-like overlay: ordered `DeltaRule`s selecting one block
kind (or all), applied at every matching node during resolution, so an
override on `Document` propagates while one on `Heading(1)` stays local.
`parskip` can be replaced globally.

### Combining vertical skips (for the adapter)

TeX does not simply sum adjacent skips. The rules an adapter must apply:

1. Baseline pitch inside a block is `baselineskip` (interline glue), as long
   as the previous depth plus the next height stays below it.
2. `\addvspace` (used by `\@startsection`, `\list`, `\endlist`): when a skip
   meets glue already at the end of the list, only the larger natural part
   survives (`Skip::addvspace`).
3. `\parskip` glue is added when a paragraph starts, after any `\addvspace`.

Hence body -> heading baseline = `heading.baselineskip + heading.space_before`,
heading -> body = `body.baselineskip + heading.space_after`
(`Stylesheet::heading_gap_before/after`), list boundaries =
`body.baselineskip + \topsep + \parskip`, and a nested list's end followed by
the next outer item = `max(\topsep_inner, \itemsep_outer) + \parsep_outer`.

## Oracle check: baseline gaps

`tests/oracle_gaps.rs` asserts baseline-to-baseline distances that pdflatex
actually shipped (marks written with `\pdfsavepos`/`\pdflastypos` at
shipout, letterpaper, each size):

```latex
\documentclass[12pt,letterpaper]{article}
\newcommand\mk[1]{\pdfsavepos\write16{POS #1 \the\pdflastypos}}
\begin{document}
Body line one.\mk{p1}

Body line two.\mk{p2}
\section[Heading text]{Heading text\mk{h1}}
Text after the section.\mk{p3}

More body text here.\mk{p4}
\subsection[Subheading text]{Subheading text\mk{h2}}
After the subsection.\mk{p5}
\subsubsection[Subsub text]{Subsub text\mk{h3}}
After the subsubsection.\mk{p6}
\begin{itemize}
\item First item\mk{i1}
\item Second item\mk{i2}
\begin{itemize}
\item Nested item\mk{i3}
\end{itemize}
\item Third item\mk{i4}
\end{itemize}
After the list.\mk{p7}

Last paragraph.\mk{p8}
\end{document}
```

| gap (pt) | 10pt | 11pt | 12pt | model |
|---|---|---|---|---|
| p2 -> h1 (body -> `\section`) | 33.0694 | 34.5010 | 40.0833 | `\Large` leading + 3.5ex |
| h1 -> p3 | 21.9028 | 24.4435 | 26.3833 | body leading + 2.3ex |
| p4 -> h2 (`\subsection`) | 27.9930 | 29.3223 | 34.7917 | `\large` leading + 3.25ex |
| h2 -> p5 | 18.4583 | 20.6719 | 22.2500 | body leading + 1.5ex |
| p5 -> h3 (`\subsubsection`) | 25.9930 | 28.9223 | 31.2917 | body leading + 3.25ex |
| h3 -> p6 | 18.4583 | 20.6719 | 22.2500 | body leading + 1.5ex |
| p6 -> i1, i1 -> i2, i2 -> i3, i3 -> i4, i4 -> p7 | 20.0 | 22.6 | 24.5 | body leading + 8 / 9 / 10 |
| body -> body | 12.0 | 13.6 | 14.5 | body leading (+ 0pt `\parskip`) |

All match the model to 1e-3 pt. This confirms the issue #10 findings: article
adds **no** 6pt paragraph gap (`\parskip` is 0pt plus 1pt), and the space
after a `\section` at 12pt is 11.88pt on top of the 14.5pt baseline pitch.

## JSON form

`Stylesheet::to_json()` writes `{"schema": "flashtex-document-style/1",
"class": "article", "options": {...}, "geometry": {...}|null, "delta": {...},
"export": {...}}`. `options`, `geometry`, and `delta` round-trip through
`Stylesheet::from_json`; `export` is derived and regenerated (page layout in
pt and bp, `\parskip`, heading baseline gaps, and the resolved style of every
canonical path such as `document/heading1`, `document/itemize/item/paragraph`,
`document/paragraph/emph`). Numbers use Rust's shortest round-trip formatting,
so output is byte-for-byte deterministic. This schema is a proposal for
adapter negotiation and is **not** part of `docs/contracts/runtime-v1.md`.

## Unsupported (reported honestly)

- `twocolumn`, `twoside`, `landscape`, `titlepage`, `executive`/`b5` paper,
  `report`/`book` classes.
- Headers and footers (only their geometry — `\headheight`, `\headsep`,
  `\footskip` — is modelled), floats, footnotes, `\maketitle`, abstracts,
  display-math skips, `\@afterheading` club penalties.
- `geometry` keys beyond margin/top/bottom/left/right/textwidth/textheight/
  includehead/includefoot (no `scale`, `ratio`, `lines`, `includemp`,
  `bindingoffset`, `landscape`, `paper=` — paper comes from the class).
- Custom `hmarginratio`/`vmarginratio`.
- Stretch/shrink are carried but not set: page-break glue setting is the
  layout engine's job.
- Font metrics: only the Computer Modern ex/em needed for class arithmetic
  are stored. If the compiler sets Times (as `de1020c` does) the class
  arithmetic is still CM-based, exactly as pdflatex with `times` behaves
  (`\parindent` etc. are fixed at class load time).

## Proposed adapter integration (for the compiler lead and PDF/preview owners)

Nothing here touches `crates/compiler`, `crates/pdf`, or runtime-v1. A
proposed, negotiable shape:

1. **Compiler (`crates/compiler/src/layout.rs`)**: replace the constants
   `PAGE_WIDTH_PT/PAGE_HEIGHT_PT/MARGIN_PT/BODY_SIZE_PT/LINE_SPACING/
   PARAGRAPH_GAP_PT` and the `17.0`/`14.0` heading sizes with a
   `Stylesheet` built from the parsed `\documentclass` options (and
   `\usepackage[..]{geometry}` when the parser exposes it). The cursor keeps
   a block path; `resolve` gives `font_size`, `baselineskip`, `parindent`,
   `first_line_indent`, `space_before/after`, `left_margin`. Vertical
   advance uses the combining rules above. Positions stay in the compiler's
   bp convention via `Pt::to_bp`; for the `rendering-v2-proposal` wire
   (`coordinate_unit: "bp_2pow20"`, page top-left origin) multiply the bp
   value by 2^20 and round once, at emission.
2. **PDF (`crates/pdf`)**: `PageLayout::paper_bp()` is the MediaBox;
   `text_area_bp()` the body; `first_baseline_y` the first line.
3. **Preview / Mac UI**: `export.page` and `export.styles` in the JSON let the
   preview draw margins and estimate layout without the compiler.
4. **Validation**: `tools/native-validation/oracle_compare` can diff
   `export.heading_baseline_gaps` against PDFKit-extracted baselines.

Dependency direction: `flashtex-document-style` depends on nothing;
`flashtex-compiler` would depend on it. Whether to add a Cargo workspace is
the integration owner's call (each crate currently builds standalone).
