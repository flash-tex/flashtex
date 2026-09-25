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
    /// exam.cls `\ps@head`: exam's head, empty foot.
    Head,
    /// exam.cls `\ps@foot`: empty head, exam's foot.
    Foot,
    /// exam.cls `\ps@headandfoot` (the class default).
    HeadAndFoot,
}

impl PageStyle {
    pub fn parse(s: &str) -> Option<PageStyle> {
        match s.trim() {
            "empty" => Some(PageStyle::Empty),
            "plain" => Some(PageStyle::Plain),
            "headings" => Some(PageStyle::Headings),
            "myheadings" => Some(PageStyle::MyHeadings),
            "head" => Some(PageStyle::Head),
            "foot" => Some(PageStyle::Foot),
            "headandfoot" => Some(PageStyle::HeadAndFoot),
            _ => None,
        }
    }

    /// The styles only exam.cls defines (`\ps@head`, `\ps@foot`,
    /// `\ps@headandfoot`); in any other class `\pagestyle{head}` is an
    /// undefined style and changes nothing.
    pub fn is_exam_only(self) -> bool {
        matches!(self, PageStyle::Head | PageStyle::Foot | PageStyle::HeadAndFoot)
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
    /// exam.cls head slot 0/1/2 (left/center/right): the first-page text
    /// on page 1 (`\value{page}=1`), the running text elsewhere (see
    /// [`ExamChrome`]).
    ExamHead(u8),
    /// exam.cls foot slot 0/1/2, like [`Field::ExamHead`].
    ExamFoot(u8),
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
        let exam_head = Line {
            left: Field::ExamHead(0),
            center: Field::ExamHead(1),
            right: Field::ExamHead(2),
        };
        let exam_foot = Line {
            left: Field::ExamFoot(0),
            center: Field::ExamFoot(1),
            right: Field::ExamFoot(2),
        };
        match style {
            PageStyle::Empty => self = StyleMacros::EMPTY,
            // exam.cls lines 1066-1089: `\@dohead`/`\@nohead` and
            // `\@dofoot`/`\@nofoot` set odd and even alike.
            PageStyle::Head | PageStyle::Foot | PageStyle::HeadAndFoot => {
                let head = if style == PageStyle::Foot { Line::EMPTY } else { exam_head };
                let foot = if style == PageStyle::Head { Line::EMPTY } else { exam_foot };
                self = StyleMacros {
                    oddhead: head,
                    evenhead: head,
                    oddfoot: foot,
                    evenfoot: foot,
                };
            }
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
    match kind {
        ClassKind::Book => PageStyle::Headings,
        // beamer ships its own headline/footline templates through its
        // output routine; the default theme's are empty and there is no
        // folio (measured: no digit on any page of the beamer corpus). The
        // kernel page style underneath is irrelevant, so `empty` models it.
        ClassKind::Beamer => PageStyle::Empty,
        // exam.cls line 1449.
        ClassKind::Exam => PageStyle::HeadAndFoot,
        _ => PageStyle::Plain,
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
    // `scrbook` has `\frontmatter` / `\mainmatter` like `book`; `scrartcl`
    // marks like `article` (no chapters to mark).
    let book = matches!(kind, ClassKind::Book | ClassKind::Scrbook);
    match (kind, class_twoside) {
        (ClassKind::Article | ClassKind::Exam | ClassKind::Scrartcl, true) => vec![
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
        (ClassKind::Article | ClassKind::Exam | ClassKind::Scrartcl, false) => vec![r(
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

/// exam.cls's running head and foot contents (lines 1383-1450,
/// 1578-1597): three slots each for the first page (`\value{page}=1`)
/// and for every other page, as raw TeX from the preamble, plus the rule
/// switches. `macros` are the preamble's argument-free user macros
/// (`\newcommand{\myname}{..}`, `\def\myname{..}`), which the slots
/// commonly use and which the renderer expands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExamChrome {
    pub first_head: [String; 3],
    pub run_head: [String; 3],
    pub first_foot: [String; 3],
    pub run_foot: [String; 3],
    pub first_headrule: bool,
    pub run_headrule: bool,
    pub first_footrule: bool,
    pub run_footrule: bool,
    pub macros: Vec<(String, String)>,
}

impl Default for ExamChrome {
    /// exam.cls lines 1451-1456: every slot empty except the running
    /// center foot, `\cfoot[]{Page \thepage}`; no rules.
    fn default() -> ExamChrome {
        let empty = || [String::new(), String::new(), String::new()];
        ExamChrome {
            first_head: empty(),
            run_head: empty(),
            first_foot: empty(),
            run_foot: [String::new(), "Page \\thepage".to_string(), String::new()],
            first_headrule: false,
            run_headrule: false,
            first_footrule: false,
            run_footrule: false,
            macros: Vec::new(),
        }
    }
}

impl ExamChrome {
    /// The raw text of `field` on a page whose `\value{page}` is `page`.
    pub fn slot(&self, field: Field, page: i64) -> Option<&str> {
        let first = page == 1;
        let (line, k) = match field {
            Field::ExamHead(k) => (if first { &self.first_head } else { &self.run_head }, k),
            Field::ExamFoot(k) => (if first { &self.first_foot } else { &self.run_foot }, k),
            _ => return None,
        };
        line.get(k as usize).map(String::as_str)
    }

    /// Whether the head (`head == true`) or foot rule is drawn on `page`.
    pub fn rule(&self, head: bool, page: i64) -> bool {
        match (head, page == 1) {
            (true, true) => self.first_headrule,
            (true, false) => self.run_headrule,
            (false, true) => self.first_footrule,
            (false, false) => self.run_footrule,
        }
    }

    /// Apply one preamble command (`name` without the backslash) with its
    /// optional and mandatory arguments. Returns how many mandatory
    /// arguments it consumed (`None`: not an exam chrome command).
    pub fn command(&mut self, name: &str, opt: Option<&str>, args: &[String]) -> Option<usize> {
        let three = |a: &[String]| -> Option<[String; 3]> {
            Some([a.first()?.clone(), a.get(1)?.clone(), a.get(2)?.clone()])
        };
        let slot = |name: &str| -> Option<(bool, usize)> {
            Some(match name {
                "lhead" => (true, 0),
                "chead" => (true, 1),
                "rhead" => (true, 2),
                "lfoot" => (false, 0),
                "cfoot" => (false, 1),
                "rfoot" => (false, 2),
                _ => return None,
            })
        };
        match name {
            "header" | "firstpageheader" | "runningheader" | "footer" | "firstpagefooter"
            | "runningfooter" => {
                let t = three(args)?;
                let (first, run) = if name.starts_with("header") || name.starts_with("footer") {
                    (true, true)
                } else {
                    (name.starts_with("firstpage"), name.starts_with("running"))
                };
                let head = name.ends_with("header");
                if first {
                    *(if head { &mut self.first_head } else { &mut self.first_foot }) = t.clone();
                }
                if run {
                    *(if head { &mut self.run_head } else { &mut self.run_foot }) = t;
                }
                Some(3)
            }
            _ if slot(name).is_some() => {
                let (head, k) = slot(name)?;
                let run = args.first()?.clone();
                // `\lhead[first]{running}`; without the option both.
                let first = opt.map_or_else(|| run.clone(), str::to_string);
                if head {
                    self.first_head[k] = first;
                    self.run_head[k] = run;
                } else {
                    self.first_foot[k] = first;
                    self.run_foot[k] = run;
                }
                Some(1)
            }
            _ => {
                let (head, first, run, on) = match name {
                    "headrule" => (true, true, true, true),
                    "noheadrule" => (true, true, true, false),
                    "firstpageheadrule" => (true, true, false, true),
                    "nofirstpageheadrule" => (true, true, false, false),
                    "runningheadrule" => (true, false, true, true),
                    "norunningheadrule" => (true, false, true, false),
                    "footrule" => (false, true, true, true),
                    "nofootrule" => (false, true, true, false),
                    "firstpagefootrule" => (false, true, false, true),
                    "nofirstpagefootrule" => (false, true, false, false),
                    "runningfootrule" => (false, false, true, true),
                    "norunningfootrule" => (false, false, true, false),
                    _ => return None,
                };
                if first {
                    *(if head { &mut self.first_headrule } else { &mut self.first_footrule }) = on;
                }
                if run {
                    *(if head { &mut self.run_headrule } else { &mut self.run_footrule }) = on;
                }
                Some(0)
            }
        }
    }
}
