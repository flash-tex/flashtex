//! `\chapter` inside an `\include`d / `\input` file is a chapter: it
//! clears to a recto page, is numbered, sets its marks and writes its
//! contents line — exactly as when the same text sits in the entry file.
//!
//! The adapter used to read chapter (and part, appendix, mark, page-style
//! and `\addcontentsline`) commands from the entry document's source only,
//! so a chapter in an included file was set as a plain paragraph on the
//! current page. The corpus document `extended/project-book-include` (#42)
//! came out on 5 pages against pdflatex's 7.
//!
//! Expected data: MacTeX 2026 pdflatex, three runs, `pdftotext` per page
//! of the document below (page count and each page's first text line). No
//! TeX runs here.

mod common;

use common::*;

const MAIN: &str = "\\documentclass[twoside,openright]{book}
\\begin{document}\\frontmatter\\tableofcontents\\mainmatter
\\include{chapters/one}\\include{chapters/two}
\\appendix\\chapter{Appendix}See Chapter~\\ref{ch:one}.\\end{document}
";
const ONE: &str = "\\chapter{First}\\label{ch:one}First chapter text.";
const TWO: &str = "\\chapter{Second}Second chapter cites Chapter~\\ref{ch:one}.";

/// Each page's topmost line of text, its words in reading order.
fn first_lines(r: &flashtex_render_pipeline::Rendered) -> Vec<String> {
    let words = words_of(r);
    r.v2
        .pages
        .iter()
        .map(|page| {
            let on_page: Vec<&Word> = words.iter().filter(|w| w.page == page.number).collect();
            let Some(top) = on_page.iter().map(|w| w.baseline).reduce(f64::min) else {
                return String::new();
            };
            let mut line: Vec<&&Word> = on_page.iter().filter(|w| (w.baseline - top).abs() < 0.5).collect();
            line.sort_by(|a, b| a.x.total_cmp(&b.x));
            line.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
        })
        .collect()
}

#[test]
fn chapters_of_included_files_start_their_own_pages() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let r = render_docs(&[("main.tex", MAIN), ("chapters/one.tex", ONE), ("chapters/two.tex", TWO)], "main.tex");
    let lines = first_lines(&r);
    eprintln!("{lines:#?}");
    // pdflatex: Contents | ii (verso, blank) | Chapter 1 | 2 CHAPTER 1. FIRST
    // (verso) | Chapter 2 | 4 CHAPTER 2. SECOND (verso) | Appendix A.
    assert_eq!(r.v2.pages.len(), 7, "page count (pdflatex: 7)");
    let expected = ["Contents", "ii", "Chapter 1", "2 CHAPTER 1. FIRST", "Chapter 2", "4 CHAPTER 2. SECOND", "Appendix A"];
    for (page, (got, want)) in lines.iter().zip(expected).enumerate() {
        assert!(got.starts_with(want), "page {}: first line {got:?}, pdflatex {want:?}", page + 1);
    }
}

/// The same chapters written in the entry file: the control the included
/// form must reproduce.
#[test]
fn inline_chapters_control() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = MAIN.replace("\\include{chapters/one}\\include{chapters/two}", &format!("{ONE}{TWO}"));
    let inline = render_docs(&[("main.tex", main.as_str())], "main.tex");
    let included = render_docs(&[("main.tex", MAIN), ("chapters/one.tex", ONE), ("chapters/two.tex", TWO)], "main.tex");
    assert_eq!(inline.v2.pages.len(), 7);
    assert_eq!(first_lines(&included), first_lines(&inline));
}
