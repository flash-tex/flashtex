//! `\documentclass[..pt]` body size and preamble page/paragraph lengths:
//! `\setlength`, `\addtolength`, and TeX assignments `\len=<dimen>`.
use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::{LayoutConstraints, Page, PARAGRAPH_GAP_PT};
use flashtex_compiler::parser::{parse, Block, Inline, SourceDocument};

fn compile(text: &str) -> (Vec<Page>, Vec<String>) {
    let out = compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text,
        }],
        "main.tex",
        LayoutConstraints::default(),
    );
    let messages = out.diagnostics.into_iter().map(|d| d.message).collect();
    (out.pages, messages)
}

/// Supported preamble-length input must not hide behind an "unsupported" diagnostic.
fn assert_no_diagnostics(text: &str) {
    let parsed = parse(text);
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    let (_, messages) = compile(text);
    assert!(messages.is_empty(), "{messages:?}");
}

fn baseline_gap(text: &str) -> f64 {
    let (pages, _) = compile(text);
    let y = |word: &str| {
        pages[0]
            .items
            .iter()
            .find(|item| item.text == word)
            .map(|item| item.baseline_y_pt)
            .unwrap()
    };
    y("Two") - y("One")
}

#[test]
fn class_option_sets_the_body_size() {
    let src11 = "\\documentclass[11pt]{article}\\begin{document}x\\end{document}";
    let parsed = parse(src11);
    assert_eq!(parsed.class_size_pt, Some(11.0));
    assert_no_diagnostics(src11);
    let src10 = "\\documentclass[a4paper,10pt]{article}\\begin{document}Body\\end{document}";
    assert_no_diagnostics(src10);
    let (pages, _) = compile(src10);
    assert_eq!(pages[0].items[0].font_size_pt, 10.0);
    let src_default = "\\documentclass{article}\\begin{document}Body\\end{document}";
    assert_no_diagnostics(src_default);
    let (pages, _) = compile(src_default);
    assert_eq!(
        pages[0].items[0].font_size_pt,
        LayoutConstraints::default().font_size_pt,
        "no size option keeps the existing default"
    );
}

#[test]
fn parskip_replaces_the_paragraph_gap_with_em_relative_to_the_class_size() {
    let doc = |preamble: &str| {
        format!("\\documentclass[11pt]{{article}}{preamble}\\begin{{document}}One\n\nTwo\\end{{document}}")
    };
    let default_gap = baseline_gap(&doc(""));
    let parskip_src = doc("\\setlength{\\parskip}{0.65em}");
    assert_no_diagnostics(&parskip_src);
    let parskip_gap = baseline_gap(&parskip_src);
    let expected = 0.65 * 11.0 - PARAGRAPH_GAP_PT;
    assert!(
        (parskip_gap - default_gap - expected).abs() < 0.02,
        "gap grew by {} not {expected}",
        parskip_gap - default_gap
    );
    let with_indent = doc("\\setlength{\\parskip}{0.65em}\\setlength{\\parindent}{0pt}");
    assert_no_diagnostics(&with_indent);
}

fn no_preamble_length_noise(messages: &[String]) {
    for m in messages {
        assert!(
            !m.contains("is not supported in the document preamble"),
            "{messages:?}"
        );
        assert!(
            !m.contains("is recognised but not implemented here"),
            "{messages:?}"
        );
        assert!(!m.contains("after \\advance"), "{messages:?}");
        assert!(!m.contains("You can't use"), "{messages:?}");
    }
}

/// Joined paragraph text after expansion (so `\\the\\mylen` is the printed skip).
fn paragraph_text(text: &str) -> String {
    parse(text)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => {
                let mut out = String::new();
                for inline in inlines {
                    if let Inline::Text {
                        text, space_before, ..
                    } = inline
                    {
                        if *space_before && !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(text);
                    }
                }
                Some(out)
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn assert_the_mylen(src: &str, expected: &str) {
    assert_no_diagnostics(src);
    assert_eq!(paragraph_text(src), expected, "{src}");
}

#[test]
fn preamble_setlength_of_page_geometry_is_accepted() {
    let src = "\\documentclass{article}\\setlength{\\textwidth}{6in}\\addtolength{\\oddsidemargin}{-.5in}\
         \\begin{document}Hello\\end{document}";
    assert_no_diagnostics(src);
}

#[test]
fn preamble_tex_assignments_are_accepted() {
    let src = "\\documentclass{article}\\textwidth=6.5in\\paperwidth=8.5in\\oddsidemargin=0in\
         \\parindent=0pt\\parskip=12pt\\begin{document}Hello\\end{document}";
    assert_no_diagnostics(src);
}

#[test]
fn preamble_setlength_space_form_and_length_reference() {
    let literals = "\\documentclass{article}\\textwidth 6in\\setlength{\\textheight}{9in}\
         \\setlength{\\topmargin}{-.5in}\\begin{document}Hello\\end{document}";
    assert_no_diagnostics(literals);
    let (_, messages) = compile(
        "\\documentclass{article}\\textwidth 6in\\setlength{\\textheight}{9in}\
         \\setlength{\\topmargin}{-.5in}\\setlength{\\oddsidemargin}{\\textwidth}\
         \\begin{document}Hello\\end{document}",
    );
    no_preamble_length_noise(&messages);
}

#[test]
fn unimplemented_lengths_are_reported_not_silently_ignored() {
    let (_, messages) = compile(
        "\\documentclass{article}\\setlength{\\parindent}{15pt}\\setlength{\\textwidth}{5in}\
         \\begin{document}x\\setlength{\\parskip}{1em}\\setlength{\\parskip}{banana}\\end{document}",
    );
    for expected in [
        "\\parindent is recognised but paragraph indentation is not implemented",
        "\\setlength{\\parskip} is recognised but not implemented here",
        "\\setlength requires a recognised dimension, got 'banana'",
    ] {
        assert!(
            messages.iter().any(|m| m == expected),
            "missing {expected:?} in {messages:?}"
        );
    }
    assert!(
        !messages
            .iter()
            .any(|m| m.contains("textwidth") && m.contains("not implemented")),
        "preamble \\textwidth must be accepted: {messages:?}"
    );
}

#[test]
fn preamble_parskip_length_reference_is_not_silently_ignored() {
    let src = "\\documentclass{article}\n\\setlength{\\parskip}{\\textwidth}\n\\begin{document}\nOne\n\nTwo\n\\end{document}";
    let parsed = parse(src);
    assert_ne!(
        parsed.parskip_pt,
        Some(0.0),
        "must not apply a length reference as 0pt"
    );
    let (_, messages) = compile(src);
    assert!(
        messages.iter().any(|m| m.contains("unsupported length expression")),
        "missing unsupported length expression in {messages:?}"
    );
}

#[test]
fn preamble_page_length_reference_is_not_silently_ignored() {
    let src = "\\documentclass{article}\\setlength{\\textwidth}{\\paperwidth}\\begin{document}x\\end{document}";
    let parsed = parse(src);
    assert!(
        parsed.diagnostics.iter().any(|d| {
            d.message == "unsupported length expression"
                && d.code == Some(DiagnosticCode::UnsupportedFeature)
                && d.recovery.as_deref() == Some("ignored the length assignment")
        }),
        "page-length refs must resolve or warn, got {:?}",
        parsed.diagnostics
    );
}

#[test]
fn preamble_parskip_one_bp_is_tex_big_point_not_pt() {
    let src = "\\documentclass{article}\n\\setlength{\\parskip}{1bp}\n\\begin{document}x\\end{document}";
    let parsed = parse(src);
    assert_eq!(parsed.parskip_pt, Some(72.27 / 72.0));
    assert_no_diagnostics(src);
}

#[test]
fn newlength_bare_assignment_and_addtolength_in_the_preamble() {
    let with_eq = "\\documentclass{article}\\newlength{\\mylen}\\mylen=5pt\
         \\addtolength{\\mylen}{3pt}\\begin{document}\\the\\mylen\\end{document}";
    assert_no_diagnostics(with_eq);
    let without_eq = "\\documentclass{article}\\newlength{\\mylen}\\mylen 5pt\
         \\addtolength{\\mylen}{3pt}\\begin{document}\\the\\mylen\\end{document}";
    assert_no_diagnostics(without_eq);
    let from_len = "\\documentclass{article}\\newlength{\\lena}\\newlength{\\lenb}\\lena=5pt\
         \\lenb=\\lena\\addtolength{\\lenb}{3pt}\\begin{document}\\the\\lenb\\end{document}";
    assert_no_diagnostics(from_len);
    let setlength = "\\documentclass{article}\\newlength{\\mylen}\\setlength{\\mylen}{5pt}\
         \\addtolength{\\mylen}{3pt}\\begin{document}\\the\\mylen\\end{document}";
    assert_no_diagnostics(setlength);
}

#[test]
fn newlength_bare_assignment_and_addtolength_in_the_body() {
    let with_eq = "\\documentclass{article}\\newlength{\\mylen}\\begin{document}\
         \\mylen=5pt\\addtolength{\\mylen}{3pt}\\the\\mylen\\end{document}";
    assert_no_diagnostics(with_eq);
    let without_eq = "\\documentclass{article}\\newlength{\\mylen}\\begin{document}\
         \\mylen 5pt\\addtolength{\\mylen}{3pt}\\the\\mylen\\end{document}";
    assert_no_diagnostics(without_eq);
}

#[test]
fn builtin_parindent_assignment_and_addtolength() {
    let src = "\\documentclass{article}\\parindent=10pt\\addtolength{\\parindent}{2pt}\
         \\begin{document}x\\end{document}";
    assert_no_diagnostics(src);
}

#[test]
fn hw1_keeps_its_three_reference_pages_with_parskip_applied() {
    let hw1 = include_str!("../../../fixtures/real-world/hw1/HW1.tex");
    let parsed = parse(hw1);
    assert_eq!(parsed.class_size_pt, Some(11.0));
    assert_eq!(parsed.parskip_pt, Some(0.65 * 11.0));
    let (pages, messages) = compile(hw1);
    assert_eq!(pages.len(), 3, "HW1-reference.pdf has 3 pages");
    assert!(
        !messages.iter().any(|m| m.contains("setlength")),
        "{messages:?}"
    );
}

fn scaled_class_diag(expr: &str) -> String {
    format!(
        "scaled class lengths like {expr} are not supported yet; the length is left unchanged"
    )
}

const CALC_DIAG: &str = "calc-style length expressions are not supported";

fn assert_clean_unsupported(messages: &[String]) {
    for m in messages {
        assert!(!m.contains("Illegal unit"), "{messages:?}");
        assert!(!m.contains("got ''"), "{messages:?}");
        assert!(!m.contains("got '"), "{messages:?}");
        assert!(
            !m.contains("csname") || !m.contains("endcsname"),
            "must not report a truncated/empty csname: {messages:?}"
        );
    }
}

/// `<factor>` times a skip that actually exists (`\newlength`).
#[test]
fn scaled_factor_times_real_register_setlength_and_addtolength() {
    // Source skip is 10pt; (expr, expected `\the` of the target).
    let cases: &[(&str, &str)] = &[
        ("2\\mylen", "20.0pt"),
        ("-1.5\\mylen", "-15.0pt"),
        (".25\\mylen", "2.5pt"),
    ];
    for &(expr, expected) in cases {
        let preamble_set = format!(
            "\\documentclass{{article}}\\newlength{{\\mylen}}\\setlength{{\\mylen}}{{10pt}}\\newlength{{\\acc}}\\setlength{{\\acc}}{{{expr}}}\\begin{{document}}\\the\\acc\\end{{document}}"
        );
        let preamble_add = format!(
            "\\documentclass{{article}}\\newlength{{\\mylen}}\\setlength{{\\mylen}}{{10pt}}\\newlength{{\\acc}}\\addtolength{{\\acc}}{{{expr}}}\\begin{{document}}\\the\\acc\\end{{document}}"
        );
        let body_set = format!(
            "\\documentclass{{article}}\\newlength{{\\mylen}}\\setlength{{\\mylen}}{{10pt}}\\newlength{{\\acc}}\\begin{{document}}\\setlength{{\\acc}}{{{expr}}}\\the\\acc\\end{{document}}"
        );
        let body_add = format!(
            "\\documentclass{{article}}\\newlength{{\\mylen}}\\setlength{{\\mylen}}{{10pt}}\\newlength{{\\acc}}\\begin{{document}}\\addtolength{{\\acc}}{{{expr}}}\\the\\acc\\end{{document}}"
        );
        let direct = format!(
            "\\documentclass{{article}}\\newlength{{\\mylen}}\\setlength{{\\mylen}}{{10pt}}\\newlength{{\\acc}}\\acc={expr}\\begin{{document}}\\the\\acc\\end{{document}}"
        );
        for src in [&preamble_set, &preamble_add, &body_set, &body_add, &direct] {
            assert_the_mylen(src, expected);
        }
    }
}

/// Class lengths are not real registers here. A factor times one of them
/// must not guess a class-geometry / article-10pt value.
#[test]
fn scaled_class_length_is_diagnosed_and_leaves_the_target_unchanged() {
    let exprs = ["0.5\\textwidth", "0.5\\linewidth", "-1.5\\parindent", "0.5\\paperwidth"];
    let wraps: &[(&str, &str)] = &[
        ("\\documentclass{article}", ""),
        ("\\documentclass{letter}", ""),
        (
            "\\documentclass{article}",
            "\\usepackage[margin=1in]{geometry}",
        ),
    ];
    // Pre-geometry: the scaled assignment sits before `\usepackage{geometry}`.
    let pre_geometry = "\\documentclass{article}\\newlength{\\mylen}\\setlength{\\mylen}{10pt}\\setlength{\\mylen}{0.5\\textwidth}\\usepackage[margin=1in]{geometry}\\begin{document}\\the\\mylen\\end{document}";
    let mut sources = vec![pre_geometry.to_string()];
    for expr in exprs {
        for &(head, mid) in wraps {
            sources.push(format!(
                "{head}\\newlength{{\\mylen}}\\setlength{{\\mylen}}{{10pt}}{mid}\\setlength{{\\mylen}}{{{expr}}}\\begin{{document}}\\the\\mylen\\end{{document}}"
            ));
            sources.push(format!(
                "{head}\\newlength{{\\mylen}}\\setlength{{\\mylen}}{{10pt}}{mid}\\addtolength{{\\mylen}}{{{expr}}}\\begin{{document}}\\the\\mylen\\end{{document}}"
            ));
        }
    }
    for src in &sources {
        let expr = if src.contains("-1.5\\parindent") {
            "-1.5\\parindent"
        } else if src.contains("0.5\\linewidth") {
            "0.5\\linewidth"
        } else if src.contains("0.5\\paperwidth") {
            "0.5\\paperwidth"
        } else {
            "0.5\\textwidth"
        };
        let expected = scaled_class_diag(expr);
        let (_, messages) = compile(src);
        assert!(
            messages.iter().any(|m| m == &expected),
            "missing {expected:?} in {messages:?} for {src}"
        );
        assert_eq!(
            messages.iter().filter(|m| *m == &expected).count(),
            1,
            "exactly one scaled-class diagnostic, got {messages:?} for {src}"
        );
        assert_clean_unsupported(&messages);
        assert_eq!(paragraph_text(src), "10.0pt", "{src}");
    }
}

#[test]
fn unknown_length_register_is_named_in_the_diagnostic() {
    let src = "\\documentclass{article}\\newlength{\\mylen}\\setlength{\\mylen}{0.5\\foo}\\begin{document}x\\end{document}";
    let (_, messages) = compile(src);
    assert!(
        messages.iter().any(|m| m == "\\foo is not a known length"),
        "expected a diagnostic naming \\foo, got {messages:?}"
    );
    assert!(
        messages.iter().all(|m| !m.contains("got ''") && !m.contains("got '0.5")),
        "must not truncate the register name: {messages:?}"
    );
}

#[test]
fn calc_length_expressions_keep_a_clean_diagnostic() {
    let cases: &[(&str, &str)] = &[
        (
            "\\documentclass{article}\\newlength{\\mylen}\\setlength{\\mylen}{10pt}\\setlength{\\mylen}{\\textwidth-2cm}\\begin{document}\\the\\mylen\\end{document}",
            "10.0pt",
        ),
        (
            "\\documentclass{article}\\newlength{\\mylen}\\setlength{\\mylen}{10pt}\\setlength{\\mylen}{\\mylen+1pt}\\begin{document}\\the\\mylen\\end{document}",
            "10.0pt",
        ),
        (
            "\\documentclass{article}\\setlength{\\textwidth}{\\textwidth-2cm}\\begin{document}x\\end{document}",
            "x",
        ),
        (
            "\\documentclass{article}\\newlength{\\mylen}\\setlength{\\mylen}{\\mylen+1pt}\\begin{document}x\\end{document}",
            "x",
        ),
        (
            "\\documentclass{article}\\setlength{\\textwidth}{\\widthof{Hello}}\\begin{document}x\\end{document}",
            "x",
        ),
    ];
    for &(src, printed) in cases {
        let (_, messages) = compile(src);
        assert!(
            messages.iter().any(|m| m == CALC_DIAG),
            "missing {CALC_DIAG:?} in {messages:?} for {src}"
        );
        assert_eq!(
            messages.iter().filter(|m| *m == CALC_DIAG).count(),
            1,
            "exactly one calc diagnostic, got {messages:?} for {src}"
        );
        assert_clean_unsupported(&messages);
        assert_eq!(paragraph_text(src), printed, "{src}");
    }
}
