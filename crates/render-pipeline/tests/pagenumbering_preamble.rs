//! `\pagenumbering` in the preamble takes effect on page 1.
//!
//! latex.ltx `\@pagenumbering` runs wherever it stands, resetting `\c@page`
//! and the folio style before page 1 ships. The pipeline only scanned body
//! commands for it, so a preamble `\pagenumbering{roman}` left page 1's
//! folio arabic (`1`); only `\pagenumbering` is preamble-scanned, every
//! other command of that scan stays body-only.
//!
//! Every expected number is pdflatex (TeX Live 2026), article 10pt,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 pdflatex
//! -interaction=batchmode` on exactly [`SOURCE`], read per glyph with the
//! page-frame oracle's PDF reader (bp, y from the page top). The oracle
//! never runs here: the table is committed evidence. Before the fix page
//! 1's footer was `1` at x 303.133; after it every word below is within
//! 0.005 bp.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\pagenumbering{roman}
\\begin{document}
Text. \\newpage
\\pagenumbering{arabic} More. \\newpage \\setcounter{page}{10} Last.
\\end{document}
";

/// (page, text, x bp, baseline y bp) for the body words and folios.
const EXPECTED: &[(u32, &str, f64, f64)] = &[
    (1, "Text.", 148.712, 134.765),
    (1, "i", 304.240, 702.635),
    (2, "More.", 148.712, 134.765),
    (2, "1", 303.133, 702.635),
    (3, "Last.", 148.712, 134.765),
    (3, "10", 300.643, 702.635),
];

#[test]
fn preamble_pagenumbering_styles_page_one() {
    if !lm_available() {
        return;
    }
    let r = render_one(SOURCE);
    assert_eq!(r.v2.pages.len(), 3, "three pages");
    let words = words_of(&r);
    let mut misses = Vec::new();
    for (page, text, x, y) in EXPECTED {
        if !words.iter().any(|w| w.page == *page && w.text == **text && (w.x - x).abs() <= 0.1 && (w.baseline - y).abs() <= 0.1) {
            let near: Vec<String> = words
                .iter()
                .filter(|w| w.page == *page)
                .map(|w| format!("{:?} ({:.3}, {:.3})", w.text, w.x, w.baseline))
                .collect();
            misses.push(format!("p{page} {text:?} at ({x}, {y}): {near:?}"));
        }
    }
    assert!(misses.is_empty(), "{} of {} words misplaced:\n{}", misses.len(), EXPECTED.len(), misses.join("\n"));
}
