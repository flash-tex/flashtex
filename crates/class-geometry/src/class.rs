//! The LaTeX2e standard classes `article`, `report` and `book` (v1.4n,
//! 2025/01/22) and their size files `size1x.clo` / `bk1x.clo`.
//!
//! Line citations are to TeX Live 2026 `texmf-dist/tex/latex/base/`
//! (`article.cls`, `report.cls`, `book.cls`, `size10.clo`, `size11.clo`,
//! `size12.clo`, `bk10.clo`, `bk11.clo`, `bk12.clo`). Only the
//! non-`\if@compatibility` branches are modelled (LaTeX 2.09 compatibility
//! mode is not supported).

use crate::tex::Sp;

fn len(s: &str) -> Sp {
    Sp::parse(s).expect("static TeX length")
}

/// `\documentclass{<kind>}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassKind {
    Article,
    Report,
    Book,
    /// `letter.cls` v1.3c (2024/08/12). It shares `size1x.clo` with article,
    /// so `\textwidth`/`\textheight`/`\baselineskip` are article's, but it
    /// then overrides the whole page frame around them (see
    /// [`class_params`]) and defines no sectioning commands at all —
    /// `\section`, `\chapter`, `\part`, `\maketitle` and `\tableofcontents`
    /// are all undefined in a `letter` document (probed with
    /// `\@ifundefined`, TeX Live 2025).
    Letter,
}

impl ClassKind {
    pub fn parse(name: &str) -> Option<ClassKind> {
        match name.trim() {
            "article" => Some(ClassKind::Article),
            "report" => Some(ClassKind::Report),
            "book" => Some(ClassKind::Book),
            "letter" => Some(ClassKind::Letter),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ClassKind::Article => "article",
            ClassKind::Report => "report",
            ClassKind::Book => "book",
            ClassKind::Letter => "letter",
        }
    }
    pub fn has_chapters(self) -> bool {
        matches!(self, ClassKind::Report | ClassKind::Book)
    }
    /// Whether the class defines `\section` and friends at all. `letter.cls`
    /// does not, so a `letter` document has no heading specs to resolve.
    pub fn has_sections(self) -> bool {
        self != ClassKind::Letter
    }
}

/// `10pt` / `11pt` / `12pt` (`\@ptsize` 0/1/2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseSize {
    Pt10,
    Pt11,
    Pt12,
}

/// Class paper options (article.cls lines 53–70; identical in report/book).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paper {
    A4,
    A5,
    B5,
    Letter,
    Legal,
    Executive,
}

impl Paper {
    /// (`\paperwidth`, `\paperheight`) before `landscape`.
    pub fn size(self) -> (Sp, Sp) {
        match self {
            Paper::A4 => (len("210mm"), len("297mm")),
            Paper::A5 => (len("148mm"), len("210mm")),
            Paper::B5 => (len("176mm"), len("250mm")),
            Paper::Letter => (len("8.5in"), len("11in")),
            Paper::Legal => (len("8.5in"), len("14in")),
            Paper::Executive => (len("7.25in"), len("10.5in")),
        }
    }
}

/// Resolved class options after `\ExecuteOptions` + `\ProcessOptions`.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassOptions {
    pub kind: ClassKind,
    pub size: BaseSize,
    pub paper: Paper,
    pub landscape: bool,
    pub twoside: bool,
    pub twocolumn: bool,
    pub titlepage: bool,
    /// `openright` (report/book only; book default, report `openany`).
    pub openright: bool,
    pub fleqn: bool,
    pub leqno: bool,
    pub draft: bool,
    pub openbib: bool,
    /// `\@classoptionslist` in source order (geometry re-reads these).
    pub given: Vec<String>,
    /// Given options the class did not declare (`Unused global option(s)`).
    pub unused: Vec<String>,
}

impl ClassOptions {
    /// Parse a `\documentclass[...]` option list. `\ProcessOptions`
    /// (unstarred) runs declared options in *declaration* order, so e.g.
    /// `landscape,a4paper` still swaps A4, and `twoside,oneside` is two-sided.
    pub fn parse(kind: ClassKind, options: &str) -> ClassOptions {
        let given: Vec<String> = options
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        // \ExecuteOptions defaults: article.cls line 111; report.cls line 117
        // (adds openany); book.cls line 119 (twoside, openright).
        let mut o = ClassOptions {
            kind,
            size: BaseSize::Pt10,
            paper: Paper::Letter,
            landscape: false,
            twoside: kind == ClassKind::Book,
            twocolumn: false,
            // article.cls line 51 \@titlepagefalse; report.cls line 51 true.
            // letter.cls has no title page at all (no `\maketitle`).
            titlepage: matches!(kind, ClassKind::Report | ClassKind::Book),
            openright: kind == ClassKind::Book,
            fleqn: false,
            leqno: false,
            draft: false,
            openbib: false,
            given: given.clone(),
            unused: Vec::new(),
        };
        let mut declared: Vec<&str> = vec![
            "a4paper",
            "a5paper",
            "b5paper",
            "letterpaper",
            "legalpaper",
            "executivepaper",
            "landscape",
            "10pt",
            "11pt",
            "12pt",
            "oneside",
            "twoside",
            "draft",
            "final",
        ];
        // letter.cls declares exactly the paper/size/side/draft options plus
        // `leqno`/`fleqn` (lines 50-83): `onecolumn`, `twocolumn`,
        // `titlepage`, `notitlepage` and `openbib` are NOT declared, so
        // pdflatex answers "Unused global option(s)" for each of them
        // (verified with all six at once, TeX Live 2025). Every other class
        // keeps its previous declaration list and order exactly.
        let letter = kind == ClassKind::Letter;
        if !letter {
            declared.extend(["titlepage", "notitlepage"]);
        }
        if kind.has_chapters() {
            declared.extend(["openright", "openany"]);
        }
        if letter {
            declared.extend(["leqno", "fleqn"]);
        } else {
            declared.extend(["onecolumn", "twocolumn", "leqno", "fleqn", "openbib"]);
        }
        for name in &declared {
            if !given.iter().any(|g| g == name) {
                continue;
            }
            match *name {
                "a4paper" => o.paper = Paper::A4,
                "a5paper" => o.paper = Paper::A5,
                "b5paper" => o.paper = Paper::B5,
                "letterpaper" => o.paper = Paper::Letter,
                "legalpaper" => o.paper = Paper::Legal,
                "executivepaper" => o.paper = Paper::Executive,
                "landscape" => o.landscape = true,
                "10pt" => o.size = BaseSize::Pt10,
                "11pt" => o.size = BaseSize::Pt11,
                "12pt" => o.size = BaseSize::Pt12,
                "oneside" => o.twoside = false,
                "twoside" => o.twoside = true,
                "draft" => o.draft = true,
                "final" => o.draft = false,
                "titlepage" => o.titlepage = true,
                "notitlepage" => o.titlepage = false,
                "openright" => o.openright = true,
                "openany" => o.openright = false,
                "onecolumn" => o.twocolumn = false,
                "twocolumn" => o.twocolumn = true,
                "leqno" => o.leqno = true,
                "fleqn" => o.fleqn = true,
                "openbib" => o.openbib = true,
                _ => {}
            }
        }
        o.unused = given
            .into_iter()
            .filter(|g| !declared.contains(&g.as_str()))
            .collect();
        o
    }

    /// Paper after the `landscape` swap (article.cls lines 71–74).
    pub fn paper_size(&self) -> (Sp, Sp) {
        let (w, h) = self.paper.size();
        if self.landscape {
            (h, w)
        } else {
            (w, h)
        }
    }
}

/// Body-font dimensions pdflatex reports for `\normalsize` Computer Modern
/// (`\fontdimen6` quad and `\fontdimen5` x-height): cmr10 at 10pt, cmr10 at
/// 10.95pt, cmr12 at 12pt. Measured with pdfTeX 1.40.29 / TeX Live 2026.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontMetrics {
    pub em: Sp,
    pub ex: Sp,
}

pub fn body_font(size: BaseSize) -> FontMetrics {
    let (em, ex) = match size {
        BaseSize::Pt10 => ("10.00002pt", "4.30554pt"),
        BaseSize::Pt11 => ("10.95003pt", "4.71457pt"),
        BaseSize::Pt12 => ("11.74988pt", "5.16667pt"),
    };
    FontMetrics {
        em: len(em),
        ex: len(ex),
    }
}

/// Finite TeX glue (`natural plus stretch minus shrink`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Glue {
    pub natural: Sp,
    pub stretch: Sp,
    pub shrink: Sp,
}

impl Glue {
    pub const fn fixed(natural: Sp) -> Glue {
        Glue {
            natural,
            stretch: Sp(0),
            shrink: Sp(0),
        }
    }
    pub fn new(natural: &str, stretch: &str, shrink: &str) -> Glue {
        Glue {
            natural: len(natural),
            stretch: len(stretch),
            shrink: len(shrink),
        }
    }
}

/// `\the<skip>`: TeX's `print_spec`.
impl std::fmt::Display for Glue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.natural)?;
        if self.stretch != Sp(0) {
            write!(f, " plus {}", self.stretch)?;
        }
        if self.shrink != Sp(0) {
            write!(f, " minus {}", self.shrink)?;
        }
        Ok(())
    }
}

/// Every page-frame length the class (and later geometry) decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageParams {
    pub paperwidth: Sp,
    pub paperheight: Sp,
    pub textwidth: Sp,
    pub textheight: Sp,
    pub oddsidemargin: Sp,
    pub evensidemargin: Sp,
    pub topmargin: Sp,
    pub headheight: Sp,
    pub headsep: Sp,
    pub footskip: Sp,
    pub topskip: Sp,
    pub baselineskip: Sp,
    pub parindent: Sp,
    pub parskip: Glue,
    pub marginparwidth: Sp,
    pub marginparsep: Sp,
    pub marginparpush: Sp,
    pub columnsep: Sp,
    pub columnseprule: Sp,
    pub maxdepth: Sp,
    pub footnotesep: Sp,
    pub skip_footins: Glue,
    pub overfullrule: Sp,
    pub leftmargini: Sp,
    pub labelsep: Sp,
    /// `\mathindent` (fleqn.clo: `\AtEndOfClass{\mathindent\leftmargini}`).
    pub mathindent: Option<Sp>,
    /// `\hoffset` / `\voffset` (geometry `hoffset`/`voffset`; 0 otherwise).
    pub hoffset: Sp,
    pub voffset: Sp,
}

impl PageParams {
    /// `\columnwidth` as `\begin{document}` sets it (latex.ltx lines
    /// 9479–9483): `(\textwidth - \columnsep) / 2` in two-column mode.
    pub fn columnwidth(&self, twocolumn: bool) -> Sp {
        if twocolumn {
            (self.textwidth - self.columnsep).over(2)
        } else {
            self.textwidth
        }
    }
}

/// Compute the class's page parameters exactly as `size1x.clo`/`bk1x.clo`
/// do (non-compatibility branches).
pub fn class_params(o: &ClassOptions) -> PageParams {
    if o.kind == ClassKind::Letter {
        return letter_params(o);
    }
    let bk = o.kind == ClassKind::Book;
    let size = o.size;
    let fm = body_font(size);
    let (pw, ph) = o.paper_size();
    let inch = len("1in");

    // size1x.clo line 48: \normalsize -> \baselineskip 12pt / 13.6pt / 14.5pt.
    let baselineskip = match size {
        BaseSize::Pt10 => len("12pt"),
        BaseSize::Pt11 => len("13.6pt"),
        BaseSize::Pt12 => len("14.5pt"),
    };
    // lines 87–91: \parindent 1em (twocolumn) else 15pt / 17pt / 1.5em.
    let parindent = if o.twocolumn {
        fm.em
    } else {
        match size {
            BaseSize::Pt10 => Sp::pt(15),
            BaseSize::Pt11 => Sp::pt(17),
            BaseSize::Pt12 => fm.em.scaled("1.5").unwrap(),
        }
    };
    // line 95 \headheight 12pt; line 96 \headsep 25pt (bk10 .25in, bk11/12
    // .275in); line 98 \footskip 30pt (bk10 .35in, bk11 .38in, bk12 30pt).
    let headheight = Sp::pt(12);
    let headsep = match (bk, size) {
        (false, _) => Sp::pt(25),
        (true, BaseSize::Pt10) => len(".25in"),
        (true, _) => len(".275in"),
    };
    let footskip = match (bk, size) {
        (true, BaseSize::Pt10) => len(".35in"),
        (true, BaseSize::Pt11) => len(".38in"),
        _ => Sp::pt(30),
    };
    // line 97 \topskip 10/11/12pt; line 100 \maxdepth .5\topskip.
    let topskip = match size {
        BaseSize::Pt10 => Sp::pt(10),
        BaseSize::Pt11 => Sp::pt(11),
        BaseSize::Pt12 => Sp::pt(12),
    };
    let maxdepth = topskip.scaled(".5").unwrap();

    // lines 107–127: \textwidth = min(\paperwidth-2in, 345/360/390pt)
    // (twocolumn: 2x), truncated to whole points.
    let nominal = match size {
        BaseSize::Pt10 => Sp::pt(345),
        BaseSize::Pt11 => Sp::pt(360),
        BaseSize::Pt12 => Sp::pt(390),
    };
    let avail = pw - len("2in");
    let textwidth = if o.twocolumn {
        if avail > nominal.times(2) {
            nominal.times(2)
        } else {
            avail
        }
    } else if avail > nominal {
        nominal
    } else {
        avail
    }
    .settopoint();

    // lines 130–138: whole lines of \baselineskip in \paperheight-3.5in,
    // plus \topskip.
    let room = ph - len("2in") - len("1.5in");
    let lines = room.over(baselineskip.0);
    let textheight = baselineskip.times(lines.0) + topskip;

    // lines 139–144.
    let marginparsep = if o.twocolumn {
        Sp::pt(10)
    } else if bk {
        Sp::pt(7)
    } else if size == BaseSize::Pt10 {
        Sp::pt(11)
    } else {
        Sp::pt(10)
    };
    let marginparpush = if size == BaseSize::Pt12 {
        Sp::pt(7)
    } else {
        Sp::pt(5)
    };

    // lines 161–188.
    let spare = pw - textwidth;
    let (mut odd, mut mpw) = if o.twoside {
        (
            spare.scaled(".4").unwrap() - inch,
            spare.scaled(".6").unwrap() - marginparsep - len(".4in"),
        )
    } else {
        (
            spare.scaled(".5").unwrap() - inch,
            spare.scaled(".5").unwrap() - marginparsep - len(".4in") - len(".4in"),
        )
    };
    if mpw > len("2in") {
        mpw = len("2in");
    }
    odd = odd.settopoint();
    mpw = mpw.settopoint();
    let even = (pw - len("2in") - textwidth - odd).settopoint();

    // lines 193–200.
    let mut topmargin = ph - len("2in") - headheight - headsep - textheight - footskip;
    topmargin = topmargin - topmargin.scaled(".5").unwrap();
    let topmargin = topmargin.settopoint();

    // lines 202–203.
    let (footnotesep, skip_footins) = match size {
        BaseSize::Pt10 => (len("6.65pt"), Glue::new("9pt", "4pt", "2pt")),
        BaseSize::Pt11 => (len("7.7pt"), Glue::new("10pt", "4pt", "2pt")),
        BaseSize::Pt12 => (len("8.4pt"), Glue::new("10.8pt", "4pt", "2pt")),
    };
    // article.cls lines 322–326, 338.
    let leftmargini = fm.em.scaled(if o.twocolumn { "2" } else { "2.5" }).unwrap();
    let labelsep = fm.em.scaled(".5").unwrap();

    PageParams {
        paperwidth: pw,
        paperheight: ph,
        textwidth,
        textheight,
        oddsidemargin: odd,
        evensidemargin: even,
        topmargin,
        headheight,
        headsep,
        footskip,
        topskip,
        baselineskip,
        parindent,
        // article.cls line 117.
        parskip: Glue::new("0pt", "1pt", "0pt"),
        marginparwidth: mpw,
        marginparsep,
        marginparpush,
        // article.cls lines 627–628.
        columnsep: Sp::pt(10),
        columnseprule: Sp::ZERO,
        maxdepth,
        footnotesep,
        skip_footins,
        // article.cls lines 87–90.
        overfullrule: if o.draft { Sp::pt(5) } else { Sp::ZERO },
        leftmargini,
        labelsep,
        mathindent: if o.fleqn { Some(leftmargini) } else { None },
        hoffset: Sp::ZERO,
        voffset: Sp::ZERO,
    }
}

/// `letter.cls` (v1.3c 2024/08/12) lines 86-119, non-`\if@compatibility`
/// branches.
///
/// The class inputs the *article* size file (`\input{size1\@ptsize.clo}`),
/// so `\textwidth`, `\textheight`, `\baselineskip`, `\topskip` and
/// `\maxdepth` are article's, and then replaces almost everything around
/// them. Each value below was read back out of pdflatex (`\the\...`, TeX
/// Live 2025) at 10pt, 11pt and 12pt on both Letter and A4 paper; the
/// differences from article are worth naming, because they are exactly what
/// a letter rendered with article geometry gets wrong:
///
/// | length | article (11pt, Letter) | letter |
/// |---|---|---|
/// | `\topmargin` | 8pt (derived) | **27pt** (fixed, line 116) |
/// | `\headsep` | 25pt | **45pt** |
/// | `\footskip` | 30pt | **25pt** |
/// | `\oddsidemargin` | 55pt (`\@settopoint`ed) | **54.8775pt** (not) |
/// | `\evensidemargin` | derived separately | **= `\oddsidemargin`** |
/// | `\parindent` | 17pt | **0pt** |
/// | `\parskip` | 0pt plus 1pt | **7.66498pt** rigid (0.7em) |
/// | `\marginparwidth` | derived | **90pt** |
/// | `\labelsep` | .5em | **5pt** |
/// | `\footnotesep` | 7.7pt | **12pt** |
/// | `\skip\footins` | 10pt plus 4 minus 2 | **10pt plus 2 minus 4** |
///
/// `\oddsidemargin` is `.5\@tempdima` of `\paperwidth - 2in - \textwidth`
/// with **no** `\@settopoint` (lines 106-112), unlike article's, which is
/// why it keeps a fraction: 54.8775pt at 11pt Letter, 31.48393pt at 12pt A4.
/// `twoside` does not change either side margin (both measured 54.8775pt
/// under `[twoside,11pt]`).
fn letter_params(o: &ClassOptions) -> PageParams {
    let size = o.size;
    let fm = body_font(size);
    let (pw, ph) = o.paper_size();

    // size1x.clo, shared with article.
    let baselineskip = match size {
        BaseSize::Pt10 => len("12pt"),
        BaseSize::Pt11 => len("13.6pt"),
        BaseSize::Pt12 => len("14.5pt"),
    };
    let topskip = match size {
        BaseSize::Pt10 => Sp::pt(10),
        BaseSize::Pt11 => Sp::pt(11),
        BaseSize::Pt12 => Sp::pt(12),
    };
    let maxdepth = topskip.scaled(".5").unwrap();
    let nominal = match size {
        BaseSize::Pt10 => Sp::pt(345),
        BaseSize::Pt11 => Sp::pt(360),
        BaseSize::Pt12 => Sp::pt(390),
    };
    let avail = pw - len("2in");
    let textwidth = if avail > nominal { nominal } else { avail }.settopoint();
    let room = ph - len("2in") - len("1.5in");
    let lines = room.over(baselineskip.0);
    let textheight = baselineskip.times(lines.0) + topskip;

    // letter.cls line 91: `\setlength\parskip{0.7em}`, evaluated in the
    // class body font, and rigid (no plus/minus). TeX's own fixed-point
    // scaling of `0.7em` is what pdflatex reports, so the three values are
    // measured rather than recomputed from `fm.em`.
    let parskip = match size {
        BaseSize::Pt10 => len("6.99997pt"),
        BaseSize::Pt11 => len("7.66498pt"),
        BaseSize::Pt12 => len("8.22487pt"),
    };

    // lines 95-97, 116.
    let headheight = Sp::pt(12);
    let headsep = Sp::pt(45);
    let footskip = Sp::pt(25);
    let topmargin = Sp::pt(27);

    // lines 106-113.
    let sidemargin = (pw - len("2in") - textwidth).scaled(".5").unwrap();

    PageParams {
        paperwidth: pw,
        paperheight: ph,
        textwidth,
        textheight,
        oddsidemargin: sidemargin,
        evensidemargin: sidemargin,
        topmargin,
        headheight,
        headsep,
        footskip,
        topskip,
        baselineskip,
        // line 92.
        parindent: Sp::ZERO,
        parskip: Glue::fixed(parskip),
        // lines 111, 114-115.
        marginparwidth: Sp::pt(90),
        marginparsep: Sp::pt(11),
        marginparpush: Sp::pt(5),
        // line 358.
        columnsep: Sp::pt(10),
        columnseprule: Sp::ZERO,
        maxdepth,
        // lines 117-118.
        footnotesep: Sp::pt(12),
        skip_footins: Glue::new("10pt", "2pt", "4pt"),
        // lines 80-81.
        overfullrule: if o.draft { Sp::pt(5) } else { Sp::ZERO },
        // lines 283-291: `\leftmargini` 2.5em, `\labelsep` a flat 5pt
        // (article derives `.5em`).
        leftmargini: fm.em.scaled("2.5").unwrap(),
        labelsep: Sp::pt(5),
        mathindent: if o.fleqn {
            Some(fm.em.scaled("2.5").unwrap())
        } else {
            None
        },
        hoffset: Sp::ZERO,
        voffset: Sp::ZERO,
    }
}

/// `\longindentation` (`letter.cls` line 219: `.5\textwidth`) and
/// `\indentedwidth` (lines 220-222: `\textwidth` less `\longindentation`) —
/// the two lengths `\closing` sets its parbox at. Meaningful only for
/// [`ClassKind::Letter`].
///
/// It takes the *class* options, not a resolved [`PageParams`], because
/// `letter.cls` assigns both at class-load time from the class's own
/// `\textwidth` and nothing updates them afterwards. Under
/// `\usepackage[margin=1in]{letter}`-style geometry the text is 469.75502pt
/// wide at 11pt but `\longindentation` is still 180pt — half of the class's
/// 360pt, not half the measure (pdflatex, TeX Live 2025). A closing block
/// indented by half the *geometry* measure would be 54.88pt too far right.
pub fn letter_indentation(options: &ClassOptions) -> (Sp, Sp) {
    let textwidth = class_params(options).textwidth;
    let long = textwidth.scaled(".5").unwrap();
    (long, textwidth - long)
}

/// The class's font-size commands (size1x.clo lines 47–86; sizes from
/// latex.ltx `\@xpt`…`\@xxvpt`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontSize {
    Tiny,
    ScriptSize,
    FootnoteSize,
    Small,
    NormalSize,
    Large,
    LargeL,
    LARGE,
    Huge,
    HugeH,
}

impl FontSize {
    /// (font size, `\baselineskip`) for this command in a class of `base` size.
    pub fn metrics(self, base: BaseSize) -> (Sp, Sp) {
        use BaseSize::*;
        use FontSize::*;
        let (s, b) = match (base, self) {
            (Pt10, Tiny) => ("5pt", "6pt"),
            (Pt10, ScriptSize) => ("7pt", "8pt"),
            (Pt10, FootnoteSize) => ("8pt", "9.5pt"),
            (Pt10, Small) => ("9pt", "11pt"),
            (Pt10, NormalSize) => ("10pt", "12pt"),
            (Pt11, Tiny) | (Pt12, Tiny) => ("6pt", "7pt"),
            (Pt11, ScriptSize) | (Pt12, ScriptSize) => ("8pt", "9.5pt"),
            (Pt11, FootnoteSize) => ("9pt", "11pt"),
            (Pt11, Small) => ("10pt", "12pt"),
            (Pt11, NormalSize) => ("10.95pt", "13.6pt"),
            (Pt12, FootnoteSize) => ("10pt", "12pt"),
            (Pt12, Small) => ("10.95pt", "13.6pt"),
            (Pt12, NormalSize) => ("12pt", "14.5pt"),
            (Pt10 | Pt11, Large) => ("12pt", "14pt"),
            (Pt10 | Pt11, LargeL) => ("14.4pt", "18pt"),
            (Pt10 | Pt11, LARGE) => ("17.28pt", "22pt"),
            (Pt10 | Pt11, Huge) => ("20.74pt", "25pt"),
            (Pt10 | Pt11, HugeH) => ("24.88pt", "30pt"),
            (Pt12, Large) => ("14.4pt", "18pt"),
            (Pt12, LargeL) => ("17.28pt", "22pt"),
            (Pt12, LARGE) => ("20.74pt", "25pt"),
            // size12.clo line 86: \let\Huge=\huge.
            (Pt12, Huge) | (Pt12, HugeH) => ("24.88pt", "30pt"),
        };
        (len(s), len(b))
    }

    pub fn command(self) -> &'static str {
        match self {
            FontSize::Tiny => "\\tiny",
            FontSize::ScriptSize => "\\scriptsize",
            FontSize::FootnoteSize => "\\footnotesize",
            FontSize::Small => "\\small",
            FontSize::NormalSize => "\\normalsize",
            FontSize::Large => "\\large",
            FontSize::LargeL => "\\Large",
            FontSize::LARGE => "\\LARGE",
            FontSize::Huge => "\\huge",
            FontSize::HugeH => "\\Huge",
        }
    }
}
