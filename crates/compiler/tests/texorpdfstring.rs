//! hyperref's `\texorpdfstring{TeX text}{bookmark string}` (GH-38 stress
//! sweep): the first argument is the document text in a section title, a
//! caption and body text, while the second feeds only the PDF
//! outline/bookmarks and never reaches the page.
//!
//! Ground truth measured with `pdflatex -interaction=nonstopmode`
//! (TeX Live 2026) on a minimal hyperref document: the heading and TOC
//! read "x2 and more" (with x2 as math), the body reads
//! "Body plain here.", the caption reads "Figure 1: Cap y tail", with no
//! error; under plain article every use is "! Undefined control sequence."

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::parser::{parse, Block, Inline};

/// The text runs of one inline list, joined the way the words read.
fn joined(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        if let Inline::Text {
            text, space_before, ..
        } = inline
        {
            if *space_before && !out.is_empty() {
                out.push(' ');
            }
            out.push_str(text);
        }
    }
    out
}

fn math_count(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .filter(|inline| matches!(inline, Inline::Math { .. }))
        .count()
}

fn has_text(inlines: &[Inline], needle: &str) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Text { text, .. } => text.contains(needle),
        _ => false,
    })
}

#[test]
fn texorpdfstring_typesets_only_its_first_argument() {
    let parsed = parse(concat!(
        "\\documentclass{article}\\usepackage{hyperref}\\begin{document}",
        "\\section{\\texorpdfstring{$x^2$}{x2} and more}",
        "Body \\texorpdfstring{plain $z$}{other} here.",
        "\\begin{figure}\\caption{\\texorpdfstring{Cap $y$}{Y} tail}\\end{figure}",
        "\\end{document}",
    ));
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    // The section title: the math of the first argument, then "and more";
    // the bookmark string "x2" never reaches the page.
    let heading = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Heading { content, .. } => Some(content),
            _ => None,
        })
        .expect("section heading");
    assert_eq!(math_count(heading), 1, "{heading:#?}");
    assert_eq!(joined(heading), "and more", "{heading:#?}");
    assert!(!has_text(heading, "x2"), "{heading:#?}");
    // The body paragraph: the first argument's words and math, with the
    // command-site space kept ("Body plain here.", as pdflatex sets it);
    // the bookmark string "other" never reaches the page.
    let body = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) if has_text(inlines, "Body") => Some(inlines),
            _ => None,
        })
        .expect("body paragraph");
    assert_eq!(math_count(body), 1, "{body:#?}");
    assert_eq!(joined(body), "Body plain here.", "{body:#?}");
    assert!(!has_text(body, "other"), "{body:#?}");
    // The figure caption: the label, then the first argument's words and
    // math; the bookmark string "Y" never reaches the page.
    let caption = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::FigureCaption { content } => Some(content),
            _ => None,
        })
        .expect("figure caption");
    assert_eq!(math_count(caption), 1, "{caption:#?}");
    assert_eq!(joined(caption), "Figure 1: Cap tail", "{caption:#?}");
    assert!(!has_text(caption, "Y"), "{caption:#?}");
}

#[test]
fn texorpdfstring_without_hyperref_is_rejected_like_pdflatex() {
    // pdflatex under plain article: "! Undefined control sequence." This
    // compiler's equivalent is the unknown-command error naming it.
    let parsed = parse(concat!(
        "\\documentclass{article}\\begin{document}",
        "Body \\texorpdfstring{plain}{other} here.",
        "\\end{document}",
    ));
    assert_eq!(
        parsed.diagnostics.len(),
        1,
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let diagnostic = &parsed.diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(
        diagnostic.message.contains("\\texorpdfstring"),
        "{}",
        diagnostic.message
    );
}
