//! The position and width arguments of `minipage`, `wrapfigure` and
//! `wraptable` (not set as boxes by this compiler: their bodies are plain
//! text) are read as arguments. pdflatex prints none of them, and a width
//! like `0.5\textwidth` must not reach the paragraph, where the register was
//! reported as an unknown command.

use flashtex_compiler::parser::parse;

fn check(body: &str, width_text: &str) {
    let source = format!(
        "\\documentclass{{article}}\n\\usepackage{{wrapfig}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    );
    let parsed = parse(&source);
    let errors: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|d| format!("{:?}", d.severity) == "Error")
        .collect();
    assert!(errors.is_empty(), "{body}: {errors:?}");
    // The honest "not implemented" warning stays.
    assert!(
        parsed.diagnostics.iter().any(|d| d.message.contains("is not implemented")),
        "{body}: {:?}",
        parsed.diagnostics
    );
    let blocks = format!("{:?}", parsed.blocks);
    assert!(!blocks.contains(width_text), "{body}: width typeset in {blocks}");
    assert!(blocks.contains("Body"), "{body}: body lost in {blocks}");
}

#[test]
fn minipage_width_registers_are_arguments() {
    check("\\begin{minipage}{0.37\\textwidth} Body\\end{minipage}", "0.37");
    check("\\begin{minipage}[t]{.41\\linewidth}Body\\end{minipage}", ".41");
    check("\\begin{minipage}[c][3cm][t]{0.43\\columnwidth}Body\\end{minipage}", "0.43");
    check("\\begin{minipage}{0.47\\hsize}Body\\end{minipage}", "0.47");
    check("\\begin{minipage}[l]{\\textwidth}Body\\end{minipage}", "textwidth");
}

#[test]
fn minipage_inside_a_macro_reads_its_width() {
    check(
        "\\newcommand\\foo{\\begin{minipage}{.53\\textwidth} Body\\end{minipage}}\\foo",
        ".53",
    );
}

#[test]
fn wrapfig_width_registers_are_arguments() {
    check("\\begin{wrapfigure}{r}{0.42\\textwidth}Body\\end{wrapfigure}", "0.42");
    check("\\begin{wrapfigure}[16]{r}{0.44\\textwidth}Body\\end{wrapfigure}", "0.44");
    check("\\begin{wraptable}[14]{R}[2pt]{0.46\\textwidth}Body\\end{wraptable}", "0.46");
}
