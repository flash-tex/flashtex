//! multicol parsing: `multicols`/`multicols*` arguments and preface,
//! `\columnbreak`, and multicol's own diagnostics (multicol.sty v2.0b).
use flashtex_compiler::parser::{parse, Block, Inline};

fn words(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every paragraph and heading as text, in order.
fn outline(source: &str) -> (Vec<String>, Vec<String>) {
    let parsed = parse(source);
    let blocks = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(inlines) => Some(format!("P:{}", words(inlines))),
            Block::Heading { content, .. } => Some(format!("H:{}", words(content))),
            _ => None,
        })
        .collect();
    let diagnostics = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    (blocks, diagnostics)
}

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{preamble}\n\\usepackage{{multicol}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn arguments_are_consumed_and_the_preface_is_body_material() {
    let (blocks, diagnostics) = outline(&doc(
        "{article}",
        "Before \\begin{multicols}{3}[\\section{Head}][80pt]\nOne two.\\columnbreak\n\nThree. \\raggedcolumns\n\\end{multicols} After",
    ));
    assert_eq!(
        blocks,
        ["P:Before", "H:Head", "P:One two.", "P:Three.", "P:After"],
        "{diagnostics:?}"
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn a_text_preface_ends_with_its_own_paragraph() {
    let (blocks, diagnostics) = outline(&doc(
        "{article}",
        "\\begin{multicols*}{2}[Intro text] Body text.\n\\end{multicols*}",
    ));
    assert_eq!(blocks, ["P:Intro text", "P:Body text."], "{diagnostics:?}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn column_counts_are_checked_like_multicols() {
    let (_, few) = outline(&doc(
        "{article}",
        "\\begin{multicols}{1}\nx\n\\end{multicols}",
    ));
    assert!(
        few.iter()
            .any(|m| m.contains("Using `1' columns doesn't seem a good idea")),
        "{few:?}"
    );
    let (_, many) = outline(&doc(
        "{article}",
        "\\begin{multicols}{21}\nx\n\\end{multicols}",
    ));
    assert!(
        many.iter().any(|m| m.contains("Too many columns")),
        "{many:?}"
    );
}

#[test]
fn columnbreak_outside_multicols_is_multicols_error() {
    let (_, diagnostics) = outline(&doc("{article}", "Text \\columnbreak more."));
    assert!(
        diagnostics
            .iter()
            .any(|m| m.contains("\\columnbreak outside multicols")),
        "{diagnostics:?}"
    );
}

#[test]
fn the_package_is_silent_except_with_the_twocolumn_option() {
    let (_, plain) = outline(&doc("{article}", "x"));
    assert!(plain.is_empty(), "{plain:?}");
    let (_, twocolumn) = outline(&doc("[twocolumn]{article}", "x"));
    assert!(
        twocolumn
            .iter()
            .any(|m| m.contains("May not work with the twocolumn option")),
        "{twocolumn:?}"
    );
}
