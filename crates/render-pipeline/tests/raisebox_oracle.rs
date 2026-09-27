//! Kernel text-mode `\raisebox{<lift>}[<height>][<depth>]{...}` (compiler
//! `Inline::RaiseBox`, #1028) against pdflatex. latex.ltx `\@iirsbox` sets
//! the argument as `\hbox{{#4}}`, raises it by the lift inside a new
//! `\hbox` (height `max(0, ht + lift)`, depth `max(0, dp - lift)`), then sets
//! `\ht`/`\dp` to the optional arguments. The paragraphs below measure the
//! raise itself (`pt`, `ex`, `em`, negative, at `\large`, with font changes
//! inside) and the official extents through interline glue: a 12pt lift
//! pushes its line down by `\lineskip` rules, `[0pt][0pt]` does not, a 12pt
//! drop pushes the next line, and `[20pt]` replaces the height.
//!
//! Expected values are the word origins of pdflatex's own PDF of `DOC`
//! (MacTeX 2026, `SOURCE_DATE_EPOCH=0`), in bp from the page top, read with
//! `tools/visual-oracle/pdftext.py`.
#![cfg(feature = "compiler-raisebox")]

mod common;

use common::*;
use flashtex_render_pipeline::display::{GlyphRun, Item};

const TOL_BP: f64 = 0.01;

const DOC: &str = r"\documentclass{article}
\begin{document}
First line of a paragraph with a raised \raisebox{2pt}{word} and a lowered \raisebox{-3pt}{down} one, plus \raisebox{0.5ex}{halfex} and \raisebox{1em}{onem} too.

Some filler text that goes on for a while so that the paragraph wraps onto more lines here and there \raisebox{12pt}{TALL} more filler words to continue the paragraph so we see the next line after.

Some filler text that goes on for a while so that the paragraph wraps onto more lines here and there \raisebox{12pt}[0pt][0pt]{ZERO} more filler words to continue the paragraph so we see the next line after.

Some filler text that goes on for a while so that the paragraph wraps onto more lines here and there \raisebox{-12pt}{DEEP} more filler words to continue the paragraph so we see the next line after.

Some filler text that goes on for a while so that the paragraph wraps onto more lines here and there \raisebox{-12pt}[20pt]{HIGH} more filler words to continue the paragraph so we see the next line after.

{\large Large text with \raisebox{1ex}{bigex} inside it and more words.}

End \raisebox{2pt}{\textbf{bold} and \emph{it}} words.
\end{document}
";

/// `(run text, occurrence, origin x, baseline y)` from pdflatex, in bp.
const EXPECT: &[(&str, usize, f64, f64)] = &[
    ("with", 0, 260.911, 134.765),
    ("word", 0, 321.834, 132.772),
    ("down", 0, 411.591, 137.753),
    ("halfex", 0, 133.768, 150.857),
    ("onem", 0, 182.475, 143.039),
    ("Some", 0, 148.712, 164.957),
    ("more", 0, 133.768, 186.653),
    ("TALL", 0, 247.136, 174.698),
    ("more", 1, 276.590, 186.653),
    ("after.", 0, 222.933, 198.609),
    ("Some", 1, 148.712, 210.564),
    ("more", 2, 133.768, 222.519),
    ("ZERO", 0, 246.604, 210.564),
    ("after.", 1, 222.933, 234.474),
    ("more", 4, 133.768, 258.384),
    ("DEEP", 0, 246.497, 270.340),
    ("after.", 2, 222.933, 278.254),
    ("Some", 3, 148.712, 290.210),
    ("more", 6, 133.768, 313.068),
    ("HIGH", 0, 247.109, 325.023),
    ("after.", 3, 222.933, 332.938),
    ("Large", 0, 148.712, 347.384),
    ("bigex", 0, 232.440, 342.237),
    ("End", 0, 148.712, 359.339),
    ("bold", 0, 169.883, 357.347),
];

#[test]
fn raisebox_lift_and_official_extents_match_pdflatex() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(DOC);
    assert_eq!(rendered.v2.pages.len(), 1);
    let runs: Vec<&GlyphRun> = rendered.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run) if !run.glyphs.is_empty() => Some(run),
            _ => None,
        })
        .collect();
    let summary: Vec<String> = runs
        .iter()
        .map(|r| format!("{:?}@({:.3},{:.3})", r.text, r.glyphs[0].origin_x.to_bp(), r.glyphs[0].baseline_y.to_bp()))
        .collect();
    let mut bad = Vec::new();
    for &(text, nth, x, y) in EXPECT {
        let run = runs
            .iter()
            .filter(|r| r.text == text)
            .nth(nth)
            .unwrap_or_else(|| panic!("{text:?} #{nth} missing: {summary:?}"));
        let (rx, ry) = (run.glyphs[0].origin_x.to_bp(), run.glyphs[0].baseline_y.to_bp());
        if (rx - x).abs() > TOL_BP || (ry - y).abs() > TOL_BP {
            bad.push(format!("{text:?} #{nth}: got ({rx:.3}, {ry:.3}) want ({x:.3}, {y:.3})"));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}
