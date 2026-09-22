//! #959: an inline formula that ends in a subscript is followed by its
//! punctuation with no gap (`$\bigcup_{i\in I} B_i$.`).
//!
//! The report read `pdftotext`'s `Bi .` as a gap; pdftotext prints the same
//! `Bi .` for pdfTeX's own PDF (the subscript's baseline differs, so it
//! starts a new word). The glyphs are where pdfTeX puts them, on main at
//! 351f1db1a and here; this pins them, `\scriptspace` included.
//!
//! ## Oracle
//!
//! pdfTeX 1.40.29 (TeX Live 2026, `/Library/TeX/texbin/pdflatex`, two runs):
//! the origin of each word's first glyph from the PDF's content stream
//! (`tools/visual-oracle/pdftext.py`), in bp, baseline from the page top;
//! the big operators (cmex glyphs) and `\in` are not compared by text.
//! Measured: every glyph within 0.006 bp. pdflatex is an oracle only.

mod common;

const PROBE: &str = r#"\documentclass{article}
\usepackage{amsmath}
\pagestyle{empty}
\begin{document}
Inline: $\bigcap_{i=1}^n A_i$ and $\bigcup_{i\in I} B_i$.
Also $x_i$. and $x^2$. and $x_i$, then $A_{ij}$; and $f$.
\end{document}
"#;

const EXPECT: &[(&str, &str, f64, f64)] = &[
    ("I", "CMR10", 148.712, 134.765), // Inline:
    ("n", "CMMI7", 188.840, 129.756), // n
    ("i", "CMMI7", 188.840, 137.754), // i=1
    ("A", "CMMI10", 203.904, 134.765), // A
    ("i", "CMMI7", 211.376, 136.259), // i
    ("a", "CMR10", 218.014, 134.765), // and
    ("B", "CMMI10", 260.107, 134.765), // B
    ("i", "CMMI7", 267.664, 136.259), // i
    (".", "CMR10", 270.980, 134.765), // .
    ("A", "CMR10", 278.171, 134.765), // Also
    ("x", "CMMI10", 300.649, 134.765), // x
    ("i", "CMMI7", 306.341, 136.259), // i
    (".", "CMR10", 309.658, 134.765), // .
    ("a", "CMR10", 316.849, 134.765), // and
    ("x", "CMMI10", 336.228, 134.765), // x
    ("2", "CMR7", 341.919, 131.149), // 2
    (".", "CMR10", 346.388, 134.765), // .
    ("a", "CMR10", 353.579, 134.765), // and
    ("x", "CMMI10", 372.958, 134.765), // x
    ("i", "CMMI7", 378.649, 136.259), // i
    (",", "CMR10", 381.966, 134.765), // ,
    ("t", "CMR10", 388.051, 134.765), // then
    ("A", "CMMI10", 410.751, 134.765), // A
    ("i", "CMMI7", 418.219, 136.259), // ij
    (";", "CMR10", 425.236, 134.765), // ;
    ("a", "CMR10", 431.321, 134.765), // and
    ("f", "CMMI10", 450.700, 134.765), // f.
];

#[test]
fn punctuation_after_a_subscript_matches_pdftex() {
    if !common::lm_available() {
        eprintln!("SKIP subscript_then_period: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(PROBE, EXPECT, 0.5);
}
