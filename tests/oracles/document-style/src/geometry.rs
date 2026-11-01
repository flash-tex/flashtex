//! Page geometry: paper sizes, `article` defaults, and `geometry`-style overrides.
//!
//! Provenance:
//! - Paper sizes: `article.cls` v1.4n `\DeclareOption{...paper}` (letter 8.5in
//!   x 11in, a4 210mm x 297mm, a5 148mm x 210mm, legal 8.5in x 14in).
//! - Default text area: `size1x.clo` v1.4n, the non-compatibility branches
//!   (`\if@compatibility` false, `\if@twoside` false, `\if@twocolumn` false).
//! - Overrides: `geometry.sty` v5.9 (2020/01/02) `\Gm@detall`, `\Gm@adjustbody`,
//!   `\Gm@@process`, with defaults `\Gm@Dhscale`/`\Gm@Dvscale` 0.7,
//!   `\Gm@Dhratio` 1:1 (oneside), `\Gm@Dvratio` 2:3.

use crate::fonts::{BaseSize, SizeName, font_size, size_params};
use crate::length::Pt;

/// Supported paper sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Paper {
    Letter,
    A4,
    A5,
    Legal,
}

impl Paper {
    pub fn name(self) -> &'static str {
        match self {
            Paper::Letter => "letter",
            Paper::A4 => "a4",
            Paper::A5 => "a5",
            Paper::Legal => "legal",
        }
    }
    pub fn parse(s: &str) -> Option<Paper> {
        Some(match s {
            "letter" | "letterpaper" => Paper::Letter,
            "a4" | "a4paper" => Paper::A4,
            "a5" | "a5paper" => Paper::A5,
            "legal" | "legalpaper" => Paper::Legal,
            _ => return None,
        })
    }
    /// (width, height) in TeX points.
    pub fn size(self) -> (Pt, Pt) {
        match self {
            Paper::Letter => (Pt::inches(8.5), Pt::inches(11.0)),
            Paper::A4 => (Pt::mm(210.0), Pt::mm(297.0)),
            Paper::A5 => (Pt::mm(148.0), Pt::mm(210.0)),
            Paper::Legal => (Pt::inches(8.5), Pt::inches(14.0)),
        }
    }
}

/// `\documentclass[<size>,<paper>]{article}`; oneside, onecolumn, final.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClassOptions {
    pub paper: Paper,
    pub size: BaseSize,
}

impl Default for ClassOptions {
    fn default() -> Self {
        ClassOptions {
            paper: Paper::Letter,
            size: BaseSize::Pt10,
        }
    }
}

/// The raw LaTeX page parameters, as `\showthe` would print them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatexPageParams {
    pub paperwidth: Pt,
    pub paperheight: Pt,
    pub textwidth: Pt,
    pub textheight: Pt,
    pub oddsidemargin: Pt,
    pub evensidemargin: Pt,
    pub topmargin: Pt,
    pub headheight: Pt,
    pub headsep: Pt,
    pub footskip: Pt,
    pub topskip: Pt,
    pub marginparwidth: Pt,
    pub marginparsep: Pt,
}

/// The article defaults from `size1x.clo`.
pub fn article_page_params(options: ClassOptions) -> LatexPageParams {
    let p = size_params(options.size);
    let (paperwidth, paperheight) = options.paper.size();
    let baselineskip = font_size(options.size, SizeName::NormalSize).baselineskip;
    let one_inch = Pt::inches(1.0);

    // \textwidth: min(\paperwidth - 2in, nominal) then \@settopoint.
    let avail = paperwidth - one_inch * 2.0;
    let textwidth = if avail > p.nominal_text_width {
        p.nominal_text_width
    } else {
        avail
    }
    .settopoint();

    // \textheight: floor((\paperheight - 2in - 1.5in) / \baselineskip) lines
    // plus \topskip. `\divide` of two dimens truncates toward zero.
    let avail_h = paperheight - one_inch * 2.0 - one_inch * 1.5;
    let lines = (avail_h.0 / baselineskip.0).trunc();
    let textheight = baselineskip * lines + p.topskip;

    // Side margins (oneside): .5(\paperwidth - \textwidth) - 1in, then
    // \@settopoint; \evensidemargin = \paperwidth - 2in - \textwidth - odd.
    let marginparsep = match options.size {
        BaseSize::Pt10 => Pt(11.0),
        _ => Pt(10.0),
    };
    let rest = paperwidth - textwidth;
    let oddsidemargin = (rest * 0.5 - one_inch).settopoint();
    let mut marginparwidth = rest * 0.5 - marginparsep - Pt::inches(0.4) - Pt::inches(0.4);
    if marginparwidth > Pt::inches(2.0) {
        marginparwidth = Pt::inches(2.0);
    }
    let marginparwidth = marginparwidth.settopoint();
    let evensidemargin = (paperwidth - one_inch * 2.0 - textwidth - oddsidemargin).settopoint();

    // \topmargin: half of the leftover vertical space, then \@settopoint.
    let leftover =
        paperheight - one_inch * 2.0 - p.headheight - p.headsep - textheight - p.footskip;
    let topmargin = (leftover * 0.5).settopoint();

    LatexPageParams {
        paperwidth,
        paperheight,
        textwidth,
        textheight,
        oddsidemargin,
        evensidemargin,
        topmargin,
        headheight: p.headheight,
        headsep: p.headsep,
        footskip: p.footskip,
        topskip: p.topskip,
        marginparwidth,
        marginparsep,
    }
}

/// A `geometry`-package-style override. Every field is optional; the
/// completion rules of `geometry.sty` v5.9 fill in the rest.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Geometry {
    /// `margin=`: sets all four margins unless a specific side overrides it.
    pub margin: Option<Pt>,
    pub top: Option<Pt>,
    pub bottom: Option<Pt>,
    pub left: Option<Pt>,
    pub right: Option<Pt>,
    /// `textwidth=` (the body width; `width=` is the same without marginpar).
    pub textwidth: Option<Pt>,
    /// `textheight=`.
    pub textheight: Option<Pt>,
    /// `includehead`: the header is inside the top margin's body area.
    pub includehead: bool,
    /// `includefoot`.
    pub includefoot: bool,
}

impl Geometry {
    pub fn margin(m: Pt) -> Geometry {
        Geometry {
            margin: Some(m),
            ..Geometry::default()
        }
    }

    fn left(&self) -> Option<Pt> {
        self.left.or(self.margin)
    }
    fn right(&self) -> Option<Pt> {
        self.right.or(self.margin)
    }
    fn top(&self) -> Option<Pt> {
        self.top.or(self.margin)
    }
    fn bottom(&self) -> Option<Pt> {
        self.bottom.or(self.margin)
    }
}

const GM_DEFAULT_SCALE: f64 = 0.7;

/// One axis of `\Gm@detall`. `ratio` is (a, b) as in `a:b` for
/// (first, second) margin; returns (first margin, body, second margin).
fn gm_axis(
    layout: Pt,
    first: Option<Pt>,
    body: Option<Pt>,
    second: Option<Pt>,
    ratio: (f64, f64),
) -> (Pt, Pt, Pt) {
    let split = |body: Pt, (a, b): (f64, f64)| -> (Pt, Pt) {
        let rest = layout - body;
        let first = rest * (a / (a + b));
        (first, rest - first)
    };
    let default_body = layout * GM_DEFAULT_SCALE;
    let case = (first.is_some() as u8) * 4 + (body.is_some() as u8) * 2 + second.is_some() as u8;
    match case {
        0 => {
            let (f, s) = split(default_body, ratio);
            (f, default_body, s)
        }
        1 => {
            // Only the second margin: the first takes its default-ratio value
            // for the default body, then the body fills the remainder.
            let (f, _) = split(default_body, ratio);
            let s = second.unwrap();
            (f, layout - f - s, s)
        }
        2 => {
            let b = body.unwrap();
            let (f, s) = split(b, ratio);
            (f, b, s)
        }
        3 => {
            let (b, s) = (body.unwrap(), second.unwrap());
            (layout - b - s, b, s)
        }
        4 => {
            // Only the first margin: geometry calls \Gm@detiiandiii with the
            // margin names swapped, so the *second* margin receives the `a`
            // share of the default-body remainder.
            let (s, _) = split(default_body, ratio);
            let f = first.unwrap();
            (f, layout - f - s, s)
        }
        5 => {
            let (f, s) = (first.unwrap(), second.unwrap());
            (f, layout - f - s, s)
        }
        6 => {
            let (f, b) = (first.unwrap(), body.unwrap());
            (f, b, layout - f - b)
        }
        _ => {
            // Over-specified: geometry warns and ignores the body.
            let (f, s) = (first.unwrap(), second.unwrap());
            (f, layout - f - s, s)
        }
    }
}

/// Apply a `geometry` override to article defaults, returning the LaTeX
/// parameters `geometry.sty` would set.
pub fn apply_geometry(base: LatexPageParams, g: &Geometry) -> LatexPageParams {
    let one_inch = Pt::inches(1.0);
    let (left, width, _right) = gm_axis(
        base.paperwidth,
        g.left(),
        g.textwidth,
        g.right(),
        (1.0, 1.0),
    );
    let mut height_req = g.textheight;
    if let Some(h) = height_req {
        let mut h = h;
        if g.includehead {
            h += base.headheight + base.headsep;
        }
        if g.includefoot {
            h += base.footskip;
        }
        height_req = Some(h);
    }
    let (top, mut height, _bottom) = gm_axis(
        base.paperheight,
        g.top(),
        height_req,
        g.bottom(),
        (2.0, 3.0),
    );
    let mut topmargin = top - one_inch;
    if g.includehead {
        height = height - base.headheight - base.headsep;
    } else {
        topmargin = topmargin - base.headheight - base.headsep;
    }
    if g.includefoot {
        height = height - base.footskip;
    }
    LatexPageParams {
        textwidth: width,
        textheight: height,
        oddsidemargin: left - one_inch,
        evensidemargin: left - one_inch,
        topmargin,
        ..base
    }
}

/// An axis-aligned rectangle with a top-left origin, in TeX points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: Pt,
    pub y: Pt,
    pub width: Pt,
    pub height: Pt,
}

/// The resolved page: paper, the text area, and the vertical furniture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageLayout {
    pub paper: Paper,
    pub paper_width: Pt,
    pub paper_height: Pt,
    /// The text body (`\textwidth` x `\textheight`) with a top-left origin.
    pub text_area: Rect,
    pub columns: u8,
    /// `\topskip`: the first baseline sits this far below the text area top
    /// when the first line's height does not exceed it.
    pub top_skip: Pt,
    pub head_height: Pt,
    pub head_sep: Pt,
    pub foot_skip: Pt,
    /// The raw LaTeX parameters this layout was derived from.
    pub latex: LatexPageParams,
}

impl PageLayout {
    pub fn from_params(paper: Paper, latex: LatexPageParams) -> PageLayout {
        let one_inch = Pt::inches(1.0);
        PageLayout {
            paper,
            paper_width: latex.paperwidth,
            paper_height: latex.paperheight,
            text_area: Rect {
                x: one_inch + latex.oddsidemargin,
                y: one_inch + latex.topmargin + latex.headheight + latex.headsep,
                width: latex.textwidth,
                height: latex.textheight,
            },
            columns: 1,
            top_skip: latex.topskip,
            head_height: latex.headheight,
            head_sep: latex.headsep,
            foot_skip: latex.footskip,
            latex,
        }
    }

    /// The same layout in PDF big points (72 per inch), top-left origin.
    pub fn text_area_bp(&self) -> (f64, f64, f64, f64) {
        let r = self.text_area;
        (r.x.to_bp(), r.y.to_bp(), r.width.to_bp(), r.height.to_bp())
    }

    /// Paper size in PDF big points (the PDF MediaBox).
    pub fn paper_bp(&self) -> (f64, f64) {
        (self.paper_width.to_bp(), self.paper_height.to_bp())
    }

    /// Y (top-left origin, pt) of the first baseline on a page.
    pub fn first_baseline_y(&self) -> Pt {
        self.text_area.y + self.top_skip
    }
}
