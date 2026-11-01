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

/// A wrapper's own arguments are read before its body: the head separator
/// goes after `{Important}` (the note `(Important)` is part of the head),
/// after an optional `[Hint]` plus a mandatory `{Two}`, after a mandatory
/// argument on the next line, and after an argument the begin code drops.
/// pdflatex (pdfTeX 1.40.29, TeX Live 2026, `SOURCE_DATE_EPOCH=0
/// FORCE_SOURCE_DATE=1`), word origins from the PDF.
#[test]
fn a_wrapper_reads_its_arguments_before_the_head_separator() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let source = r"\documentclass{article}
\usepackage{amsthm}
\pagestyle{empty}
\newtheorem{lemma}{Lemma}
\theoremstyle{remark}
\newtheorem{remark}{Remark}
\newenvironment{keylemma}[1]{\begin{lemma}[#1]}{\end{lemma}}
\newenvironment{keyremark}[2][Note]{\begin{remark}[#1: #2]}{\end{remark}}
\newenvironment{plainremark}[1]{\begin{remark}}{\end{remark}}
\begin{document}
Alpha opening paragraph.
\begin{keylemma}{Important}Beta body.\end{keylemma}
\begin{keyremark}[Hint]{Two}Delta body.\end{keyremark}
\begin{keyremark}
{Three}
Epsilon body.\end{keyremark}
\begin{plainremark}{Dropped}Zeta body.\end{plainremark}
Gamma closing paragraph.
\end{document}
";
    assert_pdftex_glyphs(
        source,
        &[
            ("L", "CMBX10", 133.768, 154.690), // Lemma
            ("(", "CMR10", 183.944, 154.690),  // (Important).
            ("B", "CMTI10", 244.440, 154.690), // Beta
            ("(", "CMR10", 180.313, 174.615),  // (Hint:
            ("D", "CMR10", 241.763, 174.615),  // Delta
            ("(", "CMR10", 180.313, 190.555),  // (Note:
            ("E", "CMR10", 249.542, 190.555),  // Epsilon
            ("Z", "CMR10", 184.783, 206.496),  // Zeta
            ("G", "CMR10", 148.712, 222.436),  // Gamma
        ],
        0.01,
    );
}

/// `%` comments split a wrapper's declaration over lines and sit between
/// its call's arguments: TeX drops each comment with its line end and the
/// spaces that open the next line, so the declarations are wrappers all
/// the same and the head separator still follows the last argument (and a
/// `[<note>]` the inner environment reads from the next line). pdflatex
/// (pdfTeX 1.40.29, TeX Live 2026, `SOURCE_DATE_EPOCH=0
/// FORCE_SOURCE_DATE=1`), word origins from the PDF.
#[test]
fn comments_split_wrapper_declarations_and_arguments() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let source = r"\documentclass{article}
\usepackage{amsthm}
\pagestyle{empty}
\newtheorem{lemma}{Lemma}
\newenvironment{wrap}%
  {\begin{lemma}}%
  {\end{lemma}}
\newenvironment{keylemma}%
  [2][Key]%
  {\begin{lemma}[#1: #2]}%
  {\end{lemma}}
\begin{document}
Alpha opening paragraph.
\begin{wrap}Beta body.\end{wrap}
\begin{keylemma}%
  [Hint]%
  {Two}%
  Delta body.\end{keylemma}
\begin{keylemma}% a comment
  {Three}% another
Epsilon body.\end{keylemma}
\begin{wrap}% the note is on the next line
  [Named]Zeta body.\end{wrap}
Gamma closing paragraph.
\end{document}
";
    assert_pdftex_glyphs(
        source,
        &[
            ("L", "CMBX10", 133.768, 154.690), // Lemma 1.
            ("B", "CMTI10", 188.281, 154.690), // Beta
            ("(", "CMR10", 183.944, 174.615),  // (Hint:
            ("D", "CMTI10", 245.519, 174.615), // Delta
            ("(", "CMR10", 183.944, 194.540),  // (Key:
            ("E", "CMTI10", 249.969, 194.540), // Epsilon
            ("(", "CMR10", 183.944, 214.466),  // (Named).
            ("Z", "CMTI10", 230.574, 214.466), // Zeta
            ("G", "CMR10", 148.712, 234.391),  // Gamma
        ],
        0.01,
    );
}
