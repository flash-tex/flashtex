//! #956: a `\bibitem` label is TeX source, and citations set it.
//!
//! natbib-style `.bbl` files (`apsrmp`, `plainnat`, every revtex bibliography
//! style) write the author-year label with markup in it:
//!
//! ```latex
//! \bibitem[{\citenamefont{Abeyesinghe}
//!   \emph{et~al.}(2006)\citenamefont{Abeyesinghe, Devetak, Hayden, and
//!   Winter}}]{AbeyesingheDHW2006-fullSW}
//! ```
//!
//! The pre-scan flattened that label to characters (braces dropped, control
//! words kept as `\name`), so every citation of an "et al." paper printed
//! `\emphet~al.` literally -- 1253 times in the Horodecki entanglement review
//! (arXiv quant-ph/0702225). pdflatex sets `\emph{et~al.}` as italic "et al."
//! with a tie; the kernel's `\@lbibitem` and natbib's `\@lbibitem` both
//! typeset what they take from the label.
//!
//! Oracle (text only): pdfTeX 1.40.29, TeX Live 2026, `pdftotext` of the
//! `revtex4` `rmp` document below with the same `.bbl`:
//! `Text (Abeyesinghe et al., 2006) and (Abe and Rajagopal, 2001).`
//! pdflatex is an oracle only and never runs here.

use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument};

const BBL: &str = r#"\begin{thebibliography}{2}
\expandafter\ifx\csname natexlab\endcsname\relax\def\natexlab#1{#1}\fi
\expandafter\ifx\csname bibnamefont\endcsname\relax
  \def\bibnamefont#1{#1}\fi
\expandafter\ifx\csname citenamefont\endcsname\relax
  \def\citenamefont#1{#1}\fi
\providecommand{\bibinfo}[2]{#2}

\bibitem[{\citenamefont{Abe and Rajagopal}(2001)}]{Abe}
\bibinfo{author}{\bibnamefont{Abe}}, \bibinfo{year}{2001}.

\bibitem[{\citenamefont{Abeyesinghe}
  \emph{et~al.}(2006)\citenamefont{Abeyesinghe, Devetak, Hayden, and
  Winter}}]{AbeyesingheDHW2006-fullSW}
\bibinfo{author}{\bibnamefont{Abeyesinghe}}, \bibinfo{year}{2006}.

\bibitem[{\citenamefont{Gr{\"o}{\ss}er}(1999)}]{Gross}
\bibinfo{author}{\bibnamefont{Gr{\"o}{\ss}er}}, \bibinfo{year}{1999}.

\bibitem[{\citenamefont{{Smith (ed.)}}(1990)}]{Ed}
\bibinfo{author}{Smith}, \bibinfo{year}{1990}.

\end{thebibliography}
"#;

fn parse(preamble: &str, body: &str) -> flashtex_compiler::parser::Parsed {
    let main = format!("\\documentclass{{article}}{preamble}\\begin{{document}}{body}\\bibliography{{refs}}\\end{{document}}");
    let documents = [SourceDocument { path: "main.tex", text: &main }, SourceDocument { path: "refs.bbl", text: BBL }];
    parse_project(&documents, "main.tex")
}

/// The first paragraph's text runs as (text, italic).
fn first_paragraph(parsed: &flashtex_compiler::parser::Parsed) -> Vec<(String, bool)> {
    let inlines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("a paragraph");
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, style, space_before, .. } => {
                Some((format!("{}{text}", if *space_before { " " } else { "" }), style.italic))
            }
            _ => None,
        })
        .collect()
}

fn joined(runs: &[(String, bool)]) -> String {
    runs.iter().map(|(text, _)| text.as_str()).collect()
}

#[test]
fn natbib_sets_the_label_markup_instead_of_printing_it() {
    let parsed = parse("\\usepackage{natbib}", "Text \\citep{AbeyesingheDHW2006-fullSW} and \\citep{Abe}, \\citet{Gross}.");
    let runs = first_paragraph(&parsed);
    let text = joined(&runs);
    assert_eq!(
        text, "Text (Abeyesinghe et\u{a0}al., 2006) and (Abe and Rajagopal, 2001), Größer (1999).",
        "{runs:?}"
    );
    assert!(!text.contains('\\'), "{text}");
    // `\emph` in upright text is italic, and only "et al." is.
    let italic: String = runs.iter().filter(|(_, italic)| *italic).map(|(text, _)| text.as_str()).collect();
    assert_eq!(italic.trim(), "et\u{a0}al.", "{runs:?}");
}

#[test]
fn the_kernel_label_is_set_too() {
    let parsed = parse("", "Text \\cite{AbeyesingheDHW2006-fullSW}.");
    let text = joined(&first_paragraph(&parsed));
    assert!(!text.contains('\\') && !text.contains('~'), "{text}");
    assert!(text.contains("Abeyesinghe et\u{a0}al.(2006)Abeyesinghe, Devetak, Hayden, and Winter"), "{text}");
}

#[test]
fn a_label_splits_at_brace_depth_zero_only() {
    // `\NAT@ifcmd#1(@)` is a delimited argument: the `(` inside the braces
    // is part of the author list.
    let parsed = parse("\\usepackage{natbib}", "\\citet{Ed} \\citet{Gross}.");
    let text = joined(&first_paragraph(&parsed));
    assert_eq!(text.trim(), "Smith (ed.) (1990) Größer (1999).");
}

