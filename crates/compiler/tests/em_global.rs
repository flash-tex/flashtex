//! Regression coverage for font-relative dimensions and parser-owned globals.
//!
//! Every expected length below was measured with TeX Live 2026's
//! `/Library/TeX/texbin/pdflatex` (`\showthe` of a `\newlength` register),
//! not derived: TeX's `em` and `ex` are the current font's `\fontdimen6` and
//! `\fontdimen5`, which differ from the point size for Computer Modern's
//! optical designs (cmr12's quad is 11.74988pt).
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::{self, Font, LayoutConstraints};
use flashtex_compiler::parser::{parse, Block, Inline, SourceDocument};

fn document(class: &str, body: &str) -> String {
    format!("\\documentclass[{class}pt]{{article}}\\begin{{document}}{body}\\end{{document}}")
}

fn hspace(source: &str) -> (f64, f64, f64) {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::HSpace {
                    pt,
                    space_before_pt,
                    space_after_pt,
                    ..
                } => Some((*pt, *space_before_pt, *space_after_pt)),
                _ => None,
            }),
            _ => None,
        })
        .expect("the document has no hspace")
}

fn vspace(source: &str) -> f64 {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::VSpace { pt, .. } => Some(*pt),
            _ => None,
        })
        .expect("the document has no vspace")
}

fn paragraph_text(source: &str) -> String {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    paragraph_text_from(&parsed)
}

fn paragraph_text_from(parsed: &flashtex_compiler::parser::Parsed) -> String {
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(
                inlines
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .expect("the document has no paragraph")
}

/// A pdflatex `\showthe` value (`"9.24994pt"`) in points.
fn pt(showthe: &str) -> f64 {
    showthe.trim_end_matches("pt").parse().expect("a length")
}

/// pdflatex `\showthe` of `\setlength\x{1em}` and `{1ex}` under each
/// declaration, per `\documentclass[<class>pt]{article}`.
const SHOWTHE: &[(&str, &[(&str, &str, &str)])] = &[
    (
        "10",
        &[
            ("small", "9.24994pt", "3.87498pt"),
            ("normalsize", "10.00002pt", "4.30554pt"),
            ("large", "11.74988pt", "5.16667pt"),
            ("bfseries", "11.49994pt", "4.44444pt"),
            ("sffamily", "10.00002pt", "4.44444pt"),
            ("ttfamily", "10.4999pt", "4.30554pt"),
        ],
    ),
    (
        "11",
        &[
            ("small", "10.00002pt", "4.30554pt"),
            ("normalsize", "10.95003pt", "4.71457pt"),
            ("large", "11.74988pt", "5.16667pt"),
            ("bfseries", "12.59242pt", "4.86665pt"),
            ("sffamily", "10.95003pt", "4.86665pt"),
            ("ttfamily", "11.49739pt", "4.71457pt"),
        ],
    ),
    (
        "12",
        &[
            ("small", "10.95003pt", "4.71457pt"),
            ("normalsize", "11.74988pt", "5.16667pt"),
            ("large", "14.09984pt", "6.2pt"),
            ("bfseries", "13.5pt", "5.33331pt"),
            ("sffamily", "11.74988pt", "5.33331pt"),
            ("ttfamily", "12.35pt", "5.16667pt"),
        ],
    ),
];

#[test]
fn engine_registers_use_the_active_fonts_quad_and_x_height() {
    for (class, rows) in SHOWTHE {
        for (declaration, em, ex) in *rows {
            for (unit, expected) in [("em", em), ("ex", ex)] {
                let source = format!(
                    "\\documentclass[{class}pt]{{article}}\\newlength{{\\x}}\\begin{{document}}{{\\{declaration}\\setlength{{\\x}}{{1{unit}}}\\the\\x}}\\end{{document}}"
                );
                assert_eq!(
                    paragraph_text(&source),
                    *expected,
                    "{class}pt \\{declaration} 1{unit}"
                );
            }
        }
    }
}

#[test]
fn parser_dimensions_use_the_active_fonts_quad_and_x_height() {
    for (class, rows) in SHOWTHE {
        for (declaration, em, ex) in *rows {
            for (unit, expected) in [("em", em), ("ex", ex)] {
                let source = document(class, &format!("{{\\{declaration}\\vspace{{1{unit}}}}}"));
                let got = vspace(&source);
                assert!(
                    (got - pt(expected)).abs() < 1e-5,
                    "{class}pt \\{declaration} 1{unit}: {got} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn setlength_em_uses_the_class_fonts_quad() {
    // pdflatex: `\setlength\x{3em}` = 30.00005pt, 32.85008pt, 35.24963pt.
    for (class, expected) in [
        ("10", "30.00005pt"),
        ("11", "32.85008pt"),
        ("12", "35.24963pt"),
    ] {
        let source = format!(
            "\\documentclass[{class}pt]{{article}}\\setlength{{\\parskip}}{{3em}}\\begin{{document}}x\\end{{document}}"
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let got = parsed.parskip_pt.expect("parskip");
        assert!((got - pt(expected)).abs() < 1e-5, "{class}pt: {got}");

        let register = format!(
            "\\documentclass[{class}pt]{{article}}\\newlength{{\\x}}\\setlength{{\\x}}{{3em}}\\begin{{document}}\\the\\x\\end{{document}}"
        );
        assert_eq!(paragraph_text(&register), expected, "{class}pt register");
    }
    // The standard classes default to 10pt.
    assert_eq!(
        paragraph_text(
            r"\documentclass{article}\newlength{\x}\setlength{\x}{3em}\begin{document}\the\x\end{document}"
        ),
        "30.00005pt"
    );
}

#[test]
fn em_follows_shape_argument_commands_and_font_packages() {
    // `lmodern` keeps its "not implemented" warning (the compiler layout still
    // uses Core14 faces), so only the paragraph text is compared.
    let the_x = |preamble: &str, body: &str| {
        paragraph_text_from(&parse(&format!(
            "\\documentclass{{article}}{preamble}\\newlength{{\\x}}\\begin{{document}}{body}\\end{{document}}"
        )))
    };
    // cmti10, and cmbx10 only inside `\textbf`'s group.
    assert_eq!(
        the_x("", r"{\itshape\setlength{\x}{1em}\the\x}"),
        "10.22217pt"
    );
    assert_eq!(
        the_x(
            "",
            r"\textbf{\setlength{\x}{1em}\the\x},\setlength{\x}{1em}\the\x"
        ),
        "11.49994pt,10.00002pt"
    );
    // `\normalfont` keeps the size: cmr9.
    assert_eq!(
        the_x("", r"{\small\bfseries\normalfont\setlength{\x}{1em}\the\x}"),
        "9.24994pt"
    );
    // T1 selects the EC fonts (ecrm1000).
    assert_eq!(
        the_x(r"\usepackage[T1]{fontenc}", r"\setlength{\x}{1em}\the\x"),
        "9.99756pt"
    );
    assert_eq!(
        the_x(r"\usepackage{lmodern}", r"\setlength{\x}{1em}\the\x"),
        "10.0pt"
    );

    // `lmodern` changes the families at `\begin{document}`; the preamble's
    // font is still cmr12 unless a later fontenc `\selectfont` ran.
    let twelve = |preamble: &str| {
        paragraph_text_from(&parse(&format!(
            "\\documentclass[12pt]{{article}}{preamble}\\newlength{{\\x}}\\newlength{{\\y}}\\setlength{{\\x}}{{1em}}\\begin{{document}}\\setlength{{\\y}}{{3em}}\\the\\x,\\the\\y\\end{{document}}"
        )))
    };
    assert_eq!(twelve(r"\usepackage{lmodern}"), "11.74988pt,35.2495pt");
    assert_eq!(
        twelve(r"\usepackage{lmodern}\usepackage[T1]{fontenc}"),
        "11.74983pt,35.2495pt"
    );
    let parskip = |preamble: &str| {
        parse(&format!(
            "\\documentclass[12pt]{{article}}{preamble}\\setlength{{\\parskip}}{{1em}}\\begin{{document}}x\\end{{document}}"
        ))
        .parskip_pt
        .expect("parskip")
    };
    assert!((parskip(r"\usepackage{lmodern}") - 11.74988).abs() < 1e-5);
    assert!((parskip(r"\usepackage{lmodern}\usepackage[T1]{fontenc}") - 11.74983).abs() < 1e-5);
}

#[test]
fn hspace_keeps_the_interword_space_after_it() {
    // pdflatex: 10pt `2em` is 20.00003pt. The interword glue around the
    // command is this layout's own (Core14 Times) space, the same glue every
    // other space on the line gets; pdflatex's cmr10 space is 3.33333pt.
    let body = document("10", "A\\hspace{2em} B");
    let (width, before, after) = hspace(&body);
    assert!((width - 20.00003).abs() < 1e-5, "{width}");
    assert_eq!(before, 0.0);
    assert!((after - layout::word_space(10.0, Font::TimesRoman)).abs() < 1e-9);
    let out = compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: &body,
        }],
        "main.tex",
        LayoutConstraints::default(),
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let items = &out.pages[0].items;
    let a = items.iter().find(|item| item.text == "A").expect("A");
    let b = items.iter().find(|item| item.text == "B").expect("B");
    let expected_delta = layout::text_width("A", 10.0, Font::TimesRoman)
        + width
        + layout::word_space(10.0, Font::TimesRoman);
    assert!((b.x_pt - a.x_pt - expected_delta).abs() < 0.02);
}

#[test]
fn local_length_restores_and_global_length_survives_group_end() {
    for (source, expected) in [
        (r"{\parskip=1pt}", None),
        (r"{\global\parskip=1pt}", Some(1.0)),
        (r"{\setlength{\parskip}{2pt}}", None),
        (r"{\global\setlength{\parskip}{2pt}}", Some(2.0)),
        (
            r"\global\setlength{\parskip}{1pt}{\setlength{\parskip}{5pt}}",
            Some(1.0),
        ),
        (r"{\global\addtolength{\parskip}{3pt}}", Some(3.0)),
    ] {
        let parsed = parse(&format!(
            "\\documentclass{{article}}{source}\\begin{{document}}x\\end{{document}}"
        ));
        assert!(
            parsed.diagnostics.is_empty(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(parsed.parskip_pt, expected, "{source}");
    }
}

#[test]
fn global_setlength_assigns_engine_registers_globally() {
    // pdflatex accepts `\global\setlength`: `\setlength` is a macro
    // (`\def\setlength#1#2{#1 #2\relax}`), so the prefix reaches `\x`.
    assert_eq!(
        paragraph_text(
            r"\documentclass{article}\newlength{\x}\newlength{\y}{\global\setlength{\x}{1pt}\setlength{\y}{1pt}}\begin{document}\the\x,\the\y\end{document}"
        ),
        "1.0pt,0.0pt"
    );
}

#[test]
fn fbox_lengths_accept_global_assignments_in_the_body() {
    // `\fboxsep` is kept by the parser and scoped; `\parskip` in the body is
    // reported like `\setlength{\parskip}` there instead of being unknown.
    let parsed = parse(
        r"\documentclass{article}\begin{document}{\global\fboxsep=1pt \fboxrule=2pt}x\end{document}",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);

    let parsed =
        parse(r"\documentclass{article}\begin{document}\global\parskip=1pt x\end{document}");
    assert_eq!(parsed.parskip_pt, None);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message == "\\parskip assignment is recognised but not implemented here"),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn table_length_assignments_are_accepted_without_compiler_state() {
    // The compiler keeps no `\tabcolsep`/`\arrayrulewidth` value (see
    // `TABLE_LENGTHS`): both forms are accepted and there is nothing to scope.
    for source in [
        r"{\tabcolsep=1pt}",
        r"\global\tabcolsep=2pt",
        r"\arrayrulewidth=1pt",
    ] {
        let parsed = parse(&format!(
            "\\documentclass{{article}}{source}\\begin{{document}}x\\end{{document}}"
        ));
        assert!(
            parsed.diagnostics.is_empty(),
            "{source}: {:?}",
            parsed.diagnostics
        );
    }
}

#[test]
fn expansion_register_global_survives_group_end() {
    let parsed = parse(
        r"\documentclass{article}\newcount\localcount\localcount=1{\advance\localcount by 2}\newcount\globalcount\globalcount=1{\global\advance\globalcount by 2}\begin{document}\the\localcount,\the\globalcount\end{document}",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(paragraph_text_from(&parsed), "1,3");
}

#[test]
fn incremental_expansion_keeps_font_units_across_checkpoints() {
    use flashtex_compiler::expansion::{expand_project, expand_project_with_cache};
    // Large enough for checkpoints; each line restores from one taken inside
    // the previous line's state, so a restored engine must keep the metrics.
    let text = |class: &str, last: &str| {
        let mut s = format!(
            "\\documentclass[{class}pt]{{article}}\n\\newlength{{\\x}}\n\\begin{{document}}\n"
        );
        for i in 0..200 {
            s.push_str(&format!(
                "Line {i} {{\\small\\setlength{{\\x}}{{1em}}\\the\\x}}\n\n"
            ));
        }
        s.push_str(&format!(
            "{{\\large\\setlength{{\\x}}{{1ex}}\\the\\x}} {last}\n\\end{{document}}\n"
        ));
        s
    };
    let words = |expansion: &flashtex_compiler::expansion::Expansion| -> Vec<String> {
        expansion
            .tokens
            .iter()
            .filter_map(|token| match &token.token.kind {
                flashtex_compiler::lexer::TokenKind::Word(word) => Some(word.clone()),
                _ => None,
            })
            .collect()
    };
    let mut cache = None;
    for (class, last, small, large) in [
        ("10", "a", "9.24994pt", "5.16667pt"),
        ("10", "b", "9.24994pt", "5.16667pt"),
        ("12", "b", "10.95003pt", "6.2pt"),
    ] {
        let source = text(class, last);
        let docs = [SourceDocument {
            path: "main.tex",
            text: &source,
        }];
        let cached = expand_project_with_cache(&docs, 0, &mut cache);
        let full = expand_project(&docs, 0);
        assert_eq!(words(&cached), words(&full), "{class}pt {last}");
        let got = words(&cached).concat();
        assert!(got.contains(small), "{class}pt: {small}");
        assert!(got.contains(large), "{class}pt: {large}");
    }
}
