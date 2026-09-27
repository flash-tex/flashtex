//! A `\newtheorem` environment without amsthm is the LaTeX kernel's, and a
//! `\newenvironment` that opens `proof` is a proof.
//!
//! latex.ltx `\@begintheorem` is `\trivlist\item[\hskip\labelsep{\bfseries
//! #1\ #2}]\itshape` (`\@opargbegintheorem` adds `\ (#3)`): the head is an
//! `\item` label, so `\@item` sets it as one natural-width box followed by a
//! rigid `\labelsep`, with no punctuation and a bold note, and the
//! environment's skips are `\@trivlist`'s, which take `\partopsep` when
//! `\begin` is read in vertical mode. amsthm's head is ordinary text ending
//! in `.`, a `\thm@headsep` of `5pt plus1pt minus1pt`, and `\topsep` alone.
//! The pipeline used to give every document amsthm's head: in a prose stress
//! document without amsthm, 11 of 45 multi-line paragraphs broke where
//! pdflatex does not.
//!
//! Word origins are pdflatex's (TeX Live 2026, oracle only), read from its
//! PDF: bp from the page top.

mod common;

use common::*;

fn find<'a>(words: &'a [Word], text: &str, nth: usize) -> &'a Word {
    words
        .iter()
        .filter(|w| w.text == text)
        .nth(nth)
        .unwrap_or_else(|| panic!("no run {text:?} #{nth} in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
}

fn assert_at(words: &[Word], text: &str, nth: usize, x: f64, baseline: f64) {
    let w = find(words, text, nth);
    assert!(
        (w.x - x).abs() < 0.05 && (w.baseline - baseline).abs() < 0.05,
        "`{text}` #{nth}: pdflatex ({x}, {baseline}) bp, got ({}, {}) bp; its line: {:?}",
        w.x,
        w.baseline,
        words.iter().filter(|o| (o.baseline - w.baseline).abs() < 0.01).map(|o| (&o.text, o.x)).collect::<Vec<_>>()
    );
}

const KERNEL: &str = r"\documentclass[11pt]{article}
\newtheorem{theorem}{Theorem}
\begin{document}
Alpha opening paragraph with enough words to fill a line and then some more words to wrap.

\begin{theorem}[Lock cleanup explicitly]
unlocks on Store drop. A Unix fork regression reproduced before this fix when a still-running child inherited the lock's open-file description; the same test passes after the explicit unlock, without waiting for child execexit.
\end{theorem}

Charlie paragraph after a blank line.
\begin{theorem}
Delta theorem two, no blank line before it.
\end{theorem}
Juliet last paragraph.
\end{document}
";

#[test]
#[ignore = "needs vendor/compiler re-pinned past the compiler's `begin_kernel_theorem` (this branch's crates/compiler); passes with that compiler vendored"]
fn a_kernel_theorem_head_is_a_bold_label_box_in_a_plain_trivlist() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = words_of(&render_one(KERNEL));
    assert!(!words.iter().any(|w| w.text == "explicitly)."), "the kernel head has no punctuation");

    // `\partopsep` above: `\begin` after a blank line (amsthm: 2.989 bp
    // higher). The head is one box, so its glue is natural and the body
    // starts one rigid `\labelsep` after it; the paragraph breaks where
    // pdflatex's does (`Unix` / `fork`; amsthm's stretchable head broke
    // elsewhere in the stress corpus).
    // `(` sits one bold interword space after `1`: `\showlists` gives the
    // label box as `Theorem`, `\glue 4.19746 plus..`, `1`, the same glue,
    // `(` (49.27469 + 4.19746 + 6.2962 + 4.19746 pt = 63.727 bp from the
    // margin at 125.798 bp).
    for (text, x) in [("Theorem", 125.798), ("1", 179.067), ("(Lock", 189.525), ("cleanup", 224.601), ("explicitly)", 270.248), ("unlocks", 330.945), ("Unix", 461.81)] {
        assert_at(&words, text, 0, x, 179.796);
    }
    assert_at(&words, "fork", 0, 125.798, 193.345);
    assert_at(&words, "execexit.", 0, 246.373, 220.443);
    // `\partopsep` below as well, since the environment opened in vertical
    // mode.
    assert_at(&words, "Charlie", 0, 142.735, 245.948);
    // Opened straight after text (`\ifhmode`): no `\partopsep` above, nor
    // below.
    assert_at(&words, "Theorem", 1, 125.798, 268.463);
    assert_at(&words, "Delta", 0, 190.794, 268.463);
    // Text straight after `\end{theorem}`: `\@endparenv`'s `\@endpe` drops
    // its indent (amsthm's `\@endtheorem` would add `\@endpefalse`).
    assert_at(&words, "Juliet", 0, 125.798, 290.979);
}

/// `\newenvironment{solution}{\begin{proof}[Solution]}{\end{proof}}`, the
/// shape of `fixtures/proof-corpus/hw-problem-solution`: the head is
/// `proof`'s, a label box and a rigid `\labelsep` (5.475pt at 11pt), not
/// amsthm's `\thm@headsep` (5pt plus1pt minus1pt), which put `Factor`
/// 0.46 bp off.
const WRAPPED_PROOF: &str = r"\documentclass[11pt]{article}
\usepackage{amsthm}
\newenvironment{solution}{\begin{proof}[Solution]}{\end{proof}}
\begin{document}
\begin{solution}
Factor the polynomial and read off every root; the remaining cases follow from the same argument applied twice.
\end{solution}
\end{document}
";

#[test]
fn a_proof_opened_by_a_newenvironment_has_proofs_head() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = words_of(&render_one(WRAPPED_PROOF));
    for (text, x) in [("Solution.", 125.798), ("Factor", 173.629), ("polynomial", 224.866), ("root;", 366.804), ("remaining", 410.596)] {
        assert_at(&words, text, 0, x, 140.742);
    }
    assert_at(&words, "the", 2, 183.104, 154.291);
}
