//! `\includeonly{file1,file2,...}`: the kernel's preamble companion to
//! `\include`. A recorded, non-empty list selects which `\include`d files
//! are read; every other `\include` is a no-op (this compiler's `\include`
//! has no page-break side effect of its own, so a skipped file leaves no
//! break behind). With no `\includeonly`, every `\include` reads as before.
use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument};

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

fn page_breaks(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .filter(|b| matches!(b, Block::PageBreak))
        .count()
}

fn main_with(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\n{body}\\end{{document}}\n")
}

fn project<'a>(main: &'a str) -> [SourceDocument<'a>; 3] {
    [
        SourceDocument {
            path: "main.tex",
            text: main,
        },
        SourceDocument {
            path: "a.tex",
            text: "Alpha content.\n",
        },
        SourceDocument {
            path: "b.tex",
            text: "Bravo content.\n",
        },
    ]
}

#[test]
fn includeonly_skips_unlisted_files_without_a_trace() {
    let main = main_with("\\includeonly{a}\n", "Before.\n\\include{a}\n\\include{b}\nAfter.\n");
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(
        text.contains("Alpha content."),
        "listed file must be typeset: {text:?}"
    );
    assert!(
        !text.contains("Bravo"),
        "unlisted file must not be typeset: {text:?}"
    );
    assert!(text.contains("Before.") && text.contains("After."));
    // A skipped `\include` is a pure no-op: `\include` itself emits no page
    // break in this compiler, so none appears where `b` was.
    assert_eq!(page_breaks(&parsed.blocks), 0, "{:?}", parsed.blocks);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn no_includeonly_includes_everything() {
    let main = main_with("", "Before.\n\\include{a}\n\\include{b}\nAfter.\n");
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Alpha content."), "{text:?}");
    assert!(text.contains("Bravo content."), "{text:?}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn includeonly_normalizes_tex_extension_and_whitespace() {
    // Entries arrive trimmed and match with or without `.tex`, exactly the
    // two-way match `\include` itself performs against project documents.
    let main = main_with(
        "\\includeonly{  a.tex , b  }\n",
        "\\include{a}\n\\include{b.tex}\n\\include{c}\n",
    );
    let docs = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "a.tex",
            text: "Alpha content.\n",
        },
        SourceDocument {
            path: "b.tex",
            text: "Bravo content.\n",
        },
        SourceDocument {
            path: "c.tex",
            text: "Charlie content.\n",
        },
    ];
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Alpha content."), "{text:?}");
    assert!(text.contains("Bravo content."), "{text:?}");
    assert!(!text.contains("Charlie"), "{text:?}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn second_includeonly_warns_and_keeps_the_first() {
    let main = main_with(
        "\\includeonly{a}\n\\includeonly{b}\n",
        "\\include{a}\n\\include{b}\n",
    );
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Alpha content."), "{text:?}");
    assert!(!text.contains("Bravo"), "{text:?}");
    assert!(
        parsed.diagnostics.iter().any(|d| d.severity == Severity::Warning
            && d.message.contains("more than once")),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn includeonly_after_begin_document_is_ignored() {
    let main = main_with("", "Before.\n\\includeonly{a}\n\\include{a}\n\\include{b}\n");
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Alpha content."), "{text:?}");
    assert!(
        text.contains("Bravo content."),
        "misplaced list must not filter: {text:?}"
    );
    assert!(
        parsed.diagnostics.iter().any(|d| d.severity == Severity::Warning
            && d.message.contains("preamble")),
        "{:?}",
        parsed.diagnostics
    );
}
