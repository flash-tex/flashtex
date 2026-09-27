//! `\vspace` inside a paragraph is latex.ltx's
//! `\@bsphack\vadjust{\vskip<glue>}\@esphack`: the paragraph goes on and the
//! glue lands below the line the command is set on. `\smallskip`,
//! `\medskip` and `\bigskip` are `\vspace\<..>skipamount`, and `\vspace*`
//! differs only in keeping the glue at a page break.
//!
//! pdflatex, TeX Live 2026, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`,
//! `article`: in `\noindent Some text \vspace{10pt} in the middle of a
//! paragraph ...` the word `in` sits on the first line at x 182.96bp (49.19bp
//! right of `Some`), and the second line is 21.92bp (12pt `\baselineskip`
//! plus the 10pt) below the first. FlashTeX broke the paragraph at the
//! command, so `in` started a line of its own, 47.71bp left of pdflatex's.
use flashtex_compiler::parser::{parse, Block, Inline};

fn blocks(body: &str) -> Vec<Block> {
    parse(&format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")).blocks
}

fn paragraphs(blocks: &[Block]) -> Vec<&Vec<Inline>> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .collect()
}

fn text_of(inlines: &[Inline]) -> Vec<&str> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } if !text.is_empty() => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn adjust_skips(inlines: &[Inline]) -> Vec<f64> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::VAdjustSkip { pt, .. } => Some(*pt),
            _ => None,
        })
        .collect()
}

fn vspace_blocks(blocks: &[Block]) -> usize {
    blocks.iter().filter(|block| matches!(block, Block::VSpace { .. })).count()
}

#[test]
fn vspace_in_a_paragraph_does_not_break_it() {
    for body in [
        "Some text \\vspace{10pt} in the middle.",
        "Some text \\vspace*{10pt} in the middle.",
        "Some text\\vspace{10pt}in the middle.",
        "\\noindent\\vspace{10pt}Some text in the middle.",
    ] {
        let blocks = blocks(body);
        let paras = paragraphs(&blocks);
        assert_eq!(paras.len(), 1, "{body}: {blocks:?}");
        assert_eq!(adjust_skips(paras[0]), vec![10.0], "{body}");
        assert_eq!(vspace_blocks(&blocks), 0, "{body}");
    }
    let blocks = blocks("Some text \\vspace{10pt} in the middle.");
    assert_eq!(text_of(paragraphs(&blocks)[0]), ["Some", "text", "in", "the", "middle."]);
}

#[test]
fn the_skip_commands_are_vspace() {
    for (body, pt) in [("a \\smallskip b", 3.0), ("a \\medskip b", 6.0), ("a \\bigskip b", 12.0)] {
        let blocks = blocks(body);
        let paras = paragraphs(&blocks);
        assert_eq!(paras.len(), 1, "{body}: {blocks:?}");
        assert_eq!(adjust_skips(paras[0]), vec![pt], "{body}");
    }
}

#[test]
fn vspace_in_vertical_mode_stays_a_block() {
    for body in [
        "\\vspace{10pt} Some text.",
        "First.\n\n\\vspace{10pt}\n\nSecond.",
        "\\label{x}\\vspace{10pt} Some text.",
        "First.\n\n\\bigskip\nSecond.",
    ] {
        let blocks = blocks(body);
        assert_eq!(vspace_blocks(&blocks), 1, "{body}: {blocks:?}");
        assert!(paragraphs(&blocks).iter().all(|p| adjust_skips(p).is_empty()), "{body}");
    }
}

/// The interword glue nodes, in order: `true` for a run carrying a
/// `glue_before`, `false` for the adjustment.
fn glue_order(body: &str) -> Vec<bool> {
    let blocks = blocks(body);
    paragraphs(&blocks)[0]
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { glue_before: Some(_), .. } => Some(true),
            Inline::VAdjustSkip { .. } => Some(false),
            _ => None,
        })
        .collect()
}

#[test]
fn bsphack_esphack_spacing() {
    // `\@bsphack` saw the space before (`\lastskip` > 0): that glue is in
    // the list before the adjustment, and `\@esphack` skips the spaces
    // after it. One glue in all, ahead of the adjustment.
    assert_eq!(glue_order("a \\vspace{1pt} b"), [true, false]);
    // No space before: the space after is glue, after the adjustment.
    assert_eq!(glue_order("a\\vspace{1pt} b"), [false, true]);
    assert_eq!(glue_order("a\\vspace{1pt}b"), [false]);
    // After `\\` (whose `\@ifnextchar[` took the space before `\vspace`),
    // the space after it is glue behind the adjustment, which is not
    // discarded at the break: pdflatex sets `two` of
    // `\noindent First\\ \vspace{10pt} two` 3.32bp in (one interword space).
    let blocks = blocks("\\noindent First\\\\ \\vspace{10pt} two");
    let para = paragraphs(&blocks)[0];
    let at = para.iter().position(|i| matches!(i, Inline::VAdjustSkip { .. })).expect("an adjustment");
    assert!(matches!(para[at - 1], Inline::LineBreak { .. }), "{para:?}");
    assert!(matches!(&para[at + 1], Inline::Text { text, glue_before: Some(_), .. } if text == "two"), "{para:?}");
}
