//! List labels inside an italic theorem body set italic, not upright.
//!
//! `\@item` boxes `\makelabel{<label>}` in the ambient font, so inside an
//! amsthm `plain` theorem body (italic by the package's default style) the
//! counter, template and explicit labels inherit `\itshape`. The pipeline
//! always set them upright: `label_box` shaped plain-text labels in the
//! default style and `label_box_items` ran explicit content under an
//! upright base. Both now take the ambient italic from the item's own
//! anchor (never for `description`, whose `\descriptionlabel` is
//! `\normalfont`, nor beamer's own template).
//!
//! Every expected number is pdflatex (TeX Live 2026), article 10pt,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 pdflatex
//! -interaction=batchmode` on exactly [`SOURCE`], read per glyph with the
//! page-frame oracle's PDF reader (bp, y from the page top) and grouped
//! into runs. The oracle never runs here: the table is committed evidence.
//! The `~` after the head ends its paragraph, so the lists break onto
//! their own lines exactly as here. Before the fix the `1.` labels sat at
//! x 145.945 (LMRoman10-Regular) and the `(ii)` at 140.410; after it every
//! run below is within 0.005 bp.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\usepackage{amsthm}
\\usepackage{enumitem}
\\newtheorem{theorem}{Theorem}
\\begin{document}
\\begin{theorem}~
\\begin{enumerate}
\\item First item text here.
\\item Second item text here.
\\end{enumerate}
\\begin{enumerate}[label=(\\roman*)]
\\item Third item text here.
\\item[(ii)] Explicit item text here.
\\end{enumerate}
\\end{theorem}
\\end{document}
";

/// (painted run, x bp, baseline y bp) for every run of the page.
const EXPECTED: &[(&str, f64, f64)] = &[
    ("Theorem", 133.768, 134.765),
    ("1.", 182.415, 134.765),
    ("1.", 145.546, 154.690),
    ("First", 158.675, 154.690),
    ("item", 183.385, 154.690),
    ("text", 206.037, 154.690),
    ("here.", 225.427, 154.690),
    ("2.", 145.546, 174.615),
    ("Second", 158.675, 174.615),
    ("item", 191.776, 174.615),
    ("text", 214.428, 174.615),
    ("here.", 233.819, 174.615),
    ("(i)", 142.491, 196.533),
    ("Third", 158.675, 196.533),
    ("item", 186.304, 196.533),
    ("text", 208.956, 196.533),
    ("here.", 228.346, 196.533),
    ("(ii)", 139.436, 216.458),
    ("Explicit", 158.676, 216.458),
    ("item", 195.263, 216.458),
    ("text", 217.915, 216.458),
    ("here.", 237.306, 216.458),
    ("1", 303.133, 702.635),
];

#[test]
fn theorem_list_labels_inherit_the_italic_body() {
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
            .filter(|(i, w)| !used[*i] && w.text == **text && (w.x - x).abs() <= 0.05 && (w.baseline - y).abs() <= 0.05)
            .min_by(|a, b| {
                ((a.1.x - x).abs() + (a.1.baseline - y).abs()).total_cmp(&((b.1.x - x).abs() + (b.1.baseline - y).abs()))
            })
            .map(|(i, _)| i);
        match hit {
            Some(i) => used[i] = true,
            None => misses.push(format!("{text:?} at ({x}, {y})")),
        }
    }
    assert!(misses.is_empty(), "{} of {} runs unmatched within 0.05 bp: {}", misses.len(), EXPECTED.len(), misses.join(", "));
}
