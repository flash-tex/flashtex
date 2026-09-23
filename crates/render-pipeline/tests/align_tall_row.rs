//! Short row then tall row in `align*`: the opened-up `\lineskiplimit`.
//!
//! amsmath's display alignments run under `\openup\jot` (`\displ@y@`), which
//! advances all three of `\baselineskip`, `\lineskip` and `\lineskiplimit`.
//! Between two rows the interline glue is `\baselineskip+\jot` when
//! `\baselineskip - \prevdepth - height >= \lineskiplimit`, and
//! `\lineskip+\jot` otherwise.
//!
//! The block carrying the alignment rows set the opened-up `\baselineskip`
//! and `\lineskip` but compared against the page's `\lineskiplimit` (0pt)
//! instead of the opened-up one (0pt+`\jot` = 3pt). A short row (`x &= y`,
//! depth 3.6pt) followed by a tall `\frac` row (height ~11.07pt) gives
//! `15 - 3.6 - 11.07 = 0.33pt`, which is `>= 0pt` but `< 3pt`: pdflatex
//! falls into lineskip mode while the pipeline stayed in baselineskip mode
//! and produced exactly 15pt = 14.944bp instead of 18.6009bp.
//!
//! The reverse order (tall above short) already matched pdflatex because its
//! glue is negative, below either limit, so it is pinned here as a guard.
//!
//! ## Oracle
//!
//! pdflatex baseline-to-baseline gaps for the two documents below:
//! short-then-tall **18.6009bp**, tall-then-short **19.1875pt**.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// 1 bp = 1.00375 TeX pt.
const BP: f64 = 1.00375;

/// pdflatex's short-then-tall gap, in bp.
const EXPECTED_SHORT_THEN_TALL_BP: f64 = 18.6009;
/// The already-correct tall-then-short gap, in bp: the pipeline produces
/// 19.1876bp today and pdflatex agrees to the fourth decimal, so the value
/// is pinned as-is to catch any regression from the short-then-tall fix.
const EXPECTED_TALL_THEN_SHORT_BP: f64 = 19.1876;

/// Acceptance tolerance: the gap must match pdflatex within 0.01bp.
const TOL: f64 = 0.01;

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\
         \\usepackage{{amsmath}}\n\
         \\pagestyle{{empty}}\n\
         \\begin{{document}}\n\
         Before.\n\
         \\begin{{align*}}\n{body}\n\\end{{align*}}\n\
         After.\n\
         \\end{{document}}\n"
    )
}

/// The baseline of every `=` on page 1, in bp from the page top, sorted.
///
/// Each alignment row holds exactly one `=` on its own baseline and nothing
/// else in the document contains one, so this picks out the row baselines
/// without depending on the page frame.
fn equals_baselines(tex: &str) -> Vec<f64> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: tex }];
    let r = render(&docs, "main.tex", 1, "align-tall-row", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut ys = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        let mut chars = run.clusters.iter().map(|c| {
            run.text[c.text_start_byte as usize..c.text_end_byte as usize].chars().next()
        });
        for g in &run.glyphs {
            if chars.next().flatten() == Some('=') {
                ys.push(g.baseline_y.to_bp());
            }
        }
    }
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys
}

#[test]
fn align_short_then_tall_row_gap_matches_pdflatex() {
    if !common::lm_available() {
        eprintln!("SKIP align_short_then_tall_row_gap_matches_pdflatex: Latin Modern fonts not installed");
        return;
    }
    let tex = doc("x &= y \\\\\na &= \\frac{a}{b}");
    let ys = equals_baselines(&tex);
    assert_eq!(ys.len(), 2, "expected one `=` per alignment row, got {ys:?}");
    let got = ys[1] - ys[0];
    assert!(
        (got - EXPECTED_SHORT_THEN_TALL_BP).abs() <= TOL,
        "short-then-tall row gap {got:.4} bp, pdflatex {EXPECTED_SHORT_THEN_TALL_BP:.4} bp \
         (off by {:+.4} bp; 15pt = {:.3} bp means the gap stayed in baselineskip mode \
         instead of falling into lineskip mode)",
        got - EXPECTED_SHORT_THEN_TALL_BP,
        15.0 / BP,
    );
}

#[test]
fn align_tall_then_short_row_gap_is_unchanged() {
    if !common::lm_available() {
        eprintln!("SKIP align_tall_then_short_row_gap_is_unchanged: Latin Modern fonts not installed");
        return;
    }
    let tex = doc("a &= \\frac{a}{b} \\\\\nx &= y");
    let ys = equals_baselines(&tex);
    assert_eq!(ys.len(), 2, "expected one `=` per alignment row, got {ys:?}");
    let got = ys[1] - ys[0];
    assert!(
        (got - EXPECTED_TALL_THEN_SHORT_BP).abs() <= TOL,
        "tall-then-short row gap {got:.4} bp, pdflatex {EXPECTED_TALL_THEN_SHORT_BP:.4} bp \
         (off by {:+.4} bp; this order already matched and must not regress)",
        got - EXPECTED_TALL_THEN_SHORT_BP,
    );
}
