//! TeX's math font kerns (`make_ord`, tex.web §752) through the pipeline.
//!
//! An Ord math character followed by a character of the same family gets the
//! family font's kern: cmmi10 `r` `,` is -0.055555 em, -0.606 pt at 10.95 pt.
//! HW1 (`fixtures/real-world/hw1`, issue #66) sets `$r,a,b$` and the display
//! `a+r, ar, a+b, ab.`, where every glyph after an `r,` sat 0.61 bp right of
//! pdflatex's (two missing kerns in the display, so ±0.61 bp around its
//! centre).
//!
//! ## Oracle
//!
//! [`PROBE`] through `pdflatex` (pdfTeX 1.40.29, TeX Live 2026,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`); every glyph origin of its PDF
//! read with `tools/visual-oracle/pdftext.py`, grouped by baseline. The
//! numbers are in [`PDFLATEX`]. pdflatex is an oracle only and never runs in
//! the product path.
//!
//! Needs math-layout's `MathFontMetrics::ord_pair`, so the whole file is
//! behind the `math-font-kerns` feature (see `Cargo.toml`): the pinned
//! `vendor/math-layout` predates it.
#![cfg(feature = "math-font-kerns")]

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const PROBE: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage{amsmath}
\pagestyle{empty}
\setlength{\parindent}{0pt}
\begin{document}
$r,a,b$

$f,P,Y.$

$f(x)$ $\Gamma,\Delta$ $x_1,x_2$ $V_{a}$

\[
    a+r,
    \qquad
    ar,
    \qquad
    a+b,
    \qquad
    ab.
\]
\end{document}
";

/// `(what, baseline, glyph origins left to right)`, in bp from the page's
/// top-left corner.
const PDFLATEX: &[(&str, f64, &[f64])] = &[
    ("$r,a,b$", 140.742, &[125.798, 130.415, 135.267, 141.034, 145.875]),
    ("$f,P,Y.$", 154.291, &[125.798, 131.706, 136.559, 143.868, 148.709, 155.654]),
    (
        "$f(x)$ $\\Gamma,\\Delta$ $x_1,x_2$ $V_{a}$",
        167.841,
        &[125.798, 132.317, 136.56, 142.794, 150.648, 157.466, 162.318, 175.02, 185.992, 190.844, 205.424],
    ),
    ("the scripts `1`, `2` and `a`", 169.477, &[181.26, 197.075, 211.788]),
    (
        "\\[a+r, \\qquad ar, \\qquad a+b, \\qquad ab.\\]",
        194.939,
        &[
            229.63, 237.818, 248.725, 253.353, 279.893, 285.659, 290.276, 316.816, 325.015, 335.922, 340.604,
            367.144, 372.91, 377.592,
        ],
    ),
];

/// pdfTeX writes three decimals; the missing kern is worth 0.604 bp.
const TOL: f64 = 0.02;

#[test]
fn math_font_kerns_place_glyphs_where_pdflatex_does() {
    if !common::lm_available() {
        eprintln!("SKIP math_font_kerns_place_glyphs_where_pdflatex_does: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: PROBE }];
    let r = render(&docs, "main.tex", 1, "math-font-kerns", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut glyphs: Vec<(f64, f64)> = Vec::new();
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        glyphs.extend(run.glyphs.iter().map(|g| (g.baseline_y.to_bp(), g.origin_x.to_bp())));
    }
    let mut bad = Vec::new();
    for (what, baseline, want) in PDFLATEX {
        let mut got: Vec<f64> = glyphs.iter().filter(|(y, _)| (y - baseline).abs() < 0.05).map(|&(_, x)| x).collect();
        got.sort_by(f64::total_cmp);
        if got.len() != want.len() {
            bad.push(format!("{what}: {} glyphs on baseline {baseline}, pdflatex {} ({got:?})", got.len(), want.len()));
            continue;
        }
        for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            if (g - w).abs() > TOL {
                bad.push(format!("{what}: glyph {i} at x {g:.3} bp, pdflatex {w:.3} ({:+.3})", g - w));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}
