//! A math symbol does not make its line taller than pdflatex makes it.
//!
//! A math glyph's height and depth come from the painted CFF outline's ink
//! box (`MathFonts::glyph_from` -> `LoadedFace::bounds`), not from a TFM, so
//! a wrong ink box feeds straight into the math box's height, into the line
//! box, and — through `\topskip` — into where the *first baseline of the
//! page* lands. Nothing else in the suite pins that path, which is how the
//! Type 2 width bug fixed alongside this file went unnoticed for `\ll` and
//! `\gg`: the charstrings for U+226A and U+226B declare the optional leading
//! width on their first `rmoveto`, the width was never consumed, the outline
//! was traced translated, and their ink boxes came out 1.46 pt and 2.37 pt
//! too tall. Every line containing `$a \ll b$` therefore started that much
//! lower down the page than pdflatex starts it.
//!
//! The reference is pdfTeX 3.141592653 (TeX Live 2026) on
//!
//! ```tex
//! \documentclass[10pt]{article}\begin{document}$a\ll b$ and more text here.\end{document}
//! ```
//!
//! whose first baseline is 135.27038 pt below the top of the 792 bp page —
//! the plain `\topskip` baseline, identical for every symbol here, because
//! TeX takes the box's height from cmsy/cmmi's TFM and no kernel math symbol
//! is tall enough to push `\topskip` past 10 pt.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const BP: f64 = 72.0 / 72.27;
/// The project's gate for a glyph position.
const GLYPH_GATE: f64 = 0.5 * BP;

/// pdfTeX's first baseline for every document below, in TeX pt from the top
/// of the page.
const FIRST_BASELINE: f64 = 135.27038;

/// The baseline the most glyphs of page 1 share, in TeX pt from the page top.
fn line_baseline(body: &str) -> f64 {
    let src =
        format!("\\documentclass[10pt]{{article}}\\begin{{document}}{body}\\end{{document}}");
    let r = render_one(&src);
    let mut ys: Vec<f64> = Vec::new();
    for page in r.v2.pages.iter().take(1) {
        for it in page.resident_items() {
            if let Item::GlyphRun(run) = it {
                for g in &run.glyphs {
                    let y = g.baseline_y.to_bp() / BP;
                    // Everything but the page number in the footer.
                    if y < 600.0 {
                        ys.push(y);
                    }
                }
            }
        }
    }
    assert!(!ys.is_empty(), "no glyphs painted for {body:?}");
    let mut best = (0usize, f64::MAX);
    for &y in &ys {
        let n = ys.iter().filter(|o| (**o - y).abs() < 1e-6).count();
        if n > best.0 || (n == best.0 && y < best.1) {
            best = (n, y);
        }
    }
    best.1
}

#[test]
fn a_math_symbol_leaves_the_first_baseline_where_pdflatex_leaves_it() {
    if !lm_available() {
        return;
    }
    // `\ll` and `\gg` are the two the CFF width bug moved; the rest are the
    // neighbouring relations and binary operators from the same cmsy slots,
    // as controls that the gate is not vacuous.
    for cmd in [
        "\\ll", "\\gg", "\\le", "\\ge", "\\neq", "\\equiv", "\\sim", "\\approx", "\\prec",
        "\\succ", "\\subset", "\\supseteq", "\\in", "\\perp", "\\pm", "\\times", "\\div",
        "\\cap", "\\cup", "\\vee", "\\wedge", "\\setminus", "\\oplus", "\\otimes", "\\odot",
        "\\diamond", "\\bigcirc", "\\dagger",
    ] {
        let body = format!("$a{cmd} b$ and more text here.");
        let got = line_baseline(&body);
        assert!(
            (got - FIRST_BASELINE).abs() <= GLYPH_GATE,
            "{cmd}: first baseline {got:.5} pt, pdflatex {FIRST_BASELINE:.5} pt \
             (off by {:.5} pt, gate {GLYPH_GATE:.5} pt)",
            got - FIRST_BASELINE
        );
    }
}
