//! Kernel `\mbox` and amsmath's `\text` in text mode are one `\hbox`.
//!
//! latex.ltx: `\DeclareRobustCommand\mbox[1]{\leavevmode\hbox{#1}}`;
//! amsmath.dtx: `\text` outside math is `\mbox`. Text-mode `\mbox` used to
//! be rejected with "\mbox is a math command", and its argument was set
//! (partly) as loose words; `\text` was spliced into the paragraph, so a
//! line could break inside it. Both are now an [`Inline::HBox`] whose
//! content keeps its commands, so the pipeline sets it unbreakable at its
//! natural width (`crates/render-pipeline/tests/text_mbox.rs` pins that
//! geometry against pdfTeX).
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::parser::{parse, Block, Inline};

fn document(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn paragraph(source: &str) -> Vec<Inline> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut inlines = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(body) = block {
            inlines.extend(body.clone());
        }
    }
    inlines
}

fn words(content: &[Inline]) -> Vec<&str> {
    content
        .iter()
        .map(|i| match i {
            Inline::Text { text, .. } => text.as_str(),
            other => panic!("not text: {other:?}"),
        })
        .collect()
}

#[test]
fn text_mode_mbox_is_one_hbox_without_a_diagnostic() {
    let inlines = paragraph(&document("", "A \\mbox{two words} and \\mbox{\\textbf{b} c}B"));
    let boxes: Vec<_> = inlines
        .iter()
        .filter_map(|i| match i {
            Inline::HBox(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(boxes.len(), 2, "{inlines:?}");
    assert!(boxes[0].space_before && boxes[1].space_before, "{boxes:?}");
    assert_eq!(words(&boxes[0].content), ["two", "words"]);
    // Commands inside the box are typeset, not printed.
    assert_eq!(words(&boxes[1].content), ["b", "c"]);
    // The box ends at its closing brace, so `B` directly follows it.
    let source = document("", "A \\mbox{two words} and \\mbox{\\textbf{b} c}B");
    assert_eq!(&source[boxes[1].span.start..boxes[1].span.end], "\\mbox{\\textbf{b} c}");
}

#[test]
fn amsmath_text_in_text_mode_is_the_same_hbox() {
    let inlines = paragraph(&document("\\usepackage{amsmath}\n", "A \\text{two words} B"));
    let [Inline::Text { .. }, Inline::HBox(b), Inline::Text { .. }] = &inlines[..] else {
        panic!("{inlines:?}");
    };
    assert_eq!(words(&b.content), ["two", "words"]);
}

/// An empty `\mbox{}` is still a box: alone in the body it sets a line
/// and ships a page, as pdfTeX does.
#[test]
fn an_empty_mbox_alone_ships_a_page() {
    let output = compile_full(&document("", "\\mbox{}"), LayoutConstraints::default());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(output.pages.len(), 1);
}

/// The Core 14 layout splices the box's words like any other box.
#[test]
fn core14_layout_sets_the_box_content() {
    let output = compile_full(&document("", "Before \\mbox{in box} after"), LayoutConstraints::default());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let words: Vec<&str> = output.pages.iter().flat_map(|p| p.items.iter()).map(|i| i.text.as_str()).collect();
    assert_eq!(words.join(" ").split_whitespace().collect::<Vec<_>>(), ["Before", "in", "box", "after"], "{words:?}");
}
