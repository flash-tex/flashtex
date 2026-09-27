//! `\renewcommand{\qedsymbol}{..}`, against pdflatex.
//!
//! amsthm defines `\qedsymbol` (amsthm.sty 430, `\openbox`), and student
//! templates very often replace it with `$\blacksquare$`. The pinned compiler
//! reported `LaTeX Error: Command \qedsymbol undefined.` on that line and the
//! proofs still ended with the open box. `crate::qedsymbol` now reads the
//! redefinition and ends each later proof with the formula.
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, glyph origins read from the PDF
//! (`tools/visual-oracle/pdftext.py`) on [`PROBE`]. Each mark is MSAM10
//! (`\blacksquare` "04, `\square` "03, both 0.77779em wide), flush right:
//!
//! | proof | glyph | x (bp) | baseline (bp) | size (bp) |
//! |---|---|---|---|---|
//! | Beta | `\blacksquare` | 469.732 | 154.690 | 9.963 |
//! | Gamma (`\qedhere` in running text) | `\blacksquare` | 469.732 | 174.615 | 9.963 |
//! | Delta (`\large`) | `\blacksquare` | 468.185 | 196.533 | 11.955 |
//! | Epsilon (`\ensuremath{\square}`) | `\square` | 469.727 | 216.458 | 9.963 |
//!
//! The text body of the last proof (`\textsc{qed}`) is not set yet: that
//! proof keeps the open box and says so in a limitation.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const PROBE: &str = r"\documentclass{article}
\usepackage{amsmath,amssymb,amsthm}
\pagestyle{empty}
\begin{document}
\begin{proof}
Alpha default box.
\end{proof}
\renewcommand{\qedsymbol}{$\blacksquare$}
\begin{proof}
Beta black square.
\end{proof}
\begin{proof}
Gamma ends early here. \qedhere
\end{proof}
{\large
\begin{proof}
Delta large.
\end{proof}}
\renewcommand*{\qedsymbol}{\ensuremath{\square}}
\begin{proof}
Epsilon white square.
\end{proof}
\renewcommand{\qedsymbol}{\textsc{qed}}
\begin{proof}
Zeta small caps.
\end{proof}
\end{document}
";

/// The word model agrees with pdflatex to 0.01 bp on text; a math glyph's
/// origin is the same TFM arithmetic.
const TOL: f64 = 0.05;

#[test]
fn a_math_qedsymbol_ends_every_later_proof_with_that_formula() {
    if !common::lm_available() {
        eprintln!("SKIP a_math_qedsymbol_ends_every_later_proof_with_that_formula: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: PROBE }];
    let r = render(&docs, "main.tex", 1, "qedsymbol-renew", &fonts, &RenderOptions::default());
    assert!(
        !r.v2.diagnostics.iter().any(|d| d.message.contains("qedsymbol undefined")),
        "amsthm defines \\qedsymbol, so pdflatex reports nothing: {:?}",
        r.v2.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(
        r.v2.diagnostics.iter().any(|d| d.message.contains("text body")),
        "the \\textsc body is reported as not set yet"
    );
    // Every glyph on the page right of x = 440 bp: only the four marks.
    let mut marks = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        for g in &run.glyphs {
            let x = g.origin_x.to_bp();
            if x > 440.0 {
                marks.push((x, g.baseline_y.to_bp(), run.font_size.to_bp()));
            }
        }
    }
    let expect = [(469.732, 154.690, 9.963), (469.732, 174.615, 9.963), (468.185, 196.533, 11.955), (469.727, 216.458, 9.963)];
    assert_eq!(marks.len(), expect.len(), "one glyph per redefined mark, got {marks:?}");
    for ((x, y, size), (ex, ey, esize)) in marks.iter().zip(expect) {
        assert!((x - ex).abs() <= TOL && (y - ey).abs() <= TOL && (size - esize).abs() <= TOL, "mark at ({x:.3}, {y:.3}) size {size:.3}, pdflatex ({ex:.3}, {ey:.3}) size {esize:.3}");
    }
}
