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
//!
//! A user's own breaks stay (opus-review-2 on #1065, same pdflatex):
//! `A\input{a}B` with a.tex `In a.\newpage` → 2, with `\newpage In a.` → 2;
//! `A\input{a}` with a.tex `\clearpage In a.` → 2; `\maketitle\newpage B`
//! → 2; `\tableofcontents\newpage\section{S}B` → 2. And against an
//! `\include`: `A\clearpage\include{c1}B` → 3, `A\newpage\include{c1}B` →
//! 3, `A\include{c1}\newpage B` → 3, `A\include{c1}\cleardoublepage B` → 3,
//! `A\include{c1}\pagebreak B` → 3, c1.tex `\newpage Inside c1.` → 3,
//! `\include{nb}B` with nb.tex `In nb.\newpage` → 2.

mod common;

use common::*;

fn pages_with(main: &str, extra: &[(&str, &str)]) -> usize {
    let mut docs = vec![("main.tex", main), ("c1.tex", "Inside c1.\n"), ("c2.tex", "Inside c2.\n")];
    docs.extend_from_slice(extra);
    let r = render_docs(&docs, "main.tex");
    r.v2.pages.len()
}

fn pages(main: &str) -> usize {
    pages_with(main, &[])
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

#[test]
fn user_page_breaks_next_to_files_and_includes_are_kept() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let doc = |pre: &str, body: &str| format!("\\documentclass{{article}}{pre}\\begin{{document}}{body}\\end{{document}}\n");
    let cases: Vec<(String, Vec<(&str, &str)>, usize)> = vec![
        (doc("", "A\\input{a}B"), vec![("a.tex", "In a.\\newpage\n")], 2),
        (doc("", "A\\input{a}B"), vec![("a.tex", "\\newpage In a.\n")], 2),
        (doc("", "A\\input{a}"), vec![("a.tex", "\\clearpage In a.\n")], 2),
        (doc("\\title{T}\\author{U}\\date{}", "\\maketitle\\newpage B"), vec![], 2),
        (doc("", "\\tableofcontents\\newpage\\section{S}B"), vec![], 2),
        (doc("", "A\\clearpage\\include{c1}B"), vec![], 3),
        (doc("", "A\\newpage\\include{c1}B"), vec![], 3),
        (doc("", "A\\include{c1}\\newpage B"), vec![], 3),
        (doc("", "A\\include{c1}\\cleardoublepage B"), vec![], 3),
        (doc("", "A\\include{c1}\\pagebreak B"), vec![], 3),
        (doc("", "A\\include{nc}B"), vec![("nc.tex", "\\newpage Inside c1.\n")], 3),
        (doc("", "\\include{nb}B"), vec![("nb.tex", "In nb.\\newpage\n")], 2),
    ];
    for (main, extra, want) in &cases {
        assert_eq!(pages_with(main, extra), *want, "{main} {extra:?}");
    }
}
