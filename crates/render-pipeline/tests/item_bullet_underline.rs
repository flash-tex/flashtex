//! GH-924: an itemize item whose text holds `\uline`, `\sout` or
//! `\underline` — directly or through a macro, anywhere in the item — set
//! its bullet 2.77 bp left of pdflatex's (145.94 against 148.71 bp for
//! article 10 pt, T1, `ulem`) while the item text sat at the right x.
//!
//! The compiler's `box_inlines` (the box argument's nested parse) restores
//! `pending_item_label` but not `pending_item`, so the block reaches the
//! adapter with its label text and `item: None`, and the label was set as
//! a Latin Modern Roman word (`•` at 0.7778 em) instead of `tcrm`'s 0.5 em
//! `\textbullet`. The adapter now reads article's default itemize symbol
//! back from the label text when `item` is missing.
//!
//! ## Oracle
//!
//! Every x below is a glyph-run origin (bp) read by
//! `tools/visual-oracle/pdftext.py` from pdfLaTeX's output for the same
//! document: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, MacTeX).
//! pdflatex is an oracle only, never in the product path.

mod common;

use common::{lm_available, render_one, words_of, Word};

/// The corpus harness's exact-route tolerance for a word origin.
const TOL: f64 = 0.01;

/// pdflatex: the level-1 bullet and the item text's first glyph.
const BULLET_X: f64 = 148.714;
const TEXT_X: f64 = 158.676;
/// pdflatex: the level-2 `\labelitemii` en dash and its item text.
const DASH_X: f64 = 169.884;
const DASH_TEXT_X: f64 = 180.593;

fn doc(items: &[&str]) -> String {
    let mut s = String::from(
        "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage[normalem]{ulem}\n\
         \\newcommand{\\ul}[1]{\\uline{#1}}\n\\begin{document}\n\\begin{itemize}\n",
    );
    for item in items {
        s.push_str("\\item ");
        s.push_str(item);
        s.push('\n');
    }
    s.push_str("\\end{itemize}\n\\end{document}\n");
    s
}

/// The runs of page 1 grouped by baseline, top to bottom, the page number
/// left out.
fn lines_of(words: &[Word]) -> Vec<Vec<&Word>> {
    let mut lines: Vec<Vec<&Word>> = Vec::new();
    for w in words.iter().filter(|w| w.page == 1 && w.baseline < 600.0) {
        match lines.iter_mut().find(|l| (l[0].baseline - w.baseline).abs() < 0.05) {
            Some(l) => l.push(w),
            None => lines.push(vec![w]),
        }
    }
    lines.sort_by(|a, b| a[0].baseline.partial_cmp(&b[0].baseline).unwrap());
    lines
}

/// Asserts each line's label glyph at `label_x` and the run after it at
/// `text_x`.
fn check(src: &str, label: &str, label_x: f64, text_x: f64, n: usize) {
    let r = render_one(src);
    let words = words_of(&r);
    let lines = lines_of(&words);
    assert_eq!(lines.len(), n, "{src}\n{lines:#?}");
    for line in lines {
        let mut runs = line.iter();
        let first = runs.next().unwrap();
        assert_eq!(first.text.trim(), label, "{line:?}");
        assert!((first.x - label_x).abs() <= TOL, "label at {} bp, pdflatex {label_x}: {line:?}", first.x);
        let second = runs.next().unwrap();
        assert!((second.x - text_x).abs() <= TOL, "text at {} bp, pdflatex {text_x}: {line:?}", second.x);
    }
}

#[test]
fn uline_sout_underline_items_keep_the_bullet_at_the_label_edge() {
    if !lm_available() {
        return;
    }
    let src = doc(&[
        "\"\\uline{unit}\"---one scalar",
        "\"unit\"---one scalar",
        "\"\\sout{unit}\"---one scalar",
        "\"\\underline{unit}\"---one scalar",
        "\\uline{unit} one scalar",
        "unit one scalar",
        "\"\\ul{unit}\"---one scalar",
        "one scalar, \\uline{unit} last",
    ]);
    check(&src, "•", BULLET_X, TEXT_X, 8);
}

#[test]
fn nested_itemize_dash_is_bold_and_at_the_label_edge() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage[normalem]{ulem}\n\
               \\newcommand{\\ul}[1]{\\uline{#1}}\n\\begin{document}\n\\begin{itemize}\n\\item outer\n\
               \\begin{itemize}\n\\item \"\\uline{unit}\"---one scalar\n\\item \"unit\"---one scalar\n\
               \\item \"\\ul{unit}\"---one scalar\n\\end{itemize}\n\\end{itemize}\n\\end{document}\n";
    let r = render_one(src);
    let words = words_of(&r);
    let lines = lines_of(&words);
    assert_eq!(lines.len(), 4, "{lines:#?}");
    assert!((lines[0][0].x - BULLET_X).abs() <= TOL, "{:?}", lines[0]);
    for line in &lines[1..] {
        assert_eq!(line[0].text.trim(), "–", "{line:?}");
        assert!((line[0].x - DASH_X).abs() <= TOL, "dash at {} bp, pdflatex {DASH_X}: {line:?}", line[0].x);
        assert!((line[1].x - DASH_TEXT_X).abs() <= TOL, "text at {} bp, pdflatex {DASH_TEXT_X}: {line:?}", line[1].x);
    }
}
