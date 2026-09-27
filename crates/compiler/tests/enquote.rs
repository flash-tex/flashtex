//! csquotes `\enquote` coverage: wraps text in language-appropriate
//! typographic quotation marks, alternating double and single on nesting.

use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::{text_width, word_space, LayoutConstraints};
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

/// The task oracle, measured against pdflatex (TeX Live 2026, 10pt
/// article, csquotes.sty): `tq` in double quotes, `star` and `ts` in
/// single quotes. This layout sets type in Core-14 metrics on its own
/// page frame rather than Computer Modern on the article page, so the
/// absolute page positions cannot match pdflatex; the quote characters
/// are pinned exactly and the word gaps against the layout's own metrics
/// (a dropped or zero-width quote mark would move every word after it).
const ORACLE_SOURCE: &str = "\\documentclass[10pt]{article}\\usepackage{csquotes}\n\\begin{document}\nA \\enquote{outer \\enquote{inner}} B \\textquote{tq} C \\enquote*{star} D \\textquote*{ts} E next.\n\\end{document}";

#[test]
fn oracle_source_has_no_diagnostics() {
    let parsed = parser::parse(ORACLE_SOURCE);
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn oracle_source_sets_the_task_quote_marks_in_order() {
    let out = compile_full(ORACLE_SOURCE, LayoutConstraints::default());
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let words: Vec<&str> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter(|i| i.rule.is_none() && !i.text.trim().is_empty())
        .map(|i| i.text.as_str())
        .collect();
    assert_eq!(
        words,
        [
            "A", "\u{201c}", "outer", "\u{2018}", "inner", "\u{2019}", "\u{201d}", "B",
            "\u{201c}", "tq", "\u{201d}", "C", "\u{2018}", "star", "\u{2019}", "D",
            "\u{2018}", "ts", "\u{2019}", "E", "next.",
        ],
        "outer double, inner single, tq double, star/ts single"
    );
    // One line: every word shares the baseline.
    let baselines: Vec<f64> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| i.baseline_y_pt)
        .collect();
    assert!(
        baselines.windows(2).all(|w| w[0] == w[1]),
        "{baselines:?}"
    );
}

#[test]
fn oracle_word_gaps_advance_by_real_quote_widths() {
    let out = compile_full(ORACLE_SOURCE, LayoutConstraints::default());
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let items: Vec<_> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter(|i| i.rule.is_none() && !i.text.trim().is_empty())
        .collect();
    let font = items[0].font;
    let size = items[0].font_size_pt;
    let space = word_space(size, font);
    // Every run advances its own width plus one interword space — except
    // into a closing quote mark, which glues onto its content
    // (`space_before: false` rewinds past the reserved trailing space). A
    // dropped or zero-width quote mark would move every word after it.
    for pair in items.windows(2) {
        let (curr, next) = (pair[0], pair[1]);
        let expected = text_width(&curr.text, size, font)
            + if next.text == "\u{201d}" || next.text == "\u{2019}" {
                0.0
            } else {
                space
            };
        assert!(
            ((next.x_pt - curr.x_pt) - expected).abs() < 0.03,
            "{:?}->{:?}: {} vs {expected}",
            curr.text,
            next.text,
            next.x_pt - curr.x_pt,
        );
    }
}

#[test]
fn textquote_matches_enquote_run_for_run() {
    for payload in ["hello", "she said \\enquote{hi}"] {
        let textquote = compile_full(
            &format!("\\usepackage{{csquotes}}\n\\textquote{{{payload}}}\n"),
            LayoutConstraints::default(),
        );
        let enquote = compile_full(
            &format!("\\usepackage{{csquotes}}\n\\enquote{{{payload}}}\n"),
            LayoutConstraints::default(),
        );
        let words = |o: &flashtex_compiler::incremental::CompileOutput| {
            o.pages
                .iter()
                .flat_map(|p| p.items.iter())
                .map(|i| (i.text.clone(), i.x_pt, i.baseline_y_pt))
                .collect::<Vec<_>>()
        };
        assert_eq!(words(&textquote), words(&enquote), "{payload}");
    }
}

#[test]
fn starred_nesting_continues_the_alternation() {
    // A star counts as one already-open level: outer single, inner double
    // (the interword gap between them is implicit glue, not text).
    let texts = plain_texts("\\usepackage{csquotes}\n\\enquote*{a \\enquote{b}}");
    let joined: String = texts.join("");
    assert_eq!(
        joined,
        "\u{2018}a\u{201c}b\u{201d}\u{2019}",
        "{texts:?}"
    );
    let texts = plain_texts("\\usepackage{csquotes}\n\\textquote*{a \\textquote{b}}");
    let joined: String = texts.join("");
    assert_eq!(
        joined,
        "\u{2018}a\u{201c}b\u{201d}\u{2019}",
        "{texts:?}"
    );
}

#[test]
fn textquote_and_starred_forms_without_csquotes_diagnose() {
    for (source, name) in [
        ("\\textquote{hello}", "textquote"),
        ("\\enquote*{hello}", "enquote"),
        ("\\textquote*{hello}", "textquote"),
    ] {
        let msgs = messages(source);
        assert!(
            msgs.iter().any(|m| m.contains("csquotes")),
            "{name}: expected a diagnostic mentioning csquotes: {msgs:?}"
        );
        // No stray star reaches the page, and the argument still typesets.
        let texts = plain_texts(source);
        let joined: String = texts.join("");
        assert!(
            joined.contains("hello") && !joined.contains('*'),
            "{name}: {texts:?}"
        );
    }
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
