//! Adapter from `flashtex-document-style` (the article-class model: page
//! geometry, size table, section spacing) to this crate's parameters, so page
//! geometry is owned by one crate and only *consumed* here.
//!
//! All values are TeX points, as document-style produces them.

use flashtex_document_style::{
    Alignment, Block, ClassOptions, Geometry, PARSKIP, PageLayout, ResolvedStyle, SectionAfter,
    Skip, Stylesheet, section_spec,
};

use crate::items::Glue;
use crate::linebreak::{BreakMode, LineBreakParams, Lines};
use crate::pages::{PageParams, ParagraphBlock};

fn glue(s: Skip) -> Glue {
    Glue::finite(s.pt, s.plus, s.minus)
}

/// Page parameters for a stylesheet's page layout and body style.
///
/// `\maxdepth` is `.5\topskip` (LaTeX kernel); `\parskip` is the class value
/// (`0pt plus 1pt` for article); widow/orphan minimum stays 2 lines
/// (`\clubpenalty`/`\widowpenalty` 150 in LaTeX, treated here as "keep 2").
pub fn page_params(layout: &PageLayout, body: &ResolvedStyle, parskip: Skip) -> PageParams {
    let r = layout.text_area;
    PageParams {
        page_width: layout.paper_width.0,
        page_height: layout.paper_height.0,
        margin_top: r.y.0,
        margin_bottom: layout.paper_height.0 - r.y.0 - r.height.0,
        margin_left: r.x.0,
        margin_right: layout.paper_width.0 - r.x.0 - r.width.0,
        topskip: layout.top_skip.0,
        max_depth: layout.top_skip.0 * 0.5,
        parskip: glue(parskip),
        baselineskip: body.baselineskip.0,
        lineskip: 1.0,
        lineskiplimit: 0.0,
        baseline_grid: None,
        club_lines: 2,
        widow_lines: 2,
    }
}

/// Line-breaking parameters for a block style on this page: `\hsize` is the
/// text width less the style's indents; `\parindent` and `\baselineskip`
/// follow the resolved style; alignment maps `Alignment::Left` to
/// `\raggedright`.
pub fn line_params(layout: &PageLayout, style: &ResolvedStyle) -> LineBreakParams {
    let mut p = LineBreakParams::article_12pt_letter_1in();
    p.line_width = layout.text_area.width.0 - style.left_margin.0 - style.right_margin.0;
    p.parindent = if style.first_line_indent {
        style.parindent.0
    } else {
        0.0
    };
    p.baselineskip = style.baselineskip.0;
    p.mode = match style.alignment {
        Alignment::Left => BreakMode::RaggedRight,
        _ => BreakMode::Justified,
    };
    p
}

/// A body paragraph block with the style's own vertical skips.
pub fn body_block(style: &ResolvedStyle, lines: Lines) -> ParagraphBlock {
    ParagraphBlock {
        lines,
        space_before: glue(style.space_before),
        space_after: glue(style.space_after),
        keep_with_next: false,
    }
}

/// A `\section`-style heading block: `#4`/`#5` of `\@startsection`
/// (document-style's `section_spec`) evaluated in `ex`, the x-height of the
/// *body text font in force* (TeX evaluates `ex` in the current font: 5.4pt
/// for 12pt Times, 5.17pt for 12pt Computer Modern), keep-with-next.
pub fn heading_block(level: u8, ex: f64, lines: Lines) -> Option<ParagraphBlock> {
    let spec = section_spec(level)?;
    let before = glue(spec.before_ex.scale(ex));
    let after = match spec.after {
        SectionAfter::VerticalEx(s) => glue(s.scale(ex)),
        _ => Glue::fixed(0.0),
    };
    Some(ParagraphBlock {
        lines,
        space_before: before,
        space_after: after,
        keep_with_next: true,
    })
}

/// Convenience: article class with optional `geometry`, returning the page
/// layout, the body style and the derived parameters in one go.
pub struct ArticleLayout {
    pub sheet: Stylesheet,
    pub layout: PageLayout,
    pub body: ResolvedStyle,
    pub page: PageParams,
    pub line: LineBreakParams,
}

impl ArticleLayout {
    pub fn new(options: ClassOptions, geometry: Option<Geometry>) -> ArticleLayout {
        let mut sheet = Stylesheet::article(options);
        sheet.geometry = geometry;
        let layout = sheet.page_layout();
        let body = sheet.resolve(&[Block::Document, Block::Paragraph]);
        let page = page_params(&layout, &body, PARSKIP);
        let line = line_params(&layout, &body);
        ArticleLayout {
            sheet,
            layout,
            body,
            page,
            line,
        }
    }
}
