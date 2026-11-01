//! Issue #955 (found in the section 3 residual sweep): amsmath `aligned` and
//! `alignedat` column gaps, and the `\alignsep@` after an `aligned`'s last
//! column.
//!
//! `\start@aligned` (`amsmath.sty` 1462-1491) ends every even column's
//! template with `\tabskip\alignsep@`. That is `\minalignsep` (a fixed
//! 10pt) for `aligned` and `\z@skip` for `alignedat`, where FlashTeX put a
//! text quad (10.95pt at 11pt) between every pair of both. The same glue
//! follows the last column when it is even. `\\` after an even column is
//! `&\kern-\alignsep@\cr` (`\math@cr@@@aligned`), an extra column that
//! cancels it. So a one-row `aligned` (line C, and the display) is 10pt
//! wider than its cells: its `X` sat 9.963 bp left of pdfTeX's, and the
//! displayed one 4.98 bp right. A second row ended by `\\` (lines D and F)
//! cancels the glue, and a row that ends after an odd column (line E) has
//! none to cancel.
//!
//! Every expected number is pdfTeX 1.40.29 (TeX Live 2026, amsmath
//! 2025/07/09 v2.17z), `pdflatex -interaction=batchmode`,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on exactly [`SOURCE`], read
//! with `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the
//! page top). The oracle never runs here: the table is committed evidence.
//! Before the fix, 22 of the 65 glyphs were off, by up to 10.913 bp
//! (`alignedat`). After it, all 65 are within 0.006 bp.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass[11pt]{article}
\\usepackage{amsmath}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $\\begin{aligned} a &= b & c &= d \\\\ e &= f & g &= h \\end{aligned}$ X

B $\\begin{alignedat}{2} a &= b & c &= d \\\\ e &= f & g &= h \\end{alignedat}$ X

C $\\begin{aligned} a &= b \\end{aligned}$ X

D $\\begin{aligned} a &= b \\\\ c &= d \\\\ \\end{aligned}$ X

E $\\begin{aligned} a &= b & c \\\\ e &= f \\end{aligned}$ X

F $\\begin{aligned} a &= b \\\\ cc &= dd \\end{aligned}$ X
\\[ x = \\begin{aligned}[t] a &= b \\end{aligned} \\]
\\end{document}
";

/// pdfTeX's glyphs for [`SOURCE`]: (painted text, pdfTeX font, x bp,
/// baseline y bp).
const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR10", 125.798, 147.554),
    ("a", "CMMI10", 137.616, 139.268),
    ("=", "CMR10", 146.415, 139.268),
    ("b", "CMMI10", 157.933, 139.268),
    ("c", "CMMI10", 175.281, 139.268),
    ("=", "CMR10", 183.035, 139.268),
    ("d", "CMMI10", 194.542, 139.268),
    ("e", "CMMI10", 138.303, 155.806),
    ("=", "CMR10", 146.415, 155.806),
    ("f", "CMMI10", 157.933, 155.806),
    ("g", "CMMI10", 174.401, 155.806),
    ("=", "CMR10", 183.03, 155.806),
    ("h", "CMMI10", 194.548, 155.806),
    ("X", "CMR10", 204.468, 147.554),
    ("B", "CMR10", 125.798, 178.638),
    ("a", "CMMI10", 137.162, 170.351),
    ("=", "CMR10", 145.961, 170.351),
    ("b", "CMMI10", 157.479, 170.351),
    ("c", "CMMI10", 164.867, 170.351),
    ("=", "CMR10", 172.61, 170.351),
    ("d", "CMMI10", 184.128, 170.351),
    ("e", "CMMI10", 137.849, 186.889),
    ("=", "CMR10", 145.961, 186.889),
    ("f", "CMMI10", 157.479, 186.889),
    ("g", "CMMI10", 163.987, 186.889),
    ("=", "CMR10", 172.616, 186.889),
    ("h", "CMMI10", 184.134, 186.889),
    ("X", "CMR10", 194.051, 178.638),
    ("C", "CMR10", 125.798, 201.452),
    ("a", "CMMI10", 137.313, 201.435),
    ("=", "CMR10", 146.112, 201.435),
    ("b", "CMMI10", 157.63, 201.435),
    ("X", "CMR10", 175.906, 201.452),
    ("D", "CMR10", 125.798, 224.266),
    ("a", "CMMI10", 137.768, 215.98),
    ("=", "CMR10", 146.567, 215.98),
    ("b", "CMMI10", 158.085, 215.98),
    ("c", "CMMI10", 138.813, 232.518),
    ("=", "CMR10", 146.567, 232.518),
    ("d", "CMMI10", 158.085, 232.518),
    ("X", "CMR10", 167.394, 224.266),
    ("E", "CMR10", 125.798, 255.35),
    ("a", "CMMI10", 136.859, 247.063),
    ("=", "CMR10", 145.658, 247.063),
    ("b", "CMMI10", 157.176, 247.063),
    ("c", "CMMI10", 173.651, 247.063),
    ("e", "CMMI10", 137.546, 263.601),
    ("=", "CMR10", 145.658, 263.601),
    ("f", "CMMI10", 157.176, 263.601),
    ("X", "CMR10", 182.006, 255.35),
    ("F", "CMR10", 125.798, 286.433),
    ("a", "CMMI10", 140.231, 278.147),
    ("=", "CMR10", 149.03, 278.147),
    ("b", "CMMI10", 160.548, 278.147),
    ("c", "CMMI10", 136.556, 294.685),
    ("c", "CMMI10", 141.277, 294.685),
    ("=", "CMR10", 149.032, 294.685),
    ("d", "CMMI10", 160.539, 294.685),
    ("d", "CMMI10", 166.217, 294.685),
    ("X", "CMR10", 175.536, 286.433),
    ("x", "CMMI10", 277.258, 309.23),
    ("=", "CMR10", 286.525, 309.23),
    ("a", "CMMI10", 298.043, 309.23),
    ("=", "CMR10", 306.831, 309.23),
    ("b", "CMMI10", 318.349, 309.23),
];

/// A tenth of the smallest shift the fix removes (0.943 bp, the 11pt quad
/// against `\minalignsep`).
const TOL_BP: f64 = 0.09;

#[test]
fn aligned_and_alignedat_gaps_match_pdftex() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, EXPECTED, TOL_BP);
}
