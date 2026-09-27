//! `./` and `/./` inside `\input`/`\include` targets.
//!
//! A leading `./` and an inner `/./` are TeX-neutral spelling noise:
//! `./sub/part`, `sub/./part` and `sub/part` must all resolve to the same
//! document. Project discovery already normalizes these (see
//! `project-files`' `dot_segments_resolve_to_the_same_document`); the
//! compiler's include lookup normalizes the same way at lookup time, so a
//! project discovered from disk (whose documents carry normalized paths)
//! still matches the raw spelling in the source.
//!
//! Ground truth, pdflatex (TeX Live 2026) on `main.tex` holding
//! `\input{./sub/part}` and `See \ref{sec:p}.` with `sub/part.tex`
//! holding `\section{Part}\label{sec:p}` + `Hello from part.`: the file is
//! included, the heading prints, and the reference resolves (`See 1.`).
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument};

const PART: &str = "\\section{Part}\\label{sec:p}\nHello from part.\n";

fn docs<'a>(main: &'a str, part: &'a str) -> [SourceDocument<'a>; 2] {
    [
        SourceDocument {
            path: "main.tex",
            text: main,
        },
        SourceDocument {
            path: "sub/part.tex",
            text: part,
        },
    ]
}

fn paragraph_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        let Block::Paragraph(inlines) = block else {
            continue;
        };
        for inline in inlines {
            let Inline::Text { text, .. } = inline else {
                continue;
            };
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(text);
        }
    }
    out
}

#[test]
fn dot_segment_spellings_include_with_zero_errors() {
    // No `\label` here: including the same file three times would define it
    // three times (pdflatex warns `multiply defined` too).
    let main = "\\documentclass{article}\n\\begin{document}\n\\input{./sub/part}\n\\input{sub/./part}\n\\input{sub/part}\n\\end{document}\n";
    let docs = docs(main, "Hello from part.\n");
    let parsed = parse_project(&docs, "main.tex");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let text = paragraph_text(&parsed.blocks);
    assert_eq!(text.matches("Hello from part.").count(), 3, "{text:?}");
}

#[test]
fn dotted_input_resolves_labels_like_pdflatex() {
    // pdflatex prints the `Part` heading and `See 1.`: like the
    // established `\ref` tests, read the laid-out page items.
    let main = "\\documentclass{article}\n\\begin{document}\n\\input{./sub/part}\nSee \\ref{sec:p}.\n\\end{document}\n";
    let docs = docs(main, PART);
    let out = compile_full_project(&docs, "main.tex", LayoutConstraints::default());
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let items: Vec<&str> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter().map(|i| i.text.as_str()))
        .collect();
    assert!(items.contains(&"Hello"), "{items:?}");
    assert!(items.contains(&"See"), "{items:?}");
    assert!(items.contains(&"1"), "{items:?}");
    assert!(!items.contains(&"??"), "{items:?}");
}

#[test]
fn dotted_include_resolves_to_the_same_document() {
    let main =
        "\\documentclass{article}\n\\begin{document}\n\\include{./sub/part}\n\\end{document}\n";
    let docs = docs(main, PART);
    let parsed = parse_project(&docs, "main.tex");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Hello from part."), "{text:?}");
}
