//! `\thepage` (issue #519): the current page's formatted page number,
//! resolved late — when the page is set, on the same path as `\pageref` —
//! and honouring the `\pagenumbering` style in force at its position.

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

/// Every `\thepage` in source order: the physical page holding it and the
/// text it resolved to. Items keep the command token's span (macro-produced
/// text carries the invocation span), so each occurrence is attributable.
fn thepages(source: &str, out: &CompileOutput) -> Vec<(u32, String)> {
    let mut starts = Vec::new();
    let mut base = 0;
    while let Some(found) = source[base..].find(r"\thepage") {
        starts.push(base + found);
        base += found + 1;
    }
    starts
        .into_iter()
        .map(|start| {
            let (page_number, item) = out
                .pages
                .iter()
                .find_map(|page| {
                    page.items
                        .iter()
                        .find(|item| item.span.start == start)
                        .map(|item| (page.number, item))
                })
                .unwrap_or_else(|| panic!("no laid-out item for \\thepage at byte {start}"));
            (page_number, item.text.clone())
        })
        .collect()
}

#[test]
fn thepage_is_known_and_prints_one_on_a_single_page() {
    let source = r"Hello \thepage.";
    let out = compile(source);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(thepages(source, &out), vec![(1, "1".to_string())]);
}

#[test]
fn thepage_resolves_per_page_across_an_explicit_break() {
    let source = r"First \thepage.\newpage Second \thepage.";
    let out = compile(source);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(out.pages.len(), 2);
    assert_eq!(
        thepages(source, &out),
        vec![(1, "1".to_string()), (2, "2".to_string())]
    );
}

#[test]
fn pagenumbering_roman_styles_thepage_and_resets_the_counter() {
    let source = concat!(
        r"One \thepage.",
        r"\newpage Two \thepage.",
        r"\newpage \pagenumbering{roman}Three \thepage.",
        r"\newpage Four \thepage.",
    );
    let out = compile(source);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(out.pages.len(), 4);
    assert_eq!(
        thepages(source, &out),
        vec![
            (1, "1".to_string()),
            (2, "2".to_string()),
            (3, "i".to_string()),
            (4, "ii".to_string()),
        ]
    );
}

#[test]
fn pagenumbering_alph_styles_thepage() {
    let source = r"\pagenumbering{alph}First \thepage.\newpage Second \thepage.";
    let out = compile(source);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(
        thepages(source, &out),
        vec![(1, "a".to_string()), (2, "b".to_string())]
    );
}

#[test]
fn pagenumbering_switch_mid_paragraph_applies_from_that_point() {
    let source = r"First \thepage, then \pagenumbering{roman}second \thepage.";
    let out = compile(source);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(out.pages.len(), 1);
    assert_eq!(
        thepages(source, &out),
        vec![(1, "1".to_string()), (1, "i".to_string())]
    );
}

#[test]
fn pageref_prints_the_labelled_pages_pagenumbering_style() {
    let source = r"\pagenumbering{roman}See \pageref{s}. \section{S}\label{s}";
    let out = compile(source);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let start = source.find(r"\pageref").unwrap();
    let reference = out
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.span.start == start)
        .expect("page reference item");
    assert_eq!(reference.text, "i");
}

#[test]
fn preamble_pagenumbering_styles_a_body_thepage() {
    let source = "\\documentclass{article}\n\\pagenumbering{Roman}\n\\begin{document}\nBody \\thepage.\n\\end{document}\n";
    let out = compile(source);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(thepages(source, &out), vec![(1, "I".to_string())]);
}
