//! Explicit `\;` and `\:` in a formula are muskip glue, not rigid kerns.
//!
//! latex.ltx: `\;` is `\mskip\thickmuskip` (`5mu plus 5mu`), `\:`/`\>` are
//! `\mskip\medmuskip` (`4mu plus 2mu minus 4mu`); amsmath's `\thickspace`/
//! `\medspace` are the same registers, and `\iff`/`\implies`/`\impliedby`
//! are `\DOTSB\;<arrow>\;`. The pipeline set every explicit math space as a
//! kern, so on a justified line the words after one sat left of pdflatex
//! (`\iff` in fixtures/proof-corpus/problem-set-solution-env: 2.12 bp). A
//! space with an explicit width (`\mkern5mu`) stays rigid.
//!
//! Word origins are pdflatex's (TeX Live 2026, oracle only), read from its
//! PDF: bp from the page top.

mod common;

use common::*;

const SRC: &str = r"\documentclass{article}
\usepackage{amsmath}
\parindent0pt
\begin{document}
Alpha words give $a = 0 \: u = v$ and then some more words so that this line is stretched to the full measure here.

Bravo words give $a = 0 \;\; u = v$ and then some more words so that this line is stretched to the full measure here.

Charlie words give $a = 0 \mkern5mu u = v$ and then some more words so that this line is stretched to the full measure here.

Delta words give $a = 0 \; u = v$ and then some more words so that this line is stretched out to the full measure here.

Echo words give $a = 0 \DOTSB\;\Longleftrightarrow\; u = v$ and then more words so that this line is stretched to full measure here.

Foxtrot words give $a \implies b$ and $c \thickspace d$ and then more words so that this line is stretched to the full measure here.
\end{document}
";

#[test]
fn thick_and_med_math_spaces_stretch_with_the_line() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = words_of(&render_one(SRC));
    let at = |text: &str, baseline: f64| {
        words
            .iter()
            .find(|w| w.text == text && (w.baseline - baseline).abs() < 0.05)
            .unwrap_or_else(|| {
                let line: Vec<_> = words.iter().filter(|w| (w.baseline - baseline).abs() < 0.05).map(|w| (&w.text, w.x)).collect();
                panic!("no run {text:?} on baseline {baseline}: {line:?}")
            })
            .x
    };
    // (word, baseline, pdflatex x); rigid kerns put each 0.16-1.13 bp left.
    for (text, baseline, x) in [
        ("u", 134.765, 242.493),     // `\:`
        ("then", 134.765, 292.627),
        ("u", 158.675, 244.982),     // `\;\;`
        ("then", 158.675, 294.119),
        ("u", 182.585, 246.174),     // `\mkern5mu`: rigid, unchanged
        ("then", 182.585, 294.843),
        ("u", 206.496, 241.576),     // `\;`
        ("then", 206.496, 292.079),
        ("then", 230.406, 318.316),  // `\DOTSB\;` (`⇐⇒u` is one run)
        ("and", 254.316, 263.069),   // after `\implies` (`⇒b` is one run)
        ("then", 254.316, 319.444),  // after `\thickspace`
    ] {
        let got = at(text, baseline);
        assert!((got - x).abs() < 0.05, "`{text}` on {baseline}: pdflatex {x} bp, got {got} bp");
    }
}
