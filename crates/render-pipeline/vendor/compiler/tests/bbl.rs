//! Pre-built `.bbl` consumption: `\bibliography{name}` inputs `name.bbl`
//! when the project carries it, and `\printbibliography` (biblatex) inputs
//! the job's `.bbl` the same way.
//!
//! A BibTeX `.bbl` is a `thebibliography` environment the compiler already
//! renders, so inputting it resolves `\cite` keys through the existing
//! `bib::prescan` with no new citation logic.

use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument};

const BBL: &str = r"
\begin{thebibliography}{99}
\bibitem{knuth84} D.~E. Knuth. The {\TeX}book. Addison-Wesley, 1984.
\bibitem{lamport94} L.~Lamport. {\LaTeX}: a document preparation system. Addison-Wesley, 1994.
\end{thebibliography}
";

fn main_doc(body: &str) -> String {
    format!("\\documentclass{{article}}\\begin{{document}}{body}\\end{{document}}")
}

fn inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text {
                text: value,
                space_before,
                ..
            } => {
                if *space_before && !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(value);
            }
            // A kernel citation label is an `\hbox`.
            Inline::HBox(boxed) => text.push_str(&inline_text(&boxed.content)),
            _ => {}
        }
    }
    text
}

fn block_text(block: &Block) -> String {
    match block {
        Block::Paragraph(inlines)
        | Block::Heading {
            content: inlines, ..
        } => inline_text(inlines),
        Block::ListItem { label, content, .. } => {
            let mut text = label
                .as_ref()
                .map(|label| label.0.clone())
                .unwrap_or_default();
            text.push_str(&inline_text(content));
            text
        }
        _ => String::new(),
    }
}

fn all_text(parsed: &flashtex_compiler::parser::Parsed) -> String {
    parsed
        .blocks
        .iter()
        .map(block_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn messages(parsed: &flashtex_compiler::parser::Parsed) -> Vec<&str> {
    parsed
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect()
}

#[test]
fn bibliography_inputs_a_sibling_bbl() {
    let main = main_doc("See \\cite{knuth84}. \\bibliography{refs} \\bibliographystyle{plain}");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "refs.bbl",
            text: BBL,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    let text = all_text(&parsed);
    assert!(
        text.contains("References"),
        "the .bbl's thebibliography heading renders, got: {text}"
    );
    assert!(
        text.contains("Knuth"),
        "the .bbl's entries render, got: {text}"
    );
    assert!(
        text.contains("[1]"),
        "citations resolve against the .bbl's bibitems, got: {text}"
    );
    assert!(
        !messages(&parsed)
            .iter()
            .any(|m| m.contains("\\bibliography requires BibTeX")),
        "no missing-bibliography diagnostic when the .bbl was consumed, got: {:?}",
        messages(&parsed)
    );
}

#[test]
fn a_consumed_bbl_matches_the_handwritten_thebibliography() {
    // The task's premise: a hand-written `thebibliography` matches pdflatex
    // exactly. A consumed `.bbl` must therefore produce the same blocks and
    // diagnostics as that text written inline — byte-identical entry text on
    // both sides, including a logo command.
    let entry = "\\bibitem{knuth84} D.~E. Knuth. The {\\TeX}book. Addison-Wesley, 1984.";
    let inline = main_doc(&format!(
        "See \\cite{{knuth84}}. \\begin{{thebibliography}}{{9}}\n{entry}\n\\end{{thebibliography}}"
    ));
    let via_bbl = main_doc("See \\cite{knuth84}. \\bibliography{refs}");
    let single = format!("\\begin{{thebibliography}}{{9}}\n{entry}\n\\end{{thebibliography}}\n");
    let a = parse_project(
        &[SourceDocument {
            path: "main.tex",
            text: &inline,
        }],
        "main.tex",
    );
    let b = parse_project(
        &[
            SourceDocument {
                path: "main.tex",
                text: &via_bbl,
            },
            SourceDocument {
                path: "refs.bbl",
                text: &single,
            },
        ],
        "main.tex",
    );
    assert_eq!(all_text(&a), all_text(&b), "identical rendered text");
    assert_eq!(
        messages(&a),
        messages(&b),
        "identical diagnostics (both empty)"
    );
}

#[test]
fn newblock_renders_as_a_space_with_no_diagnostic() {
    let bbl = "\\begin{thebibliography}{9}\n\\bibitem{a}Author.\\newblock Title.\\newblock Journal.\n\\end{thebibliography}\n";
    let main = main_doc("See \\cite{a}. \\bibliography{refs}");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "refs.bbl",
            text: bbl,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    assert!(
        messages(&parsed).is_empty(),
        "no diagnostics at all, got: {:?}",
        messages(&parsed)
    );
    assert!(
        all_text(&parsed).contains("Author. Title. Journal."),
        "blocks are space-separated, got: {}",
        all_text(&parsed)
    );
}

#[test]
fn an_mcite_bibliography_renders_like_thebibliography() {
    // mciteplus wraps plain `\bibitem`s in `mcitethebibliography`; the
    // entries resolve and render identically (its sublist grouping is out
    // of scope, and its package macros keep their honest diagnostics).
    let bbl = "\\begin{mcitethebibliography}{10}\n\\bibitem{k}K. Title.\n\\end{mcitethebibliography}\n";
    let main = main_doc("See \\cite{k}. \\bibliography{refs}");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "refs.bbl",
            text: bbl,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    let text = all_text(&parsed);
    assert!(
        text.contains("References") && text.contains("[1]") && text.contains("K. Title."),
        "entries render and the cite resolves, got: {text}"
    );
    assert!(
        !messages(&parsed)
            .iter()
            .any(|m| m.contains("\\bibitem is only supported")),
        "no dropped-entry errors, got: {:?}",
        messages(&parsed)
    );
}

#[test]
fn a_bib_database_is_never_typeset_as_tex() {
    // A real document writes `\bibliography{main.bib}` next to `main.bib`:
    // the name must resolve to `main.bbl` (or the job file), never to the
    // database itself, whose `@article` records are not typesettable.
    let bib = "@article{knuth84,\n  author = {Knuth, Donald E.},\n  title = {The TeXbook},\n  year = {1984},\n}\n";
    let main = main_doc("See \\cite{knuth84}. \\bibliography{main.bib}");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "main.bib",
            text: bib,
        },
        SourceDocument {
            path: "main.bbl",
            text: BBL,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    let text = all_text(&parsed);
    assert!(
        text.contains("Knuth") && !text.contains("@article"),
        "the .bbl renders and the .bib stays out of the body, got: {text}"
    );
    assert!(
        messages(&parsed).is_empty(),
        "no diagnostics at all, got: {:?}",
        messages(&parsed)
    );
}

#[test]
fn missing_bbl_and_bib_keeps_the_existing_diagnostic() {
    let main = main_doc("See \\cite{knuth84}. \\bibliography{refs}");
    let documents = [SourceDocument {
        path: "main.tex",
        text: &main,
    }];
    let parsed = parse_project(&documents, "main.tex");
    assert_eq!(
        messages(&parsed),
        vec![
            "citation 'knuth84' is undefined",
            "\\bibliography requires BibTeX/biblatex .bib input, which this compiler does not read",
        ],
        "the missing-bibliography diagnostic is unchanged"
    );
}

#[test]
fn cite_keys_resolve_against_the_bbl_bibitems() {
    let main = main_doc("\\cite{lamport94,knuth84} and \\cite{nosuch}. \\bibliography{refs}");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "refs.bbl",
            text: BBL,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    let text = all_text(&parsed);
    assert!(
        text.contains("[2, 1]"),
        "known keys resolve in order, got: {text}"
    );
    assert!(
        text.contains("[?]"),
        "unknown keys still render '?', got: {text}"
    );
    assert_eq!(
        messages(&parsed),
        vec!["citation 'nosuch' is undefined"],
        "only the unknown key warns"
    );
}

#[test]
fn thebibliography_label_width_tracks_the_widest_label_argument() {
    let bbl9 = BBL.replace("{99}", "{9}");
    let main = main_doc("\\cite{knuth84}. \\bibliography{refs}");
    let wide = parse_project(
        &[
            SourceDocument {
                path: "main.tex",
                text: &main,
            },
            SourceDocument {
                path: "refs.bbl",
                text: BBL,
            },
        ],
        "main.tex",
    );
    let narrow = parse_project(
        &[
            SourceDocument {
                path: "main.tex",
                text: &main,
            },
            SourceDocument {
                path: "refs.bbl",
                text: &bbl9,
            },
        ],
        "main.tex",
    );
    let widest = |parsed: &flashtex_compiler::parser::Parsed| {
        parsed
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::ListItem { widest_label, .. } => widest_label.clone(),
                _ => None,
            })
            .next()
    };
    assert_eq!(widest(&wide).as_deref(), Some("99"));
    assert_eq!(widest(&narrow).as_deref(), Some("9"));
}

#[test]
fn an_empty_bbl_consumes_the_command_but_resolves_nothing() {
    let main = main_doc("See \\cite{knuth84}. \\bibliography{refs}");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "refs.bbl",
            text: "",
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    assert_eq!(
        messages(&parsed),
        vec!["citation 'knuth84' is undefined"],
        "an empty .bbl is still 'found': no missing-bibliography diagnostic, cites undefined"
    );
    assert!(
        !all_text(&parsed).contains("References"),
        "an empty .bbl renders no bibliography"
    );
}

#[test]
fn bibliographystyle_stays_a_noop() {
    let main = main_doc("Body \\bibliographystyle{plain}");
    let documents = [SourceDocument {
        path: "main.tex",
        text: &main,
    }];
    let parsed = parse_project(&documents, "main.tex");
    assert_eq!(
        messages(&parsed),
        vec!["\\bibliographystyle has no effect without BibTeX/biblatex .bib support"],
    );
}

#[test]
fn bibliography_falls_back_to_the_job_bbl() {
    // Real LaTeX inputs `\jobname.bbl` whatever the argument says; arXiv's
    // pre-built bibliographies are named that way while `\bibliography`
    // names the `.bib` sources.
    let main = main_doc("See \\cite{knuth84}. \\bibliography{unrelated}");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "main.bbl",
            text: BBL,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    let text = all_text(&parsed);
    assert!(
        text.contains("Knuth") && text.contains("[1]"),
        "the job .bbl renders and resolves cites, got: {text}"
    );
    assert!(
        messages(&parsed).is_empty(),
        "no diagnostics at all, got: {:?}",
        messages(&parsed)
    );
}

#[test]
fn bibliography_inputs_every_named_bbl() {
    let bbl_b = r"
\begin{thebibliography}{9}
\bibitem{goethe} J.~W. von Goethe. Faust. 1808.
\end{thebibliography}
";
    let main = main_doc("See \\cite{knuth84} and \\cite{goethe}. \\bibliography{a, b}");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "a.bbl",
            text: BBL,
        },
        SourceDocument {
            path: "b.bbl",
            text: bbl_b,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    let text = all_text(&parsed);
    assert!(
        text.contains("Knuth") && text.contains("Goethe"),
        "both .bbl files render, got: {text}"
    );
    assert!(
        messages(&parsed).is_empty(),
        "every key resolves, got: {:?}",
        messages(&parsed)
    );
}

#[test]
fn printbibliography_inputs_the_job_bbl_when_biblatex_is_loaded() {
    let main = "\\documentclass{article}\\usepackage{biblatex}\\begin{document}See \\cite{knuth84}. \\printbibliography\\end{document}";
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: main,
        },
        SourceDocument {
            path: "main.bbl",
            text: BBL,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    let text = all_text(&parsed);
    assert!(
        text.contains("Knuth"),
        "the job .bbl's entries render under \\printbibliography, got: {text}"
    );
    assert_eq!(
        text.matches("References").count(),
        1,
        "no duplicate bibliography heading, got: {text}"
    );
}

#[test]
fn printbibliography_with_a_title_option_consumes_the_bracket() {
    let main = "\\documentclass{article}\\usepackage{biblatex}\\begin{document}Text. \\printbibliography[title={Sources}]\\end{document}";
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: main,
        },
        SourceDocument {
            path: "main.bbl",
            text: BBL,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    let text = all_text(&parsed);
    assert!(
        text.contains("Knuth"),
        "the job .bbl renders, got: {text}"
    );
    assert!(
        !text.contains("[title") && !text.contains("Sources"),
        "the swallowed command's [title=...] leaves no text behind, got: {text}"
    );
}

#[test]
fn printbibliography_without_biblatex_keeps_the_existing_error() {
    let main = main_doc("Text. \\printbibliography");
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "main.bbl",
            text: BBL,
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    assert_eq!(
        messages(&parsed),
        vec!["\\printbibliography requires \\usepackage{biblatex}"],
        "a stray .bbl does not silence the missing-package error"
    );
}
