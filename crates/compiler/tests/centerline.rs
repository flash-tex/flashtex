//! Kernel `\centerline`, `\leftline` and `\rightline`.
//!
//! latex.ltx: `\centerline{#1}` is `\line{\hss #1\hss}` (and the other two
//! are the one-sided forms), `\line` is `\hb@xt@\hsize`. Each call is
//! `\par\hb@xt@\hsize{...}\par`: it ends the running paragraph, sets its
//! argument as one full-measure line of its own (centred / flush left /
//! flush right), and leaves vertical mode, so following text starts a fresh
//! paragraph. They used to be rejected with three `unknown_command` errors
//! and their arguments ran together into one line.
//!
//! ## Oracle
//!
//! pdfTeX 1.40.29 (TeX Live 2026, `/Library/TeX/texbin/pdflatex`,
//! `pdflatex -interaction=nonstopmode probe.tex`) on
//! ```tex
//! \documentclass{article}
//! \pagestyle{empty}
//! \begin{document}
//! \centerline{K}\leftline{L}\rightline{M}N \mbox{} \par
//! \end{document}
//! ```
//! with glyph origins from the PDF content stream
//! (`python3 tools/visual-oracle/pdftext.py probe.pdf`), in bp, baseline
//! from the page top, zero errors:
//!
//! ```text
//! 301.750  134.765  CMR10  K
//! 133.768  146.720  CMR10  L
//! 468.347  158.675  CMR10  M
//! 148.712  170.630  CMR10  N
//! ```
//! (`\showthe` widths: K 7.7778pt, L 6.25002pt, M 9.16669pt, N 7.50002pt;
//! `\textwidth` 345pt, `\parindent` 15pt, `\baselineskip` 12pt. K is centred:
//! 301.750 + 7.7778pt/2 = the measure's centre; M is flush right; N is
//! indented 15pt; baselines are 12pt apart. pdflatex never runs in the
//! product path.)
//!
//! This crate's Core 14 layout keeps its own page frame (72pt margins, Times
//! metrics, no `\parindent`), so absolute bp positions are not asserted here:
//! the parse shape (four blocks, three `Styled`) plus the measure-relative
//! alignment (left edge, centred, right edge) are. The TFM-backed render
//! pipeline reproduces the oracle positions above once it re-pins this
//! compiler.
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::layout::{text_width, Font};
use flashtex_compiler::parser::{parse, Block, Inline, ParagraphStyle};

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn first_text(content: &[Inline]) -> &str {
    match content.first() {
        Some(Inline::Text { text, .. }) => text.as_str(),
        other => panic!("not leading text: {other:?}"),
    }
}

#[test]
fn line_boxes_are_own_styled_blocks_without_diagnostics() {
    let parsed = parse(&document("\\centerline{K}\\leftline{L}\\rightline{M}N \\mbox{} \\par"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.blocks.len(), 4, "{:?}", parsed.blocks);
    // The three line boxes come first, each on its own block, then N's
    // paragraph.
    let mut styles = Vec::new();
    for block in &parsed.blocks {
        match block {
            Block::Styled { style, content: _, lists, line_break_before } => {
                assert!(lists.is_empty(), "{block:?}");
                assert_eq!(*line_break_before, None, "{block:?}");
                styles.push(*style);
            }
            Block::Paragraph(content) => assert_eq!(first_text(content), "N", "{block:?}"),
            other => panic!("unexpected block: {other:?}"),
        }
    }
    assert_eq!(
        styles,
        [
            ParagraphStyle::Center,
            ParagraphStyle::FlushLeft,
            ParagraphStyle::FlushRight
        ],
        "{:?}",
        parsed.blocks
    );
    let texts: Vec<&str> = parsed
        .blocks
        .iter()
        .take(3)
        .map(|block| match block {
            Block::Styled { content, .. } => first_text(content),
            other => panic!("not styled: {other:?}"),
        })
        .collect();
    assert_eq!(texts, ["K", "L", "M"]);
}

#[test]
fn line_box_splits_a_running_paragraph() {
    let parsed = parse(&document("Before \\centerline{K} after."));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.blocks.len(), 3, "{:?}", parsed.blocks);
    match &parsed.blocks[0] {
        Block::Paragraph(content) => assert_eq!(first_text(content), "Before", "{content:?}"),
        other => panic!("{other:?}"),
    }
    match &parsed.blocks[1] {
        Block::Styled { style, content, .. } => {
            assert_eq!(*style, ParagraphStyle::Center);
            assert_eq!(first_text(content), "K");
        }
        other => panic!("{other:?}"),
    }
    match &parsed.blocks[2] {
        Block::Paragraph(content) => assert_eq!(first_text(content), "after.", "{content:?}"),
        other => panic!("{other:?}"),
    }
}

/// Core 14 page frame (`layout.rs`: US Letter, 72pt margins, so the measure
/// is 468pt wide with its centre at 306pt): each line box sits on its own
/// baseline at its own alignment, and N follows on a fourth line.
#[test]
fn line_boxes_align_to_the_measure_in_layout() {
    let output = compile_full(
        &document("\\centerline{K}\\leftline{L}\\rightline{M}N \\mbox{} \\par"),
        LayoutConstraints::default(),
    );
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(output.pages.len(), 1, "{:?}", output.pages);
    let items = &output.pages[0].items;
    let at = |text: &str| {
        items
            .iter()
            .find(|item| item.text == text)
            .unwrap_or_else(|| panic!("no {text:?} in {items:?}"))
    };
    let (k, l, m, n) = (at("K"), at("L"), at("M"), at("N"));
    // One line each, in source order.
    assert!(
        k.baseline_y_pt < l.baseline_y_pt
            && l.baseline_y_pt < m.baseline_y_pt
            && m.baseline_y_pt < n.baseline_y_pt,
        "{items:?}"
    );
    // Uniform leading: no extra glue between the line boxes.
    let gaps = [
        l.baseline_y_pt - k.baseline_y_pt,
        m.baseline_y_pt - l.baseline_y_pt,
        n.baseline_y_pt - m.baseline_y_pt,
    ];
    assert!(
        gaps.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01),
        "{gaps:?}"
    );
    // Left edge, centre and right edge of the 72pt-margined measure.
    assert!((l.x_pt - 72.0).abs() < 0.01, "{l:?}");
    let size = k.font_size_pt;
    assert!((k.x_pt + text_width("K", size, Font::TimesRoman) / 2.0 - 306.0).abs() < 0.01, "{k:?}");
    assert!((m.x_pt + text_width("M", size, Font::TimesRoman) - 540.0).abs() < 0.01, "{m:?}");
}
