//! `\include` is `\clearpage`, the file, `\clearpage` (latex.ltx
//! `\@include`). Since ac2a6f534 the compiler pushes those breaks itself, as
//! span-less `PageBreak` blocks where reading crosses a file boundary. The
//! adapter already ejects at the same place (`BodyKind::Input`), so each
//! include must still cost exactly the pages pdflatex ships.
//!
//! Page counts from TeX Live 2026 pdflatex, two runs, `article`:
//! `A\include{c1}B` → 3; under `\includeonly{c2}` → 2;
//! `A\include{c1}\include{c2}B\newpage C` → 5;
//! `\include{c1}\input{c2}B` → 2 (c1 alone, then c2 and B).

mod common;

use common::*;

fn pages(main: &str) -> usize {
    let r = render_docs(&[("main.tex", main), ("c1.tex", "Inside c1.\n"), ("c2.tex", "Inside c2.\n")], "main.tex");
    r.v2.pages.len()
}

#[test]
fn include_page_counts_match_pdflatex() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let doc = |pre: &str, body: &str| format!("\\documentclass{{article}}{pre}\\begin{{document}}{body}\\end{{document}}\n");
    for (main, want) in [
        (doc("", "A\\include{c1}B"), 3),
        (doc("\\includeonly{c2}", "A\\include{c1}B"), 2),
        (doc("", "A\\include{c1}\\include{c2}B\\newpage C"), 5),
        (doc("", "\\include{c1}\\input{c2}B"), 2),
    ] {
        assert_eq!(pages(&main), want, "{main}");
    }
}
