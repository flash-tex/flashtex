//! LaTeX's long arrows are joins, not glyphs: `\Longrightarrow` is
//! `\Relbar\joinrel\Rightarrow` (cmr `=`, `\mkern-3mu`, cmsy `⇒`), and
//! `\implies`/`\iff`/`\impliedby` add a thick space on each side. They used
//! to be Latin Modern Math's single U+27F9-style glyphs, 1.457em wide where
//! pdfTeX's join is 1.611em, which moved HW1 Problem 4(b)'s display by
//! 0.84bp on each side of its `\Longrightarrow`.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

/// The math glyph origins and run texts of `$x<arrow>y$` alone in an 11pt
/// paragraph.
fn formula(arrow: &str) -> (Vec<f64>, String) {
    let r = render_one(&format!("\\documentclass[11pt]{{article}}\\usepackage{{amsmath}}\\begin{{document}}\n${arrow} y$\n\\end{{document}}").replace('$', "$x"));
    let mut xs = Vec::new();
    let mut text = String::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            if run.role == RunRole::Math {
                xs.extend(run.glyphs.iter().map(|g| g.origin_x.to_bp()));
                text.push_str(&run.text);
            }
        }
    }
    xs.sort_by(f64::total_cmp);
    (xs, text)
}

/// pdfTeX (TeX Live 2026, 11pt, amsmath): the distance from `x`'s origin to
/// `y`'s in `$x<arrow>y$`, i.e. `\wd\hbox{$x<arrow>y$}` minus `\wd\hbox{$y$}`.
#[test]
fn long_arrows_are_as_wide_as_pdftex_joins() {
    if !lm_available() {
        return;
    }
    let cases = [
        ("\\longleftarrow", 29.9832, "x←−y"),
        ("\\longrightarrow", 29.9832, "x−→y"),
        ("\\longleftrightarrow", 32.41653, "x←→y"),
        ("\\Longleftarrow", 29.9832, "x⇐=y"),
        ("\\Longrightarrow", 29.9832, "x=⇒y"),
        ("\\Longleftrightarrow", 32.41653, "x⇐⇒y"),
        ("\\implies", 36.06642, "x=⇒y"),
        ("\\iff", 38.49976, "x⇐⇒y"),
        ("\\impliedby", 36.06642, "x⇐=y"),
        // GH-LONGMAPSTO-PIECES: `\longmapsto` is `\mapstochar\longrightarrow`,
        // bit-for-bit as wide as `\longrightarrow` (the flag nets zero width).
        // Spelled as the literal U+27FC character because the vendored
        // compiler pin on this branch predates PR #603's `("longmapsto", "⟼")`
        // row, so the command does not reach the pipeline yet; the character
        // exercises the identical `symbol_atoms` path. Switch this row to
        // `"\\longmapsto"` when vendor/compiler is re-pinned past #603.
        ("\u{27FC}", 29.9832, "x∣−→y"),
    ];
    for (arrow, pt, painted) in cases {
        let (xs, text) = formula(arrow);
        assert_eq!(
            xs.len(),
            painted.chars().count(),
            "{arrow}: x, pieces, y: {text:?}"
        );
        assert_eq!(
            text.replace('\u{2212}', "−").replace('-', "−"),
            painted,
            "{arrow}: pieces"
        );
        let span = xs[xs.len() - 1] - xs[0];
        assert!((span - bp(pt)).abs() < 0.02, "{arrow}: x to y {span:.4}bp, pdfTeX {:.4}bp", bp(pt));
    }
}
