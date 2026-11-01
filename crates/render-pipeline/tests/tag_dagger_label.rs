//! `\tag*{$\dagger$}` and the `\eqref` to it (#441, probe line 3).
//!
//! `\dagger` is fontmath.ltx 277's cmsy `\mathbin`; the compiler learnt it
//! (with `\ddagger`, `\S`, `\P`) as the last piece of #441's probe. The
//! tag is a bare `†` flush right (`\tag*`: no parentheses) and the
//! reference `\textup{\tagform@{..}}` puts them back: `(†).`.
//!
//! pdflatex (TeX Live 2026, 10pt article + amsmath, CM fonts; `pdftext.py`
//! at 400 dpi) on this file's document:
//!
//! ```text
//! 'e' 293.688  '=' 301.096  'f' 311.615  '†' 473.055   y 146.720
//! 'See' 133.768  '(†).' 151.476  'for' 170.853       y 164.653
//! ```
//!
//! This used to skip itself when the compiler reported `unknown_command` for
//! `\dagger`, because `render-pipeline` built against a frozen
//! `vendor/compiler` that predated the symbol's table entry. `vendor/` is
//! retired and `crates/compiler` carries `\dagger`, so the placements below
//! are asserted unconditionally; a regression that loses the symbol now fails
//! here instead of quietly skipping.
#![cfg(feature = "compiler-node-surface")]

mod common;

use common::*;

const DAGGER_TAG: &str = r"\documentclass{article}\usepackage{amsmath}\begin{document}
\begin{equation}e=f\tag*{$\dagger$}\label{eq:d}\end{equation}
See \eqref{eq:d}. for
\end{document}
";

#[test]
fn dagger_tag_and_eqref_match_pdflatex() {
    if !lm_available() {
        return;
    }
    let r = render_one(DAGGER_TAG);
    assert!(
        !r.v2.diagnostics.iter().any(|d| d.code == "unknown_command" && d.message.contains("\\dagger")),
        "crates/compiler must know \\dagger: {:?}",
        r.v2.diagnostics
    );
    let words = words_of(&r);
    let at = |text: &str, x: f64, y: f64| {
        words
            .iter()
            .find(|w| w.text == text && (w.x - x).abs() <= 0.5 && (w.baseline - y).abs() <= 0.5)
            .unwrap_or_else(|| panic!("{text:?} not within 0.5 bp of ({x}, {y}): {words:?}"))
    };
    // The tag: a bare dagger at the right margin, on the formula's baseline.
    at("f", 311.615, 146.720);
    at("†", 473.055, 146.720);
    // The reference: `(`, `†`, `).` as the math box's runs, then `for`
    // after the sentence space.
    at("(", 151.476, 164.653);
    at("†", 155.350, 164.653);
    at(")", 159.777, 164.653);
    at("for", 170.853, 164.653);
}
