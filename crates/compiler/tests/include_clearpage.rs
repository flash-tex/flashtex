//! `\include` is `\clearpage` before and after the included text (latex.ltx
//! `\@include`). An `\includeonly`-excluded file contributes no text but
//! keeps a single break: pdflatex runs both `\clearpage`s, and the second is
//! a no-op on the still-empty page.
//!
//! Ground truth, pdflatex (TeX Live 2026) on
//! `\documentclass{article}\begin{document}A\include{c1}B\end{document}`
//! with `c1.tex` holding `Chapter one.`:
//! `pdfinfo` reports 3 pages; `pdftotext -layout` shows `A` on page 1,
//! `Chapter one.` on page 2, `B` on page 3. With `\includeonly{c2}` in the
//! preamble (so `c1` is excluded): 2 pages, `A` on page 1, `B` on page 2.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
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

fn block_kinds(blocks: &[Block]) -> Vec<&'static str> {
    blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph(_) => "para",
            Block::PageBreak => "break",
            _ => "other",
        })
        .collect()
}

fn page_words(
    out: &flashtex_compiler::incremental::CompileOutput,
) -> Vec<Vec<String>> {
    out.pages
        .iter()
        .map(|p| p.items.iter().map(|i| i.text.clone()).collect())
        .collect()
}

fn main_with(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\\end{{document}}\n")
}

fn project<'a>(main: &'a str) -> [SourceDocument<'a>; 2] {
    [
        SourceDocument {
            path: "main.tex",
            text: main,
        },
        SourceDocument {
            path: "c1.tex",
            text: "Chapter one.\n",
        },
    ]
}

#[test]
fn include_emits_a_page_break_before_and_after() {
    let main = main_with("A\\include{c1}B\n");
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(
        block_kinds(&parsed.blocks),
        ["para", "break", "para", "break", "para"],
        "{:?}",
        parsed.blocks
    );
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains('A'), "{text:?}");
    assert!(text.contains("Chapter one."), "{text:?}");
    assert!(text.contains('B'), "{text:?}");
}

#[test]
fn include_page_count_matches_pdflatex() {
    // pdflatex: `pdfinfo` reports 3 pages; `pdftotext -layout` puts `A` on
    // page 1, `Chapter one.` on page 2 and `B` on page 3.
    let main = main_with("A\\include{c1}B\n");
    let docs = project(&main);
    let out = compile_full_project(&docs, "main.tex", LayoutConstraints::default());
    assert!(
        out.diagnostics.is_empty(),
        "{:?}",
        out.diagnostics
    );
    assert_eq!(
        page_words(&out),
        vec![
            vec!["A".to_string()],
            vec!["Chapter".to_string(), "one.".to_string()],
            vec!["B".to_string()],
        ],
        "{:?}",
        out.pages
    );
}

#[test]
fn excluded_include_keeps_its_break_but_emits_no_text() {
    // pdflatex with `\includeonly{c2}`: 2 pages, `A` on page 1, `B` on page
    // 2 — the excluded file's two `\clearpage`s collapse to one transition.
    let main = "\\documentclass{article}\n\\includeonly{c2}\n\\begin{document}\nA\\include{c1}B\n\\end{document}\n";
    let docs = project(main);
    let parsed = parse_project(&docs, "main.tex");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    let text = paragraph_text(&parsed.blocks);
    assert!(!text.contains("Chapter"), "{text:?}");
    assert!(text.contains('A') && text.contains('B'), "{text:?}");
    assert_eq!(page_breaks(&parsed.blocks), 1, "{:?}", parsed.blocks);
    let out = compile_full_project(&docs, "main.tex", LayoutConstraints::default());
    assert!(
        out.diagnostics.is_empty(),
        "{:?}",
        out.diagnostics
    );
    assert_eq!(
        page_words(&out),
        vec![vec!["A".to_string()], vec!["B".to_string()]],
        "{:?}",
        out.pages
    );
}

#[test]
fn input_still_emits_no_page_break() {
    // `\input` is the break-free inclusion: only `\include` clearpages.
    let main = main_with("A\\input{c1}B\n");
    let docs = project(&main);
    let parsed = parse_project(&docs, "main.tex");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(page_breaks(&parsed.blocks), 0, "{:?}", parsed.blocks);
    let text = paragraph_text(&parsed.blocks);
    assert!(text.contains("Chapter one."), "{text:?}");
}
