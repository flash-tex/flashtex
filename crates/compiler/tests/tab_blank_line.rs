//! Issue #839: a line containing only tabs must end a paragraph, exactly
//! like an empty line. The engine's initial catcode table gave tab a
//! non-space catcode, so a tab-only line lexed as an ordinary character
//! and the two paragraphs silently merged; `expansion::HOST_PRELUDE` now
//! sets tab to catcode 10 (as plain.tex does at format build).

use flashtex_compiler::expansion::expand_project;
use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};
use flashtex_compiler::lexer::TokenKind;
use flashtex_compiler::parser::{parse, Block, SourceDocument};

fn body(text: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{text}\\end{{document}}\n")
}

/// Laid-out text with positions, ignoring source spans (which legitimately
/// differ between the tab and space variants being compared).
fn signature(output: &CompileOutput) -> Vec<(String, f64, f64)> {
    output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| (item.text.clone(), item.x_pt, item.baseline_y_pt))
        .collect()
}

fn signatures_match(a: &CompileOutput, b: &CompileOutput) -> bool {
    let (sa, sb) = (signature(a), signature(b));
    sa.len() == sb.len()
        && sa.iter().zip(sb.iter()).all(|((ta, xa, ya), (tb, xb, yb))| {
            ta == tb && (xa - xb).abs() < 1e-9 && (ya - yb).abs() < 1e-9
        })
}

fn diagnostics_signature(output: &CompileOutput) -> Vec<(String, String)> {
    output
        .diagnostics
        .iter()
        .map(|d| (format!("{:?}", d.severity), d.message.clone()))
        .collect()
}

fn paragraphs(text: &str) -> usize {
    parse(&body(text))
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Paragraph(_)))
        .count()
}

fn has_par_break(text: &str) -> bool {
    let full = body(text);
    let docs = [SourceDocument { path: "main.tex", text: &full }];
    expand_project(&docs, 0)
        .tokens
        .iter()
        .any(|t| matches!(t.token.kind, TokenKind::ParBreak))
}

fn compile_body(text: &str) -> CompileOutput {
    compile_full(&body(text), LayoutConstraints::default())
}

fn baseline_of(output: &CompileOutput, needle: &str) -> f64 {
    output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .find(|item| item.text == needle)
        .unwrap_or_else(|| panic!("expected laid-out item {needle:?}"))
        .baseline_y_pt
}

/// The exact repro from the issue: a tab-only line between two paragraphs.
#[test]
fn tab_only_line_breaks_paragraph() {
    let tab = compile_body("Para one.\n\t\nPara two.\n");
    let empty = compile_body("Para one.\n\nPara two.\n");

    assert!(has_par_break("Para one.\n\t\nPara two.\n"));
    assert_eq!(paragraphs("Para one.\n\t\nPara two.\n"), 2);
    assert_eq!(paragraphs("Para one.\n\nPara two.\n"), 2);

    // A real page-position break: "Para two." sits below "Para one." ...
    assert!(baseline_of(&tab, "two.") > baseline_of(&tab, "one."));
    // ... and the tab-only line lays out exactly like the empty line.
    assert!(
        signatures_match(&tab, &empty),
        "tab-only line must lay out identically to an empty line:\n{tab:?}\n{empty:?}",
        tab = signature(&tab),
        empty = signature(&empty)
    );
    assert_eq!(
        diagnostics_signature(&tab),
        diagnostics_signature(&empty)
    );
}

/// Mixed whitespace-only lines (spaces + tabs) break paragraphs too.
#[test]
fn whitespace_only_lines_break_paragraph() {
    let empty = compile_body("Para one.\n\nPara two.\n");
    for line in [" \t", "\t ", "  \t  ", "\t\t", " \t \t "] {
        let source = format!("Para one.\n{line}\nPara two.\n");
        assert!(has_par_break(&source), "no ParBreak for {line:?}");
        assert_eq!(paragraphs(&source), 2, "no break for {line:?}");
        let output = compile_body(&source);
        assert!(
            signatures_match(&output, &empty),
            "{line:?}-only line must lay out like an empty line"
        );
    }
}

/// Everything the issue lists as already-correct must stay tab==space:
/// indentation, mid-line tabs, math, macro arguments, trailing tabs.
#[test]
fn tabs_elsewhere_behave_like_spaces() {
    let cases = [
        // Leading indentation.
        ("\tPara one.\n", " Para one.\n"),
        // Mid-line tab.
        ("Para\ta.\n\nPara two.\n", "Para a.\n\nPara two.\n"),
        // Tabs in math.
        ("$a\t+\tb$\n", "$a + b$\n"),
        // Tabs inside macro arguments.
        ("\\textbf{A\tB}\n", "\\textbf{A B}\n"),
        // Trailing tab at end of a non-blank line: no break either way.
        ("Para one.\t\nPara two.\n", "Para one. \nPara two.\n"),
    ];
    for (with_tab, with_space) in cases {
        let tab = compile_body(with_tab);
        let space = compile_body(with_space);
        assert!(
            signatures_match(&tab, &space),
            "{with_tab:?} must lay out like {with_space:?}:\n{tab:?}\n{space:?}",
            tab = signature(&tab),
            space = signature(&space)
        );
        assert_eq!(
            diagnostics_signature(&tab),
            diagnostics_signature(&space),
            "diagnostics differ for {with_tab:?}"
        );
    }
    // The trailing-tab case is one paragraph, not two, in both variants.
    assert_eq!(paragraphs("Para one.\t\nPara two.\n"), 1);
    assert_eq!(paragraphs("Para one. \nPara two.\n"), 1);
}
