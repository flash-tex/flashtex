//! A binary operator next to explicit glue keeps its Bin spacing.
//!
//! The pipeline cuts an inline formula at its top-level glue (`\,`, `\;`,
//! `\quad`: `split_at_spaces`) and lays the runs out separately, joined by
//! the kern and the inter-atom glue TeX puts across it. The join was
//! classified over the whole formula, but each run's own layout applied
//! Rules 5 and 6 at the run's edges as if they were the formula's: a `+`
//! opening a run (`\dots\,+b`, `a\,+b`) became Ord and lost the
//! `\medmuskip` before the `b`, and one closing a run (`a+\,b`,
//! `a\cdot\quad b`) lost the one after the `a` -- 2.21 bp each. TeX
//! classifies the formula at once and glue is not a noad, so the `+` is Bin
//! on both sides; the runs are now laid out in the context of the noads
//! across the glue (`ml::layout_in_context`, `run_neighbours`).
//!
//! Expected numbers are pdfTeX 1.40.29 (TeX Live 2026, amsmath 2025/07/09
//! v2.17z), `pdflatex -interaction=batchmode`, two passes,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on exactly [`SOURCE`], read
//! with `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the
//! page top). Identity is the character FlashTeX paints for pdfTeX's slot
//! (cmmi10 `"3A` is `.`, cmsy10 `"01` is U+22C5). The trailing `X` of each
//! line pins the formula's width. The oracle never runs here: the table is
//! committed evidence.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $a\\dots\\,+b$ X

B $a\\dots\\;+b$ X

C $a+\\,b$ X

D $a\\,+b$ X

E $a\\,+\\,b$ X

F $a\\cdot\\quad b$ X

G $a\\dots\\,= b$ X

H $(\\,+b)$ X
\\end{document}
";

const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR10", 133.768, 134.765),
    ("a", "CMMI10", 144.557, 134.765),
    (".", "CMMI10", 151.487, 134.765),
    (".", "CMMI10", 155.919, 134.765),
    (".", "CMMI10", 160.34, 134.765),
    ("+", "CMR10", 166.983, 134.765),
    ("b", "CMMI10", 176.944, 134.765),
    ("X", "CMR10", 184.547, 134.765),
    ("B", "CMR10", 133.768, 146.72),
    ("a", "CMMI10", 144.142, 146.72),
    (".", "CMMI10", 151.072, 146.72),
    (".", "CMMI10", 155.503, 146.72),
    (".", "CMMI10", 159.925, 146.72),
    ("+", "CMR10", 167.674, 146.72),
    ("b", "CMMI10", 177.644, 146.72),
    ("X", "CMR10", 185.238, 146.72),
    ("C", "CMR10", 133.768, 158.675),
    ("a", "CMMI10", 144.281, 158.675),
    ("+", "CMR10", 151.768, 158.675),
    ("b", "CMMI10", 163.383, 158.675),
    ("X", "CMR10", 170.986, 158.675),
    ("D", "CMR10", 133.768, 170.63),
    ("a", "CMMI10", 144.696, 170.63),
    ("+", "CMR10", 153.838, 170.63),
    ("b", "CMMI10", 163.798, 170.63),
    ("X", "CMR10", 171.402, 170.63),
    ("E", "CMR10", 133.768, 182.585),
    ("a", "CMMI10", 143.866, 182.585),
    ("+", "CMR10", 153.008, 182.585),
    ("b", "CMMI10", 164.632, 182.585),
    ("X", "CMR10", 172.226, 182.585),
    ("F", "CMR10", 133.768, 194.541),
    ("a", "CMMI10", 143.589, 194.541),
    ("⋅", "CMSY10", 151.077, 194.541),
    ("b", "CMMI10", 166.019, 194.541),
    ("X", "CMR10", 173.612, 194.541),
    ("G", "CMR10", 133.768, 206.496),
    ("a", "CMMI10", 144.903, 206.496),
    (".", "CMMI10", 151.833, 206.496),
    (".", "CMMI10", 156.265, 206.496),
    (".", "CMMI10", 160.686, 206.496),
    ("=", "CMR10", 167.887, 206.496),
    ("b", "CMMI10", 178.405, 206.496),
    ("X", "CMR10", 185.999, 206.496),
    ("H", "CMR10", 133.768, 218.451),
    ("(", "CMR10", 144.557, 218.451),
    ("+", "CMR10", 150.096, 218.451),
    ("b", "CMMI10", 157.845, 218.451),
    (")", "CMR10", 162.121, 218.451),
    ("X", "CMR10", 169.313, 218.451),
];

#[test]
fn a_bin_beside_explicit_glue_is_spaced_as_in_pdftex() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, EXPECTED, 0.5);
}
