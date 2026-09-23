//! amsbsy `\pmb` in math: the body overprinted three times at offsets measured
//! in the current style's mu (amsbsy.sty `\pmb@`, lines 49-57).
//!
//! Oracle (measured): TeX Live 2026 pdfTeX, `\documentclass{article}`,
//! `\usepackage{amsmath}`, glyph origins from PyMuPDF `get_texttrace` of
//! ```tex
//! Text $a\pmb{x}b$ and $a\pmb{+}b$ and $a\pmb{=}b$ ...
//! Script $y^{\pmb{x}}$ and $y^{z^{\pmb{x}}}$ ...
//! ```
//! Offsets of the three `x` copies from the last one, in bp (x right, y down):
//! text (10pt) `(-0.438, 0)`, `(-0.222, -0.277)`; script (7pt) `(-0.362, 0)`,
//! `(-0.181, -0.227)`; scriptscript (5pt) `(-0.326, 0)`, `(-0.163, -0.203)`.
//! `\pmb{+}` stays a binary operator and `\pmb{=}` a relation (amsbsy's
//! `\binrel@`): `a`→first copy and last copy→`b` gaps equal those of `a+b`
//! and `a=b`.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

/// (source char, x bp, y bp) of every math glyph on page 1, in paint order.
fn math_glyphs(body: &str) -> Vec<(String, f64, f64)> {
    let r = render_one(&format!("\\documentclass{{article}}\\usepackage{{amsmath}}\\begin{{document}}{body}\\end{{document}}"));
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Math {
                continue;
            }
            for g in &run.glyphs {
                let c = run
                    .clusters
                    .get(g.cluster as usize)
                    .map(|c| run.text[c.text_start_byte..c.text_end_byte].to_string())
                    .unwrap_or_default();
                out.push((c, g.origin_x.to_bp(), g.baseline_y.to_bp()));
            }
        }
    }
    out
}

fn copies(glyphs: &[(String, f64, f64)], ch: &str) -> Vec<(f64, f64)> {
    glyphs.iter().filter(|g| g.0 == ch).map(|g| (g.1, g.2)).collect()
}

fn assert_offsets(body: &str, want: [(f64, f64); 2]) {
    let glyphs = math_glyphs(body);
    let x = copies(&glyphs, "x");
    assert_eq!(x.len(), 3, "{body}: three painted copies of x, got {glyphs:?}");
    let last = x[2];
    for (i, w) in want.iter().enumerate() {
        let got = (x[i].0 - last.0, x[i].1 - last.1);
        assert!(
            (got.0 - w.0).abs() < 0.02 && (got.1 - w.1).abs() < 0.02,
            "{body}: copy {i} at {got:?} from the last, pdflatex {w:?}"
        );
    }
}

#[test]
fn text_and_display_offsets_match_pdflatex() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    assert_offsets("$a\\pmb{x}b$", [(-0.438, 0.0), (-0.222, -0.277)]);
    // Display: the same 10pt mu (pdflatex `\[a\pmb{x}b\]`: -0.434, -0.221/-0.277).
    assert_offsets("\\[a\\pmb{x}b\\]", [(-0.434, 0.0), (-0.221, -0.277)]);
}

#[test]
fn script_and_scriptscript_offsets_shrink_with_the_style() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    assert_offsets("$y^{\\pmb{x}}$", [(-0.362, 0.0), (-0.181, -0.227)]);
    assert_offsets("$y^{z^{\\pmb{x}}}$", [(-0.326, 0.0), (-0.163, -0.203)]);
}

#[test]
fn a_bold_binary_or_relation_keeps_its_spacing() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    for op in ["+", "="] {
        let bold = math_glyphs(&format!("$a\\pmb{{{op}}}b$"));
        let plain = math_glyphs(&format!("$a{op}b$"));
        let x = |gs: &[(String, f64, f64)], c: &str| gs.iter().filter(|g| g.0 == c).map(|g| g.1).collect::<Vec<_>>();
        let (ba, bop, bb) = (x(&bold, "a")[0], x(&bold, op), x(&bold, "b")[0]);
        let (pa, pop, pb) = (x(&plain, "a")[0], x(&plain, op)[0], x(&plain, "b")[0]);
        assert_eq!(bop.len(), 3, "{op}: {bold:?}");
        assert!((bop[2] - ba - (pop - pa)).abs() < 0.02, "{op}: a to {op} {} vs {}", bop[2] - ba, pop - pa);
        assert!((bb - ba - (pb - pa)).abs() < 0.02, "{op}: a to b {} vs {}", bb - ba, pb - pa);
    }
}
