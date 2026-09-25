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

const MIXED: &str = "% \\begin{tikzcd} commented out\n\\begin{tikzpicture}\n\\draw (0,0) -- (1,0);\n\\end{tikzpicture}\n\\begin{tikzcd}[row sep=large]\nX & Y\n\\end{tikzcd}\n\\begin{tikzcd}never closed";

#[test]
fn tikzcd_finder_skips_comments_tikzpicture_and_unclosed() {
    let pics = find_tikzcds(MIXED);
    assert_eq!(pics.len(), 1, "{pics:?}");
    assert!(pics[0].options.is_some(), "the [row sep=large] is located");
    assert!(MIXED[pics[0].body_start..pics[0].body_end].contains("X & Y"), "{pics:?}");
}
