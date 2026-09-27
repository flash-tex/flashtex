//! A `\newenvironment` that writes its own head (`\textit{Solution.}\ `) is
//! not a theorem head. The pipeline read any paragraph whose expanded text
//! carries the `\begin` span as an amsthm head and swapped the `\ ` plus the
//! blank after `\begin{solution}` for `\thm@headsep` (5pt ±1pt). pdflatex
//! sets both spaces: `\glue 3.33333 plus 1.66666 minus 1.11111` for the `\ `,
//! then `\glue 4.44444 plus 4.99997 minus 0.37036`, the newline read at the
//! `Solution.` space factor (a control space leaves `\spacefactor` alone).
//! The shape is common in student problem sets
//! (fixtures/proof-corpus/problem-set-solution-env: 118 -> 156 of 217 words
//! within 0.05 bp).
//!
//! Word origins are pdflatex's (TeX Live 2026, oracle only), read from its
//! PDF: bp from the page top.

mod common;

use common::*;

const SRC: &str = r"\documentclass{article}
\newcommand\sol{\textit{Solution.}\ }
\newenvironment{solution}{\par\noindent\textit{Solution.}\ }{\par}
\newenvironment{answer}[1][A]{\par\noindent\textbf{Answer #1.}\ }{\par}
\begin{document}
\textit{Solution.}\ Alpha line with the control space typed in the source.

\textit{Solution.}\
Bravo after a control space that ends the line.

\sol Charlie after a macro invoked as a control word.

\sol{} Delta after a macro with an empty group.

\begin{solution} Echo on the same line as the begin.
\end{solution}

\begin{answer}[B]
Foxtrot after an environment with an optional argument.
\end{answer}

\begin{solution}
Golf with a displayed equation next to test more text that wraps onto a second line of the paragraph here.
\end{solution}
\end{document}
";

#[test]
fn a_solution_environment_keeps_its_control_space_and_the_blank_after_begin() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = words_of(&render_one(SRC));
    let at = |text: &str| {
        words
            .iter()
            .find(|w| w.text == text)
            .unwrap_or_else(|| panic!("no run {text:?} in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
    };
    // `Bravo` (`\` at the end of a line, `\^^M`) is 0.53 bp off before and
    // after this change; it is not what this test is about.
    for (text, x, baseline) in [
        // A typed `\ `: one space, the blank after it skipped (state S).
        ("Alpha", 190.729, 134.765),
        // `\sol` is a control word: it eats the blank.
        ("Charlie", 190.729, 158.675),
        // `\sol{}`: `\ ` then a sentence space (factor 3000 kept by `\ `).
        ("Delta", 195.163, 170.63),
        // `\begin{solution}` then a blank: both spaces.
        ("Echo", 180.219, 182.585),
        ("Foxtrot", 193.033, 194.541),
        ("Golf", 183.765, 206.496),
        ("wraps", 451.936, 206.496),
    ] {
        let w = at(text);
        assert!(
            (w.x - x).abs() < 0.05 && (w.baseline - baseline).abs() < 0.05,
            "`{text}`: pdflatex ({x}, {baseline}) bp, got ({}, {}) bp",
            w.x,
            w.baseline
        );
    }
}
