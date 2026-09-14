//! `thebibliography` is `\chapter*{\bibname}` in report/book and
//! `\section*{\refname}` in article (report.cls lines 664-676 against
//! article.cls lines 693-705). The compiler synthesises one unnumbered
//! level-1 heading reading `References` for every class, so a `report`
//! bibliography used to be set in the flow of the preceding page under the
//! wrong name; `\chapter*`'s `\clearpage` and `\@makeschapterhead` are what
//! put it on a page of its own.
//!
//! Expected coordinates measured with `tools/visual-oracle/pdftext.py` on
//! the PDF pdflatex (TeX Live 2025, pdfTeX 3.141592653-2.6-1.40.27,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) produced from exactly this
//! source. The oracle is not run here; pdflatex is an oracle only and never
//! in the product path. Coordinates are bp from the page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.3;

const REPORT: &str = concat!(
    "\\documentclass[11pt,a4paper]{report}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\begin{document}\n",
    "\\chapter{A chapter}\n",
    "Some body text in the chapter.\n\n",
    "\\section{A section}\n",
    "More body text here.\n\n",
    "\\begin{thebibliography}{99}\n",
    "\\bibitem{a} A. Author. A title. Publisher, 1999.\n",
    "\\bibitem{b} B. Buthor. B title. Publisher, 2000.\n",
    "\\end{thebibliography}\n",
    "\\end{document}\n",
);

const ARTICLE: &str = concat!(
    "\\documentclass[11pt,a4paper]{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\begin{document}\n",
    "\\section{A section}\n",
    "More body text here.\n\n",
    "\\begin{thebibliography}{99}\n",
    "\\bibitem{a} A. Author. A title. Publisher, 1999.\n",
    "\\end{thebibliography}\n",
    "\\end{document}\n",
);

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

#[test]
fn report_starts_the_bibliography_on_its_own_page_as_a_starred_chapter() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(REPORT);
    assert_eq!(r.v2.pages.len(), 2, "\\chapter*'s \\clearpage: {:?}", r.v2.diagnostics);
    let w = words_of(&r);

    // `\bibname`, not article's `\refname`.
    assert!(!w.iter().any(|x| x.text == "References"), "report uses \\bibname: {w:?}");
    let head = word(&w, "Bibliography");
    assert_eq!(head.page, 2, "the bibliography head opens page 2");
    // `\@makeschapterhead`: `\vspace*{50pt}` then the title at `\Huge`, with
    // no `Chapter <n>` line above it (pdflatex: 162.660).
    assert!((head.baseline - 162.660).abs() < TOL, "\\@makeschapterhead drop: {}", head.baseline);
    assert!((head.x - 72.000).abs() < TOL, "flush at the margin: {}", head.x);

    // The chapter body stays on page 1 and is unmoved.
    let body = word(&w, "here.");
    assert_eq!(body.page, 1, "the chapter body stays on page 1");
    assert!((body.baseline - 319.610).abs() < TOL, "chapter body baseline: {}", body.baseline);

    // The entries follow `\vskip 40pt` below the head, at the class's
    // `thebibliography` `\list` geometry (pdflatex: 77.424 / 216.060).
    let first = word(&w, "[1]");
    assert_eq!(first.page, 2);
    assert!((first.x - 77.424).abs() < TOL, "first entry x: {}", first.x);
    assert!((first.baseline - 216.060).abs() < TOL, "first entry baseline: {}", first.baseline);
}

#[test]
fn article_keeps_the_bibliography_in_the_flow_as_a_starred_section() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(ARTICLE);
    assert_eq!(r.v2.pages.len(), 1, "no \\clearpage in article: {:?}", r.v2.diagnostics);
    let w = words_of(&r);
    assert!(!w.iter().any(|x| x.text == "Bibliography"), "article uses \\refname: {w:?}");
    let head = word(&w, "References");
    assert_eq!(head.page, 1, "article sets it in the flow");
    // Below the body text it follows, not at a chapter-head drop.
    assert!(head.baseline > word(&w, "here.").baseline, "after the body: {w:?}");
}
