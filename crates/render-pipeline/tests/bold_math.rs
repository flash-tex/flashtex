//! The bold math version (`\boldsymbol`, bm's `\bm`): the compiler sets the
//! argument's letters, digits, Greek and `\nabla` as Unicode mathematical
//! bold (italic) symbols (`flashtex_compiler::math::bold_math_char`), and the
//! pipeline boxes them from the TFMs pdfLaTeX uses for them (cmmib, cmbsy,
//! cmbx; `crate::mathalpha::bold_math`), painting Latin Modern Math's bold
//! outlines.
//!
//! This test types those Unicode symbols directly, so it holds whichever
//! compiler the pipeline is built against. The expected origins are
//! pdflatex's (TeX Live 2026) for the same line written with `\bm` and
//! `\boldsymbol`:
//!
//! ```tex
//! A $\bm{x}+\boldsymbol{y}$ B $\bm{v}_i \cdot \bm{w}^2$ C
//! $\bm{\alpha}\bm{\beta}\boldsymbol{\omega}$ D $\bm{A}\bm{Mx}=\bm{b}$ E
//! $\bm{2}\bm{\nabla}\bm{\Gamma}$ F $x_{\bm{k}}$ G.
//! ```
//!
//! whose glyphs are CMMIB10 (letters, Greek), CMBX10 (`2`, `\Gamma`),
//! CMBSY10 (`\nabla`) and CMMIB7 (the script `k`). Without the bold metrics
//! the line drifted 14 bp by `G`.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, RenderOptions};
use std::path::PathBuf;

/// `apps/mac/Fonts` and its `texmf` metrics only (as `declared_math_oracle`):
/// the same fonts on every machine, never a host TeX Live's.
fn bundled_fonts() -> flashtex_render_pipeline::fonts::FontSet {
    let fonts = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/mac/Fonts"));
    let tfm = |sub: &str| fonts.join("texmf").join(sub);
    let tfm_dirs = ["fonts/tfm/public/lm", "fonts/tfm/jknappen/ec", "fonts/tfm/public/amsfonts/symbols", "fonts/tfm/public/amsfonts/euler", "fonts/tfm/public/amsfonts/cmextra", "fonts/tfm/public/cm"].map(tfm).to_vec();
    flashtex_render_pipeline::fonts::FontSet::with_dirs(vec![fonts.clone()], tfm_dirs).with_index_dirs(Vec::new())
}

#[test]
fn bold_math_symbols_take_the_bold_math_fonts_metrics() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let text = "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\n\\pagestyle{empty}\n\
        A $\u{1D499}+\u{1D49A}$ B $\u{1D497}_i \\cdot \u{1D498}^2$ C $\u{1D736}\u{1D737}\u{1D74E}$ D \
        $\u{1D468}\u{1D474}\u{1D499}=\u{1D483}$ E $\u{1D7D0}\u{1D6C1}\u{1D6AA}$ F $x_{\u{1D48C}}$ G.\n\\end{document}\n";
    let fonts = bundled_fonts();
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "bold-math", &fonts, &RenderOptions::default());
    let mut glyphs: Vec<(char, f64, f64)> = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.chars().count() != run.glyphs.len() {
            continue;
        }
        for (c, g) in run.text.chars().zip(&run.glyphs) {
            if !c.is_whitespace() {
                glyphs.push((c, g.origin_x.to_bp(), g.baseline_y.to_bp()));
            }
        }
    }
    glyphs.sort_by(|a, b| a.1.total_cmp(&b.1));
    // pdflatex origins (bp) of the line's glyphs, in x order; `A` anchors.
    let expected: [(char, f64, f64); 28] = [
        ('A', 148.712, 134.765),
        ('\u{1D499}', 159.502, 134.765),
        ('+', 168.289, 134.765),
        ('\u{1D49A}', 178.251, 134.765),
        ('B', 187.815, 134.765),
        ('\u{1D497}', 198.196, 134.765),
        ('i', 203.841, 136.259),
        ('\u{22C5}', 209.372, 134.765),
        ('\u{1D498}', 214.353, 134.765),
        ('2', 222.914, 131.149),
        ('C', 230.704, 134.765),
        ('\u{1D736}', 241.215, 134.765),
        ('\u{1D737}', 248.796, 134.765),
        ('\u{1D74E}', 255.71, 134.765),
        ('D', 266.559, 134.765),
        ('\u{1D468}', 277.488, 134.765),
        ('\u{1D474}', 286.146, 134.765),
        ('\u{1D499}', 298.659, 134.765),
        ('=', 307.994, 134.765),
        ('\u{1D483}', 318.514, 134.765),
        ('E', 327.022, 134.765),
        ('\u{1D7D0}', 337.134, 134.765),
        ('\u{1D6C1}', 342.863, 134.765),
        ('\u{1D6AA}', 352.407, 134.765),
        ('F', 362.619, 134.765),
        ('x', 372.442, 134.765),
        ('\u{1D48C}', 378.125, 136.259),
        ('G', 386.793, 134.765),
    ];
    let got: Vec<(char, f64, f64)> = glyphs.into_iter().filter(|g| g.1 < 390.0).collect();
    assert_eq!(got.len(), expected.len(), "{got:?}");
    let (ax, ay) = (got[0].1 - expected[0].1, got[0].2 - expected[0].2);
    for (g, e) in got.iter().zip(&expected) {
        assert_eq!(g.0, e.0, "{got:?}");
        let (dx, dy) = (g.1 - ax - e.1, g.2 - ay - e.2);
        assert!(dx.abs() <= 0.05 && dy.abs() <= 0.05, "{}: dx {dx:+.3} dy {dy:+.3} bp against pdflatex", e.0);
    }
}
