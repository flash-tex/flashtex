//! `\markboth` / `\markright` set the running head under
//! `\pagestyle{headings}`, from the page they run on, and a mark a macro
//! produced counts exactly as a written-out one does (PLAN1 site 17: the
//! marks are the compiler's `Inline::Mark` now, not `\markboth{` in the
//! source bytes).
//!
//! No fixture in `fixtures/` sets a mark by hand, so without this test the
//! corpus gates cannot see a mark that stops reaching its page — which is
//! exactly what the first draft of the site-17 migration did, by letting the
//! marker's own span become the paragraph's layout position.

mod common;

use common::{lm_available, render_one, words_of};

/// The words of page `page`, in paint order.
fn page_words(source: &str, page: u32) -> Vec<String> {
    let rendered = render_one(source);
    assert!(!rendered.v2.pages.is_empty(), "{:?}", rendered.v2.diagnostics);
    words_of(&rendered).into_iter().filter(|w| w.page == page).map(|w| w.text).collect()
}

const DIRECT: &str = "\\documentclass{article}\n\\begin{document}\n\
    \\pagestyle{headings}\\markboth{LeftMark}{RightMark}Text on the page.\n\
    \\newpage\nMore text.\n\\end{document}\n";

const VIA_MACRO: &str = "\\documentclass{article}\n\\newcommand\\mb{\\markboth{LeftMark}{RightMark}}\n\\begin{document}\n\
    \\pagestyle{headings}\\mb Text on the page.\n\
    \\newpage\nMore text.\n\\end{document}\n";

const MARKRIGHT: &str = "\\documentclass{article}\n\\begin{document}\n\
    \\pagestyle{headings}\\markright{OnlyRight}Text on the page.\n\
    \\newpage\nMore text.\n\\end{document}\n";

#[test]
fn markboth_heads_the_page_it_runs_on() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // article's one-sided `headings` head is `\rightmark \hfil \thepage`,
    // so the right mark stands on the page the command ran on and on every
    // page after it.
    for page in [1, 2] {
        let words = page_words(DIRECT, page);
        assert_eq!(words.first().map(String::as_str), Some("RightMark"), "page {page}: {words:?}");
    }
    // The mark is not body text: `LeftMark` is set nowhere on the page.
    let body: Vec<String> = page_words(DIRECT, 1);
    assert!(!body.iter().any(|w| w == "LeftMark"), "{body:?}");
}

#[test]
fn a_mark_from_a_macro_sets_the_same_head() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    for page in [1, 2] {
        assert_eq!(page_words(DIRECT, page), page_words(VIA_MACRO, page), "page {page}");
    }
}

#[test]
fn markright_sets_the_right_mark_alone() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = page_words(MARKRIGHT, 1);
    assert_eq!(words.first().map(String::as_str), Some("OnlyRight"), "{words:?}");
}
