//! `\InputIfFileExists{file}{true}{false}` (ltfiles.dtx): when the project
//! carries `file` (as `file` or `file.tex`) the true branch runs and then
//! the file is input; otherwise only the false branch runs -- never an
//! error, so packages can guard optional inputs with it.
//!
//! Oracle: `pdflatex -interaction=nonstopmode main.tex` (TeX Live 2026)
//! over `main.tex` (`\InputIfFileExists{plain}{Y}{N}
//! \InputIfFileExists{nothere}{Y}{N}.`) plus `plain.tex` (`Plain.`)
//! prints `YPlain. N.` with no error.

use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument};

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
fn existing_file_runs_true_branch_then_inputs_it() {
    // The task's repro: `plain.tex` is in the project, `nothere` is not.
    let body = "\\InputIfFileExists{plain}{Y}{N} \\InputIfFileExists{nothere}{Y}{N}.";
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main_doc(body),
        },
        SourceDocument {
            path: "plain.tex",
            text: "Plain.",
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    assert_eq!(
        all_text(&parsed),
        "YPlain. N.",
        "messages: {:?}",
        messages(&parsed)
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "no diagnostics, got: {:?}",
        messages(&parsed)
    );
}

#[test]
fn missing_file_runs_only_false_branch() {
    let documents = [SourceDocument {
        path: "main.tex",
        text: &main_doc("\\InputIfFileExists{nothere}{Y}{N}"),
    }];
    let parsed = parse_project(&documents, "main.tex");
    assert_eq!(all_text(&parsed), "N", "messages: {:?}", messages(&parsed));
    assert!(
        parsed.diagnostics.is_empty(),
        "no diagnostics, got: {:?}",
        messages(&parsed)
    );
}

#[test]
fn explicit_tex_extension_inputs_the_file() {
    // pdflatex `(./plain.tex)` for `{plain.tex}` too: exact names resolve.
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main_doc("\\InputIfFileExists{plain.tex}{Y}{N}"),
        },
        SourceDocument {
            path: "plain.tex",
            text: "Plain.",
        },
    ];
    let parsed = parse_project(&documents, "main.tex");
    assert_eq!(
        all_text(&parsed),
        "YPlain.",
        "messages: {:?}",
        messages(&parsed)
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "no diagnostics, got: {:?}",
        messages(&parsed)
    );
}
