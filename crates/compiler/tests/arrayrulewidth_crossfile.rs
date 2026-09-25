//! Issue #1088: `\arrayrulewidth` and `\doublerulesep` are TeX registers,
//! global to the whole run — a `\setlength` in the entry file reaches a
//! math `array` in an `\input`/`\include`-d file, for `\hline` thickness
//! and consecutive-`\hline` spacing alike.
//!
//! Oracle: TeX Live 2026 pdflatex `\showbox` on the two-file project below
//! (`\setlength{\arrayrulewidth}{2pt}`, `\setlength{\doublerulesep}{5pt}`
//! in `main.tex`, the array in `body.tex`): every rule is
//! `\rule(2.0+0.0)`, and `\hline\hline` lays out as rule, `\glue 5.0`,
//! `\glue -2.0`, rule — 5pt apart top-to-top. All assertions below are in
//! TeX pt within 0.1bp.
use flashtex_compiler::math::{layout, MathBox, MathItem};
use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument};

/// 0.1bp in TeX pt.
const TOL_PT: f64 = 0.1 * 72.27 / 72.0;

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOL_PT,
        "{what}: {actual} != {expected} (tolerance 0.1bp)"
    );
}

const MAIN: &str = "\\documentclass{article}\n\\setlength{\\arrayrulewidth}{2pt}\n\\setlength{\\doublerulesep}{5pt}\n\\begin{document}\n\\input{body}\n\\end{document}\n";
const BODY: &str = "$\\begin{array}{c}\\hline b\\\\\\hline\\end{array}$\n\n$\\begin{array}{c}\\hline\\hline b\\\\\\hline\\end{array}$\n";

/// The laid-out rule items of every inline formula in the project, in order.
fn formula_rules(main: &str, body: &str) -> Vec<Vec<MathItem>> {
    let docs = [
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "body.tex", text: body },
    ];
    let parsed = parse_project(&docs, "main.tex");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Math { list, .. } = inline {
                    let mut diagnostics = Vec::new();
                    let laid: MathBox = layout(list, 10.0, &mut diagnostics);
                    assert!(diagnostics.is_empty(), "{diagnostics:?}");
                    out.push(
                        laid.items
                            .into_iter()
                            .filter(|item| item.rule.is_some())
                            .collect(),
                    );
                }
            }
        }
    }
    out
}

/// The issue's repro: rules set in `main.tex`, the array in `body.tex`.
#[test]
fn array_in_input_file_uses_entry_register_values() {
    let formulas = formula_rules(MAIN, BODY);
    assert_eq!(formulas.len(), 2, "two arrays");
    // Single `\hline`s: 2pt thick, like pdflatex's `\rule(2.0+0.0)`.
    for rule in &formulas[0] {
        close(rule.rule.as_ref().unwrap().height, 2.0, "hline thickness");
    }
    assert_eq!(formulas[0].len(), 2, "two hlines");
    // `\hline\hline`: 2pt rules 5pt apart top-to-top (rule, glue 5.0,
    // glue -2.0, rule), then the trailing rule.
    assert_eq!(formulas[1].len(), 3, "three hlines");
    for rule in &formulas[1] {
        close(rule.rule.as_ref().unwrap().height, 2.0, "hline thickness");
    }
    let tops: Vec<f64> = formulas[1]
        .iter()
        .map(|rule| rule.rule.as_ref().unwrap().y)
        .collect();
    close(tops[1] - tops[0], 5.0, "consecutive hline spacing");
}

/// Control: without any assignment the kernel defaults still apply.
#[test]
fn array_without_assignment_keeps_kernel_defaults() {
    let main = "\\documentclass{article}\n\\begin{document}\n\\input{body}\n\\end{document}\n";
    let formulas = formula_rules(main, BODY);
    assert_eq!(formulas.len(), 2, "two arrays");
    for rules in &formulas {
        for rule in rules {
            close(rule.rule.as_ref().unwrap().height, 0.4, "default hline thickness");
        }
    }
    let tops: Vec<f64> = formulas[1]
        .iter()
        .map(|rule| rule.rule.as_ref().unwrap().y)
        .collect();
    close(tops[1] - tops[0], 2.0, "default consecutive hline spacing");
}

/// A grouped assignment in the entry file does not leak into the run,
/// exactly as pdflatex restores the register at the group's end.
#[test]
fn grouped_assignment_in_entry_file_does_not_leak() {
    let main = "\\documentclass{article}\n{\\setlength{\\arrayrulewidth}{9pt}}\n\\begin{document}\n\\input{body}\n\\end{document}\n";
    let formulas = formula_rules(main, BODY);
    assert_eq!(formulas.len(), 2, "two arrays");
    for rule in &formulas[0] {
        close(rule.rule.as_ref().unwrap().height, 0.4, "grouped assignment is undone");
    }
}
