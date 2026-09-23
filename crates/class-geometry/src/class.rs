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
    /// `exam.cls` (Philip Hirschhorn, TeX Live 2026): `\LoadClass{article}`
    /// after passing it every option exam does not declare itself, then its
    /// own page frame (exam.cls lines 747-760, see [`exam_params`]).
    /// Sizes, sections and lists are article's. Default page style
    /// `headandfoot` (line 1449).
    Exam,
    /// KOMA-Script `scrartcl` (v3.49.2, TeX Live 2026): default paper A4,
    /// default size 11pt, oneside. Geometry comes from `typearea`, not
    /// `size1x.clo` (see [`koma_params`]).
    Scrartcl,
    /// KOMA-Script `scrreprt`: like `scrartcl` but with chapters and a
    /// title page (report's shape).
    Scrreprt,
    /// KOMA-Script `scrbook`: like `scrreprt` but twoside with `openright`
    /// (book's shape).
    Scrbook,
    /// `beamer.cls` (v3.x): a slide class, not a `size1x.clo` consumer.
    /// The paper is beamer's own (128mm × 96mm 4:3 by default, changed by
    /// the `aspectratio` class option — see [`beamer_paper_size`]), the
    /// text block is inset 1cm left and right and spans the full paper
    /// height (zero vertical margins; only a 4pt footskip reservation),
    /// and the body size is
    /// 11pt (`\baselineskip` 13.6pt). Frame pagination (`\frame`
    /// starting a new page, `\pause`, overlays) is deliberately NOT
    /// modelled here — only the page geometry.
    Beamer,
}

impl ClassKind {
    pub fn parse(name: &str) -> Option<ClassKind> {
        match name.trim() {
            "article" => Some(ClassKind::Article),
            "report" => Some(ClassKind::Report),
            "book" => Some(ClassKind::Book),
            "letter" => Some(ClassKind::Letter),
            "exam" => Some(ClassKind::Exam),
            // `scrarticle.cls` only forwards its options to `scrartcl`.
            "scrartcl" | "scrarticle" => Some(ClassKind::Scrartcl),
            "scrreprt" => Some(ClassKind::Scrreprt),
            "scrbook" => Some(ClassKind::Scrbook),
            "beamer" => Some(ClassKind::Beamer),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ClassKind::Article => "article",
            ClassKind::Report => "report",
            ClassKind::Book => "book",
            ClassKind::Letter => "letter",
            ClassKind::Exam => "exam",
            ClassKind::Scrartcl => "scrartcl",
            ClassKind::Scrreprt => "scrreprt",
            ClassKind::Scrbook => "scrbook",
            ClassKind::Beamer => "beamer",
        }
    }
    /// Whether the class runs KOMA's `typearea` instead of `size1x.clo`.
    pub fn is_koma(self) -> bool {
        matches!(
            self,
            ClassKind::Scrartcl | ClassKind::Scrreprt | ClassKind::Scrbook
        )
    }
    pub fn has_chapters(self) -> bool {
        matches!(
            self,
            ClassKind::Report
                | ClassKind::Book
                | ClassKind::Scrreprt
                | ClassKind::Scrbook
        )
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

/// KOMA `typearea`'s `DIV` (v3.49.2, TeX Live 2026, `typearea.sty`): the
/// internal `default` (A4 table, else calculated), `calc`, `classic`, or an
/// explicit number. Numbers below 4 are typearea's own sentinels (0 is
/// another spelling of `default`, 1–2 calculate, 3 is classic), so they
/// decode the same way here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivSpec {
    Default,
    Calc,
    Classic,
    Num(u32),
}

/// KOMA `parskip` paragraph separation (`scrartcl.cls`
/// `scrkernel-paragraphs.dtx`, TeX Live 2026): `false` (the default) and
/// `never` keep the 1em paragraph indent; every `half…` spelling zeroes the
/// indent with `\parskip` half the body `\baselineskip` (natural and
/// stretch); every `full…` spelling zeroes the indent with `\parskip` one
/// `\baselineskip` plus a tenth. The `-` / `+` / `*` suffixes only move
/// `\parfillskip` (not modelled: [`PageParams`] has no such length), so all
/// `half…` spellings share one `\parskip` and all `full…` share another.
/// `relative` / `absolute` only switch the `\selectfont` re-evaluation, so
/// they leave the resolved lengths alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parskip {
    False,
    Never,
    Half,
    Full,
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
    /// KOMA `BCOR` (binding correction; standard classes have none).
    pub bcor: Sp,
    /// KOMA `DIV` (standard classes have none).
    pub div: DivSpec,
    /// KOMA `parskip` paragraph separation (standard classes have none;
    /// `false` is the default there too, but it is not a declared option).
    pub parskip: Parskip,
    /// KOMA `headinclude` / `footinclude` / `mpinclude`.
    pub head_include: bool,
    pub foot_include: bool,
    pub marginpar_include: bool,
    /// KOMA `pagesize`: false keeps pdfTeX's engine-default media instead
    /// of setting it from the paper (like the standard classes always do).
    pub pagesize_pdf: bool,
    /// beamer's resolved paper size from the `aspectratio` class option
    /// (see [`beamer_paper_size`]; default 128mm × 96mm). `None` for every
    /// other class, whose paper comes from [`Paper`] instead.
    pub beamer_paper: Option<(Sp, Sp)>,
    /// `\@classoptionslist` in source order (geometry re-reads these).
    pub given: Vec<String>,
    /// Given options the class did not declare (`Unused global option(s)`).
    pub unused: Vec<String>,
}

impl ClassOptions {
    /// Parse a `\documentclass[...]` option list. `\ProcessOptions`
    /// (unstarred) runs declared options in *declaration* order, so e.g.
    /// `landscape,a4paper` still swaps A4, and `twoside,oneside` is two-sided.
    /// KOMA classes instead process key=value options in *given* order
    /// (later wins), with different defaults (11pt, A4).
    pub fn parse(kind: ClassKind, options: &str) -> ClassOptions {
        if kind == ClassKind::Beamer {
            return Self::parse_beamer(kind, options);
        }
        if kind.is_koma() {
            return Self::parse_koma(kind, options);
        }
        Self::parse_standard(kind, options)
    }

    fn parse_standard(kind: ClassKind, options: &str) -> ClassOptions {
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
            bcor: Sp::ZERO,
            div: DivSpec::Default,
            parskip: Parskip::False,
            head_include: false,
            foot_include: false,
            marginpar_include: false,
            pagesize_pdf: true,
            beamer_paper: None,
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
        // exam.cls lines 688-717 declare these itself (the rest go to
        // article through `\DeclareOption*`), so they are never unused.
        let exam_own: &[&str] = if kind == ClassKind::Exam {
            &[
                "answers",
                "noanswers",
                "cancelspace",
                "nocancelspace",
                "solutionsreseteqcounter",
                "nosolutionsreseteqcounter",
                "addpoints",
            ]
        } else {
            &[]
        };
        o.unused = given
            .into_iter()
            .filter(|g| !declared.contains(&g.as_str()) && !exam_own.contains(&g.as_str()))
            .collect();
        o
    }

    /// Parse a beamer `\documentclass[...]` option list.
    ///
    /// beamer declares its own option set: the `aspectratio=<n>` key
    /// (resolved once into [`ClassOptions::beamer_paper`]), its font-size
    /// options, the presentation modes, and a handful of layout keys. The
    /// page frame never depends on any of them except `aspectratio`:
    /// beamer ignores the standard paper/size/side options (they are not
    /// declared, so `a4paper`, `twoside`, `landscape`, … land in `unused`
    /// exactly like pdflatex's `Unused global option(s)`), and the body
    /// size stays 11pt (beamer's own size files for 8/9/14/17/20pt are not
    /// modelled; those options are accepted but change nothing).
    fn parse_beamer(kind: ClassKind, options: &str) -> ClassOptions {
        let given: Vec<String> = options
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut o = ClassOptions {
            kind,
            size: BaseSize::Pt11,
            // Unused: beamer's paper comes from `aspectratio`
            // (`beamer_paper` below), never from the standard paper keys.
            paper: Paper::Letter,
            landscape: false,
            twoside: false,
            twocolumn: false,
            titlepage: false,
            openright: false,
            fleqn: false,
            leqno: false,
            draft: false,
            openbib: false,
            bcor: Sp::ZERO,
            div: DivSpec::Default,
            parskip: Parskip::False,
            head_include: false,
            foot_include: false,
            marginpar_include: false,
            pagesize_pdf: true,
            beamer_paper: Some(beamer_paper_size(&given)),
            given: given.clone(),
            unused: Vec::new(),
        };
        for g in &given {
            if Self::apply_beamer_option(&mut o, g) {
                continue;
            }
            o.unused.push(g.clone());
        }
        o
    }

    /// One beamer option. Returns false when the option is unknown (→
    /// `unused`). `aspectratio` (bare or `=value`) is always accepted here;
    /// its dimensions were already resolved into `beamer_paper`.
    fn apply_beamer_option(o: &mut ClassOptions, g: &str) -> bool {
        // The `aspectratio` key in any spelling this parser resolves.
        if g == "aspectratio" {
            return true;
        }
        if let Some((key, _)) = g.split_once('=') {
            if key.trim() == "aspectratio" {
                return true;
            }
        }
        match g {
            "8pt" | "9pt" | "11pt" => {}
            // Accepted (beamer's own size files), but the body metrics stay
            // 11pt: only the default size's geometry is modelled.
            "10pt" | "12pt" => {}
            "14pt" | "17pt" | "20pt" => {}
            "draft" => o.draft = true,
            "final" => o.draft = false,
            // Presentation modes.
            "presentation" | "handout" | "trans" | "article" | "book" => {}
            // Note handling.
            "notes" | "notes=show" | "notes=hide" | "notes=only" => {}
            "compress" => {}
            // Vertical alignment of frames.
            "t" | "c" | "b" => {}
            "leqno" => o.leqno = true,
            "fleqn" => o.fleqn = true,
            "envcountsec" | "notheorems" | "noamsthm" => {}
            // Passed through to the hyperref / xcolor packages.
            "hyperref" | "xcolor" => {}
            _ => {
                if let Some((key, _)) = g.split_once('=') {
                    match key.trim() {
                        "hyperref" | "xcolor" => return true,
                        _ => return false,
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }

    /// Parse a KOMA `\documentclass[...]` option list. Unlike the standard
    /// classes, KOMA processes options in *given* order (later wins) and
    /// defaults to 11pt on A4, oneside (`scrbook`: twoside, title page,
    /// `openright`). Legacy names (`a4paper`, `10pt`, `DIV12`, `BCOR5mm`)
    /// and `key=value` (`paper=`, `fontsize=`, `DIV=`, `BCOR=`,
    /// `headinclude=` …) both work. Options KOMA declares but that do not
    /// move the page frame (`version`, `headings`, `captions`, …) are
    /// accepted silently; anything else lands in `unused` exactly like
    /// pdflatex's `Unused global option(s)`.
    fn parse_koma(kind: ClassKind, options: &str) -> ClassOptions {
        let given: Vec<String> = options
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut o = ClassOptions {
            kind,
            size: BaseSize::Pt11,
            paper: Paper::A4,
            landscape: false,
            twoside: kind == ClassKind::Scrbook,
            twocolumn: false,
            titlepage: kind != ClassKind::Scrartcl,
            openright: kind == ClassKind::Scrbook,
            fleqn: false,
            leqno: false,
            draft: false,
            openbib: false,
            bcor: Sp::ZERO,
            div: DivSpec::Default,
            parskip: Parskip::False,
            head_include: false,
            foot_include: false,
            marginpar_include: false,
            pagesize_pdf: true,
            beamer_paper: None,
            given: Vec::new(),
            unused: Vec::new(),
        };
        for g in given {
            if !Self::apply_koma_option(&mut o, &g) {
                o.unused.push(g.clone());
            }
            o.given.push(g);
        }
        o
    }

    /// One KOMA option. Returns false when the option is unknown (→
    /// `unused`).
    fn apply_koma_option(o: &mut ClassOptions, g: &str) -> bool {
        // Legacy names first (also the bare `key` form of boolean keys).
        match g {
            "a4paper" => o.paper = Paper::A4,
            "a5paper" => o.paper = Paper::A5,
            "b5paper" => o.paper = Paper::B5,
            "letterpaper" => o.paper = Paper::Letter,
            "legalpaper" => o.paper = Paper::Legal,
            "executivepaper" => o.paper = Paper::Executive,
            "landscape" => o.landscape = true,
            "portrait" => o.landscape = false,
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
            "headinclude" => o.head_include = true,
            "footinclude" => o.foot_include = true,
            "mpinclude" => o.marginpar_include = true,
            "pagesize" => o.pagesize_pdf = true,
            // Bare `parskip` takes the key default, `true` (= `full`).
            "parskip" => o.parskip = Parskip::Full,
            // Deprecated spellings (`\KOMA@DeclareDeprecatedOption`):
            // `parskip-`/`parskip+`/`parskip*` mean `full-`/`full+`/`full*`,
            // `halfparskip…` the `half…` variants, `parindent` is
            // `parskip=false`.
            "parskip-" | "parskip+" | "parskip*" => o.parskip = Parskip::Full,
            "halfparskip" | "halfparskip-" | "halfparskip+" | "halfparskip*" => {
                o.parskip = Parskip::Half
            }
            "parindent" => o.parskip = Parskip::False,
            "DIVcalc" => o.div = DivSpec::Calc,
            "DIVclassic" => o.div = DivSpec::Classic,
            // Bare `DIV` takes the key default, `calc`.
            "DIV" => o.div = DivSpec::Calc,
            _ => {
                // `key=value` before the legacy prefixes: `DIV=12` must
                // not parse as legacy `DIV` + `=12`.
                if let Some((key, value)) = g.split_once('=') {
                    return Self::apply_koma_key(o, key.trim(), value.trim());
                }
                if let Some(n) = g.strip_prefix("DIV") {
                    // Deprecated `DIV12`: an explicit number.
                    return match n.parse::<u32>() {
                        Ok(v) => {
                            o.div = Self::div_num(v);
                            true
                        }
                        Err(_) => false,
                    };
                }
                if let Some(d) = g.strip_prefix("BCOR") {
                    // Deprecated `BCOR5mm`: a bare dimension.
                    return match Sp::parse(d) {
                        Some(v) => {
                            o.bcor = v;
                            true
                        }
                        None => false,
                    };
                }
                return false;
            }
        }
        true
    }

    /// One KOMA `key=value` option. Returns false when the key or value is
    /// unknown (→ `unused`).
    fn apply_koma_key(o: &mut ClassOptions, key: &str, value: &str) -> bool {
        match key {
            "paper" => match value {
                "a4" => o.paper = Paper::A4,
                "a5" => o.paper = Paper::A5,
                "b5" => o.paper = Paper::B5,
                "letter" => o.paper = Paper::Letter,
                "legal" => o.paper = Paper::Legal,
                "executive" => o.paper = Paper::Executive,
                "landscape" | "seascape" => o.landscape = true,
                "portrait" => o.landscape = false,
                _ => return false,
            },
            "fontsize" => {
                let v = value.strip_suffix("pt").unwrap_or(value);
                match v {
                    "10" => o.size = BaseSize::Pt10,
                    "11" => o.size = BaseSize::Pt11,
                    "12" => o.size = BaseSize::Pt12,
                    _ => return false,
                }
            }
            "DIV" => match value {
                "default" | "current" | "last" => o.div = DivSpec::Default,
                "calc" => o.div = DivSpec::Calc,
                "classic" => o.div = DivSpec::Classic,
                _ => {
                    if value.starts_with('-') {
                        return false;
                    }
                    match value.parse::<u32>() {
                        Ok(v) => o.div = Self::div_num(v),
                        Err(_) => return false,
                    }
                }
            },
            "BCOR" => match Sp::parse(value) {
                Some(v) => o.bcor = v,
                None => return false,
            },
            "headinclude" => match Self::koma_bool(value) {
                Some(v) => o.head_include = v,
                None => return false,
            },
            "footinclude" => match Self::koma_bool(value) {
                Some(v) => o.foot_include = v,
                None => return false,
            },
            "mpinclude" => match Self::koma_bool(value) {
                Some(v) => o.marginpar_include = v,
                None => return false,
            },
            "twoside" => match value {
                // `semi` keeps one-sided margins (`\@twosidefalse`), so for
                // the page frame it is `oneside`.
                "semi" => o.twoside = false,
                _ => match Self::koma_bool(value) {
                    Some(v) => o.twoside = v,
                    None => return false,
                },
            },
            "twocolumn" => match Self::koma_bool(value) {
                Some(v) => o.twocolumn = v,
                None => return false,
            },
            "titlepage" => match Self::koma_bool(value) {
                Some(v) => o.titlepage = v,
                None => return false,
            },
            "draft" => match Self::koma_bool(value) {
                Some(v) => o.draft = v,
                None => return false,
            },
            "open" => match value {
                "right" => o.openright = true,
                "any" | "left" => o.openright = false,
                _ => return false,
            },
            "pagesize" => match value {
                "false" | "no" | "off" => o.pagesize_pdf = false,
                _ => o.pagesize_pdf = true,
            },
            // `parskip` (`scrkernel-paragraphs.dtx`): `false` (the default)
            // and `never` keep the 1em indent; each `half…` spelling zeroes
            // it with half-line separation, each `full…` with full-line.
            // `relative` / `absolute` only switch the `\selectfont`
            // re-evaluation, so they are accepted without moving anything.
            "parskip" => match value {
                "false" | "off" | "no" => o.parskip = Parskip::False,
                "never" => o.parskip = Parskip::Never,
                "full-" | "full" | "true" | "on" | "yes" | "full+" | "full*" => {
                    o.parskip = Parskip::Full
                }
                "half-" | "half" | "half+" | "half*" => o.parskip = Parskip::Half,
                "relative" | "absolute" => {}
                _ => return false,
            },
            // Declared KOMA keys that never move the page frame.
            "version" | "numbers" | "headings" | "captions" | "toc" | "abstract" | "bibliography"
            | "index" | "listof" | "cleardoublepage" | "chapterprefix" | "appendixprefix"
            | "footnotes" => {}
            _ => return false,
        }
        true
    }

    /// `DIV=n` with typearea's own sentinel decoding (0 is `default`,
    /// 1–2 calculate, 3 is classic).
    fn div_num(n: u32) -> DivSpec {
        match n {
            0 => DivSpec::Default,
            1 | 2 => DivSpec::Calc,
            3 => DivSpec::Classic,
            _ => DivSpec::Num(n),
        }
    }

    fn koma_bool(value: &str) -> Option<bool> {
        match value {
            "true" | "on" | "yes" => Some(true),
            "false" | "off" | "no" => Some(false),
            _ => None,
        }
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
    if o.kind == ClassKind::Beamer {
        return beamer_params(o);
    }
    if o.kind == ClassKind::Letter {
        return letter_params(o);
    }
    if o.kind == ClassKind::Exam {
        return exam_params(o);
    }
    if o.kind.is_koma() {
        return koma_params(o);
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

/// KOMA `typearea` (v3.49.2 `typearea.sty`, `\@typearea`): the page is split
/// into DIV columns/rows after subtracting the binding correction; the
/// text block keeps DIV−3 blocks horizontally (margins 1.5 + 1.5 oneside,
/// 1 + 2 twoside) and the height rounds *up* to whole lines in what is
/// left of the page after top (1 block) and bottom (2 blocks).
///
/// Every branch below was validated to the scaled point against live
/// pdflatex (`\number` probes, TeX Live 2026): default DIV on A4 at
/// 10/11/12pt (8/10/12), explicit `DIV=4…15`, `DIV=calc` on A4 and letter,
/// `DIV=classic`, `BCOR`, `twoside`, `mpinclude`, `headinclude`,
/// `footinclude`, `pagesize=false`, A4/A5/B5/letter/legal/executive and
/// landscape. `DIV=calc` measures the "good line width" from the Computer
/// Modern alphabet widths hardcoded below (typearea's `\ta@temp@goodwidth`
/// with the live font); with another body font pdflatex picks a different
/// DIV, which this model cannot see.
pub fn koma_params(o: &ClassOptions) -> PageParams {
    let (pw, ph) = koma_paper_size(o.paper, o.landscape);
    let fm = body_font(o.size);
    let inch = len("1in");
    let baselineskip = match o.size {
        BaseSize::Pt10 => len("12pt"),
        BaseSize::Pt11 => len("13.6pt"),
        BaseSize::Pt12 => len("14.5pt"),
    };
    let topskip = match o.size {
        BaseSize::Pt10 => Sp::pt(10),
        BaseSize::Pt11 => Sp::pt(11),
        BaseSize::Pt12 => Sp::pt(12),
    };
    // `\typearea`: `\headheight=1.25\baselineskip`,
    // `\headsep=1.5\baselineskip`, `\footheight=1.25\baselineskip`,
    // `\footskip=\footheight+2.25\baselineskip`, `\marginparsep=1cc`,
    // `\marginparpush=0.45\baselineskip`.
    let headheight = baselineskip.scaled("1.25").unwrap();
    let headsep = baselineskip.scaled("1.5").unwrap();
    let footskip = headheight + baselineskip.scaled("2.25").unwrap();
    let marginparsep = len("1cc");
    let marginparpush = baselineskip.scaled("0.45").unwrap();
    let div = koma_div(o, pw, ph, headheight, headsep, footskip);

    let hblk = (pw - o.bcor).over(div);
    let vblk = ph.over(div);
    // `\@typearea` margins. `1.5\ta@hblk` is `hblk + hblk/2`, truncated,
    // exactly like TeX's factor scan (`Sp::scaled`).
    let half3 = hblk.scaled("1.5").unwrap();
    let marginparwidth = if o.marginpar_include {
        hblk - marginparsep
    } else if o.twoside {
        half3
    } else {
        hblk
    };
    let oddsidemargin =
        -inch + o.bcor + if o.twoside { hblk } else { half3 };
    let evensidemargin = if o.twoside {
        let mut e = -inch + hblk.times(2);
        if o.marginpar_include {
            e = e + marginparwidth + marginparsep;
        }
        e
    } else {
        oddsidemargin
    };
    let mut textwidth = pw - o.bcor - hblk.times(3);
    if o.marginpar_include {
        textwidth = textwidth - marginparwidth - marginparsep;
    }
    let mut topmargin = -inch + vblk;
    if !o.head_include {
        topmargin = topmargin - headheight - headsep;
    }
    let mut room = ph - vblk.times(3);
    if o.head_include {
        room = room - headheight - headsep;
    }
    if o.foot_include {
        room = room - footskip;
    }
    // `\@whiledim\textheight<\ta@temp`: rounds UP to whole lines, unlike
    // the standard classes, which round down.
    let mut textheight = topskip;
    while textheight < room {
        textheight = textheight + baselineskip;
    }

    let maxdepth = topskip.over(2);
    let (footnotesep, skip_footins) = match o.size {
        BaseSize::Pt10 => (len("6.65pt"), Glue::new("9pt", "4pt", "2pt")),
        BaseSize::Pt11 => (len("7.7pt"), Glue::new("10pt", "4pt", "2pt")),
        BaseSize::Pt12 => (len("8.4pt"), Glue::new("10.8pt", "4pt", "2pt")),
    };
    let leftmargini = fm
        .em
        .scaled(if o.twocolumn { "2" } else { "2.5" })
        .unwrap();
    let labelsep = fm.em.scaled("0.5").unwrap();
    // `parskip` (`scrkernel-paragraphs.dtx`): the separation scales with
    // the body `\baselineskip` (re-evaluated at every `\selectfont`, so it
    // always tracks the final size whatever the option order).
    let (parindent, parskip) = match o.parskip {
        Parskip::False => (fm.em, Glue::new("0pt", "1pt", "0pt")),
        Parskip::Never => (fm.em, Glue::fixed(Sp::ZERO)),
        Parskip::Half => {
            let half = baselineskip.scaled("0.5").unwrap();
            (
                Sp::ZERO,
                Glue {
                    natural: half,
                    stretch: half,
                    shrink: Sp::ZERO,
                },
            )
        }
        Parskip::Full => (
            Sp::ZERO,
            Glue {
                natural: baselineskip,
                stretch: baselineskip.scaled("0.1").unwrap(),
                shrink: Sp::ZERO,
            },
        ),
    };

    PageParams {
        paperwidth: pw,
        paperheight: ph,
        textwidth,
        textheight,
        oddsidemargin,
        evensidemargin,
        topmargin,
        headheight,
        headsep,
        footskip,
        topskip,
        baselineskip,
        parindent,
        parskip,
        marginparwidth,
        marginparsep,
        marginparpush,
        columnsep: Sp::pt(10),
        columnseprule: Sp::ZERO,
        maxdepth,
        footnotesep,
        skip_footins,
        overfullrule: if o.draft { Sp::pt(5) } else { Sp::ZERO },
        leftmargini,
        labelsep,
        mathindent: if o.fleqn { Some(leftmargini) } else { None },
        hoffset: Sp::ZERO,
        voffset: Sp::ZERO,
    }
}

/// typearea's own paper sizes as `\number\paperwidth` / `\number\paperheight`
/// read them out of pdflatex (TeX Live 2026): letter is 612bp by 792bp and
/// A4/A5/B5 come from the ISO halving chain, so plain `8.5in` / `210mm`
/// conversions are a few sp off. `landscape` swaps, like the paper key.
fn koma_paper_size(paper: Paper, landscape: bool) -> (Sp, Sp) {
    let (w, h) = match paper {
        Paper::A4 => (Sp(39_158_280), Sp(55_380_996)),
        Paper::A5 => (Sp(27_597_264), Sp(39_158_280)),
        Paper::B5 => (Sp(32_818_368), Sp(46_617_000)),
        Paper::Letter => (Sp(40_258_437), Sp(52_099_153)),
        Paper::Legal => (Sp(40_258_437), Sp(66_308_014)),
        Paper::Executive => (Sp(34_338_078), Sp(49_731_010)),
    };
    if landscape { (h, w) } else { (w, h) }
}

/// Resolve the DIV: the A4 default table, an explicit number, `classic`,
/// or the good-line-width calculation.
fn koma_div(o: &ClassOptions, pw: Sp, ph: Sp, hh: Sp, hs: Sp, fs: Sp) -> i64 {
    match o.div {
        DivSpec::Num(n) if n >= 4 => n as i64,
        DivSpec::Num(3) | DivSpec::Classic => koma_classic(o, pw, ph, hh, hs, fs),
        DivSpec::Num(1) | DivSpec::Num(2) | DivSpec::Calc => {
            koma_calc(o, pw, ph, hh, hs, fs)
        }
        // `DIV=0` is another spelling of `default` (both leave `\ta@div`
        // at zero, which takes the `\ta@divfor` table path).
        DivSpec::Num(_) | DivSpec::Default => {
            if !o.landscape && o.paper == Paper::A4 {
                // `\ta@divlist` for (almost) A4.
                match o.size {
                    BaseSize::Pt10 => 8,
                    BaseSize::Pt11 => 10,
                    BaseSize::Pt12 => 12,
                }
            } else {
                koma_calc(o, pw, ph, hh, hs, fs)
            }
        }
    }
}

/// `DIV=classic`: fit the ISO-proportioned block, else fall back to `calc`.
fn koma_classic(o: &ClassOptions, pw: Sp, ph: Sp, hh: Sp, hs: Sp, fs: Sp) -> i64 {
    let mut temp = pw - o.bcor;
    if !o.head_include {
        temp = temp + hh + hs;
    }
    if !o.foot_include {
        temp = temp + fs;
    }
    if temp > ph {
        koma_calc(o, pw, ph, hh, hs, fs)
    } else {
        koma_modiv(ph, (ph - pw + o.bcor).over(3))
    }
}

/// `DIV=calc` (`\ta@temp@goodwidth` + `\ta@modiv`): DIV from the good line
/// width of the Computer Modern body font. The alphabet widths are live
/// `\settowidth` measurements (sp) at 10/11/12pt normalsize, TeX Live 2026.
fn koma_calc(o: &ClassOptions, pw: Sp, ph: Sp, hh: Sp, hs: Sp, fs: Sp) -> i64 {
    let (lower, upper) = match o.size {
        BaseSize::Pt10 => (8_361_325, 12_201_554),
        BaseSize::Pt11 => (9_155_641, 13_360_694),
        BaseSize::Pt12 => (9_822_288, 14_332_489),
    };
    let good = if Sp(lower) > Sp::pt(200) {
        Sp(lower).scaled("2.53846").unwrap() + Sp(upper).scaled("0.11538").unwrap()
    } else {
        (Sp(lower).times(66) + Sp(upper).times(3)).over(26)
    };
    let mut temp = good;
    if o.twocolumn {
        temp = temp.times(2) + Sp::pt(10);
    }
    let mut hblk = (pw - temp).over(3);
    if hblk < Sp::ZERO {
        hblk = len("5mm");
    }
    let t = if o.marginpar_include {
        hblk.scaled("0.75").unwrap()
    } else {
        hblk
    };
    let div = koma_modiv(pw, t);
    // When the head runs off the top (`\topmargin < 5mm-1in`), typearea
    // re-derives DIV from the page height instead.
    let inch = len("1in");
    let mut top = -inch + ph.over(div);
    if !o.head_include {
        top = top - hh - hs;
    }
    if top < len("5mm") - inch {
        let mut ht = len("15mm");
        if !o.head_include {
            ht = ht + hh + hs;
        }
        if !o.foot_include {
            ht = ht + fs;
        }
        return koma_modiv(ph, ht.over(3));
    }
    div
}

/// `\ta@modiv{a}{b}`: DIV from the ratio, rounded by comparing neighbours,
/// at least 4. TeX's `\divide<dimen> by<number>` truncates, and assigning
/// the quotient dimen to the `\ta@div` count keeps its sp value — plain
/// truncating integer division on both sides reproduces it exactly.
fn koma_modiv(a: Sp, b: Sp) -> i64 {
    let d = a.0 / b.0;
    if d < 4 {
        return 4;
    }
    let below = a.0 / d;
    let above = a.0 / (d + 1);
    if 2 * d - below < above { d } else { d + 1 }
}

/// beamer's paper size from the `aspectratio` class option.
///
/// beamer.cls (`\DeclareOptionBeamer{aspectratio}[43]`) maps each value to
/// a fixed slide size; anything else (including a bare `aspectratio`, which
/// means 43) keeps the 4:3 default. Later options win, like keyval keys.
///
/// | `aspectratio` | ratio | paper (mm) |
/// |---|---|---|
/// | 32 | 3:2 | 135 × 90 |
/// | 43 (default) | 4:3 | 128 × 96 |
/// | 54 | 5:4 | 125 × 100 |
/// | 141 | √2:1 (ISO A-paper ratio) | 148.5 × 105 |
/// | 149 | 14:9 | 140 × 90 |
/// | 169 | 16:9 | 160 × 90 |
/// | 1610 | 16:10 | 160 × 100 |
///
/// Provenance: the value set {32, 43, 54, 141, 149, 169, 1610} is beamer's
/// documented set (beamer user guide; pandoc's beamer docs list the same
/// seven). The 1610/169/149/54 dimensions are quoted `beamer.cls`
/// fragments (`16.00cm`×`10.00cm`, `16.00cm`×`9.00cm`,
/// `14.00cm`×`9.00cm`, `12.50cm`×`10.00cm`). The 43 default
/// (128mm × 96mm), the 32 size (135mm × 90mm, exactly 3:2) and the 141
/// size (148.5mm × 105mm, the ISO √2 ratio — NOT round centimetres) were
/// measured against real pdflatex output (`\the\paperwidth` /
/// `\the\paperheight`, TeX Live 2026; see CHECKIN.md).
pub fn beamer_paper_size(given: &[String]) -> (Sp, Sp) {
    let mut aspect: Option<u32> = None;
    for g in given {
        let g = g.trim();
        if g == "aspectratio" {
            aspect = Some(43);
            continue;
        }
        if let Some((key, value)) = g.split_once('=') {
            if key.trim() == "aspectratio" {
                aspect = value.trim().parse::<u32>().ok();
            }
        }
    }
    let (w, h) = match aspect {
        Some(32) => ("135mm", "90mm"),
        Some(54) => ("125mm", "100mm"),
        Some(141) => ("148.5mm", "105mm"),
        Some(149) => ("140mm", "90mm"),
        Some(169) => ("160mm", "90mm"),
        Some(1610) => ("160mm", "100mm"),
        // 43, bare `aspectratio`, absent, non-numeric and unlisted numbers:
        // beamer's `\ifnum` chain matches nothing, so the default stands.
        _ => ("128mm", "96mm"),
    };
    (len(w), len(h))
}

/// `beamer.cls` page geometry (presentation mode).
///
/// Measured against real pdflatex output (`\the\<dimen>` readings, TeX
/// Live 2026; see CHECKIN.md): the paper is 128mm × 96mm 4:3 by default
/// ([`beamer_paper_size`] for the `aspectratio` variants); the side
/// margins are 1cm each (`\textwidth` = paper − 2cm,
/// `\oddsidemargin` = `\evensidemargin` = 1cm − 1in, kept exact like
/// `letter.cls`'s own unrounded side margin); and the vertical margins are
/// effectively zero — slides are full-bleed top-to-bottom. `\topmargin`
/// is −1in exactly, cancelling TeX's 1in vertical origin so the text top
/// sits at the paper's top edge (`1in + \topmargin + \headheight +
/// \headsep` = 0); `\headheight` and `\headsep` are 0pt; and
/// `\paperheight − \textheight` is exactly the 4pt `\footskip` (the folio
/// reservation), so `\textheight` = paper − 4pt with no separate bottom
/// margin. `\marginparwidth` is 4pt. The body size is 11pt
/// (`\baselineskip` 13.6pt, i.e. `size11.clo`'s `\normalsize`).
///
/// Everything else follows the 11pt article conventions (`size11.clo`):
/// `\topskip` 11pt, `\maxdepth` half that, `\footnotesep` 7.7pt,
/// `\skip\footins` 10pt plus 4pt minus 2pt, `\columnsep` 10pt,
/// `\labelsep` .5em, and the draft `\overfullrule`. beamer's own settings
/// (measured `\the` readings inside a frame, TeX Live 2026): `\parindent`
/// 0pt and `\parskip` 0pt (`beamer.cls` `\parskip=0pt`; the gaps between
/// paragraphs on a slide are only the list `\topsep`s), `\leftmargini` 2em
/// (`beamerbaselocalstructure.sty` line 144: 21.90005pt; the level-1 list
/// skips are in [`crate::beamer::list_level`]).
fn beamer_params(o: &ClassOptions) -> PageParams {
    let (pw, ph) = o.beamer_paper.unwrap_or((len("128mm"), len("96mm")));
    let fm = body_font(BaseSize::Pt11);
    let cm = len("1cm");
    let inch = len("1in");
    let sidemargin = cm - inch;
    // `2cm` parsed once, like TeX's own `{-2cm}` length scan (doubling the
    // already-rounded `1cm` is 1sp off for some papers).
    let textwidth = pw - len("2cm");
    // The whole paper-minus-text gap is the 4pt footskip reservation.
    let footskip = len("4pt");
    let textheight = ph - footskip;
    let leftmargini = fm.em.scaled("2").unwrap();
    PageParams {
        paperwidth: pw,
        paperheight: ph,
        textwidth,
        textheight,
        oddsidemargin: sidemargin,
        evensidemargin: sidemargin,
        // Exactly −1in: cancels TeX's 1in vertical origin so the text top
        // sits at the paper's top edge (measured `\topmargin` −72.26999pt).
        topmargin: Sp::ZERO - inch,
        headheight: Sp::ZERO,
        headsep: Sp::ZERO,
        footskip,
        topskip: Sp::pt(11),
        baselineskip: len("13.6pt"),
        parindent: Sp::ZERO,
        parskip: Glue::fixed(Sp::ZERO),
        marginparwidth: len("4pt"),
        marginparsep: Sp::pt(10),
        marginparpush: Sp::pt(5),
        columnsep: Sp::pt(10),
        columnseprule: Sp::ZERO,
        maxdepth: Sp::pt(11).scaled(".5").unwrap(),
        footnotesep: len("7.7pt"),
        skip_footins: Glue::new("10pt", "4pt", "2pt"),
        overfullrule: if o.draft { Sp::pt(5) } else { Sp::ZERO },
        leftmargini,
        labelsep: fm.em.scaled(".5").unwrap(),
        mathindent: if o.fleqn { Some(leftmargini) } else { None },
        hoffset: Sp::ZERO,
        voffset: Sp::ZERO,
    }
}

/// `exam.cls`: article's parameters (it `\LoadClass`es article with the
/// size/paper/side options), then its own frame, lines 747-760:
/// `\textwidth = \paperwidth - 2in`, both side margins 0pt,
/// `\headheight 15pt`, `\headsep 15pt`, `\topmargin = -\headheight
/// -\headsep`, `\footskip 29pt`, `\textheight = \paperheight - 2.2in`
/// (not whole lines), `\marginparwidth .5in`, `\marginparsep 5pt`.
/// Measured with pdflatex (letter paper, 10/11/12pt alike): textwidth
/// 469.755pt, textheight 635.97621pt, topmargin -30pt.
fn exam_params(o: &ClassOptions) -> PageParams {
    let article = ClassOptions {
        kind: ClassKind::Article,
        ..o.clone()
    };
    let mut p = class_params(&article);
    let (pw, ph) = o.paper_size();
    p.textwidth = pw - len("2in");
    p.oddsidemargin = Sp::ZERO;
    p.evensidemargin = Sp::ZERO;
    p.headheight = Sp::pt(15);
    p.headsep = Sp::pt(15);
    p.topmargin = -(p.headheight + p.headsep);
    p.footskip = Sp::pt(29);
    p.textheight = ph - len("2.2in");
    p.marginparwidth = len(".5in");
    p.marginparsep = Sp::pt(5);
    p
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
