//! `\verbatiminput{file}` (verbatim.sty) and `\lstinputlisting[options]{file}`
//! (listings): a project file's raw bytes typeset exactly like a `verbatim`
//! / `lstlisting` environment body holding those bytes, each gated on its
//! package being loaded via the `self.packages` check (like `\uline` needs
//! ulem). Without the package the command diagnoses
//! `\command needs \usepackage{...}` and typesets nothing.
use flashtex_compiler::parser::{parse_project, Block, Parsed, SourceDocument};
use flashtex_compiler::DocumentId;

const CODE: &str = "100% \\foo ${x}\nline two\n";

fn parse<'a>(documents: &[SourceDocument<'a>]) -> Parsed {
    parse_project(documents, "main.tex")
}

fn verbatim_texts(parsed: &Parsed) -> Vec<String> {
    let mut out = Vec::new();
    for block in &parsed.blocks {
        if let Block::Verbatim { lines, .. } = block {
            out.extend(lines.iter().map(|line| line.text.clone()));
        }
    }
    out
}

fn has_verbatim_block(parsed: &Parsed) -> bool {
    parsed.blocks.iter().any(|block| matches!(block, Block::Verbatim { .. }))
}

fn messages(parsed: &Parsed) -> Vec<String> {
    parsed.diagnostics.iter().map(|d| d.message.clone()).collect()
}

/// `\verbatiminput` with the package renders the file's bytes exactly like
/// the equivalent `verbatim` environment body (specials kept literally, the
/// file's trailing newline contributing no extra line).
#[test]
fn verbatiminput_renders_the_file_like_a_verbatim_body() {
    let main = "\\documentclass{article}\n\\usepackage{verbatim}\n\\begin{document}\n\\verbatiminput{code.txt}\n\\end{document}\n";
    let parsed = parse(&[
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "code.txt", text: CODE },
    ]);
    assert_eq!(verbatim_texts(&parsed), ["100% \\foo ${x}", "line two"]);
    assert!(
        !messages(&parsed).iter().any(|m| m.contains("needs \\usepackage")),
        "no package gate should fire when verbatim is loaded: {:?}",
        messages(&parsed)
    );
    // The lines point at the file's own bytes, one span per file line.
    let lines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Verbatim { lines, .. } => Some(lines),
            _ => None,
        })
        .expect("one verbatim block");
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].span.document, &CODE[lines[0].span.start..lines[0].span.end]), (DocumentId(1), "100% \\foo ${x}"));
    assert_eq!((lines[1].span.document, &CODE[lines[1].span.start..lines[1].span.end]), (DocumentId(1), "line two"));
}

/// The equivalent inline environment renders byte-identical lines.
#[test]
fn verbatiminput_matches_the_inline_verbatim_environment() {
    let from_file = parse(&[
        SourceDocument {
            path: "main.tex",
            text: "\\documentclass{article}\n\\usepackage{verbatim}\n\\begin{document}\n\\verbatiminput{code.txt}\n\\end{document}\n",
        },
        SourceDocument { path: "code.txt", text: CODE },
    ]);
    let from_env = parse(&[SourceDocument {
        path: "main.tex",
        text: "\\documentclass{article}\n\\begin{document}\n\\begin{verbatim}\n100% \\foo ${x}\nline two\n\\end{verbatim}\n\\end{document}\n",
    }]);
    assert_eq!(verbatim_texts(&from_file), verbatim_texts(&from_env));
}

/// Without `\usepackage{verbatim}` the command diagnoses the missing
/// package and typesets nothing (no silent render).
#[test]
fn verbatiminput_without_the_package_diagnoses_and_renders_nothing() {
    let main = "\\documentclass{article}\n\\begin{document}\n\\verbatiminput{code.txt}\n\\end{document}\n";
    let parsed = parse(&[
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "code.txt", text: CODE },
    ]);
    assert!(
        messages(&parsed).iter().any(|m| m == "\\verbatiminput needs \\usepackage{verbatim}"),
        "expected the package gate diagnostic, got: {:?}",
        messages(&parsed)
    );
    assert!(!has_verbatim_block(&parsed), "nothing may be typeset without the package: {:?}", parsed.blocks);
}

/// `\lstinputlisting` with the package renders the file like an inline
/// `lstlisting` body; its options are read exactly like the environment's
/// and likewise ignored with a warning.
#[test]
fn lstinputlisting_renders_the_file_like_an_lstlisting_body() {
    let main = "\\documentclass{article}\n\\usepackage{listings}\n\\begin{document}\n\\lstinputlisting[basicstyle=\\ttfamily]{code.txt}\n\\end{document}\n";
    let parsed = parse(&[
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "code.txt", text: CODE },
    ]);
    assert_eq!(verbatim_texts(&parsed), ["100% \\foo ${x}", "line two"]);
    assert!(
        messages(&parsed).iter().any(|m| m.contains("lstlisting options")),
        "non-empty options are diagnosed like the environment's: {:?}",
        messages(&parsed)
    );
    assert!(
        !messages(&parsed).iter().any(|m| m.contains("needs \\usepackage")),
        "no package gate should fire when listings is loaded: {:?}",
        messages(&parsed)
    );
    let from_env = parse(&[SourceDocument {
        path: "main.tex",
        text: "\\documentclass{article}\n\\begin{document}\n\\begin{lstlisting}\n100% \\foo ${x}\nline two\n\\end{lstlisting}\n\\end{document}\n",
    }]);
    assert_eq!(verbatim_texts(&parsed), verbatim_texts(&from_env));
}

/// Without `\usepackage{listings}` the command diagnoses the missing
/// package and typesets nothing (no silent render).
#[test]
fn lstinputlisting_without_the_package_diagnoses_and_renders_nothing() {
    let main = "\\documentclass{article}\n\\begin{document}\n\\lstinputlisting{code.txt}\n\\end{document}\n";
    let parsed = parse(&[
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "code.txt", text: CODE },
    ]);
    assert!(
        messages(&parsed).iter().any(|m| m == "\\lstinputlisting needs \\usepackage{listings}"),
        "expected the package gate diagnostic, got: {:?}",
        messages(&parsed)
    );
    assert!(!has_verbatim_block(&parsed), "nothing may be typeset without the package: {:?}", parsed.blocks);
}

/// `\verbatiminput*` marks spaces, like the `verbatim*` environment.
#[test]
fn verbatiminput_star_marks_spaces() {
    let main = "\\documentclass{article}\n\\usepackage{verbatim}\n\\begin{document}\n\\verbatiminput*{code.txt}\n\\end{document}\n";
    let parsed = parse(&[
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "code.txt", text: "a b\n" },
    ]);
    assert_eq!(verbatim_texts(&parsed), ["a\u{b7}b"]);
}
