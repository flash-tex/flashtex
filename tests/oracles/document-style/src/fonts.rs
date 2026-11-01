//! Font size tables and per-size class parameters for LaTeX's `article`.
//!
//! Provenance (BasicTeX, TeX Live 2026, pdfTeX 1.40.29):
//! - `/usr/local/texlive/2026basic/texmf-dist/tex/latex/base/size10.clo`,
//!   `size11.clo`, `size12.clo` (v1.4n 2025/01/22): `\@setfontsize` calls,
//!   `\parindent`, `\headheight`, `\headsep`, `\topskip`, `\footskip`,
//!   `\@tempdimb` (nominal text width), `\@listi`..`\@listiii`, `\partopsep`.
//! - `/usr/local/texlive/2026basic/texmf-dist/tex/latex/base/latex.ltx`:
//!   `\@vpt`..`\@xxvpt` size macros (5, 6, 7, 8, 9, 10, 10.95, 12, 14.4, 17.28,
//!   20.74, 24.88).
//! - `/usr/local/texlive/2026basic/texmf-dist/tex/latex/base/article.cls`
//!   (v1.4n): `\parskip`, `\leftmargini`..`\leftmarginiv`, `\labelsep`,
//!   `\@startsection` arguments.
//! - Computer Modern font dimensions (`\fontdimen5` x-height, `\fontdimen6`
//!   quad) were read from a running pdflatex; they are TFM values of
//!   `cmr10`, `cmr10 at 10.95pt`, and `cmr12`.

use crate::length::{Pt, Skip};

/// The `10pt` / `11pt` / `12pt` class option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaseSize {
    Pt10,
    Pt11,
    Pt12,
}

impl BaseSize {
    pub fn name(self) -> &'static str {
        match self {
            BaseSize::Pt10 => "10pt",
            BaseSize::Pt11 => "11pt",
            BaseSize::Pt12 => "12pt",
        }
    }
    pub fn parse(s: &str) -> Option<BaseSize> {
        match s {
            "10pt" => Some(BaseSize::Pt10),
            "11pt" => Some(BaseSize::Pt11),
            "12pt" => Some(BaseSize::Pt12),
            _ => None,
        }
    }
}

/// LaTeX's named size commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SizeName {
    Tiny,
    ScriptSize,
    FootnoteSize,
    Small,
    NormalSize,
    Large,
    LARGE2,
    LARGE3,
    Huge,
    HUGE2,
}

impl SizeName {
    /// The LaTeX command name without the backslash.
    pub fn command(self) -> &'static str {
        match self {
            SizeName::Tiny => "tiny",
            SizeName::ScriptSize => "scriptsize",
            SizeName::FootnoteSize => "footnotesize",
            SizeName::Small => "small",
            SizeName::NormalSize => "normalsize",
            SizeName::Large => "large",
            SizeName::LARGE2 => "Large",
            SizeName::LARGE3 => "LARGE",
            SizeName::Huge => "huge",
            SizeName::HUGE2 => "Huge",
        }
    }
    pub fn parse(s: &str) -> Option<SizeName> {
        Some(match s {
            "tiny" => SizeName::Tiny,
            "scriptsize" => SizeName::ScriptSize,
            "footnotesize" => SizeName::FootnoteSize,
            "small" => SizeName::Small,
            "normalsize" => SizeName::NormalSize,
            "large" => SizeName::Large,
            "Large" => SizeName::LARGE2,
            "LARGE" => SizeName::LARGE3,
            "huge" => SizeName::Huge,
            "Huge" => SizeName::HUGE2,
            _ => return None,
        })
    }
}

/// A font size with its `\baselineskip`, as set by `\@setfontsize`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontSize {
    pub size: Pt,
    pub baselineskip: Pt,
}

const fn fs(size: f64, baselineskip: f64) -> FontSize {
    FontSize {
        size: Pt(size),
        baselineskip: Pt(baselineskip),
    }
}

/// The size table from `size10.clo`, `size11.clo`, and `size12.clo`.
pub fn font_size(base: BaseSize, name: SizeName) -> FontSize {
    use SizeName::*;
    match (base, name) {
        // size10.clo
        (BaseSize::Pt10, Tiny) => fs(5.0, 6.0),
        (BaseSize::Pt10, ScriptSize) => fs(7.0, 8.0),
        (BaseSize::Pt10, FootnoteSize) => fs(8.0, 9.5),
        (BaseSize::Pt10, Small) => fs(9.0, 11.0),
        (BaseSize::Pt10, NormalSize) => fs(10.0, 12.0),
        (BaseSize::Pt10, Large) => fs(12.0, 14.0),
        (BaseSize::Pt10, LARGE2) => fs(14.4, 18.0),
        (BaseSize::Pt10, LARGE3) => fs(17.28, 22.0),
        (BaseSize::Pt10, Huge) => fs(20.74, 25.0),
        (BaseSize::Pt10, HUGE2) => fs(24.88, 30.0),
        // size11.clo
        (BaseSize::Pt11, Tiny) => fs(6.0, 7.0),
        (BaseSize::Pt11, ScriptSize) => fs(8.0, 9.5),
        (BaseSize::Pt11, FootnoteSize) => fs(9.0, 11.0),
        (BaseSize::Pt11, Small) => fs(10.0, 12.0),
        (BaseSize::Pt11, NormalSize) => fs(10.95, 13.6),
        (BaseSize::Pt11, Large) => fs(12.0, 14.0),
        (BaseSize::Pt11, LARGE2) => fs(14.4, 18.0),
        (BaseSize::Pt11, LARGE3) => fs(17.28, 22.0),
        (BaseSize::Pt11, Huge) => fs(20.74, 25.0),
        (BaseSize::Pt11, HUGE2) => fs(24.88, 30.0),
        // size12.clo (`\let\Huge=\huge`)
        (BaseSize::Pt12, Tiny) => fs(6.0, 7.0),
        (BaseSize::Pt12, ScriptSize) => fs(8.0, 9.5),
        (BaseSize::Pt12, FootnoteSize) => fs(10.0, 12.0),
        (BaseSize::Pt12, Small) => fs(10.95, 13.6),
        (BaseSize::Pt12, NormalSize) => fs(12.0, 14.5),
        (BaseSize::Pt12, Large) => fs(14.4, 18.0),
        (BaseSize::Pt12, LARGE2) => fs(17.28, 22.0),
        (BaseSize::Pt12, LARGE3) => fs(20.74, 25.0),
        (BaseSize::Pt12, Huge) => fs(24.88, 30.0),
        (BaseSize::Pt12, HUGE2) => fs(24.88, 30.0),
    }
}

/// Font-relative units of the body (`\normalsize`, Computer Modern Roman) font.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontParams {
    /// `\fontdimen5`: one `ex`.
    pub x_height: Pt,
    /// `\fontdimen6`: one `em` (the quad).
    pub quad: Pt,
}

impl FontParams {
    pub fn ex(&self, n: f64) -> Pt {
        self.x_height * n
    }
    pub fn em(&self, n: f64) -> Pt {
        self.quad * n
    }
}

/// Class parameters that depend only on the size option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeParams {
    pub base: BaseSize,
    pub normal: FontParams,
    /// `\parindent` (`15pt`, `17pt`, or `1.5em` of the 12pt font).
    pub parindent: Pt,
    /// `\@tempdimb` in the text-width computation: 345pt / 360pt / 390pt.
    pub nominal_text_width: Pt,
    pub headheight: Pt,
    pub headsep: Pt,
    pub topskip: Pt,
    pub footskip: Pt,
    /// `\partopsep` for a list that starts a new paragraph.
    pub partopsep: Skip,
    pub smallskip: Skip,
    pub medskip: Skip,
    pub bigskip: Skip,
}

/// `\parskip` from `article.cls`: `0pt plus 1pt` at every size.
pub const PARSKIP: Skip = Skip::new(0.0, 1.0, 0.0);

/// `\labelsep` from `article.cls`: `.5em`.
pub const LABELSEP_EM: f64 = 0.5;

pub fn size_params(base: BaseSize) -> SizeParams {
    match base {
        BaseSize::Pt10 => SizeParams {
            base,
            normal: FontParams {
                x_height: Pt(4.30554),
                quad: Pt(10.00002),
            },
            parindent: Pt(15.0),
            nominal_text_width: Pt(345.0),
            headheight: Pt(12.0),
            headsep: Pt(25.0),
            topskip: Pt(10.0),
            footskip: Pt(30.0),
            partopsep: Skip::new(2.0, 1.0, 1.0),
            smallskip: Skip::new(3.0, 1.0, 1.0),
            medskip: Skip::new(6.0, 2.0, 2.0),
            bigskip: Skip::new(12.0, 4.0, 4.0),
        },
        BaseSize::Pt11 => SizeParams {
            base,
            normal: FontParams {
                x_height: Pt(4.71457),
                quad: Pt(10.95003),
            },
            parindent: Pt(17.0),
            nominal_text_width: Pt(360.0),
            headheight: Pt(12.0),
            headsep: Pt(25.0),
            topskip: Pt(11.0),
            footskip: Pt(30.0),
            partopsep: Skip::new(3.0, 1.0, 1.0),
            smallskip: Skip::new(3.0, 1.0, 1.0),
            medskip: Skip::new(6.0, 2.0, 2.0),
            bigskip: Skip::new(12.0, 4.0, 4.0),
        },
        BaseSize::Pt12 => {
            let normal = FontParams {
                x_height: Pt(5.16667),
                quad: Pt(11.74988),
            };
            SizeParams {
                base,
                normal,
                parindent: normal.em(1.5),
                nominal_text_width: Pt(390.0),
                headheight: Pt(12.0),
                headsep: Pt(25.0),
                topskip: Pt(12.0),
                footskip: Pt(30.0),
                partopsep: Skip::new(3.0, 2.0, 2.0),
                smallskip: Skip::new(3.0, 1.0, 1.0),
                medskip: Skip::new(6.0, 2.0, 2.0),
                bigskip: Skip::new(12.0, 4.0, 4.0),
            }
        }
    }
}

/// Vertical list parameters for one nesting level (`\@listi`..`\@listiii`).
/// Levels 4 and beyond only redefine `\leftmargin`/`\labelwidth` and inherit
/// the level-3 skips from the enclosing list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListLevelParams {
    /// `\leftmargin` for this level (em multiples of the body font).
    pub leftmargin: Pt,
    /// `\labelwidth = \leftmargin - \labelsep`.
    pub labelwidth: Pt,
    pub labelsep: Pt,
    pub topsep: Skip,
    pub partopsep: Skip,
    pub parsep: Skip,
    pub itemsep: Skip,
}

/// Parameters for list nesting `depth` (1-based). Depth 0 is clamped to 1 and
/// depths above 4 reuse the level-4 margin (`\leftmarginv`/`vi` are `1em` and
/// only reachable through `\@listv`/`\@listvi`, which article never redefines
/// beyond margins; LaTeX errors above depth 6 anyway).
pub fn list_level(base: BaseSize, depth: u8) -> ListLevelParams {
    let p = size_params(base);
    let em = |n: f64| p.normal.em(n);
    let labelsep = em(LABELSEP_EM);
    let leftmargin = match depth {
        0 | 1 => em(2.5),
        2 => em(2.2),
        3 => em(1.87),
        4 => em(1.7),
        _ => em(1.0),
    };
    let (topsep, parsep, itemsep, partopsep) = match (base, depth.clamp(1, 3)) {
        (BaseSize::Pt10, 1) => (
            Skip::new(8.0, 2.0, 4.0),
            Skip::new(4.0, 2.0, 1.0),
            Skip::new(4.0, 2.0, 1.0),
            p.partopsep,
        ),
        (BaseSize::Pt10, 2) => (
            Skip::new(4.0, 2.0, 1.0),
            Skip::new(2.0, 1.0, 1.0),
            Skip::new(2.0, 1.0, 1.0),
            p.partopsep,
        ),
        (BaseSize::Pt10, _) => (
            Skip::new(2.0, 1.0, 1.0),
            Skip::ZERO,
            Skip::new(2.0, 1.0, 1.0),
            Skip::new(1.0, 0.0, 1.0),
        ),
        (BaseSize::Pt11, 1) => (
            Skip::new(9.0, 3.0, 5.0),
            Skip::new(4.5, 2.0, 1.0),
            Skip::new(4.5, 2.0, 1.0),
            p.partopsep,
        ),
        (BaseSize::Pt11, 2) => (
            Skip::new(4.5, 2.0, 1.0),
            Skip::new(2.0, 1.0, 1.0),
            Skip::new(2.0, 1.0, 1.0),
            p.partopsep,
        ),
        (BaseSize::Pt11, _) => (
            Skip::new(2.0, 1.0, 1.0),
            Skip::ZERO,
            Skip::new(2.0, 1.0, 1.0),
            Skip::new(1.0, 0.0, 1.0),
        ),
        (BaseSize::Pt12, 1) => (
            Skip::new(10.0, 4.0, 6.0),
            Skip::new(5.0, 2.5, 1.0),
            Skip::new(5.0, 2.5, 1.0),
            p.partopsep,
        ),
        (BaseSize::Pt12, 2) => (
            Skip::new(5.0, 2.5, 1.0),
            Skip::new(2.5, 1.0, 1.0),
            Skip::new(2.5, 1.0, 1.0),
            p.partopsep,
        ),
        (BaseSize::Pt12, _) => (
            Skip::new(2.5, 1.0, 1.0),
            Skip::ZERO,
            Skip::new(2.5, 1.0, 1.0),
            Skip::new(1.0, 0.0, 1.0),
        ),
    };
    ListLevelParams {
        leftmargin,
        labelwidth: leftmargin - labelsep,
        labelsep,
        topsep,
        partopsep,
        parsep,
        itemsep,
    }
}

/// `\@startsection` arguments for article's sectioning commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionSpec {
    pub level: u8,
    pub name: &'static str,
    /// Extra horizontal indent of the heading (`#3`).
    pub indent_parindent: bool,
    /// `#4` in `ex` of the body font. Negative means "suppress the indent of
    /// the following paragraph"; the skip itself is the absolute value.
    pub before_ex: Skip,
    /// `#5`: positive values are vertical `ex` skips; negative values (in
    /// `em`) make a run-in heading followed by that much horizontal space.
    pub after: SectionAfter,
    pub font: SizeName,
    pub bold: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SectionAfter {
    VerticalEx(Skip),
    RunInEm(f64),
}

/// `article.cls` lines 302–321 (v1.4n).
pub fn section_spec(level: u8) -> Option<SectionSpec> {
    Some(match level {
        1 => SectionSpec {
            level,
            name: "section",
            indent_parindent: false,
            before_ex: Skip::new(3.5, 1.0, 0.2),
            after: SectionAfter::VerticalEx(Skip::new(2.3, 0.2, 0.0)),
            font: SizeName::LARGE2,
            bold: true,
        },
        2 => SectionSpec {
            level,
            name: "subsection",
            indent_parindent: false,
            before_ex: Skip::new(3.25, 1.0, 0.2),
            after: SectionAfter::VerticalEx(Skip::new(1.5, 0.2, 0.0)),
            font: SizeName::Large,
            bold: true,
        },
        3 => SectionSpec {
            level,
            name: "subsubsection",
            indent_parindent: false,
            before_ex: Skip::new(3.25, 1.0, 0.2),
            after: SectionAfter::VerticalEx(Skip::new(1.5, 0.2, 0.0)),
            font: SizeName::NormalSize,
            bold: true,
        },
        4 => SectionSpec {
            level,
            name: "paragraph",
            indent_parindent: false,
            before_ex: Skip::new(3.25, 1.0, 0.2),
            after: SectionAfter::RunInEm(1.0),
            font: SizeName::NormalSize,
            bold: true,
        },
        5 => SectionSpec {
            level,
            name: "subparagraph",
            indent_parindent: true,
            before_ex: Skip::new(3.25, 1.0, 0.2),
            after: SectionAfter::RunInEm(1.0),
            font: SizeName::NormalSize,
            bold: true,
        },
        _ => return None,
    })
}

/// Whether the paragraph following this heading is indented. In
/// `\@startsection` a negative before-skip sets `\@afterindentfalse`;
/// article's `\section`..`\subsubsection` use negative before-skips and
/// `\paragraph`/`\subparagraph` positive ones.
pub fn indent_after_heading(level: u8) -> bool {
    level >= 4
}
