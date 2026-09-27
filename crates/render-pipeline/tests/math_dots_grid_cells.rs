//! Issue #955 (corpus residual `math-sheet` p2, +0.91 bp on a whole
//! `bmatrix`): amsmath's `\cdots` at the end of a grid cell that `&` ends.
//!
//! `\cdots` is `\extrap@\@cdots` (`amsmath.sty` 609-619): it looks at the
//! next token with `\futurelet`, and before `$` (or a right delimiter or
//! `,;.`) it sets `\@cdots\,`. When that next token is an alignment tab, TeX
//! first inserts the column's v-template (tex.web §342), and in `array`, the
//! matrix environments and `cases` (latex.ltx `\@arrayclassz`), in
//! `smallmatrix` (`amsmath.sty` 732) and in mathtools' `dcases`/`rcases` it
//! opens with the cell's closing `$`. So the cell is a thin space wider, and
//! the widest cell of math-sheet's `\cdots` column set the column 1.825 pt
//! wider than FlashTeX did (the display is centred: +0.91 bp left of it,
//! -0.91 bp right of it). A cell that `\\` or `\end` ends sees that macro
//! and gets no thin space (lines B and H), and `aligned` closes its cells
//! with `{##}$`, whose `}` is no follower either (line E). `\dots` takes
//! `\extra@`'s thin space there too (line G, low dots).
//!
//! Every expected number is pdfTeX 1.40.29 (TeX Live 2026, amsmath
//! 2025/07/09 v2.17z), `pdflatex -interaction=batchmode`,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on exactly [`SOURCE`], read
//! with `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the
//! page top). The oracle never runs here: the table is committed evidence.
//! Identity is the character FlashTeX paints for pdfTeX's slot: cmsy
//! `"01` (`\cdotp`) is U+22C5, cmmi `"3A` (`\ldotp`) is `.`. The `cases`
//! brace is left out. Before the fix every glyph after a `\cdots` cell sat
//! 1.661 bp (1.360 bp in `smallmatrix`, a script-style thin space) left of
//! pdfTeX's; after it every glyph is within 0.002 bp.
//!
//! Line E's trailing `X` is not pinned: an inline `aligned` box ends with
//! the `\alignsep@` tabskip after its last column in pdfTeX, which FlashTeX
//! does not set yet (X is 9.962 bp left of pdfTeX's, before and after this
//! change; a separate item).

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $\\begin{matrix} a & \\cdots & b \\end{matrix}$ X

B $\\begin{matrix} a & \\cdots \\end{matrix}$ X

C $\\begin{array}{cc} \\cdots & b \\end{array}$ X

D $\\begin{cases} \\cdots & b \\end{cases}$ X

E $\\begin{aligned} a\\cdots &= b \\end{aligned}$ X

F $\\begin{smallmatrix} \\cdots & b \\end{smallmatrix}$ X

G $\\begin{matrix} a & \\dots & b \\end{matrix}$ X

H $\\begin{matrix} a & \\cdots \\\\ c & d \\end{matrix}$ X
\\end{document}
";

/// pdfTeX's glyphs for [`SOURCE`]: (painted text, pdfTeX font, x bp,
/// baseline y bp).
const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR10", 133.768, 134.765),
    ("a", "CMMI10", 144.561, 134.665),
    ("⋅", "CMSY10", 159.79, 134.665),
    ("⋅", "CMSY10", 164.221, 134.665),
    ("⋅", "CMSY10", 168.643, 134.665),
    ("b", "CMMI10", 183.037, 134.665),
    ("X", "CMR10", 190.633, 134.765),
    ("B", "CMR10", 133.768, 146.72),
    ("a", "CMMI10", 144.146, 146.62),
    ("⋅", "CMSY10", 159.375, 146.62),
    ("⋅", "CMSY10", 163.806, 146.62),
    ("⋅", "CMSY10", 168.228, 146.62),
    ("X", "CMR10", 174.319, 146.72),
    ("C", "CMR10", 133.768, 158.675),
    ("⋅", "CMSY10", 149.266, 158.575),
    ("⋅", "CMSY10", 153.697, 158.575),
    ("⋅", "CMSY10", 158.119, 158.575),
    ("b", "CMMI10", 172.513, 158.575),
    ("X", "CMR10", 185.09, 158.675),
    ("D", "CMR10", 133.768, 174.615),
    ("⋅", "CMSY10", 151.341, 174.994),
    ("⋅", "CMSY10", 155.772, 174.994),
    ("⋅", "CMSY10", 160.194, 174.994),
    ("b", "CMMI10", 174.588, 174.994),
    ("X", "CMR10", 183.379, 174.615),
    ("E", "CMR10", 133.768, 190.556),
    ("a", "CMMI10", 143.869, 190.456),
    ("⋅", "CMSY10", 150.799, 190.456),
    ("⋅", "CMSY10", 155.22, 190.456),
    ("⋅", "CMSY10", 159.652, 190.456),
    ("=", "CMR10", 165.189, 190.456),
    ("b", "CMMI10", 175.698, 190.456),
    ("F", "CMR10", 133.768, 202.511),
    ("⋅", "CMSY7", 145.253, 202.442),
    ("⋅", "CMSY7", 147.619, 202.442),
    ("⋅", "CMSY7", 149.985, 202.442),
    ("b", "CMMI7", 156.48, 202.442),
    ("X", "CMR10", 164.963, 202.511),
    ("G", "CMR10", 133.768, 214.466),
    ("a", "CMMI10", 144.907, 214.366),
    (".", "CMMI10", 160.136, 214.366),
    (".", "CMMI10", 164.567, 214.366),
    (".", "CMMI10", 168.989, 214.366),
    ("b", "CMMI10", 183.383, 214.366),
    ("X", "CMR10", 190.978, 214.466),
    ("H", "CMR10", 133.768, 233.395),
    ("a", "CMMI10", 144.561, 227.318),
    ("⋅", "CMSY10", 159.79, 227.318),
    ("⋅", "CMSY10", 164.221, 227.318),
    ("⋅", "CMSY10", 168.643, 227.318),
    ("c", "CMMI10", 145.039, 239.273),
    ("d", "CMMI10", 163.01, 239.273),
    ("X", "CMR10", 174.734, 233.395),
];

/// A twelfth of the smallest shift the fix removes (1.360 bp).
const TOL_BP: f64 = 0.1;

#[test]
fn cdots_before_an_alignment_tab_takes_its_thin_space() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, EXPECTED, TOL_BP);
}
