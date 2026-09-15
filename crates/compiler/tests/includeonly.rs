//! `\includeonly{file1,file2,...}`: the kernel's preamble companion to
//! `\include`. A recorded, non-empty list selects which `\include`d files
//! are read; every other `\include` leaves only the `\clearpage` that
//! latex.ltx's `\@include` runs before testing the list. With no
//! `\includeonly`, every `\include` reads as before.
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
    // `\@include`: `\clearpage` before and after the read file `a`, and
    // only the one before for the skipped `b` (pdflatex: `Before.` /
    // `Alpha content.` / `After.` on three pages).
    assert_eq!(page_breaks(&parsed.blocks), 3, "{:?}", parsed.blocks);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn input_never_consults_includeonly() {
    // Real LaTeX tests `\@partlist` only in `\@include`: `\input` always
    // reads, even for files the recorded list does not name.
    let main = main_with(
        "\\includeonly{a}\n",
        "Before.\n\\include{a}\n\\include{b}\n\\input{b}\nAfter.\n",
    );
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Alpha content."), "{text:?}");
    assert_eq!(
        text.matches("Bravo content.").count(),
        1,
        "the skipped \\include{{b}} must leave no trace while \\input{{b}} reads: {text:?}"
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn empty_includeonly_includes_nothing() {
    // `\includeonly{}` still switches parts on, with an empty list: every
    // `\include` is skipped. (`None` — never called — is what allows all.)
    let main = main_with("\\includeonly{}\n", "Before.\n\\include{a}\n\\include{b}\nAfter.\n");
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(!text.contains("Alpha"), "{text:?}");
    assert!(!text.contains("Bravo"), "{text:?}");
    assert!(text.contains("Before.") && text.contains("After."));
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
fn last_includeonly_call_wins_silently() {
    // Each call resets the list (real LaTeX's `\let\@partlist\@empty`), so
    // `\includeonly{a}\includeonly{b}` selects only `b`, with no warning.
    let main = main_with(
        "\\includeonly{a}\n\\includeonly{b}\n",
        "\\include{a}\n\\include{b}\n",
    );
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(!text.contains("Alpha"), "{text:?}");
    assert!(text.contains("Bravo content."), "{text:?}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn includeonly_in_inputted_preamble_config_counts() {
    // A config file `\input` before `\begin{document}` is still preamble:
    // its `\includeonly` must be recorded, not rejected for living in
    // another document.
    let main = "\\documentclass{article}\n\\input{cfg}\n\\begin{document}\n\\include{a}\n\\include{b}\n\\end{document}\n";
    let docs = [
        SourceDocument {
            path: "main.tex",
            text: main,
        },
        SourceDocument {
            path: "cfg.tex",
            text: "\\includeonly{a}\n",
        },
        SourceDocument {
            path: "a.tex",
            text: "Alpha content.\n",
        },
        SourceDocument {
            path: "b.tex",
            text: "Bravo content.\n",
        },
    ];
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Alpha content."), "{text:?}");
    assert!(!text.contains("Bravo"), "{text:?}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn commented_begin_document_does_not_end_preamble() {
    // A `% \begin{document}` comment emits nothing, so an `\includeonly`
    // after it but before the real `\begin{document}` is still preamble.
    let main = "\\documentclass{article}\n% \\begin{document}\n\\includeonly{a}\n\\begin{document}\n\\include{a}\n\\include{b}\n\\end{document}\n";
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Alpha content."), "{text:?}");
    assert!(!text.contains("Bravo"), "{text:?}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
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

/// latex.ltx `\@include`: `\clearpage`, the file, `\clearpage`. pdflatex
/// (TeX Live 2026, two runs) sets `Before.\include{a}After.` on three
/// pages, `Before.`, `Alpha content.` and `After.`; `\input{a}` on one.
#[test]
fn include_clears_the_page_before_and_after_its_file() {
    let order = |blocks: &[Block]| -> Vec<String> {
        blocks
            .iter()
            .filter_map(|b| match b {
                Block::PageBreak => Some("|".to_string()),
                Block::Paragraph(_) => Some(paragraph_text(std::slice::from_ref(b))),
                _ => None,
            })
            .collect()
    };
    let main = main_with("", "Before.\n\\include{a}\nAfter.\n");
    let parsed = parse_project(&project(&main), "main.tex");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(
        order(&parsed.blocks),
        ["Before.", "|", "Alpha content.", "|", "After."]
    );
    // In a paragraph the first `\clearpage` ends it.
    let main = main_with("", "Before \\include{a} after.\n");
    let parsed = parse_project(&project(&main), "main.tex");
    assert_eq!(
        order(&parsed.blocks),
        ["Before", "|", "Alpha content.", "|", "after."]
    );
    // `\input` has no page break.
    let main = main_with("", "Before.\n\\input{a}\nAfter.\n");
    let parsed = parse_project(&project(&main), "main.tex");
    assert_eq!(page_breaks(&parsed.blocks), 0, "{:?}", parsed.blocks);
}
