//! The gap between an amsthm head and its body is the package's own glue,
//! not an interword space.
//!
//! Measured against pdflatex (TeX Live 2025, `\documentclass[11pt]{article}`
//! + `[T1]{fontenc}` + amsthm, `\tracingoutput=1`; oracle only, never in the
//! product path). The shipped page holds, for a `\newtheorem` environment:
//!
//! ```text
//! \T1/cmr/bx/n/10.95 .          % \the\thm@headpunct
//! \glue 5.0 plus 1.0 minus 1.0  % \hskip\thm@headsep
//! \T1/cmr/m/n/10.95 L e t       % the body
//! ```
//!
//! and, for `proof` (`\item[\hskip\labelsep \itshape #1\@addpunct{.}]`), the
//! `\@labels` box followed by `\@item`'s own trailing `\hskip\labelsep`:
//!
//! ```text
//! \hbox(7.60416+2.12917)x67.05003
//! .\glue 5.475                  % \hskip\labelsep, inside the label box
//! .\OT1/cmr/m/it/10.95 P r o o f … .
//! \glue 5.475                   % \@item's trailing \hskip\labelsep
//! \penalty 0
//! ```
//!
//! Before this, both were an interword space read from the source newline
//! after `\begin{...}` — and because the head ends in `.`, a *sentence*
//! space: `4.83948 plus 5.44014 minus 0.40297` at 11pt, five times the
//! stretch `\thm@headsep` asks for.

mod common;

use common::*;

/// TeX points to PDF points, the unit of every v2 coordinate.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

const SRC: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage{amsthm}
\newtheorem{theorem}{Theorem}
\begin{document}
\begin{theorem}
Let every integer be divisible by one.
\end{theorem}

\begin{proof}
Run the Euclidean algorithm on the pair.
\end{proof}
\end{document}
";

/// The gap between the run whose text ends `prefix` and the next run on the
/// same line.
fn gap_after(words: &[Word], prefix: &str) -> f64 {
    let i = words
        .iter()
        .position(|w| w.text.starts_with(prefix))
        .unwrap_or_else(|| panic!("no run starting {prefix:?} in {words:?}"));
    let head = &words[i];
    let next = words
        .iter()
        .skip(i + 1)
        .find(|w| (w.baseline - head.baseline).abs() < 0.01)
        .unwrap_or_else(|| panic!("nothing follows {prefix:?} on its line"));
    next.x - (head.x + head.width)
}

#[test]
fn a_newtheorem_head_is_followed_by_thm_headsep_and_proof_by_labelsep() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SRC);
    let words = words_of(&r);

    // `\thm@headsep` = 5pt plus 1pt minus 1pt, absolute: it does not scale
    // with the class size, and the line is not stretched here.
    let theorem = gap_after(&words, "Theorem");
    assert!(
        (theorem - bp(5.0)).abs() < 0.01,
        "a \\newtheorem head is followed by \\hskip\\thm@headsep = 5pt ({} bp), got {theorem} bp",
        bp(5.0)
    );

    // `proof` is a plain `\trivlist` `\item`, so the glue after its label is
    // `\@item`'s `\hskip\labelsep`: `\showthe\labelsep` prints 5.475pt in an
    // 11pt article, and it is rigid.
    let proof = gap_after(&words, "Proof");
    assert!(
        (proof - bp(5.475)).abs() < 0.01,
        "a proof head is followed by \\hskip\\labelsep = 5.475pt ({} bp), got {proof} bp",
        bp(5.475)
    );
}

/// The separator replaces the interword space rather than adding to it:
/// amsthm's head ends with `\ignorespaces`, and `\label`'s `\@esphack`
/// re-asserts it after taking the same glue off and putting it back. Both
/// spellings of the source must give the same gap.
#[test]
fn the_head_glue_replaces_the_source_gap_whatever_follows_the_begin() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let same_line = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage{amsthm}
\newtheorem{theorem}{Theorem}
\begin{document}
\begin{theorem}Let every integer be divisible by one.
\end{theorem}
\end{document}
";
    let labelled = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage{amsthm}
\newtheorem{theorem}{Theorem}
\begin{document}
\begin{theorem}\label{t}
Let every integer be divisible by one.
\end{theorem}
\end{document}
";
    for (what, src) in [("same line", same_line), ("after \\label", labelled)] {
        let words = words_of(&render_one(src));
        let gap = gap_after(&words, "Theorem");
        assert!(
            (gap - bp(5.0)).abs() < 0.01,
            "{what}: expected \\thm@headsep ({} bp), got {gap} bp",
            bp(5.0)
        );
    }
}
