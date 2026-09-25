//! Issue #1088: `\arrayrulewidth` and `\doublerulesep` are TeX registers,
//! global to the whole run — a `\setlength` in the entry file reaches a
//! text `tabular` in an `\input`/`\include`-d file, for `\hline`
//! thickness, `|` width and consecutive-`\hline` spacing alike.
//!
//! Oracle: TeX Live 2026 pdflatex `\showbox` on the two-file project below
//! (`\setlength{\arrayrulewidth}{2pt}`, `\setlength{\doublerulesep}{5pt}`
//! in `main.tex`, the table in `body.tex`): every rule is
//! `\rule(2.0+0.0)`, and `\hline\hline` lays out as rule, `\glue 5.0`,
//! `\glue -2.0`, rule — 5pt apart top-to-top.
//!
//! (A math `array`'s `\hline`s are not drawn by this pipeline yet: its
//! vendored compiler predates array rules, so they never reach `mathgrid`.
//! The cross-file values for those are covered in
//! `crates/compiler/tests/arrayrulewidth_crossfile.rs` and travel to this
//! pipeline with the vendor re-pin noted on `math::ArrayRuleWidths`.)
//!
//! All assertions below are in bp within 0.1bp (2pt = 1.9929bp,
//! 5pt = 4.9823bp).

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const MAIN: &str = "\\documentclass{article}\n\\setlength{\\arrayrulewidth}{2pt}\n\\setlength{\\doublerulesep}{5pt}\n\\begin{document}\n\\input{body}\n\\end{document}\n";
const BODY: &str = "\\begin{tabular}{c}\\hline a\\\\\\hline\\end{tabular}\n";
const BODY_DOUBLE: &str = "\\begin{tabular}{c}\\hline\\hline a\\\\\\hline\\end{tabular}\n";
const BODY_VLINE: &str = "\\begin{tabular}{|c|}\\hline a\\\\\\hline\\end{tabular}\n";

/// 2pt / 5pt in bp.
const RULE_BP: f64 = 2.0 * 72.0 / 72.27;
const DOUBLE_SEP_BP: f64 = 5.0 * 72.0 / 72.27;

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 0.1,
        "{what}: {actual} != {expected} (tolerance 0.1bp)"
    );
}

/// `[x, top, width, height]` of every rule on every page, in bp.
fn rules(main: &str, docs: &[(&str, &str)]) -> Vec<[f64; 4]> {
    let r = render_docs(docs, main);
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for item in page.resident_items() {
            if let Item::Rule(rule) = item {
                out.push([rule.x.to_bp(), rule.top.to_bp(), rule.width.to_bp(), rule.height.to_bp()]);
            }
        }
    }
    out
}

fn body_docs(body: &str) -> Vec<(&str, &str)> {
    vec![("main.tex", MAIN), ("body.tex", body)]
}

/// The issue's repro: `\hline`s set in `main.tex`, the table in `body.tex`.
#[test]
fn tabular_in_input_file_uses_entry_register_values() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let rules = rules("main.tex", &body_docs(BODY));
    assert_eq!(rules.len(), 2, "two hlines, got {rules:?}");
    for rule in &rules {
        close(rule[3], RULE_BP, "hline thickness");
        close(rule[2], rules[0][2], "both hlines span the table");
    }
}

/// `\hline\hline` across files: consecutive rules `\doublerulesep` apart.
#[test]
fn double_hline_in_input_file_uses_entry_doublerulesep() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let rules = rules("main.tex", &body_docs(BODY_DOUBLE));
    assert_eq!(rules.len(), 3, "three hlines, got {rules:?}");
    for rule in &rules {
        close(rule[3], RULE_BP, "hline thickness");
    }
    close(rules[1][1] - rules[0][1], DOUBLE_SEP_BP, "consecutive hline spacing");
}

/// `|` rules take `\arrayrulewidth` of width across files too.
#[test]
fn vline_in_input_file_uses_entry_register_value() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let rules = rules("main.tex", &body_docs(BODY_VLINE));
    let vertical: Vec<[f64; 4]> = rules.iter().copied().filter(|rule| rule[3] > rule[2]).collect();
    assert_eq!(vertical.len(), 2, "two vertical rules, got {rules:?}");
    for rule in &vertical {
        close(rule[2], RULE_BP, "vline width");
    }
}

/// Control: the same assignment in the table's own file still applies.
#[test]
fn tabular_in_entry_file_keeps_local_read() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = MAIN.replace("\\input{body}", BODY);
    let rules = rules("main.tex", &[("main.tex", main.as_str())]);
    assert_eq!(rules.len(), 2, "two hlines, got {rules:?}");
    for rule in &rules {
        close(rule[3], RULE_BP, "hline thickness");
    }
}

/// A grouped assignment in the entry file does not leak into the run.
#[test]
fn grouped_assignment_in_entry_file_does_not_leak() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = MAIN.replace(
        "\\setlength{\\arrayrulewidth}{2pt}",
        "{\\setlength{\\arrayrulewidth}{9pt}}",
    );
    let rules = rules("main.tex", &[("main.tex", main.as_str()), ("body.tex", BODY)]);
    assert_eq!(rules.len(), 2, "two hlines, got {rules:?}");
    for rule in &rules {
        close(rule[3], 0.4 * 72.0 / 72.27, "grouped assignment is undone");
    }
}

/// An assignment AFTER the `\input` that reached the table must not leak into
/// it: a nested file inherits the register value as of the exact byte
/// position of the `\input` that pulled it in, not the value at end-of-file.
///
/// Oracle: TeX Live 2026 pdflatex `\typeout{\the\arrayrulewidth}` just before
/// the table in `b.tex` prints `2.0pt` for the project below (the `9pt` set
/// after `\input{b}` in the fragment `a.tex` — which has no
/// `\begin/\end{document}` group to undo it at end-of-file — runs after the
/// table).
#[test]
fn later_assignment_does_not_leak_into_earlier_input() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = "\\documentclass{article}\n\\begin{document}\n\\input{a}\n\\end{document}\n";
    let a = "\\setlength{\\arrayrulewidth}{2pt}\n\\input{b}\n\\setlength{\\arrayrulewidth}{9pt}\n";
    let b = "\\begin{tabular}{c}\\hline a\\\\\\hline\\end{tabular}\n";
    let docs = vec![("main.tex", main), ("a.tex", a), ("b.tex", b)];
    let rules = rules("main.tex", &docs);
    assert_eq!(rules.len(), 2, "two hlines, got {rules:?}");
    for rule in &rules {
        close(rule[3], RULE_BP, "table sees the 2pt in force at \\input{b}");
    }
}

/// A group still open at the `\input` point crosses the file boundary: the
/// table sees the value in force there, not the value the later `\endgroup`
/// restores.
///
/// Oracle: TeX Live 2026 pdflatex `\typeout{\the\arrayrulewidth}` just before
/// the table in `body.tex` prints `9.0pt` for the project below.
#[test]
fn open_group_value_crosses_into_input_file() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = "\\documentclass{article}\n\\begin{document}\n\\begingroup\n\\setlength{\\arrayrulewidth}{9pt}\n\\input{body}\n\\endgroup\n\\end{document}\n";
    let docs = vec![("main.tex", main), ("body.tex", BODY)];
    let rules = rules("main.tex", &docs);
    assert_eq!(rules.len(), 2, "two hlines, got {rules:?}");
    for rule in &rules {
        close(rule[3], 9.0 * 72.0 / 72.27, "table sees the grouped 9pt");
    }
}

/// `\global\setlength` inside a group escapes it for a text table in a
/// nested file, exactly as for math arrays.
///
/// Oracle: TeX Live 2026 pdflatex `\typeout{\the\arrayrulewidth}` just before
/// the table in `body.tex` prints `5.0pt` for the project below (the grouped
/// `9pt` is undone, the `\global` `5pt` stands).
#[test]
fn global_setlength_escapes_group_for_nested_text_table() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = "\\documentclass{article}\n\\begin{document}\n\\input{body}\n\\end{document}\n";
    let body = "{\\setlength{\\arrayrulewidth}{9pt}\\global\\setlength{\\arrayrulewidth}{5pt}}\n\\begin{tabular}{c}\\hline a\\\\\\hline\\end{tabular}\n";
    let docs = vec![("main.tex", main), ("body.tex", body)];
    let rules = rules("main.tex", &docs);
    assert_eq!(rules.len(), 2, "two hlines, got {rules:?}");
    for rule in &rules {
        close(rule[3], 5.0 * 72.0 / 72.27, "table sees the global 5pt");
    }
}

/// A file the run reaches after the table's own cannot move its rules.
#[test]
fn later_file_does_not_move_an_earlier_table() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = "\\documentclass{article}\n\\setlength{\\arrayrulewidth}{2pt}\n\\begin{document}\n\\input{body}\n\\input{later}\n\\end{document}\n";
    let later = "\\setlength{\\arrayrulewidth}{9pt}\nLater text.\n";
    let docs = vec![("main.tex", main), ("body.tex", BODY), ("later.tex", later)];
    let rules = rules("main.tex", &docs);
    assert_eq!(rules.len(), 2, "two hlines, got {rules:?}");
    for rule in &rules {
        close(rule[3], RULE_BP, "hline thickness");
    }
}
