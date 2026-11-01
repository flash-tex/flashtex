//! A REVTeX `rmp` document's bibliography is natbib's author-year list.
//!
//! The REVTeX classes load natbib themselves (compiler
//! `natbib::Options::revtex`); under the `rmp` journal it is author-year,
//! so `thebibliography` has no `[n]` label and each entry starts flush at
//! the margin with `\bibhang` for its continuation lines, as pdfTeX sets
//! the Horodecki review (arXiv quant-ph/0702225, `revtex4` `[aps,rmp]`):
//! entry, heading and column margin all at x = 54.000 bp in pdfTeX's PDF.
//! The class's own page geometry is not modelled (#905), so only the
//! relation between margin, heading and entries is pinned, and the
//! citation text. pdflatex is an oracle only.

mod common;

const PROBE: &str = r#"\documentclass[aps,rmp]{revtex4}
\begin{document}
Text \cite{Abe} and \cite{Gross}.
\begin{thebibliography}{2}
\expandafter\ifx\csname citenamefont\endcsname\relax
  \def\citenamefont#1{#1}\fi
\bibitem[{\citenamefont{Abe and Rajagopal}(2001)}]{Abe}
Abe, S., and A.~K. Rajagopal, 2001, Physica A 289, 157, with enough words
to run onto a second line of the entry.
\bibitem[{\citenamefont{Gr{\"o}{\ss}er}(1999)}]{Gross}
Gross, 1999.
\end{thebibliography}
\end{document}
"#;

#[test]
fn rmp_entries_start_at_the_margin_and_cite_by_author_and_year() {
    if !common::lm_available() {
        eprintln!("SKIP revtex_rmp_bibliography: Latin Modern not installed");
        return;
    }
    let r = common::render_one(PROBE);
    let words = common::words_of(&r);
    let text: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    let joined = text.join(" ");
    // `words_of` splits at font changes, so compare without the blanks.
    assert!(joined.replace(' ', "").contains("(AbeandRajagopal,2001)and(Größer,1999)."), "{joined}");
    assert!(!joined.contains('['), "{joined}");
    let x_of = |t: &str| words.iter().find(|w| w.text == t).map(|w| w.x).unwrap_or_else(|| panic!("no {t:?} in {joined}"));
    let margin = x_of("References");
    assert!((x_of("Abe,") - margin).abs() < 0.01, "entry at {} vs margin {margin}", x_of("Abe,"));
    assert!((x_of("Gross,") - margin).abs() < 0.01, "entry at {} vs margin {margin}", x_of("Gross,"));
}
