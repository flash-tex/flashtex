//! A blank in front of a text font command in an amsthm body is set in the
//! font in force at the blank, not in the font of the word after it.
//!
//! amsthm theorem-like bodies (and `proof`) are laid out with bold and
//! italic from the compiler's scoping (`compiler_weight`), because the
//! package, not the source, declares their fonts. That path copied the
//! next run's bold/italic onto the interword glue in front of it, so in
//! `A \emph{word} is a \textbf{string}` the blank before `word` was a cmti
//! space and the one before `string` a cmbx space: +0.27 bp and +0.55 bp at
//! 11pt, accumulating along the line (1.09 bp by `text.` here). TeX reads
//! the space token before `\emph` in the body font; the compiler already
//! says so in `glue_before`.
//!
//! Oracle: pdflatex (TeX Live 2026, pdfTeX 1.40.29, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`) of `SRC`, the first glyph of each word, origin in
//! bp from the page top (oracle only, never in the product path).

mod common;

use common::*;

const SRC: &str = r"\documentclass[11pt]{article}
\usepackage{amsthm}
\theoremstyle{definition}
\newtheorem{definition}{Definition}
\theoremstyle{remark}
\newtheorem{remark}{Remark}
\begin{document}
\begin{definition}
A \emph{word} is a \textbf{string} and \textit{more} text.
\end{definition}
\begin{remark}
A \emph{word} is a \textbf{string} and \textit{more} text.
\end{remark}
\begin{proof}
A \emph{word} is a \textbf{string} and \textit{more} text.
\end{proof}
\end{document}
";

#[test]
fn a_font_command_in_a_theorem_body_keeps_the_body_font_space_before_it() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    assert_pdftex_glyphs(
        SRC,
        &[
            // definition (upright body)
            ("w", "CMTI10 word", 210.922, 140.742),
            ("i", "CMR10 is", 238.129, 140.742),
            ("s", "CMBX10 string", 258.191, 140.742),
            ("a", "CMR10 and", 293.722, 140.742),
            ("m", "CMTI10 more", 314.940, 140.742),
            ("t", "CMR10 text.", 342.965, 140.742),
            // remark (upright body, italic head)
            ("w", "CMTI10 word", 193.009, 163.258),
            ("i", "CMR10 is", 220.216, 163.258),
            ("s", "CMBX10 string", 240.267, 163.258),
            ("a", "CMR10 and", 275.809, 163.258),
            ("m", "CMTI10 more", 297.027, 163.258),
            ("t", "CMR10 text.", 325.042, 163.258),
            // proof
            ("w", "CMTI10 word", 171.802, 185.773),
            ("i", "CMR10 is", 199.009, 185.773),
            ("s", "CMBX10 string", 219.060, 185.773),
            ("a", "CMR10 and", 254.602, 185.773),
            ("m", "CMTI10 more", 275.820, 185.773),
            ("t", "CMR10 text.", 303.835, 185.773),
        ],
        0.05,
    );
}
