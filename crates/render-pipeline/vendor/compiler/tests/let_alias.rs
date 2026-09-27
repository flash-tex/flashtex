//! `\let` aliases of host commands (`flashtex-2a/opus-13 wave 7`): TeX copies
//! the meaning, so `\let\oldsection\section` keeps typesetting numbered
//! headings even after `\section` is redefined. The expansion engine emits
//! the alias under its own name, so the converter maps it back to the host
//! command the parser implements.
//!
//! Probe: `pdflatex main.tex` on the article repro below reports 0 errors
//! with `1 A` on page 1 and `2 B` on page 2.
use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::parser::{parse_project, Block, SourceDocument};

fn parse(main: &str) -> flashtex_compiler::parser::Parsed {
    parse_project(
        &[SourceDocument { path: "main.tex", text: main }],
        "main.tex",
    )
}

fn errors(main: &str) -> Vec<String> {
    parse(main)
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

fn heading_numbers(main: &str) -> Vec<(u8, String)> {
    parse(main)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Heading { level, number, .. } => Some((*level, number.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn let_alias_of_section_survives_renewcommand() {
    let main = "\\documentclass{article}\n\\let\\oldsection\\section\n\\renewcommand{\\section}{\\clearpage\\oldsection}\n\\begin{document}\n\\section{A}\nx\n\\section{B}\ny\n\\end{document}\n";
    assert_eq!(errors(main), Vec::<String>::new());
    assert_eq!(
        heading_numbers(main),
        vec![(1, "1".to_string()), (1, "2".to_string())]
    );
    // pdflatex puts `1 A` on page 1 and `2 B` on page 2 (the renewed
    // `\section` still `\clearpage`s through the alias).
    let result = compile_full(main, LayoutConstraints::default());
    let pages: Vec<Vec<&str>> = result
        .pages
        .iter()
        .map(|page| page.items.iter().map(|item| item.text.as_str()).collect())
        .collect();
    assert!(pages.len() >= 2, "expected two pages: {pages:?}");
    assert!(pages[0].contains(&"1") && pages[0].contains(&"A"), "page 1: {pages:?}");
    assert!(pages[1].contains(&"2") && pages[1].contains(&"B"), "page 2: {pages:?}");
}

#[test]
fn let_alias_of_section_takes_titles_directly() {
    let main =
        "\\documentclass{article}\n\\let\\newsection\\section\n\\begin{document}\n\\newsection{T}\n\\end{document}\n";
    assert_eq!(errors(main), Vec::<String>::new());
    assert_eq!(heading_numbers(main), vec![(1, "1".to_string())]);
}

#[test]
fn let_alias_of_item_makes_list_items() {
    let main = "\\documentclass{article}\n\\let\\myitem\\item\n\\begin{document}\n\\begin{itemize}\n\\myitem one\n\\myitem two\n\\end{itemize}\n\\end{document}\n";
    assert_eq!(errors(main), Vec::<String>::new());
    let items = parse(main)
        .blocks
        .iter()
        .filter(|block| matches!(block, Block::ListItem { .. }))
        .count();
    assert_eq!(items, 2);
}

#[test]
fn let_alias_of_label_resolves_refs() {
    let source = "\\documentclass{article}\n\\let\\mylabel\\label\n\\begin{document}\n\\section{One}\\mylabel{sec:one} Back \\ref{sec:one}.\n\\end{document}\n";
    let result = compile_full(source, LayoutConstraints::default());
    let output: Vec<_> = result
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
        .collect();
    assert!(
        !output.iter().any(|text| text == "??"),
        "unresolved ref: {output:?}"
    );
    assert!(
        output.iter().filter(|text| text.as_str() == "1").count() >= 2,
        "heading number and ref should both print 1: {output:?}"
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error && d.message.contains("mylabel")),
        "unexpected errors: {:?}",
        result.diagnostics
    );
}
