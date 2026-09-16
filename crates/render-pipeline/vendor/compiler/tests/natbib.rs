//! natbib citations, end to end through the parser.
//!
//! Every expectation here is the string pdfLaTeX actually sets: each form was
//! compiled with `pdflatex` (TeX Live 2025, `pdfTeX 3.141592653-2.6-1.40.27`)
//! against `natbib.sty` 2010/09/13 8.31b, with `\showthe\wd` of an `\hbox`
//! around it and the word positions read back out of the PDF. The box widths
//! that pin them are in the table below (11 pt article, `[authoryear,round]`,
//! the bibliography of `forms()`), so a change that alters a character also
//! alters a width someone can re-measure:
//!
//! | form | `\showthe\wd` |
//! |---|---|
//! | `\citet{kp}` | 114.33638pt |
//! | `\citep{kp}` | 117.37805pt |
//! | `\citep[p.~7]{kp}` | 142.31973pt |
//! | `\citep[see][p.~7]{kp}` | 160.02223pt |
//! | `\citep[see][]{kp}` | 135.08055pt |
//! | `\citet[p.~7]{kp}` | 139.27806pt |
//! | `\citet[see][p.~7]{kp}` | 156.98056pt |
//! | `\citealt{kp}` | 105.81969pt |
//! | `\citealp{kp}` | 108.86136pt |
//! | `\citeauthor{kp}` | 80.26967pt |
//! | `\citeyear{kp}` | 21.90002pt |
//! | `\citeyearpar{kp}` | 30.41672pt |
//! | `\cite{kp}` | 114.33638pt |
//! | `\cite[p.~7]{kp}` | 142.31973pt |
//! | `\citep{plass81,hobby,frank}` | 191.47311pt |
//! | `\citet{plass81,hobby}` | 130.7614pt |
//! | `\citep{plass81,plass90}` | 90.30717pt |
//! | `\citet{plass81,plass90}` | 87.2655pt |
//! | `\citeauthor{plass81,hobby}` | 62.62798pt |
//! | `\citeyear{plass81,hobby}` | 50.49171pt |
//! | `\citep{nosuch}` | 14.4631pt |
//! | `\citet{jones}` | 88.42134pt |
//! | `\citet*{jones}` | 165.31477pt |

use flashtex_compiler::parser::{parse, Block, Inline};

/// The bibliography every citation below resolves against.
fn bibliography() -> &'static str {
    r"\begin{thebibliography}{99}
\bibitem[Knuth and Plass(1981)]{kp}
D.~E. Knuth and M.~F. Plass.
\bibitem[Plass(1981)]{plass81}
M.~F. Plass.
\bibitem[Plass(1990)]{plass90}
M.~F. Plass.
\bibitem[Hobby(1986)]{hobby}
J.~D. Hobby.
\bibitem[Frank(1990)]{frank}
A.~Frank.
\bibitem[Jones et al.(1990)Jones, Baker, and Williams]{jones}
A.~Jones.
\end{thebibliography}"
}

/// The inline text of the first paragraph of `body`, with natbib loaded
/// under `options` and the shared bibliography after it.
fn set_with(options: &str, body: &str) -> String {
    let source = format!(
        "\\documentclass[11pt]{{article}}\\usepackage[{options}]{{natbib}}\\begin{{document}}\n{body}\n\n{}\n\\end{{document}}",
        bibliography()
    );
    let parsed = parse(&source);
    let mut out = String::new();
    for block in &parsed.blocks {
        let Block::Paragraph(inlines) = block else {
            continue;
        };
        for inline in inlines {
            if let Inline::Text { text, .. } = inline {
                out.push_str(text);
            }
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

fn set(body: &str) -> String {
    set_with("authoryear,round", body)
}

#[test]
fn citet_and_citep_are_the_two_basic_shapes() {
    assert_eq!(set(r"\citet{kp}"), "Knuth and Plass (1981)");
    assert_eq!(set(r"\citep{kp}"), "(Knuth and Plass, 1981)");
}

/// The classic natbib trap, and the one that motivated this test file: a
/// single optional argument is the **post**-note. `\citep[see][p.~7]` is also
/// a single lexer word (`[`, `]` and `~` are ordinary characters), so the
/// reader has to split it rather than consume the token.
#[test]
fn one_optional_argument_is_the_post_note() {
    assert_eq!(
        set(r"\citep[p.~7]{kp}"),
        "(Knuth and Plass, 1981, p. 7)"
    );
    assert_eq!(
        set(r"\citep[see][p.~7]{kp}"),
        "(see Knuth and Plass, 1981, p. 7)"
    );
    assert_eq!(set(r"\citep[see][]{kp}"), "(see Knuth and Plass, 1981)");
    assert_eq!(set(r"\citet[p.~7]{kp}"), "Knuth and Plass (1981, p. 7)");
    assert_eq!(
        set(r"\citet[see][p.~7]{kp}"),
        "Knuth and Plass (see 1981, p. 7)"
    );
    // Separated brackets are the same two arguments.
    assert_eq!(
        set(r"\citep[see] [p.~7]{kp}"),
        "(see Knuth and Plass, 1981, p. 7)"
    );
}

#[test]
fn the_unparenthesised_and_partial_forms() {
    assert_eq!(set(r"\citealt{kp}"), "Knuth and Plass 1981");
    assert_eq!(set(r"\citealp{kp}"), "Knuth and Plass, 1981");
    assert_eq!(set(r"\citealp[p.~7]{kp}"), "Knuth and Plass, 1981, p. 7");
    assert_eq!(set(r"\citeauthor{kp}"), "Knuth and Plass");
    assert_eq!(set(r"\citeyear{kp}"), "1981");
    assert_eq!(set(r"\citeyearpar{kp}"), "(1981)");
    assert_eq!(set(r"\citetext{cf.}"), "(cf.)");
    assert_eq!(set(r"\citenum{hobby}"), "4");
}

#[test]
fn starred_forms_take_the_long_author_list() {
    assert_eq!(set(r"\citet{jones}"), "Jones et al. (1990)");
    assert_eq!(set(r"\citep{jones}"), "(Jones et al., 1990)");
    assert_eq!(
        set(r"\citet*{jones}"),
        "Jones, Baker, and Williams (1990)"
    );
    assert_eq!(
        set(r"\citeauthor*{jones}"),
        "Jones, Baker, and Williams"
    );
    assert_eq!(
        set(r"\citefullauthor{jones}"),
        "Jones, Baker, and Williams"
    );
    // `\citet*[p.~7]{...}` is one lexer word too.
    assert_eq!(
        set(r"\citet*[p.~7]{jones}"),
        "Jones, Baker, and Williams (1990, p. 7)"
    );
}

#[test]
fn cite_is_citet_without_a_note_and_citep_with_one() {
    assert_eq!(set(r"\cite{kp}"), "Knuth and Plass (1981)");
    assert_eq!(set(r"\cite[p.~7]{kp}"), "(Knuth and Plass, 1981, p. 7)");
}

#[test]
fn several_keys_keep_their_order_and_use_the_separator() {
    assert_eq!(
        set(r"\citep{plass81,hobby,frank}"),
        "(Plass, 1981; Hobby, 1986; Frank, 1990)"
    );
    assert_eq!(
        set(r"\citep{plass81, hobby}"),
        "(Plass, 1981; Hobby, 1986)"
    );
    assert_eq!(set(r"\citet{plass81,hobby}"), "Plass (1981); Hobby (1986)");
    assert_eq!(
        set(r"\citep[see][p.~7]{plass81,hobby}"),
        "(see Plass, 1981; Hobby, 1986, p. 7)"
    );
    assert_eq!(set(r"\citeauthor{plass81,hobby}"), "Plass; Hobby");
    assert_eq!(set(r"\citeyear{plass81,hobby}"), "1981; 1986");
}

#[test]
fn a_repeated_author_collapses_to_the_bare_year() {
    assert_eq!(set(r"\citep{plass81,plass90}"), "(Plass, 1981, 1990)");
    assert_eq!(set(r"\citet{plass81,plass90}"), "Plass (1981, 1990)");
}

#[test]
fn an_undefined_key_is_a_question_mark_inside_the_delimiters() {
    assert_eq!(set(r"\citep{nosuch}"), "(?)");
    assert_eq!(
        set(r"\citep{plass81,nosuch,hobby}"),
        "(Plass, 1981; ?; Hobby, 1986)"
    );
}

#[test]
fn the_uppercasing_forms() {
    assert_eq!(set(r"\Citet{kp}"), "Knuth and Plass (1981)");
    assert_eq!(set(r"\Citep{hobby}"), "(Hobby, 1986)");
}

/// `numbers` re-executes `square,comma` (natbib.sty line 238), and
/// `\ProcessOptions` runs options in *declaration* order, so a later `round`
/// still wins the delimiters while `numbers` keeps the comma separator.
#[test]
fn the_numbers_option_and_its_implied_square_comma() {
    assert_eq!(set_with("numbers", r"\citep{plass81,hobby}"), "[2, 4]");
    assert_eq!(set_with("numbers", r"\citet{plass81}"), "Plass [2]");
    assert_eq!(
        set_with("numbers", r"\citep[p.~7]{plass81}"),
        "[2, p. 7]"
    );
    assert_eq!(set_with("numbers", r"\cite{plass81}"), "[2]");
    assert_eq!(set_with("numbers,round", r"\citep{plass81}"), "(2)");
    assert_eq!(set_with("round,numbers", r"\citep{plass81}"), "(2)");
    assert_eq!(set_with("square", r"\citep{plass81}"), "[Plass, 1981]");
}

/// natbib's `\@lbibitem` advances `\c@NAT@ctr` for **every** entry, labelled
/// or not, unlike the kernel's — and in author-year mode `\@biblabel` is
/// `\hfill`, so the printed list carries no marker at all.
#[test]
fn the_bibliography_list_has_no_marker_under_author_year() {
    let source = format!(
        "\\documentclass[11pt]{{article}}\\usepackage[authoryear,round]{{natbib}}\\begin{{document}}\n\\citep{{hobby}}\n\n{}\n\\end{{document}}",
        bibliography()
    );
    let parsed = parse(&source);
    let markers: Vec<String> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { label, .. } => label.as_ref().map(|(text, _)| text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(markers.len(), 6, "one marker per \\bibitem: {markers:?}");
    assert!(
        markers.iter().all(String::is_empty),
        "author-year entries print no label: {markers:?}"
    );
}

#[test]
fn the_bibliography_list_keeps_bracketed_numbers_under_numbers() {
    let source = format!(
        "\\documentclass[11pt]{{article}}\\usepackage[numbers]{{natbib}}\\begin{{document}}\n\\citep{{hobby}}\n\n{}\n\\end{{document}}",
        bibliography()
    );
    let parsed = parse(&source);
    let markers: Vec<String> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { label, .. } => label.as_ref().map(|(text, _)| text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(markers, ["[1]", "[2]", "[3]", "[4]", "[5]", "[6]"]);
}

/// Without natbib the kernel's `\cite` is unchanged, and a `\bibitem`'s
/// optional label is still used verbatim and consumes no number.
#[test]
fn the_kernel_cite_is_untouched_when_natbib_is_not_loaded() {
    let source = r"\documentclass[11pt]{article}\begin{document}
\cite{a} \cite[p.~7]{b}

\begin{thebibliography}{9}\bibitem{a}A.\bibitem[Knuth 1984]{b}B.\end{thebibliography}
\end{document}";
    let parsed = parse(source);
    let mut out = String::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { text, .. } = inline {
                    out.push_str(text);
                }
            }
            break;
        }
    }
    assert_eq!(out, "[1][Knuth 1984, p. 7]");
}

/// A `\bibitem` with no author-year data sets natbib's `\NAT@stdbst`, and
/// `\NAT@force@numbers` then makes the whole document numeric — the steady
/// state a two-pass pdfLaTeX run reaches, with natbib's own error text.
#[test]
fn a_non_compliant_bibitem_forces_numeric_citations() {
    let source = r"\documentclass[11pt]{article}\usepackage[authoryear,round]{natbib}\begin{document}
\citep{a}

\begin{thebibliography}{9}\bibitem[Knuth(1984)]{a}A.\bibitem{b}B.\end{thebibliography}
\end{document}";
    let parsed = parse(source);
    let mut out = String::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { text, .. } = inline {
                    out.push_str(text);
                }
            }
            break;
        }
    }
    assert_eq!(out, "(1)");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Bibliography not compatible with author-year")),
        "{:?}",
        parsed.diagnostics
    );
}

/// The options this implementation reproduces are silent; the ones it parses
/// but does not apply keep the package warning and say which they are.
#[test]
fn unimplemented_options_keep_the_package_warning() {
    let quiet = parse(
        r"\documentclass[11pt]{article}\usepackage[authoryear,round]{natbib}\begin{document}x\end{document}",
    );
    assert!(
        !quiet
            .diagnostics
            .iter()
            .any(|d| d.message.contains("natbib") && d.message.contains("not implemented")),
        "{:?}",
        quiet.diagnostics
    );
    let noisy = parse(
        r"\documentclass[11pt]{article}\usepackage[sort&compress]{natbib}\begin{document}x\end{document}",
    );
    assert!(
        noisy
            .diagnostics
            .iter()
            .any(|d| d.message.contains("natbib") && d.message.contains("not implemented")),
        "{:?}",
        noisy.diagnostics
    );
}
