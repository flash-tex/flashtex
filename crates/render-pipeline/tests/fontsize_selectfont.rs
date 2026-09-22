//! NFSS `\fontsize{<size>}{<skip>}\selectfont` sets its exact size and
//! baselineskip (the compiler's `FontSizeLevel::Explicit`), with the font
//! the family's `.fd` loads for that size. Expected positions are
//! pdflatex's (TeX Live 2026, MacTeX; oracle only), read with
//! `tools/visual-oracle/pdftext.py`: word origin x and baseline y, in bp.
//!
//! - OT1 `cmr` has no 13pt: LaTeX substitutes `cmr12` (11.955 bp glyphs),
//!   and the paragraph's lines are 15pt (14.944 bp) apart.
//! - `lmodern` in T1 scales: `ec-lmr12` at 13pt (12.951 bp).

mod common;

use flashtex_render_pipeline::display::Item;

const PARA: &str = "{\\fontsize{13}{15}\\selectfont Some larger words that run on for a while so that the paragraph wraps over at least two lines of the page here.\\par}";

/// `(text, x, baseline, font size)` of every glyph run on page 1, in bp.
fn runs(preamble: &str) -> Vec<(String, f64, f64, f64)> {
    let source = format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\nBefore text here.\n\n{PARA}\n\nAfter text here.\n\\end{{document}}\n");
    let r = common::render_docs(&[("main.tex", &source)], "main.tex");
    assert!(r.v2.diagnostics.is_empty(), "{:?}", r.v2.diagnostics);
    r.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run) => run.glyphs.first().map(|g| (run.text.clone(), g.origin_x.to_bp(), g.baseline_y.to_bp(), run.font_size.to_bp())),
            _ => None,
        })
        .collect()
}

fn check(runs: &[(String, f64, f64, f64)], word: &str, x: f64, y: f64, size: f64) {
    let (_, gx, gy, gs) = runs.iter().find(|(t, ..)| t == word).unwrap_or_else(|| panic!("no run {word:?} in {runs:?}"));
    assert!((gx - x).abs() < 0.01 && (gy - y).abs() < 0.01, "{word}: ({gx:.3}, {gy:.3}), pdflatex ({x:.3}, {y:.3})");
    assert!((gs - size).abs() < 0.01, "{word}: size {gs:.3} bp, pdflatex {size:.3}");
}

#[test]
fn ot1_cmr_substitutes_twelve_point_at_a_fifteen_point_leading() {
    assert!(common::lm_available());
    let runs = runs("");
    check(&runs, "Some", 148.712, 149.709, 11.955);
    check(&runs, "paragraph", 425.449, 149.709, 11.955);
    check(&runs, "wraps", 133.768, 164.653, 11.955);
    check(&runs, "After", 148.712, 176.608, 9.963);
}

#[test]
fn latin_modern_sets_the_exact_thirteen_points() {
    assert!(common::lm_available());
    let runs = runs("\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n");
    check(&runs, "Some", 148.712, 149.709, 12.951);
    check(&runs, "least", 249.030, 164.653, 12.951);
    check(&runs, "page", 367.880, 164.653, 12.951);
}
