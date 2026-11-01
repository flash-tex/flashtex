# class-geometry → render-pipeline integration contract (proposal)

Status: steps 1–3 adopted by `crates/render-pipeline` (branch
`agent/kabir-claude/render-pipeline-class-geometry`, see its README "Page
frame"); steps 4–5 adopted on `agent/kabir-claude/render-pipeline-class-geometry-2`
(per-page left edges, two-column frames, page styles and marks, chapter
openers; `tests/page_frame.rs`, 22 pdflatex fixtures); step 6 open. The original proposal text follows. This document says what the crate
guarantees, what render-pipeline would change, and in what order.

## What the crate provides

```rust
use flashtex_class_geometry::{resolve, DocumentSetup};

let setup = DocumentSetup::from_preamble(source)?;   // \documentclass, geometry, \geometry{}, \pagestyle
let doc = resolve(&setup);
```

`ResolvedDocument` (all lengths `Sp`, TeX scaled points, top-left origin):

| field | meaning | oracle status |
|---|---|---|
| `params: PageParams` | `\paperwidth` … `\columnsep`, `\topskip`, `\baselineskip`, `\parindent`, `\parskip`, `\leftmargini`, `\labelsep`, `\skip\footins`, `\mathindent` | every value exact to the sp on 93 fixtures |
| `flags` | final `twoside` / `mparswitch` / `twocolumn` / `reversemargin` (geometry may change them) | exercised |
| `frame: PageFrame` | `pdf_page_width/height` (MediaBox), `text_left(page)`, `text_top`, `first_baseline`, `head_baseline`, `foot_baseline`, `columns[]`, `marginpar_side(page, col)` | positions exact (0sp) against `\pdfsavepos` |
| `headings: Vec<HeadingSpec>` | section … subparagraph: indent, before/after glue, size, run-in, indent-after, numbered; helpers `baseline_after_body`, `body_after_heading` | section/subsection/subsubsection/paragraph baselines exact |
| `chapter: Option<ChapterSpec>` | report/book `\chapter`: page break kind, `title_baseline(starred)`, `body_after_title` | exact for numbered, starred and after-text chapters |
| `part: PartSpec` | article in-flow vs report/book own page | transcribed, not oracle-tested |
| `style_macros`, `head_foot(page)` | header/footer slots (page number, left/right mark) per page | page-number baselines exact; x exact for left-aligned numbers |
| `mark_rules` | what `\chaptermark`/`\sectionmark`/`\subsectionmark` put in marks | transcribed, not oracle-tested |
| `secnumdepth`, `tocdepth` | class defaults | exact |

Units: convert once at the boundary. `Sp::to_pt()` gives TeX points (what
render-pipeline's `Stylesheet` stores today); PDF coordinates are
`Sp::to_bp()` of the value, with y = `pdf_page_height - y`.

## Findings render-pipeline currently gets wrong

1. **MediaBox without geometry.** pdflatex does not set `\pdfpagewidth`
   from `\paperwidth` unless geometry (or a similar package) is loaded: an
   `a4paper` article without geometry ships a US Letter page with an A4 text
   block positioned from the top (MacTeX default `pdftexconfig.tex`). The
   pipeline uses the paper size as the page size. Use
   `frame.pdf_page_width/height` for the page and `frame.text_top` etc. for
   content.
2. **Only `article`.** `Stylesheet::article` ignores `report`/`book`
   (`bk1x.clo` headsep/footskip/marginparsep differ; book is two-sided with
   `headings` by default).
3. **No two-sided margins, no two-column frame.** `\evensidemargin`,
   `\columnwidth = (\textwidth-\columnsep)/2`, `\parindent = 1em` and
   `\leftmargini = 2em` in two-column mode.
4. **geometry subset.** `geometry_from_options` reads only
   `margin/left/right/top/bottom/textwidth/textheight`; geometry defaults
   (0.7 scale, 2:3 vertical ratio, 2:3 inner:outer when two-sided),
   `includehead/foot/mp`, `hmargin={a,b}`, `scale`, `ratio`, `lines`,
   `heightrounded`, `bindingoffset`, `landscape`, paper keys and multiple
   `\geometry{}` calls all change the frame.
5. **No header/footer.** Page numbers (`plain`: centered at
   `foot_baseline`; `headings`: at `head_baseline`, left on even pages when
   two-sided) are not drawn.
6. **Chapters.** `\chapter` title baseline is `\topskip + 50pt + huge
   \baselineskip + 20pt + Huge \baselineskip` below the text top, body at
   `+40pt + \parskip + \baselineskip`.

## Proposed render-pipeline diff (for its owner)

1. `Cargo.toml`: add `flashtex-class-geometry = { path = "../class-geometry" }`.
2. `style.rs`: add `Stylesheet::from_resolved(doc: &ResolvedDocument, family)`
   that fills the existing fields from `doc.params`/`doc.frame`:
   `page_width_pt/page_height_pt ← frame.pdf_page_*`,
   `text_x_pt ← frame.text_left(page)` (becomes per-page; see 4),
   `text_y_pt ← frame.text_top`, `text_width_pt ← frame.columns[0].width`,
   `text_height_pt ← params.textheight`, `topskip_pt`, `maxdepth_pt ←
   params.maxdepth`, `parindent_pt`, `parskip`, `leftmargini_pt`,
   `labelsep_pt`, and `headings[i].before/after ← HeadingSpec::space_before /
   afterskip` (already absolute; the ex is the class body font's — keep the
   TFM-derived ex only for non-CM families).
3. `adapter.rs`: replace the hand-rolled geometry parsing and
   `Geometry::margin(1in)` default for body-only input with
   `DocumentSetup::from_preamble`; for body-only input synthesize
   `DocumentSetup { geometry: Some("margin=1in") }` to keep today's behavior.
4. `typeset.rs` page emission: take `text_x` per page
   (`frame.text_left(page_number)`), and for two-column pages break into
   `frame.columns` (column 2 x = left + `columns[1].offset`).
5. Page chrome: after each page is built, emit `doc.head_foot(page)` fields:
   page number string `doc.numbering.format(n)` at `frame.head_baseline` or
   `frame.foot_baseline`, left/centered/right inside `[text_left, text_left +
   text_width]`; marks from the running `mark_rules`.
6. Keep `flashtex-document-style` for list/size tables until its owner
   decides; class-geometry's `FontSize::metrics` and size tables are the same
   numbers for article.

Ordering: 1–3 are behavior-preserving for letter + `margin=1in` fixtures
(the class-geometry values equal document-style's there); 4–5 change output
and should be gated on the visual oracle.

## Not provided (yet)

Floats, footnote placement, `\marginpar` stacking, `\maketitle`/title page
layout (daniel-parent/maketitle, daniel-title/title-layout), list spacing
(daniel-parent-b/setlist-spacing), page-control commands
(daniel-parent/page-control), `\newgeometry`/`\restoregeometry`, `mag`,
`truedimen`, geometry lengths written as macros (`0.1\paperwidth`), and
non-CM body fonts (em/ex are Computer Modern values; pass other metrics by
building `PageParams` from `class_params` with your own `FontMetrics`).
