//! `\lstinputlisting[caption=..]{file}` (listings): the named project file is
//! read from disk and set as a literal listing, counted for
//! `\lstlistoflistings` purposes exactly like an inline `lstlisting` block.
//!
//! Fixtures are staged the way the `\input`/`\include` tests stage theirs:
//! extra project documents in the `parse_project` array (see
//! `tests/includeonly.rs`), so the `{file}` argument resolves through the
//! same `document_by_path` lookup.
use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::parser::{parse_project, Block, SourceDocument};

fn main_with(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\\end{{document}}\n")
}

fn verbatim_texts(blocks: &[Block]) -> Vec<Vec<String>> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Verbatim { lines, .. } => {
                Some(lines.iter().map(|line| line.text.clone()).collect())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn file_content_appears_as_a_literal_listing() {
    // `%`, backslashes and braces must survive literally: the file is never
    // parsed, so a `%` starts no comment and `\end{verbatim}` ends nothing.
    let code = "x = 1 % not a comment\n\\end{verbatim}\nprint({x})\n";
    let main = main_with("Before.\n\\lstinputlisting[caption=Example]{code.py}\nAfter.\n");
    let docs = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "code.py",
            text: code,
        },
    ];
    let parsed = parse_project(&docs, "main.tex");
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(|d| !matches!(d.severity, Severity::Error)),
        "unexpected error: {:?}",
        parsed.diagnostics
    );
    let listings = verbatim_texts(&parsed.blocks);
    assert_eq!(listings.len(), 1, "{:?}", parsed.blocks);
    assert_eq!(
        listings[0],
        vec![
            "x = 1 % not a comment".to_string(),
            "\\end{verbatim}".to_string(),
            "print({x})".to_string(),
        ]
    );
}

#[test]
fn caption_option_is_accepted_the_same_way_as_lstlisting() {
    let main = main_with(
        "\\begin{lstlisting}[caption=Example]\nx = 1\n\\end{lstlisting}\n\\lstinputlisting[caption=Example]{code.py}\n",
    );
    let docs = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "code.py",
            text: "y = 2\n",
        },
    ];
    let parsed = parse_project(&docs, "main.tex");
    let mut option_notes: Vec<&str> = parsed
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("options are not implemented; typeset as plain verbatim"))
        .map(|d| d.message.as_str())
        .collect();
    option_notes.sort_unstable();
    assert_eq!(
        option_notes,
        [
            "\\lstinputlisting options are not implemented; typeset as plain verbatim",
            "lstlisting options are not implemented; typeset as plain verbatim",
        ],
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn listing_count_increments_for_listoflistings_purposes() {
    // An inline `lstlisting` registers by pushing one `Block::Verbatim`; a
    // listing from a file must push the same single block, so the two are
    // indistinguishable to whatever counts listings.
    let main = main_with(
        "\\begin{lstlisting}\nx = 1\n\\end{lstlisting}\n\\lstinputlisting{snippet}\n",
    );
    let docs = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        // No extension in the argument: `\input` appends `.tex`, and
        // `\lstinputlisting` must resolve the same way.
        SourceDocument {
            path: "snippet.tex",
            text: "y = 2\n",
        },
    ];
    let parsed = parse_project(&docs, "main.tex");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let listings = verbatim_texts(&parsed.blocks);
    assert_eq!(listings.len(), 2, "{:?}", parsed.blocks);
    assert_eq!(listings[0], vec!["x = 1".to_string()]);
    assert_eq!(listings[1], vec!["y = 2".to_string()]);
}

#[test]
fn missing_file_reports_a_clear_diagnostic() {
    let main = main_with("Before.\n\\lstinputlisting{nosuchfile}\nAfter.\n");
    let docs = [SourceDocument {
        path: "main.tex",
        text: &main,
    }];
    let parsed = parse_project(&docs, "main.tex");
    let missing: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("listed file not found"))
        .collect();
    assert_eq!(missing.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        missing[0]
            .message
            .contains("looked for 'nosuchfile' and 'nosuchfile.tex'"),
        "{}",
        missing[0].message
    );
    // No panic, and no silent empty listing either: nothing is pushed.
    assert!(verbatim_texts(&parsed.blocks).is_empty(), "{:?}", parsed.blocks);
}
