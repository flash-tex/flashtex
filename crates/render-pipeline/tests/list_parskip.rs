//! A nonzero `\parskip` and lists, against pdflatex.
//!
//! `\list` sets `\parskip\parsep`, so the paragraphs of an `itemize` or
//! `enumerate` item are separated by the level's `\parsep` (4pt plus 2pt
//! minus 1pt at the first level of a 10pt article), never by the body's
//! `\parskip`. `\trivlist` instead sets `\parsep\parskip`, so the second
//! paragraph of an amsthm theorem does take the body's `\parskip`. The body
//! `\parskip` also applies between ordinary paragraphs and in front of the
//! paragraph after a list. The same page results whether the assignment is
//! in the preamble or at the start of the body.
//!
//! Oracle: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), two runs,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, word origins read from the PDF
//! (`tools/visual-oracle/pdftext.py`). The preamble and the body placement
//! give identical positions. pdflatex is an oracle only and never runs here.

mod common;

use common::*;

const BODY: &str = r"\documentclass{article}
\usepackage{amsthm}
\pagestyle{empty}
\newtheorem{theorem}{Theorem}
\begin{document}
Alpha opening paragraph.
\setlength{\parskip}{10pt}

Beta second paragraph.
\begin{itemize}
\item Gamma first item.

Delta second paragraph of the item.
\item Epsilon second item.
\end{itemize}
Zeta between.
\begin{enumerate}
\item Eta first.

Theta continues.
\item Iota second.
\end{enumerate}
Kappa after.
\begin{theorem}Lambda theorem body.

Mu second theorem paragraph.
\end{theorem}
Nu closing paragraph.
\end{document}
";

/// (first glyph of the word, pdfTeX font, x bp, baseline bp), both placements.
const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR10", 148.712, 134.765),  // Alpha
    ("B", "CMR10", 148.712, 156.682),  // Beta: + \parskip 10pt
    ("G", "CMR10", 158.676, 186.570),  // Gamma
    ("D", "CMR10", 158.675, 202.511),  // Delta: + \parsep 4pt, not 10pt
    ("E", "CMR10", 158.676, 222.436),  // Epsilon: + \itemsep + \parsep
    ("Z", "CMR10", 133.768, 252.324),  // Zeta
    ("E", "CMR10", 158.675, 282.212),  // Eta
    ("T", "CMR10", 158.675, 298.152),  // Theta: + \parsep
    ("I", "CMR10", 158.675, 318.077),  // Iota
    ("K", "CMR10", 133.768, 347.965),  // Kappa
    ("L", "CMTI10", 196.307, 367.890), // Lambda
    ("M", "CMTI10", 148.712, 389.808), // Mu: \trivlist keeps \parskip
    ("N", "CMR10", 148.712, 419.696),  // Nu
];

#[test]
fn a_body_parskip_leaves_list_paragraphs_at_parsep() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_pdftex_glyphs(BODY, EXPECTED, 0.01);
}

#[test]
fn a_preamble_parskip_sets_the_same_page() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let preamble = BODY
        .replace("\\setlength{\\parskip}{10pt}\n\n", "\n")
        .replace("\\begin{document}\n", "\\setlength{\\parskip}{10pt}\n\\begin{document}\n");
    assert!(preamble.contains("\\setlength{\\parskip}{10pt}\n\\begin{document}\nAlpha opening paragraph.\n\nBeta"));
    assert_pdftex_glyphs(&preamble, EXPECTED, 0.01);
}
