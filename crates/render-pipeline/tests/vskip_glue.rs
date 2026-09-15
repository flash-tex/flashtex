//! `\vspace` stretch/shrink survive the adapter's `pending_vspace` path.
//!
//! Slice 2 of GH-VSKIP-GLUE-STRETCH: the compiler's `Block::VSpace` carries
//! `stretch_pt`/`shrink_pt` (slice 1, PR #606) and `adapter.rs` accumulates
//! them into `Block::Paragraph::vspace_flex` beside `vspace_before` (the gap
//! re-read reports the same glue as the existing `style::Skip`, not a new
//! type). Needs a `vendor/compiler` re-pin past #606: the frozen vendor
//! still reports flat `VSpace { pt }` with no rubber, so this file only
//! passes once the new fields are pinned.

use flashtex_compiler::parser::parse;
use flashtex_render_pipeline::adapter::{self, Block, Labels};
use flashtex_render_pipeline::RenderOptions;

/// `(vspace_before, vspace_flex)` of every adapter paragraph, in order.
fn paragraph_skips(src: &str) -> Vec<(f64, (f64, f64))> {
    let parsed = parse(src);
    let texts = [src];
    let doc = adapter::adapt(&texts, 0, &parsed, &RenderOptions::default(), &Labels::default());
    doc.blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph { vspace_before, vspace_flex, .. } => Some((*vspace_before, *vspace_flex)),
            _ => None,
        })
        .collect()
}

fn body(before_second: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\begin{{document}}\nFirst paragraph.\n\n{before_second}\n\nSecond paragraph.\n\\end{{document}}\n"
    )
}

/// Explicit glue: the natural width and both rubber components reach the
/// second paragraph (via the gap re-read, which parses `plus`/`minus` at
/// the class size).
#[test]
fn vspace_plus_minus_reaches_paragraph_flex() {
    let src = body("\\vspace{12pt plus 4pt minus 2pt}");
    let skips = paragraph_skips(&src);
    assert_eq!(skips.len(), 2, "two paragraphs, got {skips:?}");
    assert_eq!(skips[0], (0.0, (0.0, 0.0)));
    assert_eq!(skips[1], (12.0, (4.0, 2.0)));
}

/// `\bigskip` is real LaTeX glue (`12pt plus 4pt minus 4pt`): no literal
/// `\vspace` stands in the gap, so this exercises the `CBlock::VSpace`
/// accumulation path itself rather than the re-read.
#[test]
fn bigskip_carries_its_latex_glue() {
    let src = body("\\bigskip");
    let skips = paragraph_skips(&src);
    assert_eq!(skips.len(), 2, "two paragraphs, got {skips:?}");
    assert_eq!(skips[0], (0.0, (0.0, 0.0)));
    assert_eq!(skips[1], (12.0, (4.0, 4.0)));
}

/// A bare `\vspace` has no rubber: the flat path is unchanged.
#[test]
fn plain_vspace_stays_rigid() {
    let src = body("\\vspace{6pt}");
    let skips = paragraph_skips(&src);
    assert_eq!(skips.len(), 2, "two paragraphs, got {skips:?}");
    assert_eq!(skips[0], (0.0, (0.0, 0.0)));
    assert_eq!(skips[1], (6.0, (0.0, 0.0)));
}
