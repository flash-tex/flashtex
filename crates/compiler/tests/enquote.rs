//! csquotes `\enquote` coverage: wraps text in language-appropriate
//! typographic quotation marks, alternating double and single on nesting.

use flashtex_compiler::parser::{self, Block, Inline};

/// Collect all `Inline::Text` runs from the parsed output, in document order.
fn plain_texts(source: &str) -> Vec<String> {
    parser::parse(source)
        .blocks
        .into_iter()
        .flat_map(|block| match block {
            Block::Paragraph(inlines) => inlines,
            _ => Vec::new(),
        })
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

fn messages(source: &str) -> Vec<String> {
    parser::parse(source)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

#[test]
fn basic_enquote_produces_double_quotes() {
    let source = r"\usepackage{csquotes}
\enquote{hello}";
    let texts = plain_texts(source);
    let joined: String = texts.join("");
    assert!(
        joined.contains('\u{201c}'),
        "expected left double quote U+201C in {texts:?}"
    );
    assert!(
        joined.contains('\u{201d}'),
        "expected right double quote U+201D in {texts:?}"
    );
    assert!(
        joined.contains("hello"),
        "expected argument text in {texts:?}"
    );
}

#[test]
fn nested_enquote_alternates_single_quotes() {
    let source = r"\usepackage{csquotes}
\enquote{she said \enquote{hi}}";
    let texts = plain_texts(source);
    let joined: String = texts.join("");
    // Outer: double quotes U+201C / U+201D
    assert!(
        joined.contains('\u{201c}'),
        "outer must have left double quote: {texts:?}"
    );
    assert!(
        joined.contains('\u{201d}'),
        "outer must have right double quote: {texts:?}"
    );
    // Inner: single quotes U+2018 / U+2019
    assert!(
        joined.contains('\u{2018}'),
        "inner must have left single quote: {texts:?}"
    );
    assert!(
        joined.contains('\u{2019}'),
        "inner must have right single quote: {texts:?}"
    );
}

#[test]
fn triple_nested_enquote() {
    let source = r"\usepackage{csquotes}
\enquote{A \enquote{B \enquote{C}}}";
    let texts = plain_texts(source);
    let joined: String = texts.join("");
    // Level 0 (outer): double
    // Level 1 (middle): single
    // Level 2 (inner): double again
    // Count the double left quotes (should be 2: outer and innermost)
    let double_left = joined.matches('\u{201c}').count();
    let single_left = joined.matches('\u{2018}').count();
    assert_eq!(
        double_left, 2,
        "expected 2 left double quotes for levels 0 and 2: {texts:?}"
    );
    assert_eq!(
        single_left, 1,
        "expected 1 left single quote for level 1: {texts:?}"
    );
}

#[test]
fn multiple_sequential_enquotes() {
    let source = r"\usepackage{csquotes}
\enquote{first} and \enquote{second}";
    let texts = plain_texts(source);
    let joined: String = texts.join("");
    // Both should use double quotes (each is at depth 0)
    let double_left = joined.matches('\u{201c}').count();
    let double_right = joined.matches('\u{201d}').count();
    assert_eq!(
        double_left, 2,
        "two sequential \\enquote should each open with double: {texts:?}"
    );
    assert_eq!(
        double_right, 2,
        "two sequential \\enquote should each close with double: {texts:?}"
    );
}

#[test]
fn enquote_without_csquotes_diagnoses() {
    let source = r"\enquote{hello}";
    let msgs = messages(source);
    assert!(
        msgs.iter().any(|m| m.contains("csquotes")),
        "expected a diagnostic mentioning csquotes: {msgs:?}"
    );
    // The argument text should still appear (plain-text fallback).
    let texts = plain_texts(source);
    let joined: String = texts.join("");
    assert!(
        joined.contains("hello"),
        "fallback should still typeset the argument: {texts:?}"
    );
}
