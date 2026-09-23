//! An amsthm environment that opens straight into a list runs the list's
//! first `\item` in on the head's line, against pdflatex.
//!
//! amsthm's head is a pending `\item` label of the environment's
//! `\trivlist` (`\if@inlabel`) until text starts a paragraph. A list begun
//! then goes through `\@trivlist`'s `\if@inlabel` branch (`\@noparlist`,
//! `\@noparitem`): no `\@topsep`, no `\@beginparpenalty`, and its first
//! `\item` runs `\@donoparitem`, which wraps the pending head as
//! `\hskip-\leftmargin <head> \hskip\labelsep \hskip\leftmargin` in front of
//! the item's own label. So `Proof. 1. First item` share a line, with the
//! head at the environment's left edge and the item shifted right by the
//! head's width plus `\labelsep`. At the list's end `\endtrivlist` skips
//! `\@endparenv` (no penalty, no skip, no `\@endpe`), so text after the list
//! is indented. The pipeline set the head on a line of its own, about 22.5bp
//! (a line plus the list's `\topsep`) above the first item.
//!
//! Oracle: MacTeX 2026 pdflatex, glyph origins from PyMuPDF `rawdict`
//! (`x` of each word's first glyph, baseline `y` from the page top, bp).
//! No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of, Word};

const SRC: &str = "\\documentclass[11pt]{article}
\\usepackage{amsmath,amsthm}
\\newtheorem{theorem}{Theorem}
\\newenvironment{solution}{\\begin{proof}[Solution]}{\\end{proof}}
\\begin{document}
Opening text of the document.

\\begin{proof}
\\begin{itemize}
\\item One item of an itemize list that is long enough to wrap onto a second line of the page here.
\\item Two.
\\end{itemize}
After the list, still in the proof.
\\end{proof}
Text after.

\\begin{theorem}
\\begin{enumerate}
\\item Theorem item.
\\item Second.
\\end{enumerate}
\\end{theorem}

\\begin{proof}

\\begin{description}
\\item[Case 1] First case.
\\item[Case 2] Second case.
\\end{description}
\\end{proof}

\\begin{solution}
% a comment
\\begin{enumerate}
\\item Last item with qed. \\qedhere
\\end{enumerate}
\\end{solution}

\\begin{theorem}[Note]
\\begin{itemize}
\\item Noted.
\\end{itemize}
\\end{theorem}
\\begin{proof}
Text first.
\\begin{enumerate}
\\item Not run in.
\\end{enumerate}
\\end{proof}
Final paragraph.
\\end{document}
";

/// The first rendered word at pdflatex's baseline `y` whose text starts with
/// `text`.
fn word<'w>(words: &'w [Word], text: &str, y: f64) -> Option<&'w Word> {
    words.iter().find(|w| w.page == 1 && (w.baseline - y).abs() < 0.1 && w.text.starts_with(text))
}

#[test]
fn a_list_right_after_a_theorem_head_runs_in_like_pdflatex() {
    if !lm_available() {
        return;
    }
    let words = words_of(&render_one(SRC));
    // (word, pdflatex x, pdflatex baseline), bp.
    let want = [
        // proof + itemize: head, bullet and text on one line.
        ("Proof.", 125.798, 163.258),
        ("One", 187.260, 163.258),
        // Text after that list is indented (no `\@endpe`).
        ("After", 142.735, 212.872),
        // A numbered theorem head and an enumerate.
        ("Theorem", 125.798, 257.903),
        // A blank line between the head and a description changes nothing
        // (`\par` in vertical mode); the label follows the head.
        ("Proof.", 125.798, 302.934),
        ("Case", 159.987, 302.934),
        ("Case", 125.798, 325.450),
        // A `\newenvironment` wrapper of `proof` with an optional head and a
        // comment line before the list.
        ("Solution.", 125.798, 361.514),
        ("Last", 200.907, 361.514),
        // A theorem with a note.
        ("Theorem", 125.798, 384.030),
        ("Noted.", 256.467, 384.030),
        // Text between the head and the list: no run-in.
        ("Proof.", 125.798, 406.546),
        ("Not", 153.074, 429.061),
        ("Final", 142.735, 474.092),
    ];
    for (text, x, y) in want {
        let w = word(&words, text, y).unwrap_or_else(|| {
            let near: Vec<_> = words.iter().filter(|w| w.text.starts_with(text)).map(|w| (w.x, w.baseline)).collect();
            panic!("{text:?}: no word on pdflatex's baseline {y}; FlashTeX has it at {near:?}")
        });
        assert!((w.x - x).abs() < 0.1, "{text:?} at y={y}: x {:.3}bp, pdflatex {x:.3}bp", w.x);
    }
}
