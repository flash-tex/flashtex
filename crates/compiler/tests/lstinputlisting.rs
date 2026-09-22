//! `\lstinputlisting{file}` (listings.sty): the named project file's bytes
//! render exactly as an inline `lstlisting` body holding the same bytes.
use flashtex_compiler::parser::{parse_project, Block, Parsed, SourceDocument};

/// Fixture bytes hostile to naive reuse: a `%` comment char, a backslash
/// command, math specials, a tab, trailing spaces, a blank line, and the
/// POSIX trailing newline.
const CODE: &str = "def f(x):  # 100% \\sure ${x}\n\treturn x  \n\nprint(f(1))\n";

fn main_doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\\end{{document}}\n")
}

fn verbatim_lines(blocks: &[Block]) -> Vec<String> {
    let mut out = Vec::new();
    for block in blocks {
        if let Block::Verbatim { lines, .. } = block {
            out.extend(lines.iter().map(|line| line.text.clone()));
        }
    }
    out
}

fn messages(parsed: &Parsed) -> Vec<String> {
    parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn listed_file_renders_like_the_same_bytes_inline() {
    let via_file = main_doc("\\lstinputlisting{code.py}\n");
    let file_docs = [
        SourceDocument {
            path: "main.tex",
            text: &via_file,
        },
        SourceDocument {
            path: "code.py",
            text: CODE,
        },
    ];
    let from_file = parse_project(&file_docs, "main.tex");
    assert!(
        from_file.diagnostics.is_empty(),
        "{:?}",
        from_file.diagnostics
    );

    let via_inline = main_doc(&format!(
        "\\begin{{lstlisting}}\n{CODE}\\end{{lstlisting}}\n"
    ));
    let inline_docs = [SourceDocument {
        path: "main.tex",
        text: &via_inline,
    }];
    let from_inline = parse_project(&inline_docs, "main.tex");
    assert!(
        from_inline.diagnostics.is_empty(),
        "{:?}",
        from_inline.diagnostics
    );

    // One verbatim block on each side, with byte-identical display lines:
    // the tab above renders as one space, exactly as inline does.
    assert_eq!(from_file.blocks.len(), 1, "{:?}", from_file.blocks);
    assert_eq!(from_inline.blocks.len(), 1, "{:?}", from_inline.blocks);
    let file_lines = verbatim_lines(&from_file.blocks);
    let inline_lines = verbatim_lines(&from_inline.blocks);
    assert_eq!(
        file_lines,
        vec![
            "def f(x):  # 100% \\sure ${x}",
            " return x  ",
            "",
            "print(f(1))",
        ]
    );
    assert_eq!(file_lines, inline_lines);
}

#[test]
fn listed_file_options_warn_exactly_like_inline_options() {
    let via_file = main_doc("\\lstinputlisting[language=Python]{code.py}\n");
    let file_docs = [
        SourceDocument {
            path: "main.tex",
            text: &via_file,
        },
        SourceDocument {
            path: "code.py",
            text: CODE,
        },
    ];
    let from_file = parse_project(&file_docs, "main.tex");

    let via_inline = main_doc(&format!(
        "\\begin{{lstlisting}}[language=Python]\n{CODE}\\end{{lstlisting}}\n"
    ));
    let inline_docs = [SourceDocument {
        path: "main.tex",
        text: &via_inline,
    }];
    let from_inline = parse_project(&inline_docs, "main.tex");

    assert_eq!(messages(&from_file), messages(&from_inline));
    assert_eq!(
        messages(&from_file),
        vec!["lstlisting options are not implemented; typeset as plain verbatim"]
    );
    assert_eq!(
        verbatim_lines(&from_file.blocks),
        verbatim_lines(&from_inline.blocks)
    );
}

#[test]
fn listed_file_tex_fallback_resolves_like_input() {
    let main = main_doc("\\lstinputlisting{notes}\n");
    let docs = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "notes.tex",
            text: "remember this\n",
        },
    ];
    let parsed = parse_project(&docs, "main.tex");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(verbatim_lines(&parsed.blocks), vec!["remember this"]);
}

#[test]
fn missing_listed_file_is_a_named_error_not_an_unknown_command() {
    let main = main_doc("\\lstinputlisting{nosuch.py}\n");
    let docs = [SourceDocument {
        path: "main.tex",
        text: &main,
    }];
    let parsed = parse_project(&docs, "main.tex");
    let found: Vec<String> = messages(&parsed);
    assert!(
        found
            .iter()
            .any(|m| m.contains("listed file not found") && m.contains("nosuch.py")),
        "{found:?}"
    );
    assert!(
        found.iter().all(|m| !m.contains("not supported")),
        "{found:?}"
    );
    assert_eq!(verbatim_lines(&parsed.blocks), Vec::<String>::new());
}

#[test]
fn unsafe_and_empty_listing_paths_are_rejected() {
    for (argument, needle) in [
        ("../evil.py", "rejected listing path"),
        (
            "",
            "\\lstinputlisting requires a non-empty project-relative path",
        ),
    ] {
        let main = main_doc(&format!("\\lstinputlisting{{{argument}}}\n"));
        let docs = [SourceDocument {
            path: "main.tex",
            text: &main,
        }];
        let parsed = parse_project(&docs, "main.tex");
        let found = messages(&parsed);
        assert!(
            found.iter().any(|m| m.contains(needle)),
            "{argument:?}: {found:?}"
        );
        assert_eq!(verbatim_lines(&parsed.blocks), Vec::<String>::new());
    }
}
