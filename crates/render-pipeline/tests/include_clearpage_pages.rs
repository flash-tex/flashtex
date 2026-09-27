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

/// The text of each page, glyph runs joined by spaces.
fn page_texts(main: &str) -> Vec<String> {
    let r = render_docs(&[("main.tex", main), ("c1.tex", "Chapter one text.\n"), ("c2.tex", "Chapter two text.\n")], "main.tex");
    r.v2
        .pages
        .iter()
        .map(|p| {
            p.resident_items()
                .iter()
                .filter_map(|it| match it {
                    flashtex_render_pipeline::display::Item::GlyphRun(run) => Some(run.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// An `\include` token TeX never runs (in an unused definition) makes no
/// `\include` break point, so it cannot swallow a user's `\newpage` at the
/// same file crossing (review round 4 on #1070; `adapter::unexecuted_ranges`).
///
/// Pages from TeX Live 2026 pdflatex (two runs, `article`; c1.tex
/// `Chapter one text.`, c2.tex `Chapter two text.`), text per page.
#[test]
fn unexecuted_include_tokens_keep_the_real_breaks() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let doc = |pre: &str, body: &str| format!("\\documentclass{{article}}{pre}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
    let cases = [
        // pdflatex: `First.` | `Second.` | `Chapter one text.` | `After.`
        (
            doc("", "First.\n\n\\newcommand{\\unused}{\\include{c1}}\n\n\\newpage\nSecond.\n\n\\include{c1}\nAfter."),
            vec!["First. 1", "Second. 2", "Chapter one text. 3", "After. 4"],
        ),
        // Two unused definitions between two `\newpage`s: the same four pages.
        (
            doc("", "First.\n\\newpage\n\\newcommand{\\unused}{\\include{c1}}\\newcommand{\\unusedb}{\\include{c1}}\n\\newpage\nSecond.\n\n\\include{c1}\nAfter."),
            vec!["First. 1", "Second. 2", "Chapter one text. 3", "After. 4"],
        ),
        // Under `\includeonly{c2}`: `First.` | `Second.` | `Chapter two text.` | `After.`
        (
            doc("\\includeonly{c2}", "First.\n\n\\newcommand{\\unused}{\\include{c1}}\n\n\\newpage\nSecond.\n\n\\include{c2}\nAfter."),
            vec!["First. 1", "Second. 2", "Chapter two text. 3", "After. 4"],
        ),
    ];
    for (main, want) in &cases {
        assert_eq!(page_texts(main), *want, "{main}");
    }
}

/// Falsifier for #1089 (pre-existing on main): an `\include` typed in a
/// verbatim body is text, not a file read. pdflatex (TeX Live 2026) sets
/// `First. \include{c1} More.` | `Chapter one text.` | `After.`; the
/// pipeline breaks the page after the verbatim because `body_commands`
/// reads the raw token.
#[test]
#[ignore = "#1089: body_commands reads \\include tokens inside verbatim"]
fn an_include_token_in_verbatim_is_not_read() {
    if !lm_available() {
        return;
    }
    let main = "\\documentclass{article}\n\\begin{document}\nFirst.\n\\begin{verbatim}\n\\include{c1}\n\\end{verbatim}\nMore.\n\n\\include{c1}\nAfter.\n\\end{document}\n";
    assert_eq!(page_texts(main).len(), 3, "{:?}", page_texts(main));
}

/// Falsifier for #1089 (pre-existing on main): `\include` through a macro
/// is read at the invocation. pdflatex (TeX Live 2026) sets `First.` |
/// `Chapter one text.` | `After.`; the pipeline sets one page.
#[test]
#[ignore = "#1089: an \\include run from a macro body is not seen at its invocation"]
fn an_include_through_a_macro_breaks_at_the_invocation() {
    if !lm_available() {
        return;
    }
    let main = "\\documentclass{article}\n\\newcommand{\\inc}{\\include{c1}}\n\\begin{document}\nFirst.\n\n\\inc\nAfter.\n\\end{document}\n";
    assert_eq!(page_texts(main).len(), 3, "{:?}", page_texts(main));
}
