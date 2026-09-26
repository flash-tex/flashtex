//! Unbalanced `\left`/`\right` diagnostics (pdflatex parity).
//!
//! Measured with TeX Live 2026 pdflatex (`pdflatex -interaction=nonstopmode`):
//! - `$\left( x + y$` stops with `! Missing \right. inserted.`
//! - `$x \right) y$` stops with `! Extra \right.`
//! - `$\left( \left( x$` stops with two `! Missing \right. inserted.`
//! - `${\left( x} y$` stops with `! Extra }, or forgotten \right.`
//!   followed by `! Missing \right. inserted.`
//! Before the fix all of these compiled with 0 diagnostics.

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::parser::parse;

fn document(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn errors(body: &str) -> Vec<String> {
    parse(&document(body))
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

#[test]
fn left_right_missing_right_is_an_error() {
    let found = errors(r"$\left( x + y$");
    assert!(
        found
            .iter()
            .any(|m| m.contains(r"\left") && m.contains(r"\right")),
        "a \\left with no matching \\right must be an error, got: {found:?}"
    );
}

#[test]
fn left_right_two_missing_rights_are_two_errors() {
    let found = errors(r"$\left( \left( x$");
    let matching = found
        .iter()
        .filter(|m| m.contains(r"\left") && m.contains(r"\right"))
        .count();
    assert_eq!(
        matching, 2,
        "each unclosed \\left needs its own diagnostic, got: {found:?}"
    );
}

#[test]
fn left_right_bare_right_is_an_error() {
    let found = errors(r"$x \right) y$");
    assert!(
        found
            .iter()
            .any(|m| m.contains(r"\right") && m.contains(r"\left")),
        "a \\right with no matching \\left must be an error, got: {found:?}"
    );
}

#[test]
fn left_right_group_end_with_open_left_is_an_error() {
    let found = errors(r"${\left( x} y$");
    assert!(
        found
            .iter()
            .any(|m| m.contains(r"\left") && m.contains(r"\right")),
        "a group ending with an open \\left must be an error, got: {found:?}"
    );
}

#[test]
fn left_right_balanced_pair_stays_silent() {
    let found = errors(r"$\left( x \right)$");
    assert!(found.is_empty(), "balanced pair must stay silent: {found:?}");
}
