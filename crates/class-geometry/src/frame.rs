//! The resolved page frame: where `\@outputpage` (latex.ltx lines
//! 20880–20960) puts the header, the text block and the footer.
//!
//! All coordinates are TeX scaled points from the top-left corner of the
//! paper. pdfTeX's origin is (1in, 1in) plus `\hoffset`/`\voffset`.
//! `\@outputpage` ships `\vskip\topmargin`, then (moved right by
//! `\@themargin`) a `\vbox to\headheight{\vfil <head>}` with depth zeroed,
//! `\vskip\headsep`, the `\@outputbox` (`\vbox to\textheight`, depth 0 from
//! `\@makecol`), and the foot line with `\baselineskip\footskip`. Hence:
//! head baseline = text top − `\headsep`; foot baseline = text bottom +
//! `\footskip` (when the foot line is no taller than `\footskip`).

use crate::class::PageParams;
use crate::geometry::LayoutFlags;
use crate::tex::Sp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column {
    /// Left edge relative to the text block's left edge.
    pub offset: Sp,
    pub width: Sp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageFrame {
    pub paper_width: Sp,
    pub paper_height: Sp,
    /// PDF MediaBox (`\pdfpagewidth`/`\pdfpageheight`). geometry's pdftex
    /// driver sets it from the paper (geometry.sty lines 1051–1054); without
    /// geometry the LaTeX kernel leaves the engine default from
    /// `pdftexconfig.tex` (US Letter in MacTeX 2026), even for `a4paper`.
    pub pdf_page_width: Sp,
    pub pdf_page_height: Sp,
    pub text_width: Sp,
    pub text_height: Sp,
    /// Text block left edge on odd pages (and every page when one-sided):
    /// 1in + `\hoffset` + `\oddsidemargin`.
    pub odd_text_left: Sp,
    /// 1in + `\hoffset` + `\evensidemargin` (used only when two-sided).
    pub even_text_left: Sp,
    /// Top of the header box: 1in + `\voffset` + `\topmargin`.
    pub head_top: Sp,
    /// Header baseline: `head_top + \headheight`.
    pub head_baseline: Sp,
    /// Text block top: `head_baseline + \headsep`.
    pub text_top: Sp,
    /// First line baseline when its height ≤ `\topskip`.
    pub first_baseline: Sp,
    /// Footer baseline: `text_top + \textheight + \footskip`.
    pub foot_baseline: Sp,
    pub columns: Vec<Column>,
    pub columnseprule: Sp,
    pub marginpar_width: Sp,
    pub marginpar_sep: Sp,
    pub marginpar_push: Sp,
    pub twoside: bool,
    pub twocolumn: bool,
    pub mparswitch: bool,
    pub reversemargin: bool,
}

/// The text block's columns: one of `\textwidth`, or two of
/// `\columnwidth` = `(\textwidth - \columnsep) / 2` separated by
/// `\columnsep` (latex.ltx `\twocolumn`/`\onecolumn`, which set
/// `\columnwidth` and `\col@number` and nothing else).
///
/// Separate from [`PageFrame::new`] because `\twocolumn` and `\onecolumn`
/// can also change `\if@twocolumn` *during* the document, long after the
/// frame was built ([`crate::ResolvedDocument::set_twocolumn`]).
pub fn columns(p: &PageParams, twocolumn: bool) -> Vec<Column> {
    let cw = p.columnwidth(twocolumn);
    if twocolumn {
        vec![
            Column {
                offset: Sp::ZERO,
                width: cw,
            },
            Column {
                offset: cw + p.columnsep,
                width: cw,
            },
        ]
    } else {
        vec![Column {
            offset: Sp::ZERO,
            width: cw,
        }]
    }
}

impl PageFrame {
    pub fn new(p: &PageParams, flags: LayoutFlags, media: (Sp, Sp)) -> PageFrame {
        let inch = Sp::parse("1in").unwrap();
        let head_top = inch + p.voffset + p.topmargin;
        let head_baseline = head_top + p.headheight;
        let text_top = head_baseline + p.headsep;
        let columns = columns(p, flags.twocolumn);
        PageFrame {
            paper_width: p.paperwidth,
            paper_height: p.paperheight,
            pdf_page_width: media.0,
            pdf_page_height: media.1,
            text_width: p.textwidth,
            text_height: p.textheight,
            odd_text_left: inch + p.hoffset + p.oddsidemargin,
            even_text_left: inch + p.hoffset + p.evensidemargin,
            head_top,
            head_baseline,
            text_top,
            first_baseline: text_top + p.topskip,
            foot_baseline: text_top + p.textheight + p.footskip,
            columns,
            columnseprule: p.columnseprule,
            marginpar_width: p.marginparwidth,
            marginpar_sep: p.marginparsep,
            marginpar_push: p.marginparpush,
            twoside: flags.twoside,
            twocolumn: flags.twocolumn,
            mparswitch: flags.mparswitch,
            reversemargin: flags.reversemargin,
        }
    }

    /// Text block left edge for a page number (`\@themargin`).
    pub fn text_left(&self, page: i64) -> Sp {
        if self.twoside && page % 2 == 0 {
            self.even_text_left
        } else {
            self.odd_text_left
        }
    }

    /// Which side `\marginpar` notes go on for a one-column page
    /// (latex.ltx `\@addmarginpar`: right by default, left with
    /// `\reversemarginpar`, flipped on even pages under `\@mparswitch`).
    /// In two-column mode notes go to the outer side of their column.
    pub fn marginpar_side(&self, page: i64, column: usize) -> Side {
        if self.twocolumn {
            return if column == 0 { Side::Left } else { Side::Right };
        }
        let mut left = self.reversemargin;
        if self.mparswitch && page % 2 == 0 {
            left = !left;
        }
        if left {
            Side::Left
        } else {
            Side::Right
        }
    }

    /// Convert a top-left y to PDF big points from the MediaBox bottom.
    pub fn y_to_pdf_bp(&self, y: Sp) -> f64 {
        (self.pdf_page_height - y).to_bp()
    }
}
