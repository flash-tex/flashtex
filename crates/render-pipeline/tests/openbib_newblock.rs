//! article.cls's `openbib` class option redefines `\newblock` from
//! `\hskip .11em \@plus.33em \@minus.07em` to `\par`: each bibliography
//! block starts its own line instead of running together separated by a
//! space. The real `.bbl` (and so `bibliography::format::Block`) is
//! identical either way -- only the class's `\newblock` changes -- so the
//! switch lives where the pipeline adapts `\newblock`, in
//! `adapter::items_from_inlines_styled`'s gap handling, fed by
//! `Stylesheet::class_geometry.options.openbib` like the other class flags.
//!
//! No pdflatex oracle here: the assertions are relative (same line vs new
//! line), measured through the public `render` entry point.

mod common;

use common::*;

fn entry_src(class_options: &str) -> String {
    format!(
        "\\documentclass{CLASS}article\n\\begin{{document}}\n\\begin{{thebibliography}}{{9}}\n\
         \\bibitem{{doe00}} Jane Doe. \\newblock A short title. \\newblock Somewhere, 2000.\n\
         \\end{{thebibliography}}\n\\end{{document}}\n",
        CLASS = class_options,
    )
}

fn words(src: &str) -> Vec<Word> {
    let r = render_one(src);
    assert_eq!(r.v2.pages.len(), 1, "one page expected: {:?}", r.v2.diagnostics);
    words_of(&r)
}

fn baseline(words: &[Word], word: &str) -> f64 {
    words
        .iter()
        .find(|w| w.text.trim() == word)
        .unwrap_or_else(|| panic!("word `{word}` not set: {words:?}"))
        .baseline
}

/// Without `openbib` the three blocks run together: every entry word sits
/// on one baseline (today's Space + Quad behavior, unchanged).
#[test]
fn newblock_joins_with_a_space_without_openbib() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = words(&entry_src(""));
    let y_doe = baseline(&words, "Doe.");
    for w in ["Jane", "A", "short", "title.", "Somewhere,", "2000."] {
        assert!(
            (baseline(&words, w) - y_doe).abs() < 0.01,
            "`{w}` shares the entry line without openbib: {words:?}",
        );
    }
}

/// With `openbib` each `\newblock` opens a new line: block 2 starts below
/// block 1 and block 3 below block 2, while block 1 itself stays one line.
#[test]
fn newblock_breaks_the_line_with_openbib() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = words(&entry_src("[openbib]"));
    let y_doe = baseline(&words, "Doe.");
    assert!(
        (baseline(&words, "Jane") - y_doe).abs() < 0.01,
        "block 1 stays one line with openbib: {words:?}",
    );
    let y_block2 = baseline(&words, "title.");
    assert!(
        y_block2 - y_doe > 1.0,
        "block 2 starts its own line with openbib: {words:?}",
    );
    assert!(
        (baseline(&words, "A") - y_block2).abs() < 0.01,
        "block 2 stays one line with openbib: {words:?}",
    );
    let y_block3 = baseline(&words, "2000.");
    assert!(
        y_block3 - y_block2 > 1.0,
        "block 3 starts its own line with openbib: {words:?}",
    );
}
