//! Diagnostics dropped inside parsed section titles (issue #38).
//!
//! A section title is set from a flattened token list
//! (`parser::inlines_from_tokens`), not re-parsed, and its catch-all used to
//! swallow every command it could not set without a word — so
//! `\section*{A \nosuchcommand B}` lost the "not supported" diagnostic the
//! same tokens raise in body text (and pdflatex's own
//! `! Undefined control sequence.` for the same input). These tests pin the
//! title/body parity: unknown commands report in both places, while the
//! commands a title legitimately carries stay silent.

use flashtex_compiler::parser;

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn messages(source: &str) -> Vec<String> {
    parser::parse(source)
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

/// An unknown command in a section title reports exactly what the same
/// tokens report in body text — one diagnostic, not zero.
#[test]
fn unknown_command_in_a_section_title_reports_like_body_text() {
    let title = doc("\\section*{A \\nosuchcommand B}\nText.");
    let body = doc("A \\nosuchcommand B\n");
    let title_messages = messages(&title);
    assert_eq!(
        title_messages,
        ["\\nosuchcommand is not supported by this compiler version"]
    );
    assert_eq!(title_messages, messages(&body));
}

/// The diagnostic points at the command inside the title, not at the
/// `\section` token, so hover and hit-testing land on the culprit.
#[test]
fn section_title_diagnostic_span_covers_the_command() {
    let source = doc("\\section*{A \\nosuchcommand B}\nText.");
    let parsed = parser::parse(&source);
    assert_eq!(parsed.diagnostics.len(), 1);
    let span = parsed.diagnostics[0].span.expect("a span on the diagnostic");
    assert!(
        source[span.start..span.end].starts_with('\\'),
        "the diagnostic points at the command, not the section"
    );
}

/// `\hfill` and `\normalfont` are set (fill glue; a face reset), not
/// dropped: pdflatex compiles `\section*{\hfill\normalfont Title}` clean,
/// so no diagnostic may fire for them.
#[test]
fn hfill_and_normalfont_in_a_section_title_stay_silent() {
    let parsed = parser::parse(&doc("\\section*{\\hfill\\normalfont Title}\nText."));
    assert!(
        parsed.diagnostics.is_empty(),
        "handled title commands must not report: {:?}",
        parsed.diagnostics
    );
}

/// `\protect` marks the next command robust and sets nothing, so it stays
/// silent in a title exactly as expansion leaves it silent in body text.
#[test]
fn protect_in_a_section_title_stays_silent() {
    let parsed = parser::parse(&doc("\\section{A\\protect B}\nText."));
    assert!(
        parsed.diagnostics.is_empty(),
        "\\protect must not report: {:?}",
        parsed.diagnostics
    );
}
