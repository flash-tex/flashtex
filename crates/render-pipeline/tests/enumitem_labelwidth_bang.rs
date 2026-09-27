//! enumitem `labelwidth=!` with an explicit `labelsep=` against pdflatex.
//!
//! `\begin{enumerate}[labelsep=1cm,labelwidth=!]` (10pt article):
//! `labelwidth=!` (`\enit@calcset\labelwidth\@ne!`, enumitem.sty 310-311)
//! sets `\enit@calc` to `labelwidth`, so `\enit@calcleft` computes
//! `\labelwidth = \leftmargin + \itemindent - \labelsep - \labelindent`
//! with no `\enit@calcwidth`. Probed with `\showthe` (TeX Live 2026
//! pdflatex): `\leftmargin` 25.00003pt (the class's `\leftmargini`),
//! `\labelwidth` -3.45271pt, `\labelsep` 28.45274pt (1cm), `\itemindent`
//! and `\labelindent` 0pt — plus enumitem's "Negative labelwidth" warning.
//!
//! A negative `\labelwidth` takes `\enit@postlabel@i`'s wide-label branch
//! (the label, `1.` at ~7.78pt, is wider than -3.45pt): the label is set
//! with `\llap` (zero width), so the item text starts at
//! `\@totalleftmargin - \labelwidth` = 28.45274pt in, i.e. exactly
//! `\labelsep` past the text edge, and the label's right edge ends
//! `\labelsep` before the text.
//!
//! Expected coordinates below were measured with
//! `tools/visual-oracle/pdftext.py` on the PDF that
//! `/Library/TeX/texbin/pdflatex -interaction=nonstopmode repro.tex`
//! produced from the same source (`repro.tex` holds just the three-line
//! body below inside `\begin{document}`): `1.` at x=126.020,
//! `a` at x=162.113 (bp). No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of};

const TOL: f64 = 0.1;

/// One document per `labelsep=`: the body word differs so the three renders
/// stay unambiguous. Expected coordinates are `tools/visual-oracle/pdftext.py`
/// origins from `/Library/TeX/texbin/pdflatex -interaction=nonstopmode`
/// PDFs of the same sources (the `1cm` row above; the other two measured
/// the same way): `5mm` gives `1.` at 136.753 and its body at 158.679 —
/// `\labelwidth` (10.77pt) still fits the label, so the text hangs at the
/// class `\leftmargini`; `15mm` gives `1.` at 126.020 and its body at
/// 176.289 — `\labelwidth` (-17.68pt) takes the wide branch and the text
/// starts a full `\labelsep` past the text edge.
const CASES: [(&str, &str, f64, f64); 3] = [
    ("5mm", "five", 136.753, 158.679),
    ("1cm", "one", 126.020, 162.113),
    ("15mm", "fifteen", 126.020, 176.289),
];

fn source(labelsep: &str, body: &str) -> String {
    format!(
        "\\documentclass[10pt]{{article}}\n\\usepackage{{enumitem}}\n\\begin{{document}}\n\\begin{{enumerate}}[labelsep={labelsep},labelwidth=!]\\item {body}\\end{{enumerate}}\n\\end{{document}}\n"
    )
}

#[test]
fn enumitem_labelwidth_bang_with_labelsep_matches_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    for (labelsep, body, label_x, body_x) in CASES {
        let rendered = render_one(&source(labelsep, body));
        assert_eq!(rendered.v2.pages.len(), 1, "one page ({labelsep})");
        let words = words_of(&rendered);
        let label = words
            .iter()
            .find(|w| w.text == "1.")
            .expect("the enumerate label");
        let text = words
            .iter()
            .find(|w| w.text == body)
            .expect("the item body");
        assert!(
            (label.x - label_x).abs() < TOL,
            "[{labelsep}] label: {} vs pdflatex {label_x}",
            label.x
        );
        assert!(
            (text.x - body_x).abs() < TOL,
            "[{labelsep}] body: {} vs pdflatex {body_x}",
            text.x
        );
    }
}
