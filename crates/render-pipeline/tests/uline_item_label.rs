//! GH-924: an itemize item whose text contains `\uline`, `\sout` or
//! `\underline` set its bullet 2.77bp left of pdflatex, while the plain
//! control item matched.
//!
//! Root cause: those commands parse their argument through the compiler's
//! `box_inlines`, which stashes `pending_item_label` without
//! `pending_item`, so the item's `ItemLabel::Symbol` is consumed while the
//! marker text survives (`label` `Some`, `item` `None`). Without the
//! classification the bullet left the `tcrm` symbol path for the text
//! font's wider bullet. The adapter recovers the default itemize symbol
//! from its text; this test pins the marker origins against pdflatex
//! (TeX Live 2026, measured with PyMuPDF span origins).

mod common;

use common::{lm_available, render_one, words_of, Word};

const SOURCE: &str = r"\documentclass[10pt]{article}
\usepackage[normalem]{ulem}
\begin{document}
\begin{itemize}
\item plain text here
\item \uline{decorated text here}
\item \sout{struck text here}
\item \underline{underlined text here}
\end{itemize}
\end{document}
";

// pdflatex span origins (bp) for the four bullets and the first word of
// each item body.
const BULLET_X: f64 = 148.714;
const BODY_X: [(&str, f64); 4] = [
    ("plain", 158.6766),
    ("decorated", 158.675),
    ("struck", 158.675),
    ("underlined", 158.6766),
];

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.page == 1 && w.text == text)
        .unwrap_or_else(|| panic!("no page 1 word {text:?}: {words:?}"))
}

#[test]
fn uline_sout_underline_item_markers_match_pdflatex() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(SOURCE);
    let words = words_of(&rendered);
    let mut bullets: Vec<&Word> =
        words.iter().filter(|w| w.page == 1 && w.text == "•").collect();
    assert_eq!(bullets.len(), 4, "four item bullets: {words:?}");
    bullets.sort_by(|a, b| a.baseline.total_cmp(&b.baseline));
    for bullet in &bullets {
        assert!(
            (bullet.x - BULLET_X).abs() < 0.05,
            "bullet at {} vs pdflatex {BULLET_X}",
            bullet.x
        );
    }
    for window in bullets.windows(2) {
        assert!(
            (window[0].x - window[1].x).abs() < 0.05,
            "decorated marker moved vs control: {} vs {}",
            window[0].x,
            window[1].x
        );
    }
    for (text, x) in BODY_X {
        let body = word(&words, text);
        assert!(
            (body.x - x).abs() < 0.1,
            "item text {text:?} at {} vs pdflatex {x}",
            body.x
        );
    }
}
