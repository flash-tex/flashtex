//! `enumerate.sty` (tools, v3.00) list margins against pdflatex: with the
//! `enumerate` package, `\begin{enumerate}[<template>]` sets this depth's
//! `\leftmargin` to the width of the template's label at counter value 7
//! plus `\labelsep` (`\@@enum@`, enumerate.sty lines 72-83), not the
//! class's `\leftmargin<i>`. `[(i)]` measures `(vii)`, `[a)]` measures
//! `g)`, `[1.]` measures `7.`, and every unbraced `A a i I 1` is a counter
//! (`\@enloop`, lines 52-65) while a braced group is literal, so `[{A}-I]`
//! measures `A-VII`. A plain `\begin{enumerate}` keeps the class value.
//!
//! Oracle: MacTeX 2026 pdflatex (`SOURCE_DATE_EPOCH=0`, two runs), word
//! origins from `tools/visual-oracle/pdftext.py`. Probed the same day in
//! this 11pt article: `\showthe\leftmargin` inside `[(i)]` 25.74338pt =
//! `\wd\hbox{(vii)}` 20.26837pt + `\labelsep` 5.475pt, where the class's
//! `\leftmargini` is 27.37506pt (1.63pt = 1.626bp further right, which was
//! lecture-notes page 2's worst delta). No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of, Word};

const TOL: f64 = 0.02;

const DOC: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{enumerate}
\begin{document}
\noindent Alpha
\begin{enumerate}[(i)]
  \item Prove
  \item Find
  \begin{enumerate}[a)]
    \item Beta
    \item Gamma
  \end{enumerate}
\end{enumerate}
\begin{enumerate}[1.]
  \item Delta
\end{enumerate}
\begin{enumerate}[{A}-I]
  \item Epsilon
\end{enumerate}
\begin{enumerate}
  \item Zeta
\end{enumerate}
\end{document}
";

/// (label, its x, item text, its x) as pdflatex set them, bp from the
/// page's left edge (`pdftext.py`; the labels are `\llap`ped so their x is
/// the text's less the label's own width less `\labelsep`).
const EXPECTED: [(&str, f64, &str, f64); 7] = [
    ("(i)", 80.739, "Prove", 97.648),
    ("(ii)", 77.725, "Find", 97.649),
    ("a)", 97.647, "Beta", 112.745),
    ("b)", 97.045, "Gamma", 112.745),
    ("1.", 72.000, "Delta", 85.894),
    ("A-I", 84.347, "Epsilon", 105.465),
    ("1.", 85.380, "Zeta", 99.274),
];

fn word<'a>(words: &'a [Word], text: &str, after: f64) -> &'a Word {
    words
        .iter()
        .filter(|w| w.page == 1 && w.baseline > after)
        .min_by(|a, b| a.baseline.total_cmp(&b.baseline))
        .filter(|w| w.text == text)
        .unwrap_or_else(|| panic!("no word {text:?} after baseline {after:.3}: {:?}", words.iter().map(|w| (w.text.as_str(), w.x, w.baseline)).collect::<Vec<_>>()))
}

#[test]
fn enumerate_package_templates_set_leftmargin_to_the_label_at_seven() {
    if !lm_available() {
        return;
    }
    let r = render_one(DOC);
    let words = words_of(&r);
    // Item lines are consecutive baselines below `Alpha`; each expected
    // row is the next line, its label first and its text second.
    let mut after = words.iter().find(|w| w.text == "Alpha").expect("Alpha").baseline;
    let mut misses = Vec::new();
    for (label, label_x, text, text_x) in EXPECTED {
        let l = word(&words, label, after + 0.5);
        let t = words
            .iter()
            .find(|w| w.page == 1 && (w.baseline - l.baseline).abs() < 0.01 && w.text == text)
            .unwrap_or_else(|| panic!("no {text:?} on the line of {label:?}"));
        if (l.x - label_x).abs() > TOL {
            misses.push(format!("label {label:?} at x {:.3}, pdflatex {label_x:.3}", l.x));
        }
        if (t.x - text_x).abs() > TOL {
            misses.push(format!("text {text:?} at x {:.3}, pdflatex {text_x:.3}", t.x));
        }
        after = l.baseline;
    }
    assert!(misses.is_empty(), "enumerate.sty margins off by more than {TOL} bp:\n{}", misses.join("\n"));
}
