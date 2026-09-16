//! Page styles: `\ps@empty` and `\ps@plain` (latex.ltx lines 18305–18311),
//! `\ps@headings` / `\ps@myheadings` (article.cls lines 131–168; report.cls
//! and book.cls variants with `\chaptermark`). The class defines
//! `\ps@headings` once, at load time, from `\if@twoside`; `\@outputpage`
//! (latex.ltx line 20880) picks the even or odd macros from `\if@twoside`
//! at shipout time.

use crate::class::ClassKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageStyle {
    Empty,
    Plain,
    Headings,
    MyHeadings,
}

impl PageStyle {
    pub fn parse(s: &str) -> Option<PageStyle> {
        match s.trim() {
            "empty" => Some(PageStyle::Empty),
            "plain" => Some(PageStyle::Plain),
            "headings" => Some(PageStyle::Headings),
            "myheadings" => Some(PageStyle::MyHeadings),
            _ => None,
        }
    }
}

/// What a header/footer slot shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Empty,
    /// `\thepage`.
    PageNumber,
    /// `\leftmark` in `\slshape` (first `\markboth` left argument on the page).
    LeftMark,
    /// `\rightmark` in `\slshape`.
    RightMark,
}

/// One header or footer line: `\hb@xt@\textwidth{left\hfil center\hfil right}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub left: Field,
    pub center: Field,
    pub right: Field,
}

impl Line {
    pub const EMPTY: Line = Line {
        left: Field::Empty,
        center: Field::Empty,
        right: Field::Empty,
    };
    pub fn is_empty(&self) -> bool {
        *self == Line::EMPTY
    }
}

/// The four macros `\@oddhead`, `\@evenhead`, `\@oddfoot`, `\@evenfoot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleMacros {
    pub oddhead: Line,
    pub evenhead: Line,
    pub oddfoot: Line,
    pub evenfoot: Line,
}

impl StyleMacros {
    pub const EMPTY: StyleMacros = StyleMacros {
        oddhead: Line::EMPTY,
        evenhead: Line::EMPTY,
        oddfoot: Line::EMPTY,
        evenfoot: Line::EMPTY,
    };

    /// Apply `\pagestyle{style}` to the current macros. Styles only
    /// redefine what their `\ps@` macro assigns, so e.g. one-sided
    /// `headings` leaves `\@evenfoot` from the previous style.
    pub fn apply(mut self, style: PageStyle, class_twoside: bool) -> StyleMacros {
        let num_center = Line {
            left: Field::Empty,
            center: Field::PageNumber,
            right: Field::Empty,
        };
        let even = Line {
            left: Field::PageNumber,
            center: Field::Empty,
            right: Field::LeftMark,
        };
        let odd = Line {
            left: Field::RightMark,
            center: Field::Empty,
            right: Field::PageNumber,
        };
        match style {
            PageStyle::Empty => self = StyleMacros::EMPTY,
            PageStyle::Plain => {
                self.oddhead = Line::EMPTY;
                self.oddfoot = num_center;
                self.evenhead = Line::EMPTY;
                self.evenfoot = num_center;
            }
            PageStyle::Headings if class_twoside => {
                self.oddfoot = Line::EMPTY;
                self.evenfoot = Line::EMPTY;
                self.evenhead = even;
                self.oddhead = odd;
            }
            PageStyle::Headings => {
                self.oddfoot = Line::EMPTY;
                self.oddhead = odd;
            }
            PageStyle::MyHeadings => {
                self.oddfoot = Line::EMPTY;
                self.evenfoot = Line::EMPTY;
                self.evenhead = even;
                self.oddhead = odd;
            }
        }
        self
    }

    /// Head and foot `\@outputpage` ships on `page` (odd macros always when
    /// one-sided).
    pub fn for_page(&self, twoside: bool, page: i64) -> (Line, Line) {
        if twoside && page % 2 == 0 {
            (self.evenhead, self.evenfoot)
        } else {
            (self.oddhead, self.oddfoot)
        }
    }
}

/// The class's default `\pagestyle` (article.cls line 629 `plain`;
/// book.cls line 734 `headings`).
pub fn class_default(kind: ClassKind) -> PageStyle {
    if kind == ClassKind::Book {
        PageStyle::Headings
    } else {
        PageStyle::Plain
    }
}

/// Which mark command a sectioning command issues under `headings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkTarget {
    /// `\markboth{text}{}`.
    Both,
    /// `\markright{text}`.
    Right,
}

/// Number prefix in the mark text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkNumber {
    /// `\thesection\quad`.
    Quad,
    /// `\@chapapp\ \thechapter. \ ` ("CHAPTER 1. ").
    ChapterDot,
    /// `\thesection. \ ` ("1.1. ").
    Dot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkRule {
    /// "chapter", "section" or "subsection".
    pub command: &'static str,
    pub target: MarkTarget,
    pub uppercase: bool,
    pub number: MarkNumber,
    /// The number is included when `\c@secnumdepth > number_if_depth_above`.
    pub number_if_depth_above: i32,
    /// book: number only `\if@mainmatter`.
    pub mainmatter_only: bool,
}

/// Mark rules active under `style` (empty for `myheadings`, whose
/// `\sectionmark` etc. are `\@gobble`, and for empty/plain whose
/// `\@mkboth` is `\@gobbletwo`).
pub fn mark_rules(kind: ClassKind, style: PageStyle, class_twoside: bool) -> Vec<MarkRule> {
    if style != PageStyle::Headings {
        return Vec::new();
    }
    // letter.cls's own `\ps@headings` (lines 120-133) builds its running
    // head from `\toname`/`\@date`/`\thepage` and issues no marks at all —
    // and the class has no sectioning commands to issue them from.
    if !kind.has_sections() {
        return Vec::new();
    }
    let r = |command, target, uppercase, number, above, mm| MarkRule {
        command,
        target,
        uppercase,
        number,
        number_if_depth_above: above,
        mainmatter_only: mm,
    };
    let book = kind == ClassKind::Book;
    match (kind, class_twoside) {
        (ClassKind::Article, true) => vec![
            r(
                "section",
                MarkTarget::Both,
                true,
                MarkNumber::Quad,
                0,
                false,
            ),
            r(
                "subsection",
                MarkTarget::Right,
                false,
                MarkNumber::Quad,
                1,
                false,
            ),
        ],
        (ClassKind::Article, false) => vec![r(
            "section",
            MarkTarget::Right,
            true,
            MarkNumber::Quad,
            -1,
            false,
        )],
        (_, true) => vec![
            r(
                "chapter",
                MarkTarget::Both,
                true,
                MarkNumber::ChapterDot,
                -1,
                book,
            ),
            r(
                "section",
                MarkTarget::Right,
                true,
                MarkNumber::Dot,
                0,
                false,
            ),
        ],
        (_, false) => vec![r(
            "chapter",
            MarkTarget::Right,
            true,
            MarkNumber::ChapterDot,
            -1,
            book,
        )],
    }
}

/// `\pagenumbering` styles (`\@arabic`, `\@roman`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Numbering {
    Arabic,
    Roman,
    UpperRoman,
    Alph,
    UpperAlph,
}

impl Numbering {
    pub fn format(self, n: i64) -> String {
        match self {
            Numbering::Arabic => n.to_string(),
            Numbering::Roman => roman(n),
            Numbering::UpperRoman => roman(n).to_uppercase(),
            Numbering::Alph => alph(n),
            Numbering::UpperAlph => alph(n).to_uppercase(),
        }
    }
}

/// TeX `\romannumeral` (empty for n <= 0).
fn roman(mut n: i64) -> String {
    let table = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut s = String::new();
    for (v, t) in table {
        while n >= v {
            s.push_str(t);
            n -= v;
        }
    }
    s
}

/// `\@alph`: 1..26 -> a..z (LaTeX errors outside that range).
fn alph(n: i64) -> String {
    if (1..=26).contains(&n) {
        char::from(b'a' + (n - 1) as u8).to_string()
    } else {
        String::new()
    }
}
