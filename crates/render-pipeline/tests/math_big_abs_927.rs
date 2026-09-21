//! Issue #927: `\bigl| |u| - |v| \bigr|` (fixtures/real-world/inline-math
//! p.2) measured ~6.8 bp narrower than pdfTeX and flipped a line break.
//!
//! On main at 892a7b109 the formula is already exact -- the delimiters,
//! the inner `|`/`\|` Ords and the Open/Close spacing all land within
//! 0.005 bp of pdfTeX, and `tools/visual-oracle/rank.py` reports 0
//! reflowed words on inline-math p.2 -- so this pins it: every glyph's
//! identity and origin within 0.5 bp under the fixture's own preamble
//! (11 pt, T1, lmodern, amsmath), with a trailing `X` after each formula
//! pinning its width. `\Bigl|`/`\Bigr|` are pinned by one piece each:
//! pdfTeX stacks three lmex10 `"0C` pieces there, Latin Modern Math's
//! assembly paints two, so the piece count is not compared here.
//!
//! Expected numbers are pdfTeX 1.40.29 (TeX Live 2026),
//! `pdflatex -interaction=batchmode`, two passes, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, on exactly [`SOURCE`], read with
//! `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the page
//! top). Identity is the character FlashTeX paints for pdfTeX's slot:
//! lmsy10 `"6A`/`"6B` are `|`/`‖`, `"00` is U+2212, `"14` is `≤`; the
//! lmex10 `"0C` pieces are `|`, pinned in x only (cmex outlines hang from
//! their top, the OpenType parts sit on a baseline). The oracle never runs
//! here: the table is committed evidence.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass[11pt]{article}
\\usepackage[T1]{fontenc}
\\usepackage{lmodern}
\\usepackage[margin=1in]{geometry}
\\usepackage{amsmath,amssymb}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $\\bigl| |u| - |v| \\bigr| \\le |u - v|$ X

B $\\bigl| \\|u\\| - \\|v\\| \\bigr|$ X

C $\\Bigl| |u| - |v| \\Bigr|$ X
\\end{document}
";

const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "LMRoman10-Regular", 72.0, 82.959),
    ("|", "LMMathExtension10-Regular", 83.818, 0.0),
    ("|", "LMMathExtension10-Regular", 83.818, 0.0),
    ("|", "LMMathSymbols10-Regular", 87.139, 82.959),
    ("u", "LMMathItalic10-Regular", 90.17, 82.959),
    ("|", "LMMathSymbols10-Regular", 96.415, 82.959),
    ("−", "LMMathSymbols10-Regular", 101.867, 82.959),
    ("|", "LMMathSymbols10-Regular", 112.774, 82.959),
    ("v", "LMMathItalic10-Regular", 115.805, 82.959),
    ("|", "LMMathSymbols10-Regular", 121.485, 82.959),
    ("|", "LMMathExtension10-Regular", 124.518, 0.0),
    ("|", "LMMathExtension10-Regular", 124.518, 0.0),
    ("≤", "LMMathSymbols10-Regular", 130.869, 82.959),
    ("|", "LMMathSymbols10-Regular", 142.387, 82.959),
    ("u", "LMMathItalic10-Regular", 145.417, 82.959),
    ("−", "LMMathSymbols10-Regular", 154.085, 82.959),
    ("v", "LMMathItalic10-Regular", 164.992, 82.959),
    ("|", "LMMathSymbols10-Regular", 170.672, 82.959),
    ("X", "LMRoman10-Regular", 177.335, 82.959),
    ("B", "LMRoman10-Regular", 72.0, 96.508),
    ("|", "LMMathExtension10-Regular", 83.363, 0.0),
    ("|", "LMMathExtension10-Regular", 83.363, 0.0),
    ("‖", "LMMathSymbols10-Regular", 86.684, 96.508),
    ("u", "LMMathItalic10-Regular", 92.139, 96.508),
    ("‖", "LMMathSymbols10-Regular", 98.384, 96.508),
    ("−", "LMMathSymbols10-Regular", 106.26, 96.508),
    ("‖", "LMMathSymbols10-Regular", 117.167, 96.508),
    ("v", "LMMathItalic10-Regular", 122.622, 96.508),
    ("‖", "LMMathSymbols10-Regular", 128.302, 96.508),
    ("|", "LMMathExtension10-Regular", 133.76, 0.0),
    ("|", "LMMathExtension10-Regular", 133.76, 0.0),
    ("X", "LMRoman10-Regular", 140.717, 96.508),
    ("C", "LMRoman10-Regular", 72.0, 113.868),
    ("|", "LMMathExtension10-Regular", 83.515, 0.0),
    ("|", "LMMathSymbols10-Regular", 86.836, 113.868),
    ("u", "LMMathItalic10-Regular", 89.867, 113.868),
    ("|", "LMMathSymbols10-Regular", 96.112, 113.868),
    ("−", "LMMathSymbols10-Regular", 101.564, 113.868),
    ("|", "LMMathSymbols10-Regular", 112.471, 113.868),
    ("v", "LMMathItalic10-Regular", 115.502, 113.868),
    ("|", "LMMathSymbols10-Regular", 121.182, 113.868),
    ("|", "LMMathExtension10-Regular", 124.215, 0.0),
    ("X", "LMRoman10-Regular", 131.172, 113.868),
];

#[test]
fn big_abs_formula_matches_pdftex_glyph_by_glyph() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, EXPECTED, 0.5);
}
