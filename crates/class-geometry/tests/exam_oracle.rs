//! `exam.cls` page geometry, page styles and head/foot contents, pinned
//! against pdflatex.
//!
//! pdflatex is an ORACLE ONLY: it never runs here. Every length below is a
//! `\the<length>` printed after `\begin{document}` by pdfTeX
//! 3.141592653-2.6-1.40.29 (TeX Live 2026, exam.cls from that
//! distribution), committed as a literal.

use flashtex_class_geometry::*;

/// (class options, [(length, pdflatex value)]).
const FIXTURES: &[(&str, &[(&str, &str)])] = &[
    (
        "10pt",
        &[
            ("textwidth", "469.755pt"),
            ("textheight", "635.97621pt"),
            ("topmargin", "-30.0pt"),
            ("topskip", "10.0pt"),
            ("maxdepth", "5.0pt"),
            ("parindent", "15.0pt"),
            ("baselineskip", "12.0pt"),
        ],
    ),
    (
        "11pt",
        &[
            ("textwidth", "469.755pt"),
            ("textheight", "635.97621pt"),
            ("topskip", "11.0pt"),
            ("maxdepth", "5.5pt"),
            ("parindent", "17.0pt"),
            ("baselineskip", "13.6pt"),
        ],
    ),
    (
        "12pt",
        &[
            ("textwidth", "469.755pt"),
            ("textheight", "635.97621pt"),
            ("parindent", "17.62482pt"),
            ("baselineskip", "14.5pt"),
        ],
    ),
    (
        "a4paper,12pt",
        &[
            ("paperwidth", "597.50787pt"),
            ("paperheight", "845.04684pt"),
            ("textwidth", "452.96788pt"),
            ("textheight", "686.05307pt"),
        ],
    ),
    ("twoside", &[("oddsidemargin", "0.0pt"), ("evensidemargin", "0.0pt")]),
];

/// Lengths every fixture shares (exam.cls lines 747-760).
const COMMON: &[(&str, &str)] = &[
    ("oddsidemargin", "0.0pt"),
    ("evensidemargin", "0.0pt"),
    ("topmargin", "-30.0pt"),
    ("headheight", "15.0pt"),
    ("headsep", "15.0pt"),
    ("footskip", "29.0pt"),
    ("marginparwidth", "36.135pt"),
    ("marginparsep", "5.0pt"),
];

fn get(p: &PageParams, name: &str) -> Sp {
    match name {
        "paperwidth" => p.paperwidth,
        "paperheight" => p.paperheight,
        "textwidth" => p.textwidth,
        "textheight" => p.textheight,
        "oddsidemargin" => p.oddsidemargin,
        "evensidemargin" => p.evensidemargin,
        "topmargin" => p.topmargin,
        "headheight" => p.headheight,
        "headsep" => p.headsep,
        "footskip" => p.footskip,
        "topskip" => p.topskip,
        "maxdepth" => p.maxdepth,
        "parindent" => p.parindent,
        "baselineskip" => p.baselineskip,
        "marginparwidth" => p.marginparwidth,
        "marginparsep" => p.marginparsep,
        other => panic!("unknown length {other}"),
    }
}

#[test]
fn exam_frame_matches_pdflatex_to_the_scaled_point() {
    for (options, dims) in FIXTURES {
        let doc = resolve(&DocumentSetup::new(ClassKind::Exam, options));
        for (name, want) in COMMON.iter().chain(dims.iter()) {
            let want = Sp::parse(want).unwrap();
            assert_eq!(get(&doc.params, name), want, "[{options}] \\{name}");
        }
    }
}

#[test]
fn exam_options_are_not_unused_and_article_options_still_apply() {
    let doc = resolve(&DocumentSetup::new(ClassKind::Exam, "addpoints,answers,12pt"));
    assert!(doc.warnings.is_empty(), "{:?}", doc.warnings);
    assert_eq!(doc.options.size, BaseSize::Pt12);
    // `answers` is exam's own; in article it is an unused option.
    let article = resolve(&DocumentSetup::new(ClassKind::Article, "answers"));
    assert_eq!(article.warnings, ["Unused global option(s): [answers]"]);
}

#[test]
fn exam_is_recognised_and_defaults_to_headandfoot() {
    let setup = DocumentSetup::from_preamble("\\documentclass{exam}\n\\begin{document}x\\end{document}").unwrap();
    assert_eq!(setup.class, ClassKind::Exam);
    let doc = resolve(&setup);
    assert_eq!(doc.pagestyle, PageStyle::HeadAndFoot);
    let exam = doc.exam.as_ref().unwrap();
    // exam.cls line 1455 `\cfoot[]{Page \thepage}`: nothing on page 1,
    // "Page N" centred on every later page (pdflatex: page 1 has no foot,
    // page 2's foot reads `Page 2`).
    let (_, foot) = doc.head_foot(1);
    assert_eq!(exam.slot(foot.center, 1), Some(""));
    assert_eq!(exam.slot(foot.center, 2), Some("Page \\thepage"));
    assert!(!exam.rule(true, 2) && !exam.rule(false, 2));
}

#[test]
fn exam_head_commands_and_user_macros_are_read_from_the_preamble() {
    // jez/latex-hw-template's preamble, abridged.
    let src = "\\documentclass[11pt]{exam}\n\\newcommand{\\myname}{Jacob Zimmerman}\n\\def\\myclass{12-345}\n\
               \\newcommand{\\withargs}[1]{#1}\n\\pagestyle{head}\n\\headrule\n\
               \\header{\\textbf{\\myclass}}%\n{\\textbf{\\myname}}%\n{\\textbf{Homework}}\n\
               \\lfoot[first]{running}\n\\begin{document}x\\end{document}";
    let doc = resolve(&DocumentSetup::from_preamble(src).unwrap());
    assert_eq!(doc.pagestyle, PageStyle::Head);
    let exam = doc.exam.as_ref().unwrap();
    assert_eq!(exam.first_head, ["\\textbf{\\myclass}", "\\textbf{\\myname}", "\\textbf{Homework}"]);
    assert_eq!(exam.run_head, exam.first_head);
    assert!(exam.first_headrule && exam.run_headrule);
    assert_eq!((exam.first_foot[0].as_str(), exam.run_foot[0].as_str()), ("first", "running"));
    assert!(exam.macros.contains(&("myname".into(), "Jacob Zimmerman".into())));
    assert!(exam.macros.contains(&("myclass".into(), "12-345".into())));
    assert!(!exam.macros.iter().any(|(n, _)| n == "withargs"));
    // `\pagestyle{head}` sets no foot at all.
    let (head, foot) = doc.head_foot(2);
    assert!(foot.is_empty());
    assert_eq!(head.center, Field::ExamHead(1));
}

#[test]
fn exam_page_styles_are_undefined_in_other_classes() {
    let src = "\\documentclass{article}\n\\pagestyle{head}\n\\header{a}{b}{c}\n\\begin{document}x\\end{document}";
    let doc = resolve(&DocumentSetup::from_preamble(src).unwrap());
    assert_eq!(doc.pagestyle, PageStyle::Plain);
    assert!(doc.exam.is_none());
}
