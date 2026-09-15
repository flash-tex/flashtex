//! Math ligatures and the kerns of one-character math alphabets
//! (`make_ord`, tex.web §752-§753) through the pipeline.
//!
//! Adjacent one-character `\mathrm` groups are family-0 math characters
//! (TeX §1186), so cmr's program joins them: `$\mathrm{f}\mathrm{i}$` is the
//! fi ligature and `$\mathrm{A}\mathrm{V}$` is kerned, exactly like
//! `$\mathrm{fi}$` and `$\mathrm{AV}$`. A one-character `\mathit{A}` or
//! `\mathbf{T}` is a character of its alphabet's family and is kerned
//! against the next one from that text font's program. Before this, every
//! such pair sat 0.9-1.2 bp apart from pdflatex (a missing kern) or was set
//! as two glyphs instead of one. The last line pins multi-character runs,
//! which the text sink already kerned and ligatured, so nothing regresses.
//!
//! ## Oracle
//!
//! [`PROBE`] through `pdflatex` (pdfTeX 1.40.29, TeX Live 2026,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) **with `lmodern`**, so the
//! roman and alphabet metrics are the Latin Modern TFMs the pipeline uses
//! (cmr's italic corrections differ from rm-lmr's by up to 0.4 bp, which
//! is not what is pinned here). Every glyph origin of its PDF was read with
//! `tools/visual-oracle/pdftext.py` and grouped by line; the numbers are in
//! [`PDFLATEX`]. pdflatex is an oracle only and never runs in the product
//! path.
//!
//! Needs math-layout's ligature step, so the whole file is behind the
//! `math-font-kerns` feature (see `Cargo.toml`), like `math_font_kerns.rs`.
#![cfg(feature = "math-font-kerns")]

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const PROBE: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage{lmodern}
\usepackage{amsmath}
\pagestyle{empty}
\setlength{\parindent}{0pt}
\begin{document}
$x\mathrm{f}\mathrm{i}x$

$x\mathrm{f}\mathrm{f}\mathrm{i}x$

$x\mathrm{f}\mathrm{i}^2x$

$x\mathrm{A}\mathrm{V}x$

$x\mathrm{T}\mathrm{r}^2x$

$x\mathit{A}\mathit{V}x$

$x\mathbf{T}\mathbf{o}x$

$x\mathrm{ffi}x$ $x\mathrm{AV}x$ $x\operatorname{Tr}x$

$x\mathit{fi}x$ $x\mathbf{To}x$ $x\mathsf{AV}x$
\end{document}
";

/// `(line, glyph origins left to right)` in bp, one line per paragraph of
/// [`PROBE`], scripts included.
const PDFLATEX: &[(&str, &[f64])] = &[
    (
        r"$x\mathrm{f}\mathrm{i}x$",
        &[125.798, 132.033, 138.093],
    ),
    (
        r"$x\mathrm{f}\mathrm{f}\mathrm{i}x$",
        &[125.798, 132.033, 141.123],
    ),
    (
        r"$x\mathrm{f}\mathrm{i}^2x$",
        &[125.798, 132.033, 138.094, 142.826],
    ),
    (
        r"$x\mathrm{A}\mathrm{V}x$",
        &[125.798, 132.033, 139.003, 147.273],
    ),
    (
        r"$x\mathrm{T}\mathrm{r}^2x$",
        &[125.798, 132.033, 139.006, 143.276, 148.009],
    ),
    (
        r"$x\mathit{A}\mathit{V}x$",
        &[125.798, 132.033, 139.029, 148.806],
    ),
    (
        r"$x\mathbf{T}\mathbf{o}x$",
        &[125.798, 132.033, 139.713, 145.985],
    ),
    (
        r"$x\mathrm{ffi}x$ $x\mathrm{AV}x$ $x\operatorname{Tr}x$",
        &[125.798, 132.033, 141.123, 150.990, 157.225, 164.196, 172.465, 182.332, 190.389, 197.362, 203.457],
    ),
    (
        r"$x\mathit{fi}x$ $x\mathbf{To}x$ $x\mathsf{AV}x$",
        &[125.798, 132.033, 138.842, 148.709, 154.944, 162.624, 168.897, 178.775, 185.009, 191.072, 198.486],
    ),
];

/// pdfTeX writes three decimals; the smallest missing kern is 0.9 bp.
const TOL: f64 = 0.02;

#[test]
fn math_ligatures_and_alphabet_kerns_place_glyphs_where_pdflatex_does() {
    if !common::lm_available() {
        eprintln!("SKIP math_ligatures_and_alphabet_kerns_place_glyphs_where_pdflatex_does: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: PROBE }];
    let r = render(&docs, "main.tex", 1, "math-ligatures", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut glyphs: Vec<(f64, f64)> = Vec::new();
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        glyphs.extend(run.glyphs.iter().map(|g| (g.baseline_y.to_bp(), g.origin_x.to_bp())));
    }
    // Lines are 13.6 bp apart; a superscript sits under 4 bp above its line.
    glyphs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut lines: Vec<(f64, Vec<f64>)> = Vec::new();
    for (y, x) in glyphs {
        match lines.last_mut() {
            Some((top, xs)) if y - *top < 6.0 => xs.push(x),
            _ => lines.push((y, vec![x])),
        }
    }
    assert_eq!(lines.len(), PDFLATEX.len(), "one line per probe paragraph");
    let mut bad = Vec::new();
    for ((what, want), (_, got)) in PDFLATEX.iter().zip(&mut lines) {
        got.sort_by(f64::total_cmp);
        if got.len() != want.len() {
            bad.push(format!("{what}: {} glyphs, pdflatex {} ({got:?})", got.len(), want.len()));
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
