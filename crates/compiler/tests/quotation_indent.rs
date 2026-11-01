//! `quotation` first-line indent: article.cls opens `quotation` as a `\list`
//! with `\listparindent 1.5em`, which `\list` copies to `\parindent`, so
//! every paragraph's first line is indented; `quote` sets no
//! `\listparindent`, so its paragraphs start at the margin.
//!
//! Oracle (TeX Live 2026, article 10pt letter):
//! `pdflatex -interaction=nonstopmode quot.tex` over
//! `Intro.` + `\begin{quotation}` (a wrapping first paragraph, a short
//! second one) puts the quotation first lines at x=173.62bp and the
//! continuation line at x=158.68bp — a 14.94bp = 1.5em indent — while the
//! same body in `quote` starts every line at x=158.68bp.
//! (`pdftotext -bbox`, first `<word>` of each `<page>` line.)
//!
//! This layout's units are TeX points, so the indent is 15.0pt here
//! (15pt = 14.944bp, within 0.1bp of the measured 14.94bp); the margin is
//! the layout's own `MARGIN_PT + QUOTE_INDENT_PT`, not article's 133.77bp.

use flashtex_compiler::layout::{self, MARGIN_PT, QUOTE_INDENT_PT, TextItem};
use flashtex_compiler::parser::{self, Block, ListEnvironment, ListLength, ListOption};

/// The layout's quotation margin: every continuation line starts here.
const MARGIN: f64 = MARGIN_PT + QUOTE_INDENT_PT;
/// `\listparindent 1.5em` at the 10pt class size: the first-line indent.
const INDENT_PT: f64 = 15.0;
/// Tight tolerance (0.1bp or less): the expected values are exact, only
/// `round2` on placed items can move them.
const TOL: f64 = 0.06;

fn doc(environment: &str) -> String {
    format!(
        "\\documentclass[10pt]{{article}}\n\\begin{{document}}\nIntro.\n\n\\begin{{{environment}}}\nFirst para of the block with enough words to wrap onto a second line for measurement purposes here.\n\nSecond para.\n\\end{{{environment}}}\n\\end{{document}}\n"
    )
}

fn laid_out(source: &str) -> Vec<TextItem> {
    let parsed = parser::parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    layout::layout(&parsed.blocks)
        .into_iter()
        .flat_map(|page| page.items)
        .collect()
}

/// The x and first word of each laid-out line, in layout order.
fn line_starts(items: &[TextItem]) -> Vec<(f64, String)> {
    let mut lines: Vec<(f64, f64, String)> = Vec::new();
    for item in items {
        let same_line =
            lines.last().is_some_and(|(y, _, _)| (y - item.baseline_y_pt).abs() < 0.01);
        if !same_line {
            lines.push((item.baseline_y_pt, item.x_pt, item.text.clone()));
        }
    }
    lines.into_iter().map(|(_, x, text)| (x, text)).collect()
}

fn styled_frames(source: &str) -> Vec<Vec<flashtex_compiler::parser::ListFrame>> {
    parser::parse(source)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Styled { lists, .. } => Some(lists.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn quotation_frame_carries_listparindent_1_5em() {
    let styled = styled_frames(&doc("quotation"));
    assert_eq!(styled.len(), 2, "two quotation paragraphs");
    for lists in &styled {
        let frame = lists.last().expect("quotation pushes a list frame");
        assert_eq!(frame.environment, ListEnvironment::Quotation);
        assert_eq!(frame.options.len(), 1, "{:?}", frame.options);
        match frame.options[0] {
            ListOption::ListParIndent(ListLength::Pt(pt)) => assert!(
                (pt - INDENT_PT).abs() < 1e-3,
                "\\listparindent 1.5em at 10pt is 15.0pt, got {pt}"
            ),
            ref other => panic!("quotation frame must carry listparindent, got {other:?}"),
        }
    }
}

#[test]
fn quotation_first_lines_indented_continuations_at_margin() {
    let lines = line_starts(&laid_out(&doc("quotation")));
    assert!(
        lines.len() >= 4,
        "Intro. + a wrapping first paragraph + Second para.: {lines:?}"
    );
    assert!(
        (lines[0].0 - MARGIN_PT).abs() < TOL,
        "Intro. stays at the body margin: {lines:?}"
    );
    assert_eq!(lines[1].1, "First");
    assert!(
        (lines[1].0 - (MARGIN + INDENT_PT)).abs() < TOL,
        "first line of the first quotation paragraph at margin + 1.5em (oracle 173.62bp): {lines:?}"
    );
    for line in &lines[2..lines.len() - 1] {
        assert!(
            (line.0 - MARGIN).abs() < TOL,
            "continuation line at the quotation margin (oracle 158.68bp): {lines:?}"
        );
    }
    let last = lines.last().unwrap();
    assert_eq!(last.1, "Second");
    assert!(
        (last.0 - (MARGIN + INDENT_PT)).abs() < TOL,
        "first line of the second quotation paragraph at margin + 1.5em (oracle 173.62bp): {lines:?}"
    );
}

#[test]
fn quote_paragraphs_start_at_margin() {
    for lists in &styled_frames(&doc("quote")) {
        assert!(
            lists.iter().all(|frame| frame.listparindent().is_none()),
            "quote sets no \\listparindent: {lists:?}"
        );
    }
    let lines = line_starts(&laid_out(&doc("quote")));
    assert!(
        lines.len() >= 4,
        "Intro. + a wrapping first paragraph + Second para.: {lines:?}"
    );
    for line in &lines[1..] {
        assert!(
            (line.0 - MARGIN).abs() < TOL,
            "every quote line starts at the margin, first lines included (oracle 158.68bp): {lines:?}"
        );
    }
}
