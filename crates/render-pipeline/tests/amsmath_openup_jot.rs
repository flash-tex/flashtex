//! `\openup\jot` opens up `\lineskip`, not only `\baselineskip`.
//!
//! amsmath's display alignments run their rows through `\openup\jot`
//! (`\displ@y@`). `\openup` is an `\advance` on *all three* of
//! `\baselineskip`, `\lineskip` and `\lineskiplimit`, so between two rows of
//! an `align` the interline glue is `\baselineskip+\jot` when the rows are
//! short and `\lineskip+\jot` = 1pt+3pt = **4pt** when they are tall enough
//! that `\baselineskip - \prevdepth - height < \lineskiplimit`.
//!
//! `rows_block` set the opened-up `\baselineskip` but left `\lineskip` at the
//! page's 1pt, so every row gap that fell into lineskip mode was one `\jot`
//! short. Short-row alignments never reach lineskip mode, which is why the
//! pinned `display-placement` alignment fixtures (`35-12pt-fleqn-leqno-align`
//! is `a &= b \\ c &= d`) all passed while a real problem set drifted 3pt per
//! row: `fixtures/real-world/ps-calculus` lost 2.989 bp at every one of the
//! five row gaps in its two alignments, and the page-2 median `dy` against
//! its committed reference was -16.458 bp.
//!
//! ## Oracle
//!
//! pdfLaTeX's own `\showoutput` for the document below (TeX Live 2025,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two passes) prints the rows of
//! the alignment as
//!
//! ```text
//! ...\glue(\abovedisplayskip) 11.0 plus 3.0 minus 6.0
//! ...\glue -3.0
//! ...\glue 0.0
//! ...\glue(\lineskip) 4.0
//! ...\hbox(17.73761+9.9293)x260.94586, display
//! ...\penalty 10000
//! ...\glue 0.0
//! ...\glue(\lineskip) 4.0
//! ...\hbox(18.52586+10.90265)x260.94586, display
//! ...\penalty 10000
//! ...\glue 0.0
//! ...\glue(\lineskip) 4.0
//! ...\hbox(14.4644+7.51115)x260.94586, display
//! ```
//!
//! so the row-to-row baseline separations are
//!
//! * 9.9293 + 4.0 + 18.52586 = 32.45516 pt = **32.334 bp**
//! * 10.90265 + 4.0 + 14.4644 = 29.36705 pt = **29.258 bp**
//!
//! These are separations *within* the alignment, so they do not depend on the
//! page frame or on anything above the display. The same three row boxes, to
//! the scaled point, are the first alignment of
//! `fixtures/real-world/ps-calculus/main.tex`, and the separations are
//! exactly what that fixture's committed `reference.pdf` shows between its
//! rows (188.014 -> 220.348 -> 249.605 bp from the page top on page 2) — a
//! pinned artefact in this repository, which the pdflatex used here
//! reproduces byte for byte. With `\lineskip` left at 1pt each separation
//! comes out 2.989 bp (one `\jot`) short: 29.345 and 26.268 bp.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// 1 bp = 1.00375 TeX pt.
const BP: f64 = 1.00375;

/// The corpus harness's word tolerance. One `\jot` is 2.989 bp, six times
/// this, so a missing `\jot` cannot hide under it.
///
/// It is not tighter because the second separation carries an unrelated
/// residual: with `\lineskip` opened up, row 1 -> row 2 lands on pdflatex to
/// **0.0001 bp**, but row 2 -> row 3 is **0.161 bp** short, because our
/// `\frac{1}{4}` row box is that much shorter than pdflatex's 14.4644 pt.
/// That is a fraction box height, not interline glue, and it is left visible
/// here rather than absorbed into a rounder expected number.
const TOL: f64 = 0.5;

const TEX: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage{amsmath}
\pagestyle{empty}
\begin{document}
Before.
\begin{align*}
  \int_0^{\pi/2} \sin^3(x)\cos(x)\,dx
    &= \int_0^1 u^3\,du \\
    &= \left[\frac{u^4}{4}\right]_0^1 \\
    &= \frac{1}{4}.
\end{align*}
After.
\end{document}
";

/// The baseline of every `=` on page 1, in bp from the page top, sorted.
///
/// Each of the three alignment rows starts with exactly one `=` on its own
/// baseline and nothing else in the document contains one, so this picks out
/// the row baselines without depending on the page frame.
fn equals_baselines() -> Vec<f64> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: TEX }];
    let r = render(&docs, "main.tex", 1, "openup-jot", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut ys = Vec::new();
    for item in &r.v2.pages[0].items {
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
fn align_rows_are_opened_up_by_jot_in_lineskip_mode() {
    if !common::lm_available() {
        eprintln!("SKIP align_rows_are_opened_up_by_jot_in_lineskip_mode: Latin Modern fonts not installed");
        return;
    }
    let ys = equals_baselines();
    assert_eq!(ys.len(), 3, "expected one `=` per alignment row, got {ys:?}");

    // pdflatex: 9.9293 + \lineskip(4.0) + 18.52586, and
    //           10.90265 + \lineskip(4.0) + 14.4644.
    let expected = [(9.9293 + 4.0 + 18.52586) / BP, (10.90265 + 4.0 + 14.4644) / BP];
    let got = [ys[1] - ys[0], ys[2] - ys[1]];
    for (i, (g, e)) in got.iter().zip(expected).enumerate() {
        assert!(
            (g - e).abs() <= TOL,
            "alignment row {} -> {}: separation {g:.4} bp, pdflatex {e:.4} bp (off by {:+.4} bp; \
             one \\jot is {:.3} bp, so a miss of that size means \\openup\\jot did not reach \
             \\lineskip)",
            i + 1,
            i + 2,
            g - e,
            3.0 / BP
        );
    }
}
