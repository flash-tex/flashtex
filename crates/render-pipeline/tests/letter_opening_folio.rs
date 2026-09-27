//! letter.cls `\opening` leaves the first page without a folio.
//!
//! `\opening` issues `\thispagestyle{firstpage}` without `\address` and
//! `\thispagestyle{empty}` with one (letter.cls 217-229); both ship a
//! folio-less page, the location/telephone footer staying empty when
//! unset. The pipeline knew neither style, so page 1 kept the class
//! default `plain` and printed `1` at (303.51, 728.54). Every opening's
//! first block (the compiler always emits its return-address/date block
//! first) now switches this page to `empty`.
//!
//! Every expected number is pdflatex (TeX Live 2026),
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 pdflatex
//! -interaction=batchmode` on exactly [`SOURCE`], read per glyph with the
//! page-frame oracle's PDF reader (bp, y from the page top). The oracle
//! never runs here: the table is committed evidence. pdflatex sets no
//! glyph at all near the folio point, and every body word below is where
//! FlashTeX already put it (within 0.005 bp), so the test pins those and
//! asserts the footer band is empty.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{letter}
\\signature{Ann}
\\address{1 Road\\\\Town}
\\begin{document}
\\begin{letter}{Bob}
\\opening{Dear Bob,}
Body text.
\\closing{Yours,}
\\end{letter}
\\end{document}
";

/// (word, left edge, baseline) read off pdflatex's PDF.
const REFERENCE: &[(&str, f64, f64)] = &[
    ("1", 408.504, 290.735),
    ("Road", 416.803, 290.735),
    ("Town", 408.504, 302.691),
    ("January", 408.504, 328.593),
    ("Bob", 134.144, 361.016),
    ("Dear", 134.144, 386.919),
    ("Bob,", 158.383, 386.919),
    ("Body", 134.144, 405.848),
    ("text.", 160.572, 405.848),
    ("Yours,", 306.000, 429.537),
    ("Ann", 306.000, 483.335),
];

#[test]
fn opening_leaves_the_first_page_without_a_folio() {
    if !lm_available() {
        return;
    }
    let r = render_one(SOURCE);
    assert_eq!(r.v2.pages.len(), 1);
    let words = words_of(&r);
    for (text, x, y) in REFERENCE {
        let Some(w) = words.iter().find(|w| w.text == **text && (w.baseline - y).abs() <= 0.5) else {
            panic!("{text:?} not set near baseline {y} (words: {words:?})");
        };
        assert!(
            (w.x - x).abs() <= 0.1 && (w.baseline - y).abs() <= 0.1,
            "{text:?}: got ({:.3}, {:.3}) bp, pdflatex ({x}, {y}) bp",
            w.x,
            w.baseline
        );
    }
    // No folio glyph at all: nothing is painted in the footer band, where
    // pdflatex's `1` used to sit at (303.51, 728.54).
    let low: Vec<&Word> = words.iter().filter(|w| w.baseline > 700.0).collect();
    assert!(low.is_empty(), "page 1 must carry no folio: {low:?}");
}
