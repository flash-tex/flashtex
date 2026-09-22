//! `calc` package slice 1: `+`/`-` chains of dimensions in
//! `\setlength`/`\addtolength`, and silent `\usepackage{calc}`.
//!
//! Real calc.sty re-tokenizes its argument with active `+ - * / ( )`
//! characters; this crate does not replicate that mechanism. It covers the
//! surface students actually write: `+`/`-` chains of dimension terms.
//! `*`, `/`, parentheses and `\widthof`/`\heightof`/`\depthof`/
//! `\totalheightof` stay out of scope and keep today's diagnostic.

use flashtex_compiler::parser::{parse, Block};

fn parskip_of(src: &str) -> (Option<f64>, Vec<String>) {
    let parsed = parse(src);
    let messages = parsed.diagnostics.iter().map(|d| d.message.clone()).collect();
    (parsed.parskip_pt, messages)
}

/// The `extra_gap_before_pt` of the second item in a two-item list: the
/// first item carries topsep, the second carries itemsep.
fn second_item_gap(decl: &str) -> (f64, Vec<String>) {
    let src = format!(
        "\\documentclass{{article}}\\begin{{document}}\\begin{{list}}{{-}}{{{decl}}}\\item A\\item B\\end{{list}}\\end{{document}}"
    );
    let parsed = parse(&src);
    let messages = parsed.diagnostics.iter().map(|d| d.message.clone()).collect();
    let mut gaps = Vec::new();
    for block in &parsed.blocks {
        if let Block::ListItem { extra_gap_before_pt, .. } = block {
            gaps.push(*extra_gap_before_pt);
        }
    }
    assert_eq!(gaps.len(), 2, "{decl}: {gaps:?} in {:?}", parsed.blocks);
    (gaps[1], messages)
}

#[test]
fn calc_load_is_silent_without_options() {
    let src = "\\documentclass{article}\n\\usepackage{calc}\n\\begin{document}\nHi\n\\end{document}\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let with_option =
        "\\documentclass{article}\n\\usepackage[foo]{calc}\n\\begin{document}\nHi\n\\end{document}\n";
    let parsed = parse(with_option);
    assert!(
        parsed.diagnostics.iter().any(|d| d.message.contains("calc")),
        "options must keep the warning: {:?}",
        parsed.diagnostics
    );
}

#[test]
fn calc_two_term_plus_chain_sums() {
    let (pt, messages) =
        parskip_of("\\documentclass{article}\\setlength{\\parskip}{1pt + 2pt}\\begin{document}x\\end{document}");
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(pt, Some(3.0));
}

#[test]
fn calc_two_term_minus_chain_subtracts() {
    let (pt, messages) =
        parskip_of("\\documentclass{article}\\setlength{\\parskip}{5pt - 2pt}\\begin{document}x\\end{document}");
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(pt, Some(3.0));
}

#[test]
fn calc_three_term_mixed_chain_and_addtolength() {
    let (pt, messages) = parskip_of(
        "\\documentclass{article}\\setlength{\\parskip}{10pt + 2pt - 3pt}\\begin{document}x\\end{document}",
    );
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(pt, Some(9.0));
    let (pt, messages) = parskip_of(
        "\\documentclass{article}\\setlength{\\parskip}{1pt}\\addtolength{\\parskip}{2pt + 3pt}\\begin{document}x\\end{document}",
    );
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(pt, Some(6.0));
}

#[test]
fn calc_baselineskip_chain_in_a_list() {
    let (gap, messages) = second_item_gap("\\setlength{\\itemsep}{1pt + 2\\baselineskip}");
    assert!(messages.is_empty(), "{messages:?}");
    // 1pt plus two of the 10pt class leading (12pt).
    assert!((gap - 25.0).abs() < 1e-9, "{gap}");
}

#[test]
fn plain_single_dimension_setlength_is_unaffected() {
    let (pt, messages) =
        parskip_of("\\documentclass{article}\\setlength{\\parskip}{24pt}\\begin{document}x\\end{document}");
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(pt, Some(24.0));
    // A leading sign is part of the first term, not a chain operator.
    let (pt, messages) = parskip_of(
        "\\documentclass{article}\\setlength{\\parskip}{-0.5in}\\begin{document}x\\end{document}",
    );
    assert!(messages.is_empty(), "{messages:?}");
    // TeX's fixed point: `-0.5in` is -2368143sp (`\showthe` -36.13498pt),
    // not the float -36.135.
    assert_eq!(pt, Some(-2_368_143.0 / 65_536.0));
    // `*`/`/` by an integer are e-TeX `\glueexpr` terms, which is what the
    // engine's `\setlength` evaluates (calc gives the same 2pt).
    let (pt, messages) = parskip_of(
        "\\documentclass{article}\\setlength{\\parskip}{1pt * 2}\\begin{document}x\\end{document}",
    );
    assert!(messages.is_empty(), "{messages:?}");
    assert_eq!(pt, Some(2.0));
    // calc's `\real`/`\ratio`/`\widthof` are outside that grammar and keep
    // an error (pdflatex without calc: "Missing number").
    let (_, messages) = parskip_of(
        "\\documentclass{article}\\setlength{\\parskip}{1pt * \\real{2}}\\begin{document}x\\end{document}",
    );
    assert!(
        messages.iter().any(|m| m.contains("Missing number")),
        "{messages:?}"
    );
}
