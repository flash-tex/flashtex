//! A `\label` between a heading and a list does not end `\@afterheading`:
//! in vertical mode `\label` is `\@bsphack` + `\protected@write`, a `\write`
//! whatsit and nothing else, so `\@nobreak` is still true when the first
//! `\item` comes and `\@item` runs `\@nbitem` (`\addvspace{\@outerparskip -
//! \parskip}`), not `\addvspace\@topsep`. The list's first line therefore
//! sits exactly where it would without the label: the heading's after-skip
//! and no `\topsep + \partopsep`. Reading the label as a paragraph put
//! hyperref-toc page 4 (`\section{Appendix: Glossary}\label{..}` then an
//! `itemize`) 1.155 bp too low, every word.
//!
//! Oracle: MacTeX 2026 pdflatex (`SOURCE_DATE_EPOCH=0`, two runs), word
//! origins from `tools/visual-oracle/pdftext.py`: the first item's baseline
//! is at y_top 107.308 bp on both pages 1 (with the label) and 2 (without).
//! `\showlists` at the first `\item` (11pt article): heading, `\penalty
//! 10000`, `\glue 10.84085 plus 0.94266`, `\write1{\newlabel..}`,
//! `\penalty 10000` x2, `\glue -4.5 plus -1.0 minus -1.0`, `\glue(\parskip)
//! 4.5 plus 2.0 minus 1.0` -- the same 10.84085 pt as the label-less page,
//! where `\@xaddvskip` folds the -4.5 into the heading's glue. After a
//! paragraph (page 3) the label changes nothing either: `\@topsep` as
//! usual. No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of};

const TOL: f64 = 0.02;

const DOC: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\begin{document}
\section{Appendix: Glossary}
\label{sec:appendix}

\begin{itemize}
  \item \textbf{Badness}: a numeric measure of how far a line.
  \item Glue: a flexible space.
\end{itemize}
Text after.
\newpage
\section{Appendix: Glossary}

\begin{itemize}
  \item \textbf{Badness}: a numeric measure of how far a line.
  \item Glue: a flexible space.
\end{itemize}
Text after.
\newpage
Some paragraph text.
\label{x}

\begin{itemize}
  \item Badness: a numeric measure of how far a line.
\end{itemize}
Text after.
\end{document}
";

#[test]
fn a_label_after_a_heading_keeps_the_first_item_where_nbitem_puts_it() {
    if !lm_available() {
        return;
    }
    let r = render_one(DOC);
    let words = words_of(&r);
    let baseline = |page: u32, text: &str| {
        words
            .iter()
            .find(|w| w.page == page && w.text.starts_with(text))
            .unwrap_or_else(|| panic!("no {text:?} on page {page}"))
            .baseline
    };
    let head = |page: u32| baseline(page, "Appendix");
    // pdflatex: heading y_top 82.959 and first item 107.308 on pages 1 and
    // 2 (24.349 bp apart); on page 3 the paragraph at 82.959 and the item
    // at 108.463 (25.504 bp).
    let gaps = [
        (1, baseline(1, "Badness") - head(1), 24.349),
        (2, baseline(2, "Badness") - head(2), 24.349),
        (3, baseline(3, "Badness") - baseline(3, "Some"), 25.504),
    ];
    let misses: Vec<String> = gaps
        .iter()
        .filter(|(_, got, want)| (got - want).abs() > TOL)
        .map(|(page, got, want)| format!("page {page}: first item {got:.3} bp below the line above, pdflatex {want:.3}"))
        .collect();
    assert!(misses.is_empty(), "{}", misses.join("\n"));
}
