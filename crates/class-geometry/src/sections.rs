//! Sectioning commands of the standard classes.
//!
//! `\@startsection`, `\@sect`, `\@xsect`: latex.ltx lines 17231–17302.
//! Parameters: article.cls lines 302–321 (`\section`…`\subparagraph`, the
//! same text in report.cls/book.cls), `\part` article.cls lines 268–301,
//! report.cls/book.cls `\part`/`\chapter`/`\@makechapterhead`.

use crate::class::{BaseSize, ClassKind, FontMetrics, FontSize, Glue, PageParams};
use crate::tex::Sp;

/// A class length written in the source (`-3.25ex`, `-1em`, `\parindent`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SrcLen {
    Zero,
    Pt(&'static str),
    Em(&'static str),
    Ex(&'static str),
    Parindent,
}

impl SrcLen {
    pub fn resolve(self, fm: FontMetrics, parindent: Sp) -> Sp {
        match self {
            SrcLen::Zero => Sp::ZERO,
            SrcLen::Pt(s) => Sp::parse(&format!("{s}pt")).unwrap(),
            SrcLen::Em(f) => fm.em.scaled(f).unwrap(),
            SrcLen::Ex(f) => fm.ex.scaled(f).unwrap(),
            SrcLen::Parindent => parindent,
        }
    }
}

/// One `\@startsection{name}{level}{indent}{beforeskip}{afterskip}{style}`.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadingSpec {
    pub name: &'static str,
    pub level: i32,
    /// `#3` indent of the heading.
    pub indent: Sp,
    /// `#4` as written (negative natural = suppress indent of next para).
    pub beforeskip: Glue,
    /// `#5` as written (<= 0 natural = run-in; `-afterskip` is the
    /// horizontal space after a run-in heading).
    pub afterskip: Glue,
    pub size: FontSize,
    pub bold: bool,
    /// `\ifnum level > \c@secnumdepth` → unnumbered.
    pub numbered: bool,
    /// `\@afterindentfalse` when `#4 < 0`.
    pub indent_after: bool,
    pub run_in: bool,
}

impl HeadingSpec {
    /// Vertical skip `\addvspace` adds before the heading (absolute value
    /// of `#4`). Not added when the heading directly follows another
    /// heading (`\if@nobreak`), and `\addvspace` keeps the larger of this
    /// and a preceding `\vskip`.
    pub fn space_before(&self) -> Glue {
        let g = self.beforeskip;
        if g.natural < Sp::ZERO {
            Glue {
                natural: -g.natural,
                stretch: -g.stretch,
                shrink: -g.shrink,
            }
        } else {
            g
        }
    }

    /// Baseline-to-baseline distance from the previous body line to this
    /// heading's first line at natural glue: `space_before + \parskip +
    /// \baselineskip` of the heading font (interline glue after a line of
    /// normal depth). Run-in headings use the running paragraph's
    /// `\baselineskip`, i.e. the heading font's (`\normalsize`) as well.
    pub fn baseline_after_body(&self, params: &PageParams, base: BaseSize) -> Sp {
        self.space_before().natural + params.parskip.natural + self.size.metrics(base).1
    }

    /// Heading last line to the next body line (display headings):
    /// `afterskip + \parskip + \baselineskip` of the body font; run-in
    /// headings share the line (0).
    pub fn body_after_heading(&self, params: &PageParams) -> Sp {
        if self.run_in {
            Sp::ZERO
        } else {
            self.afterskip.natural + params.parskip.natural + params.baselineskip
        }
    }
}

struct Raw {
    name: &'static str,
    level: i32,
    indent: SrcLen,
    before: [SrcLen; 3],
    after: [SrcLen; 3],
    size: FontSize,
}

/// article.cls lines 302–321 (report.cls/book.cls identical).
fn raw_table() -> [Raw; 5] {
    use SrcLen::*;
    [
        Raw {
            name: "section",
            level: 1,
            indent: Zero,
            before: [Ex("-3.5"), Ex("-1"), Ex("-.2")],
            after: [Ex("2.3"), Ex(".2"), Zero],
            size: FontSize::LargeL,
        },
        Raw {
            name: "subsection",
            level: 2,
            indent: Zero,
            before: [Ex("-3.25"), Ex("-1"), Ex("-.2")],
            after: [Ex("1.5"), Ex(".2"), Zero],
            size: FontSize::Large,
        },
        Raw {
            name: "subsubsection",
            level: 3,
            indent: Zero,
            before: [Ex("-3.25"), Ex("-1"), Ex("-.2")],
            after: [Ex("1.5"), Ex(".2"), Zero],
            size: FontSize::NormalSize,
        },
        Raw {
            name: "paragraph",
            level: 4,
            indent: Zero,
            before: [Ex("3.25"), Ex("1"), Ex(".2")],
            after: [Em("-1"), Zero, Zero],
            size: FontSize::NormalSize,
        },
        Raw {
            name: "subparagraph",
            level: 5,
            indent: Parindent,
            before: [Ex("3.25"), Ex("1"), Ex(".2")],
            after: [Em("-1"), Zero, Zero],
            size: FontSize::NormalSize,
        },
    ]
}

/// `\c@secnumdepth` / `\c@tocdepth` defaults: article 3 (article.cls lines
/// 255, 502); report/book 2 (report.cls `\setcounter{secnumdepth}{2}`,
/// `\setcounter{tocdepth}{2}`).
pub fn default_depths(kind: ClassKind) -> (i32, i32) {
    match kind {
        ClassKind::Article => (3, 3),
        // letter.cls sets neither counter, so both keep latex.ltx's own
        // zero (`\the\c@secnumdepth` and `\the\c@tocdepth` both read 0 in a
        // `letter` document, TeX Live 2025).
        ClassKind::Letter => (0, 0),
        ClassKind::Report | ClassKind::Book => (2, 2),
    }
}

/// The five `\@startsection` headings with lengths resolved in the body
/// font (`\@startsection` evaluates `#4` before the style switch and
/// `\@xsect` evaluates `#5` after the heading group closes).
pub fn headings(
    kind: ClassKind,
    params: &PageParams,
    fm: FontMetrics,
    secnumdepth: i32,
) -> Vec<HeadingSpec> {
    // letter.cls defines no `\section`, `\subsection`, ... at all, so there
    // is nothing to resolve; returning article's table would claim headings
    // the class does not have.
    if !kind.has_sections() {
        return Vec::new();
    }
    raw_table()
        .iter()
        .map(|r| {
            let res = |l: SrcLen| l.resolve(fm, params.parindent);
            let before = Glue {
                natural: res(r.before[0]),
                stretch: res(r.before[1]),
                shrink: res(r.before[2]),
            };
            let after = Glue {
                natural: res(r.after[0]),
                stretch: res(r.after[1]),
                shrink: res(r.after[2]),
            };
            HeadingSpec {
                name: r.name,
                level: r.level,
                indent: res(r.indent),
                beforeskip: before,
                afterskip: after,
                size: r.size,
                bold: true,
                numbered: r.level <= secnumdepth,
                indent_after: before.natural >= Sp::ZERO,
                run_in: after.natural <= Sp::ZERO,
            }
        })
        .collect()
}

/// How a new page is started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageBreak {
    None,
    ClearPage,
    /// `\cleardoublepage`: also skips to an odd page when two-sided.
    ClearDoublePage,
}

/// `\chapter` in report/book (report.cls `\chapter`, `\@makechapterhead`,
/// `\@makeschapterhead`).
#[derive(Clone, Debug, PartialEq)]
pub struct ChapterSpec {
    pub page_break: PageBreak,
    /// `\thispagestyle{plain}`.
    pub page_style: crate::pagestyle::PageStyle,
    /// `\vspace*{50\p@}`.
    pub top_space: Sp,
    /// `\huge\bfseries \@chapapp\space\thechapter` (numbered only).
    pub number_size: FontSize,
    /// `\chaptername` ("Chapter"; `\appendixname` after `\appendix`).
    pub prefix: &'static str,
    /// `\vskip 20\p@` between number line and title.
    pub number_title_skip: Sp,
    /// `\Huge\bfseries` title, `\raggedright`, `\parindent\z@`.
    pub title_size: FontSize,
    /// `\vskip 40\p@` after the title.
    pub after_title: Sp,
    /// `\chapter` numbers when `secnumdepth > -1`.
    pub numbered: bool,
    /// `\@afterindentfalse`: first paragraph not indented.
    pub indent_after: bool,
}

impl ChapterSpec {
    /// Baseline of the title's first line from the top of the text area,
    /// for a chapter that starts a page: `\topskip` glue before the
    /// `\vspace*` rule, 50pt, then each line at its font's `\baselineskip`
    /// (`\@vspacer` restores `\prevdepth`, 0 after the page break; lines
    /// must not exceed their `\baselineskip`).
    pub fn title_baseline(&self, params: &PageParams, base: BaseSize, starred: bool) -> Sp {
        let title_bs = self.title_size.metrics(base).1;
        let mut y = params.topskip + self.top_space;
        if self.numbered && !starred {
            y += self.number_size.metrics(base).1 + self.number_title_skip + title_bs;
        } else {
            y += title_bs;
        }
        y
    }

    /// Title (single line) to the first body baseline.
    pub fn body_after_title(&self, params: &PageParams) -> Sp {
        self.after_title + params.parskip.natural + params.baselineskip
    }
}

pub fn chapter(kind: ClassKind, openright: bool, secnumdepth: i32) -> Option<ChapterSpec> {
    if !kind.has_chapters() {
        return None;
    }
    Some(ChapterSpec {
        page_break: if openright {
            PageBreak::ClearDoublePage
        } else {
            PageBreak::ClearPage
        },
        page_style: crate::pagestyle::PageStyle::Plain,
        top_space: Sp::pt(50),
        number_size: FontSize::Huge,
        prefix: "Chapter",
        number_title_skip: Sp::pt(20),
        title_size: FontSize::HugeH,
        after_title: Sp::pt(40),
        numbered: secnumdepth > -1,
        indent_after: false,
    })
}

/// `\part`.
#[derive(Clone, Debug, PartialEq)]
pub struct PartSpec {
    /// article: in-flow, `\addvspace{4ex}`; report/book: own page.
    pub own_page: bool,
    pub page_break: PageBreak,
    pub space_before: Sp,
    pub centered: bool,
    /// "Part I" line size (article `\Large`, report/book `\huge`).
    pub number_size: FontSize,
    /// Space between number line and title (article: `\par\nobreak`
    /// only; report/book `\vskip 20\p@`).
    pub number_title_skip: Sp,
    /// Title size (article `\huge`, report/book `\Huge`).
    pub title_size: FontSize,
    /// article `\vskip 3ex` after; report/book ends the page (`\@endpart`,
    /// plus an empty page when two-sided + openright).
    pub space_after: Sp,
    pub numbered: bool,
    pub blank_page_after: bool,
}

/// `\part`'s spec. Meaningless for [`ClassKind::Letter`], which defines no
/// `\part` — a `letter` document cannot produce the block this describes.
/// It keeps the in-flow (article) shape rather than report/book's own-page
/// one so that nothing can turn a letter into a part page; the authoritative
/// signal that the class has no sectioning is [`headings`] returning empty.
pub fn part(
    kind: ClassKind,
    fm: FontMetrics,
    twoside: bool,
    openright: bool,
    secnumdepth: i32,
) -> PartSpec {
    if !kind.has_chapters() {
        PartSpec {
            own_page: false,
            page_break: PageBreak::None,
            space_before: fm.ex.scaled("4").unwrap(),
            centered: false,
            number_size: FontSize::LargeL,
            number_title_skip: Sp::ZERO,
            title_size: FontSize::Huge,
            space_after: fm.ex.scaled("3").unwrap(),
            numbered: secnumdepth > -1,
            blank_page_after: false,
        }
    } else {
        PartSpec {
            own_page: true,
            page_break: if openright {
                PageBreak::ClearDoublePage
            } else {
                PageBreak::ClearPage
            },
            space_before: Sp::ZERO,
            centered: true,
            number_size: FontSize::Huge,
            number_title_skip: Sp::pt(20),
            title_size: FontSize::HugeH,
            space_after: Sp::ZERO,
            numbered: secnumdepth > -2,
            blank_page_after: twoside && openright,
        }
    }
}
