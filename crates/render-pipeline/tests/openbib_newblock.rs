//! article.cls's `openbib` class option redefines `\newblock` from
//! `\hskip .11em \@plus.33em \@minus.07em` to `\par`: each bibliography
//! block starts its own line instead of running together separated by a
//! space. The real `.bbl` (and so `bibliography::format::Block`) is
//! identical either way -- only the class's `\newblock` changes -- so the
//! switch lives where the pipeline adapts `\newblock`, in
//! `adapter::items_from_inlines_styled`'s gap handling, fed by
//! `Stylesheet::class_geometry.options.openbib` like the other class flags.
//!
//! The break assertions are relative (same line vs new line), measured
//! through the public `render` entry point. The indent assertion below is
//! absolute: real pdflatex (TeX Live, 10pt `article[openbib]`) sets the
//! wrapped line 14.94bp right of the entry (`pdftotext -bbox`: `Jane` at
//! 149.27, the wrap at 164.21), i.e. `\bibindent` (15pt, `article.cls`
//! `\setlength\bibindent{1.5em}`), while `\newblock` lines stay flush.

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

fn left(words: &[Word], word: &str) -> f64 {
    words
        .iter()
        .find(|w| w.text.trim() == word)
        .unwrap_or_else(|| panic!("word `{word}` not set: {words:?}"))
        .x
}

/// An entry whose second block is long enough to wrap onto a second visual
/// line, so the test can tell a `\newblock` line (flush) from a wrapped
/// continuation line (hung by `\bibindent`).
fn wrapping_entry_src(class_options: &str) -> String {
    format!(
        "\\documentclass{CLASS}article\n\\begin{{document}}\n\\begin{{thebibliography}}{{9}}\n\
         \\bibitem{{doe00}} Jane Doe. \\newblock A short title with enough words that this second block wraps onto another line here. \\newblock Somewhere, 2000.\n\
         \\end{{thebibliography}}\n\\end{{document}}\n",
        CLASS = class_options,
    )
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

/// With `openbib` a wrapped continuation line hangs `\bibindent` (14.94bp
/// at 10pt) right of the entry, while a `\newblock` line stays flush with
/// the entry's first line (`article.cls` `\@openbib@code`: `\leftmargin`
/// advances by `\bibindent`, `\itemindent`/`\listparindent` come back by
/// the same amount).
#[test]
fn continuation_line_hangs_bibindent_with_openbib() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = words(&wrapping_entry_src("[openbib]"));
    let x_first = left(&words, "Jane");
    // Sanity: `line` really is a wrapped continuation of block 2, not a
    // `\newblock` line of its own.
    assert!(
        baseline(&words, "line") - baseline(&words, "A") > 1.0,
        "`line` wraps block 2 with openbib: {words:?}",
    );
    // The `\newblock` lines themselves stay flush with the entry.
    for w in ["A", "Somewhere,"] {
        assert!(
            (left(&words, w) - x_first).abs() < 0.05,
            "`{w}` starts flush with the entry under openbib: {words:?}",
        );
    }
    // The wrapped continuation hangs `\bibindent` (15pt = 14.94bp) in.
    let hang = left(&words, "line") - x_first;
    assert!(
        (hang - 14.94).abs() < 0.1,
        "wrapped line hangs 14.94bp under openbib, got {hang:.3}: {words:?}",
    );
}

/// Without `openbib` the same wrapping entry sets every line flush: no
/// `\bibindent` anywhere (this lane changes nothing there).
#[test]
fn continuation_line_stays_flush_without_openbib() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `[]`: a valid option-less `article` (the same 10pt geometry as the
    // `[openbib]` run, so only the option differs).
    let words = words(&wrapping_entry_src("[]"));
    let x_first = left(&words, "Jane");
    let y_first = baseline(&words, "Jane");
    // Every visual line the entry sets below its first (block 2's wrap, if
    // any, and the `\newblock` lines) opens flush: only line-initial words
    // count, mid-line words sit further right by construction.
    let mut lines: Vec<f64> = words
        .iter()
        .filter(|w| {
            !["References", "[1]"].contains(&w.text.trim())
                && w.baseline - y_first > 0.01
                && w.baseline - y_first < 60.0
        })
        .map(|w| w.baseline)
        .collect();
    lines.sort_by(|a, b| a.partial_cmp(b).unwrap());
    lines.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    assert!(!lines.is_empty(), "the entry sets later lines: {words:?}");
    for y in lines {
        let x = words
            .iter()
            .filter(|w| (w.baseline - y).abs() < 0.01)
            .map(|w| w.x)
            .fold(f64::INFINITY, f64::min);
        assert!(
            (x - x_first).abs() < 0.05,
            "line at {y:.2} opens flush without openbib: {words:?}",
        );
    }
}
