//! Box-relative lengths in kernel `\raisebox` (compiler
//! `TextDimen::parse_in_box`/`resolve_in_box`) against pdflatex. latex.ltx
//! `\@begin@tempboxa` sets the argument first and defines `\width`,
//! `\height`, `\depth` as its natural extents and `\totalheight` as their
//! height plus depth, so `\raisebox{-.5\height}{...}` centres the box on
//! the baseline, `\raisebox{\depth}{g}` sits a descender on it, and the
//! optional `[<height>][<depth>]` may read the unraised extents too. The
//! paragraphs cover `\depth`, `-0.5\height`, `-\height`, `.5\totalheight`,
//! `0.1\width`, box units in both optional arguments, a 1cm square rule
//! centred like an `\includegraphics` (its depth pushes the next line), a
//! rule with depth lowered by half its total height, and all of it under
//! `\large`.
//!
//! Expected values are the word origins of pdflatex's own PDF of `DOC`
//! (MacTeX 2026, `SOURCE_DATE_EPOCH=0`), in bp from the page top, read with
//! `tools/visual-oracle/pdftext.py`.
#![cfg(all(feature = "compiler-raisebox", feature = "compiler-box-dimens"))]

mod common;

use common::*;
use flashtex_render_pipeline::display::{GlyphRun, Item};

const TOL_BP: f64 = 0.01;

const DOC: &str = r"\documentclass{article}
\begin{document}
Alpha \raisebox{\depth}{gyp} Bravo \raisebox{-0.5\height}{Xray} Charlie \raisebox{-\height}{Yank} Delta \raisebox{.5\totalheight}{gyq} Echo.

Foxtrot \raisebox{0pt}[\height][0pt]{jog} Golf \raisebox{0pt}[0pt][\depth]{Hop} Hotel \raisebox{0.1\width}{Wide} India \raisebox{-.5\height}[.5\height][.5\height]{Jag} Juliet.

Some filler text that goes on for a while so that the paragraph wraps onto more lines here and \raisebox{-.5\height}{\rule{1cm}{1cm}} kilo words continue the paragraph so we see the next line after it.

Some filler text that goes on for a while so that the paragraph wraps onto more lines here and \raisebox{-.5\totalheight}{\rule[-4pt]{12pt}{30pt}} lima words continue the paragraph so we see the next line after it.

{\large Large \raisebox{-.5\height}{Victor} whiskey \raisebox{\depth}{gypsy} end.}
\end{document}
";

/// `(run text, occurrence, origin x, baseline y)` from pdflatex, in bp.
const EXPECT: &[(&str, usize, f64, f64)] = &[
    ("Alpha", 0, 148.712, 134.765),
    ("gyp", 0, 178.324, 132.827),
    ("Bravo", 0, 197.419, 134.765),
    ("Xray", 0, 226.366, 138.169),
    ("Charlie", 0, 251.023, 134.765),
    ("Yank", 0, 285.920, 141.683),
    ("Delta", 0, 311.657, 134.765),
    ("gyq", 0, 338.639, 131.651),
    ("Echo.", 0, 357.458, 134.765),
    ("Foxtrot", 0, 148.712, 151.895),
    ("jog", 0, 184.299, 151.895),
    ("Golf", 0, 200.624, 151.895),
    ("Hop", 0, 222.563, 151.895),
    ("Hotel", 0, 243.869, 151.895),
    ("Wide", 0, 270.713, 149.598),
    ("India", 0, 297.004, 151.895),
    ("Jag", 0, 322.741, 155.299),
    ("Juliet.", 0, 341.144, 151.895),
    ("more", 0, 133.768, 180.957),
    ("kilo", 0, 251.165, 180.957),
    ("line", 0, 133.768, 203.045),
    ("more", 1, 133.768, 228.892),
    ("lima", 0, 239.540, 228.892),
    ("line", 1, 133.768, 255.736),
    ("Large", 0, 148.712, 267.691),
    ("Victor", 0, 181.389, 271.776),
    ("whiskey", 0, 217.479, 267.691),
    ("gypsy", 0, 261.441, 265.367),
    ("end.", 0, 294.673, 267.691),
];

#[test]
fn raisebox_box_relative_lengths_match_pdflatex() {
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
