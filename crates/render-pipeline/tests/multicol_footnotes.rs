//! `multicols` with `\footnote`: pdflatex keeps the columns and sets the
//! notes at the page foot at full `\textwidth`. FlashTeX used to give up
//! here (`multicols in a document with footnotes is not implemented: the
//! environment's text is set at full width`).
//!
//! Pins, against the pipeline's own documented geometry (article 10pt:
//! `\textwidth` 345pt, `\columnsep` 10pt, `\footnoterule` `0.4\columnwidth`;
//! no TeX runs here):
//! * zero `multicol` diagnostics;
//! * the body in two column bands (wrapped lines starting at the left
//!   margin and one column plus one `\columnsep` right of it, everything
//!   inside `\textwidth`);
//! * the note on one full-width line below the body (too long to fit a
//!   column width without wrapping), under a full-width-fraction rule.

mod common;

use common::*;

const PT_TO_BP: f64 = 72.0 / 72.27;
const TEXTWIDTH_BP: f64 = 345.0 * PT_TO_BP;
const COL_STEP_BP: f64 = (167.5 + 10.0) * PT_TO_BP;
const RULE_BP: f64 = 0.4 * 345.0 * PT_TO_BP;

const INNER_DOC: &str = r"\documentclass{article}
\usepackage{multicol}
\begin{document}
A short full width line before the columns.
\begin{multicols}{2}
First column paragraph with enough words to fill part of a column here.
Second paragraph holding the note\footnote{Zebra footnote text at the foot with several more words to span wider.} and more words after the mark to fill the line.
Third paragraph with additional words so both columns hold several lines.
\end{multicols}
\end{document}
";

const OUTER_DOC: &str = r"\documentclass{article}
\usepackage{multicol}
\begin{document}
A short full width line before the columns.\footnote{Zebra footnote text at the foot with several more words to span wider.}
\begin{multicols}{2}
First column paragraph with enough words to fill part of a column here.
Second paragraph with more words after the first to fill the line well.
Third paragraph with additional words so both columns hold several lines.
\end{multicols}
\end{document}
";

fn check_columns_and_foot(r: &flashtex_render_pipeline::Rendered) {
    let multicol_diags: Vec<_> =
        r.v2.diagnostics
            .iter()
            .filter(|d| d.code == "multicol")
            .map(|d| d.message.clone())
            .collect();
    assert!(
        multicol_diags.is_empty(),
        "multicol diagnostics: {multicol_diags:?}"
    );
    assert_eq!(
        r.v2.pages.len(),
        1,
        "expected one page, got {}",
        r.v2.pages.len()
    );

    let words = words_of(r);

    // Note probes: vocabulary unique to the note text.
    let note: Vec<_> = words
        .iter()
        .filter(|w| matches!(w.text.as_str(), "Zebra" | "footnote" | "wider."))
        .collect();
    assert_eq!(note.len(), 3, "note words: {note:?}");
    // One full-width line: the note is ~280bp long, so it would wrap at a
    // column width (167.5pt) but not at the full width (345pt).
    assert!(
        (note[0].baseline - note[1].baseline).abs() < 0.01
            && (note[1].baseline - note[2].baseline).abs() < 0.01,
        "note wrapped: {note:?}"
    );
    let note_bl = note[0].baseline;

    // The body: everything above the note except the superscript marks (the
    // page-number footer is a "1" below the note).
    let body: Vec<_> = words
        .iter()
        .filter(|w| w.baseline < note_bl - 0.01 && w.text != "1")
        .collect();
    assert!(body.len() > 20, "too few body words: {}", body.len());
    let left = body.iter().map(|w| w.x).fold(f64::INFINITY, f64::min);
    let body_bottom = body.iter().map(|w| w.baseline).fold(0.0f64, f64::max);

    // Wrapped lines start at the left margin (first column) and one column
    // step right (second column): a full-width fallback has only the first.
    let col1 = body.iter().filter(|w| (w.x - left).abs() <= 0.5).count();
    let col2 = body
        .iter()
        .filter(|w| (w.x - (left + COL_STEP_BP)).abs() <= 0.5)
        .count();
    assert!(
        col1 >= 2,
        "no first-column line starts (left {left}): {col1}"
    );
    assert!(col2 >= 2, "no second-column line starts: {col2}");
    for w in &body {
        assert!(
            w.x + w.width <= left + TEXTWIDTH_BP + 0.5,
            "body word outside textwidth: {w:?}"
        );
    }

    // The note below the body, past the `\@makefntext` mark box (1.8em).
    assert!(
        note_bl > body_bottom,
        "note not below the body: {note_bl} vs {body_bottom}"
    );
    assert!(
        (note[0].x - left) >= 10.0 && (note[0].x - left) <= 20.0,
        "note not at full width past its mark box: {} vs {left}",
        note[0].x
    );
    assert!(
        note[2].x + note[2].width <= left + TEXTWIDTH_BP + 0.5,
        "note exceeds textwidth: {:?}",
        note[2]
    );
    let mark = words.iter().find(|w| {
        w.text == "1" && (w.baseline - note_bl).abs() < 5.0 && w.x >= left && w.x <= left + 20.0
    });
    assert!(mark.is_some(), "no note mark before the note");

    // The `\footnoterule` between body and note, `0.4\textwidth` wide.
    let rules = rules_of(r);
    let rule = rules[0]
        .iter()
        .find(|q| (q.2 - RULE_BP).abs() <= 0.5)
        .unwrap_or_else(|| panic!("no footnoterule-width rule: {:?}", rules[0]));
    assert!((rule.0 - left).abs() <= 0.5, "rule x: {rule:?} vs {left}");
    assert!(
        rule.1 > body_bottom && rule.1 < note_bl,
        "rule not between body and note: {rule:?}"
    );
}

#[test]
fn footnote_inside_multicols_keeps_columns() {
    if !lm_available() {
        return;
    }
    check_columns_and_foot(&render_one(INNER_DOC));
}

#[test]
fn footnote_before_multicols_keeps_columns() {
    if !lm_available() {
        return;
    }
    check_columns_and_foot(&render_one(OUTER_DOC));
}
