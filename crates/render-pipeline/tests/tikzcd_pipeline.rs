//! A `tikzcd` commutative diagram lowers to stroked arrow shafts, filled
//! heads and one text placement per cell; environment options and
//! out-of-scope arrow keys degrade to warnings, never silent drops.
//!
//! Font-free: cells are measured with `ApproxMeasurer`, so the geometry is
//! deterministic with no TeX Live install (unlike `tikz_pipeline.rs`, which
//! needs Latin Modern). End-to-end routing through `adapter`/`typeset` is a
//! later slice; this covers the lowering itself.

use flashtex_render_pipeline::tikz::{find_tikzcds, render_tikzcd};
use flashtex_vector_graphics::tikz::ApproxMeasurer;
use flashtex_vector_graphics::Item;

const SRC: &str = "Before.\n\n\\begin{tikzcd}\nA \\arrow[r] \\arrow[d] & B \\\\\nC & D\n\\end{tikzcd}\n\nAfter.\n";

#[test]
fn tikzcd_matrix_lowers_to_cells_and_arrows() {
    let pics = find_tikzcds(SRC);
    assert_eq!(pics.len(), 1, "{pics:?}");
    assert!(SRC[pics[0].body_start..pics[0].body_end].contains("\\arrow"), "{pics:?}");

    let pic = render_tikzcd(SRC, &pics[0], &ApproxMeasurer, 10.0);
    assert!(pic.diagnostics.is_empty(), "{:?}", pic.diagnostics);

    let texts: Vec<&str> = pic.texts.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(texts, ["A", "B", "C", "D"], "{texts:?}");

    let mut strokes = 0;
    let mut fills = 0;
    for item in &pic.items {
        match item {
            Item::PathStroke(_) => strokes += 1,
            Item::PathFill(_) => fills += 1,
            _ => {}
        }
    }
    assert_eq!((strokes, fills), (2, 2), "one shaft and one head per arrow");
    assert!(pic.width_bp > 0.0 && pic.height_bp > 0.0, "{}bp x {}bp", pic.width_bp, pic.height_bp);

    for t in &pic.texts {
        assert_eq!(t.after_item, pic.items.len(), "cells paint after the arrows");
        assert!(t.source.0 >= pics[0].start && t.source.1 <= pics[0].end, "{t:?}");
    }
}

const SCOPED: &str = "\\begin{tikzcd}[column sep=small]\nA \\arrow[x] & B \\arrow[r] \\\\\nC & D\n\\end{tikzcd}";

#[test]
fn tikzcd_scope_limits_warn_instead_of_dropping() {
    let pics = find_tikzcds(SCOPED);
    assert_eq!(pics.len(), 1);
    assert!(pics[0].options.is_some(), "the [column sep=small] is located");

    let pic = render_tikzcd(SCOPED, &pics[0], &ApproxMeasurer, 10.0);
    // Picture options, the unknown `x` key, the direction-less arrow and
    // B's arrow off the diagram's edge: four warnings, no items.
    assert_eq!(pic.diagnostics.len(), 4, "{:?}", pic.diagnostics);
    let messages: Vec<&str> = pic.diagnostics.iter().map(|d| d.message.as_str()).collect();
    for want in ["picture options", "unsupported", "without a direction", "outside the diagram"] {
        assert!(messages.iter().any(|m| m.contains(want)), "{messages:?}");
    }
    assert!(pic.items.is_empty(), "{:?}", pic.items);
    let texts: Vec<&str> = pic.texts.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(texts, ["A", "B", "C", "D"], "{texts:?}");
}

const GROUPED_AMP_EXACT: &str = "\\begin{tikzcd}A \\arrow[r, \"x \\& y\"] & B \\\\ C & D\\end{tikzcd}";
const GROUPED_AMP_BRACED: &str = "\\begin{tikzcd}A \\arrow[r, \"x {y & z}\"] & B \\\\ C & D\\end{tikzcd}";

#[test]
fn tikzcd_grouped_ampersand_stays_in_its_cell() {
    // The reviewer's reported shape (escaped `\&` label) plus a braced `&`
    // inside the label that actually exercises group-depth tracking: both
    // must keep the first row at exactly 2 cells with the r-arrow landing
    // on B (one shaft, one head, one unsupported-label warning).
    for src in [GROUPED_AMP_EXACT, GROUPED_AMP_BRACED] {
        let pics = find_tikzcds(src);
        assert_eq!(pics.len(), 1, "{src:?} {pics:?}");
        let pic = render_tikzcd(src, &pics[0], &ApproxMeasurer, 10.0);
        let texts: Vec<&str> = pic.texts.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["A", "B", "C", "D"], "{src:?} {texts:?}");
        let mut strokes = 0;
        let mut fills = 0;
        for item in &pic.items {
            match item {
                Item::PathStroke(_) => strokes += 1,
                Item::PathFill(_) => fills += 1,
                _ => {}
            }
        }
        assert_eq!((strokes, fills), (1, 1), "{src:?} {texts:?} {:?}", pic.diagnostics);
        assert_eq!(pic.diagnostics.len(), 1, "{src:?} {:?}", pic.diagnostics);
        assert!(pic.diagnostics[0].message.contains("unsupported"), "{src:?} {:?}", pic.diagnostics);
    }
}

const GROUPED_ROWBREAK: &str = "\\begin{tikzcd}{A \\\\ B} & C \\\\ D & E\\end{tikzcd}";

#[test]
fn tikzcd_grouped_rowbreak_stays_in_its_row() {
    let pics = find_tikzcds(GROUPED_ROWBREAK);
    assert_eq!(pics.len(), 1, "{pics:?}");
    let pic = render_tikzcd(GROUPED_ROWBREAK, &pics[0], &ApproxMeasurer, 10.0);
    let texts: Vec<&str> = pic.texts.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(texts, ["{A \\\\ B}", "C", "D", "E"], "{texts:?}");
    assert!(pic.diagnostics.is_empty(), "{:?}", pic.diagnostics);
}

const UNCLOSED_ARROW: &str = "\\begin{tikzcd}A & B \\arrow[r\\end{tikzcd}";

#[test]
fn tikzcd_unclosed_arrow_bracket_warns() {
    let pics = find_tikzcds(UNCLOSED_ARROW);
    assert_eq!(pics.len(), 1, "{pics:?}");
    let pic = render_tikzcd(UNCLOSED_ARROW, &pics[0], &ApproxMeasurer, 10.0);
    let texts: Vec<&str> = pic.texts.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(texts, ["A", "B"], "{texts:?}");
    // The dropped `\arrow[r` tail must leave a trace, spanning from the
    // `\arrow` token so the user can find the mistake.
    assert_eq!(pic.diagnostics.len(), 1, "{:?}", pic.diagnostics);
    let d = &pic.diagnostics[0];
    assert!(d.message.contains("unclosed"), "{d:?}");
    assert!(UNCLOSED_ARROW[d.start..d.end].starts_with("\\arrow"), "{d:?}");
    assert!(pic.items.is_empty(), "{:?}", pic.items);
}

const STRAY_CLOSER: &str = "\\begin{tikzcd}A $]$ & B \\\\ C & D\\end{tikzcd}";

#[test]
fn tikzcd_stray_closer_does_not_swallow_later_cells() {
    // A `]` with no opener in the same row must not drive the split depth
    // negative and silently merge the rest of the row into one cell.
    let pics = find_tikzcds(STRAY_CLOSER);
    assert_eq!(pics.len(), 1, "{pics:?}");
    let pic = render_tikzcd(STRAY_CLOSER, &pics[0], &ApproxMeasurer, 10.0);
    let texts: Vec<&str> = pic.texts.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(texts, ["A $]$", "B", "C", "D"], "{texts:?}");
    assert!(pic.diagnostics.is_empty(), "{:?}", pic.diagnostics);
}

const UNCLOSED_SPACING: &str = "\\begin{tikzcd}A \\\\ [unclosed\\end{tikzcd}";
const CLOSED_SPACING: &str = "\\begin{tikzcd}A \\\\ [2pt] B \\\\ C & D\\end{tikzcd}";

#[test]
fn tikzcd_unclosed_row_spacing_warns_without_phantom_row() {
    // An unclosed `[` after `\\` is row content, not spacing: no empty
    // trailing row, and the mistake leaves a warning trace.
    let pics = find_tikzcds(UNCLOSED_SPACING);
    assert_eq!(pics.len(), 1, "{pics:?}");
    let pic = render_tikzcd(UNCLOSED_SPACING, &pics[0], &ApproxMeasurer, 10.0);
    let texts: Vec<&str> = pic.texts.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(texts, ["A", "[unclosed"], "{texts:?}");
    assert_eq!(pic.diagnostics.len(), 1, "{:?}", pic.diagnostics);
    assert!(pic.diagnostics[0].message.contains("unclosed"), "{:?}", pic.diagnostics);

    // A closed `[2pt]` is still spacing: skipped with the usual warning,
    // and the row text after it survives.
    let pics = find_tikzcds(CLOSED_SPACING);
    assert_eq!(pics.len(), 1, "{pics:?}");
    let pic = render_tikzcd(CLOSED_SPACING, &pics[0], &ApproxMeasurer, 10.0);
    let texts: Vec<&str> = pic.texts.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(texts, ["A", "B", "C", "D"], "{texts:?}");
    assert_eq!(pic.diagnostics.len(), 1, "{:?}", pic.diagnostics);
    assert!(pic.diagnostics[0].message.contains("row spacing"), "{:?}", pic.diagnostics);
}

const MIXED: &str = "% \\begin{tikzcd} commented out\n\\begin{tikzpicture}\n\\draw (0,0) -- (1,0);\n\\end{tikzpicture}\n\\begin{tikzcd}[row sep=large]\nX & Y\n\\end{tikzcd}\n\\begin{tikzcd}never closed";

#[test]
fn tikzcd_finder_skips_comments_tikzpicture_and_unclosed() {
    let pics = find_tikzcds(MIXED);
    assert_eq!(pics.len(), 1, "{pics:?}");
    assert!(pics[0].options.is_some(), "the [row sep=large] is located");
    assert!(MIXED[pics[0].body_start..pics[0].body_end].contains("X & Y"), "{pics:?}");
}
