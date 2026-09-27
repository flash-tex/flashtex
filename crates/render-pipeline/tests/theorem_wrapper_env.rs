//! A theorem-like environment or `proof` opened from a `\newenvironment`
//! body is that environment (GH-1126).
//!
//! `\newenvironment{solution}{\begin{proof}[Solution]}{\end{proof}}` runs
//! `\begin{proof}` from inside `\solution`, so pdflatex sets the amsthm
//! `\trivlist`: the head flush left in its own font, the `\topsep` above, the
//! plain style's italic body. The adapter matches environments by the name at
//! the call site, and `solution` was not a theorem name, so the pipeline set
//! an ordinary indented paragraph in the upright body font, 14.944 bp right
//! of pdflatex and 7.970 bp high per environment.
//!
//! Oracle: pdfTeX 1.40.29 (TeX Live 2026), `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, word origins read from the PDF content stream. The
//! tables are committed evidence; pdflatex never runs here. The `x` of a
//! word after a head is the head's width, so it pins the head's font too
//! (bold `CMBX10`, italic `CMTI10`).

mod common;

use common::*;

#[test]
fn wrapped_theorem_and_proofs_are_set_as_amsthm_sets_them() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let source = r"\documentclass{article}
\usepackage{amsthm}
\newtheorem{theorem}{Theorem}
\newenvironment{mythm}{\begin{theorem}}{\end{theorem}}
\newenvironment{sol}{\begin{proof}}{\end{proof}}
\newenvironment{solx}{\begin{proof}[Solution]}{\end{proof}}
\begin{document}
\begin{mythm}
Alpha beta.
\end{mythm}
\begin{sol}
Gamma delta.
\end{sol}
\begin{solx}
Eta theta.
\end{solx}
\end{document}
";
    // (first glyph of the word, pdfTeX font, x bp, baseline bp)
    assert_pdftex_glyphs(
        source,
        &[
            ("T", "CMBX10", 133.768, 134.765), // Theorem
            ("1", "CMBX10", 182.415, 134.765),
            ("A", "CMTI10", 196.307, 134.765), // Alpha
            ("b", "CMTI10", 224.852, 134.765),
            ("P", "CMTI10", 133.768, 154.690), // Proof.
            ("G", "CMR10", 164.987, 154.690),
            ("d", "CMR10", 202.688, 154.690),
            ("S", "CMTI10", 133.768, 174.615), // Solution.
            ("E", "CMR10", 177.449, 174.615),
            ("t", "CMR10", 196.403, 174.615),
        ],
        0.01,
    );
}

#[test]
fn chained_wrappers_and_wrapper_arguments() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `outerthm` is declared before the `inner` it opens, which LaTeX allows
    // (the body runs at `\begin`); `answer`'s optional argument is the proof
    // head's.
    let source = r"\documentclass{article}
\usepackage{amsthm}
\newtheorem{lemma}{Lemma}
\newenvironment{outerthm}{\begin{inner}}{\end{inner}}
\newenvironment{inner}{\begin{lemma}}{\end{lemma}}
\newenvironment{answer}[1][Answer]{\begin{proof}[#1]}{\end{proof}}
\begin{document}
Before the environments.
\begin{outerthm}
Alpha beta.
\end{outerthm}
\begin{answer}
Gamma delta.
\end{answer}
\begin{answer}[Hint]
Eta theta.
\end{answer}
After the environments.
\end{document}
";
    assert_pdftex_glyphs(
        source,
        &[
            ("B", "CMR10", 148.712, 134.765),  // Before
            ("L", "CMBX10", 133.768, 154.690), // Lemma
            ("A", "CMTI10", 188.281, 154.690), // Alpha
            ("A", "CMTI10", 133.768, 174.615), // Answer.
            ("G", "CMR10", 174.029, 174.615),
            ("H", "CMTI10", 133.768, 194.540), // Hint.
            ("E", "CMR10", 161.176, 194.540),
            ("A", "CMR10", 148.712, 214.466), // After
        ],
        0.01,
    );
}
