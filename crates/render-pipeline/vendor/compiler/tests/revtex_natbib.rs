//! The REVTeX classes load natbib themselves and set its punctuation from
//! the society and journal (`natbib::Options::revtex`).
//!
//! `revtex4.cls`, `revtex4-1.cls` and `revtex4-2.cls` all run
//! `\RequirePackage[sort&compress]{natbib}`, so a revtex document cites in
//! natbib's forms without `\usepackage{natbib}`. The compiler did not know
//! that and set the kernel's `[Einstein et al.(1935)Einstein, ...]` labels
//! (the Horodecki entanglement review, arXiv quant-ph/0702225, is
//! `revtex4` `[aps,rmp]`).
//!
//! Oracle: pdflatex (TeX Live 2026: revtex4 4.0b, revtex4-1 4.1r, revtex4-2
//! 4.2f; two runs), `pdftotext` of the paragraph
//! `A \cite{kp} B \citep{kp} C \citet{kp} D \cite{plass81,hobby} E
//! \citep[p.~7]{kp} F.` with the bibliography below (the leading page
//! number revtex prints is dropped). pdflatex is an oracle only.
use flashtex_compiler::parser::{parse, Block, Inline};

const BIB: &str = r"\begin{thebibliography}{3}
\bibitem[{\citenamefont{Knuth and Plass}(1981)}]{kp}
D.~E. Knuth and M.~F. Plass.
\bibitem[{\citenamefont{Plass}(1981)}]{plass81}
M.~F. Plass.
\bibitem[{\citenamefont{Hobby}(1986)}]{hobby}
J.~D. Hobby.
\end{thebibliography}";

const NUMBERED: &str = "A [1] B [1] C Knuth and Plass [1] D [2, 3] E [1, p. 7] F.";
const AUTHOR_YEAR: &str =
    "A (Knuth and Plass, 1981) B (Knuth and Plass, 1981) C Knuth and Plass (1981) D (Plass, 1981; Hobby, 1986) E (Knuth and Plass, 1981, p. 7) F.";

fn text(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text { text, space_before, .. } => {
                if *space_before && !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str(text);
            }
            Inline::HBox(boxed) => text(&boxed.content, out),
            _ => {}
        }
    }
}

fn set(class: &str, options: &str) -> String {
    let source = format!(
        "\\documentclass[{options}]{{{class}}}\n\\begin{{document}}\nA \\cite{{kp}} B \\citep{{kp}} C \\citet{{kp}} D \\cite{{plass81,hobby}} E \\citep[p.~7]{{kp}} F.\n\n{BIB}\n\\end{{document}}\n"
    );
    let parsed = parse(&source);
    let mut out = String::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            text(inlines, &mut out);
            break;
        }
    }
    out.replace('\u{a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn aps_journals_cite_by_number() {
    for (class, options) in [("revtex4", "aps"), ("revtex4-1", ""), ("revtex4-2", "aps,pra"), ("revtex4-2", "aps,prb")] {
        assert_eq!(set(class, options), NUMBERED, "{class}[{options}]");
    }
}

/// `rmp`: author-year, and `\let\cite\citep`.
#[test]
fn rmp_cites_by_author_and_year() {
    for (class, options) in [("revtex4", "aps,rmp"), ("revtex4-1", "rmp"), ("revtex4-2", "aps,rmp")] {
        assert_eq!(set(class, options), AUTHOR_YEAR, "{class}[{options}]");
    }
}

/// `revtex4`/`revtex4-1` `prb`: superscript numbers without brackets
/// (pdflatex `A1 B1 C Knuth and Plass 1 D2,3 E1 (p. 7) F.`). natbib's
/// superscript form is not modelled (the numbers stay on the baseline with
/// a blank before them, and the compiler says so); only the numbered,
/// bracketless mode is pinned here.
#[test]
fn prb_numbers_have_no_brackets() {
    for class in ["revtex4", "revtex4-1"] {
        let out = set(class, "aps,prb");
        assert!(out.starts_with("A 1 B 1 C Knuth and Plass 1 D 2, 3") && !out.contains('['), "{class}: {out}");
    }
}

/// A class that is not REVTeX keeps the kernel `\cite`.
#[test]
fn article_keeps_the_kernel_cite() {
    assert!(set("article", "").starts_with("A [Knuth and Plass(1981)]"), "{}", set("article", ""));
}
