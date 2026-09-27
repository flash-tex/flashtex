//! `flalign` with exactly one column pair is centred like `align`.
//!
//! amsmath's `\measure@` (amsmath.sty 1925-1929) resets a one-pair
//! `flalign` (`\maxfields@<\thr@@`) to level 0 (`\let\xatlevel@\z@`), so
//! the row is centred; only two or more pairs spread flush-left /
//! flush-right. FlashTeX pushed every `flalign` flush left, putting the
//! one-pair `a` at x 133.768 instead of 294.211.
//!
//! Every expected number is pdflatex (TeX Live 2026), article 10pt,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 pdflatex
//! -interaction=batchmode` on exactly [`SOURCE`], read with the
//! page-frame oracle's PDF reader (bp, y from the page top). The oracle
//! never runs here: the table is committed evidence. Before the fix the
//! one-pair row sat flush left (`a` at 133.768); after it every glyph is
//! within 0.005 bp.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\begin{document}
\\begin{flalign}
a &= b
\\end{flalign}
\\begin{flalign}
a &= b & c &= d
\\end{flalign}
\\end{document}
";

/// pdfTeX's glyphs for [`SOURCE`]: (painted text, pdfTeX font, x bp,
/// baseline y bp).
const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("a", "CMMI10", 294.211, 156.682),
    ("=", "CMR10", 302.247, 156.682),
    ("b", "CMMI10", 312.765, 156.682),
    ("(", "CMR10", 464.747, 156.682),
    ("1", "CMR10", 468.621, 156.682),
    (")", "CMR10", 473.603, 156.682),
    ("a", "CMMI10", 133.768, 188.563),
    ("=", "CMR10", 141.804, 188.563),
    ("b", "CMMI10", 152.322, 188.563),
    ("c", "CMMI10", 436.986, 188.563),
    ("=", "CMR10", 444.067, 188.563),
    ("d", "CMMI10", 454.586, 188.563),
    ("(", "CMR10", 464.753, 188.563),
    ("2", "CMR10", 468.627, 188.563),
    (")", "CMR10", 473.608, 188.563),
    ("1", "CMR10", 303.133, 702.635),
];

#[test]
fn single_pair_flalign_is_centred_like_align() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, EXPECTED, 0.1);
}
