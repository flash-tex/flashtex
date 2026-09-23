//! In an italic amsthm theorem body the interword space before inline math
//! uses the italic font's space, not the upright font's.
//!
//! Measured against pdflatex (TeX Live 2026, `\documentclass[11pt]{article}`
//! + `[T1]{fontenc}` + amsthm, `\showboxbreadth=100 \showboxdepth=10
//! \tracingoutput=1 \tracingonline=1`; oracle only, never in the product
//! path). For `\begin{theorem}a $x$ b\end{theorem}` the shipped box holds:
//!
//! ```text
//! ....\T1/cmr/m/it/10.95 a
//! ....\glue 3.88765 plus 1.66487 minus 1.10992
//! ....\mathon
//! ....\mathoff
//! ....\glue 3.88765 plus 1.66487 minus 1.10992
//! ....\T1/cmr/m/it/10.95 b
//! ```
//!
//! The line's only stretch is `\parfillskip`'s `fil`, so both spaces sit at
//! their natural width: 3.88765pt. Before the fix the pipeline set the space
//! before `$x$` from the upright face (ecrm1095, 3.630pt = 3.617bp) because
//! the math arm never applied the theorem body's italic override that the
//! text arm has (`compiler_weight`).

mod common;

use common::*;

/// TeX points to PDF points, the unit of every v2 coordinate.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

const SRC: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage{amsthm}
\newtheorem{theorem}{Theorem}
\begin{document}
\begin{theorem}a $x$ b\end{theorem}
\end{document}
";

/// The gap between the end of the run holding `before` and the start of the
/// run holding `after`, both on the first body line.
fn gap_between(words: &[Word], before: &str, after: &str) -> f64 {
    let a = words.iter().find(|w| w.text == before).unwrap_or_else(|| panic!("no run {before:?} in {words:?}"));
    let baseline = a.baseline;
    let same_line = |w: &&Word| (w.baseline - baseline).abs() < 0.01;
    let b = words
        .iter()
        .filter(|w| w.text == after && same_line(w) && w.x > a.x)
        .min_by(|x, y| x.x.partial_cmp(&y.x).unwrap())
        .unwrap_or_else(|| panic!("no run {after:?} after {before:?} in {words:?}"));
    b.x - (a.x + a.width)
}

#[test]
fn space_before_math_in_theorem_body_uses_the_italic_space() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SRC);
    let words = words_of(&r);

    // pdflatex: `\glue 3.88765 plus 1.66487 minus 1.10992` (ecti1095's
    // interword glue at its natural width) on both sides of the formula.
    let expected = bp(3.88765);
    let before = gap_between(&words, "a", "x");
    assert!(
        (before - expected).abs() < 0.05,
        "space before $x$ must be the italic body's space (3.88765pt = {expected} bp), got {before} bp"
    );
    let after = gap_between(&words, "x", "b");
    assert!(
        (after - expected).abs() < 0.05,
        "space after $x$ must be the italic body's space (3.88765pt = {expected} bp), got {after} bp"
    );
}
