//! Math under a size declaration is set with that size's math fonts.
//!
//! LaTeX's `$` runs `\check@mathfonts`, which loads the math fonts
//! `\DeclareMathSizes` gives the current text size, so `{\small $x$}` in a
//! 12 pt document is cmmi10 at 10.95 pt with cmr8/cmr6 scripts, not the
//! body's cmmi12. The compiler's `Inline::Math` carried no text size, so
//! every inline formula took the paragraph's (12 pt here), and
//! `TexMathMetrics::at_text_size` had no rows above 12 pt, so `{\large
//! $x$}` and a `\section` title's formula were set at 12 pt too.
//!
//! Expected numbers are pdfTeX 1.40.29 (TeX Live 2026),
//! `pdflatex -interaction=batchmode`, two passes, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, on exactly [`SOURCE`], read with
//! `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the page
//! top). The fonts in the table are pdfTeX's, for the reader: each row's
//! size is pinned by the position of the glyph after it. `\sum` (cmex10,
//! hung from its top) is pinned in x only. The oracle never runs here: the
//! table is committed evidence.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass[12pt]{article}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
\\section{Heading $x_i^2 + \\alpha$}
A {\\small B $x+y^2$ C} D

{\\footnotesize E $x_1+\\alpha$ F} G

{\\large H $x+\\sum_i y$ I} J

\\begin{small}
K $a=\\frac{1}{2}$ L
\\end{small}

M $x+y$ N {\\scriptsize $u_v$} O
\\end{document}
";

const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("1", "CMBX12", 110.854, 139.75),
    ("H", "CMBX12", 139.905, 139.75),
    ("e", "CMBX12", 155.048, 139.75),
    ("a", "CMBX12", 163.884, 139.75),
    ("d", "CMBX12", 173.299, 139.75),
    ("i", "CMBX12", 184.059, 139.75),
    ("n", "CMBX12", 189.439, 139.75),
    ("g", "CMBX12", 200.198, 139.75),
    ("x", "CMMI12", 216.338, 139.75),
    ("2", "CMR12", 225.918, 133.503),
    ("i", "CMMI12", 225.918, 144.006),
    ("+", "CMR17", 236.094, 139.75),
    ("α", "CMMI12", 252.307, 139.75),
    ("A", "CMR12", 110.854, 166.035),
    ("B", "CMR10", 123.535, 166.035),
    ("x", "CMMI10", 134.894, 166.035),
    ("+", "CMR10", 143.551, 166.035),
    ("y", "CMMI10", 154.468, 166.035),
    ("2", "CMR8", 160.203, 162.076),
    ("C", "CMR10", 168.572, 166.035),
    ("D", "CMR12", 180.348, 166.035),
    ("E", "CMR10", 110.854, 180.481),
    ("x", "CMMI10", 120.952, 180.481),
    ("1", "CMR7", 126.649, 181.975),
    ("+", "CMR10", 133.332, 180.481),
    ("α", "CMMI10", 143.293, 180.481),
    ("F", "CMR10", 153.023, 180.481),
    ("G", "CMR12", 163.436, 180.481),
    ("H", "CMR12", 110.854, 194.926),
    ("x", "CMMI12", 126.061, 194.926),
    ("+", "CMR12", 137.242, 194.926),
    ("∑", "CMEX10", 151.351, 0.0),
    ("i", "CMMI10", 161.867, 197.078),
    ("y", "CMMI12", 168.189, 194.926),
    ("I", "CMR12", 180.231, 194.926),
    ("J", "CMR12", 189.208, 194.926),
    ("K", "CMR10", 110.854, 209.372),
    ("a", "CMMI10", 122.972, 209.372),
    ("=", "CMR10", 131.771, 209.372),
    ("1", "CMR8", 144.483, 205.077),
    ("2", "CMR8", 144.483, 213.134),
    ("L", "CMR10", 153.549, 209.372),
    ("M", "CMR12", 110.854, 223.818),
    ("x", "CMMI12", 125.478, 223.818),
    ("+", "CMR12", 134.795, 223.818),
    ("y", "CMMI12", 146.555, 223.818),
    ("N", "CMR12", 156.59, 223.818),
    ("u", "CMMI8", 169.27, 223.818),
    ("v", "CMMI6", 174.172, 224.814),
    ("O", "CMR12", 182.522, 223.818),
];

#[test]
fn math_follows_the_declared_text_size() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, EXPECTED, 0.5);
}
