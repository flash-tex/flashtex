//! A List of Tables entry sits under its own heading, before the floats.
//!
//! The float placer anchors a `[h]` float after the last block read before
//! it, but it read only each block's *first* positioned source span. A
//! contents-list entry's number and title point at its float's caption
//! (inside the environment, after the float's own offset) while its page
//! box and leader dots carry the list command's bytes -- so the List of
//! Tables entry never counted as "before" its table, the table landed
//! between the heading and the entry, and the List of Tables rendered
//! with no entries under it. The anchor now counts a block when *any* of
//! its spans precedes the float.
//!
//! Every expected number is pdflatex (TeX Live 2026), article 10pt, three
//! runs (`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 pdflatex
//! -interaction=batchmode`, so the `.lof`/`.lot` are read back) on exactly
//! [`SOURCE`], read per glyph with the page-frame oracle's PDF reader (bp,
//! y from the page top). The oracle never runs here: the table is
//! committed evidence. Before the fix the `Tab one` entry sat at y 320.221
//! after the Table 1 caption; after it every word below is within 0.005 bp
//! and the entry precedes both floats.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\begin{document}
\\listoffigures\\listoftables
\\begin{figure}[h]
\\caption{Fig one.}
\\end{figure}
\\begin{table}[h]
\\caption{Tab one.}
\\end{table}
\\end{document}
";

/// (painted text, x bp, baseline y bp) for every run of the page.
const EXPECTED: &[(&str, f64, f64)] = &[
    ("List", 133.768, 134.765),
    ("of", 165.971, 134.765),
    ("Figures", 184.352, 134.765),
    ("1", 148.712, 156.586),
    ("Fig", 171.626, 156.586),
    ("one.", 189.196, 156.586),
    ("1", 472.497, 156.586),
    ("List", 133.768, 189.531),
    ("of", 165.971, 189.531),
    ("Tables", 184.352, 189.531),
    ("1", 148.712, 211.352),
    ("Tab", 171.626, 211.352),
    ("one.", 191.828, 211.352),
    ("1", 472.497, 211.352),
    ("Figure", 266.175, 240.078),
    ("1:", 297.610, 240.078),
    ("Fig", 309.792, 240.078),
    ("one.", 327.362, 240.078),
    ("Table", 266.880, 282.807),
    ("1:", 294.277, 282.807),
    ("Tab", 306.450, 282.807),
    ("one.", 326.652, 282.807),
    ("1", 303.133, 702.635),
];

#[test]
fn list_of_tables_entry_sits_under_its_heading() {
    if !lm_available() {
        return;
    }
    let r = render_one(SOURCE);
    assert_eq!(r.v2.pages.len(), 1);
    let words = words_of(&r);
    // Pair every reference run with a distinct painted run, nearest first.
    let mut used = vec![false; words.len()];
    let mut misses = Vec::new();
    for (text, x, y) in EXPECTED {
        let hit = words
            .iter()
            .enumerate()
            .filter(|(i, w)| !used[*i] && w.text == **text && (w.x - x).abs() <= 0.1 && (w.baseline - y).abs() <= 0.1)
            .min_by(|a, b| {
                ((a.1.x - x).abs() + (a.1.baseline - y).abs()).total_cmp(&((b.1.x - x).abs() + (b.1.baseline - y).abs()))
            })
            .map(|(i, _)| i);
        match hit {
            Some(i) => used[i] = true,
            None => misses.push(format!("{text:?} at ({x}, {y})")),
        }
    }
    assert!(misses.is_empty(), "{} of {} runs unmatched within 0.1 bp: {}", misses.len(), EXPECTED.len(), misses.join(", "));
    // The entry precedes the floats in the document, not just in paint
    // order: its baseline is above both captions'.
    let baseline = |text: &str, y: f64| {
        words.iter().find(|w| w.text == text && (w.baseline - y).abs() <= 0.1).map(|w| w.baseline).expect("pinned above")
    };
    let entry = baseline("Tab", 211.352);
    assert!(entry < baseline("Figure", 240.078) && entry < baseline("Table", 282.807), "the LOT entry must precede both floats");
}
