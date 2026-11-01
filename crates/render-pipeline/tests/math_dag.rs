//! `\dag`/`\ddag` in math: latex.ltx's `{\dagger}`/`{\ddagger}`.
//!
//! `\DeclareRobustCommand{\dag}{\ifmmode{\dagger}\else\textdagger\fi}`:
//! in math the cmsy mark is braced, which makes it an ordinary atom (TeX
//! §1186 does not unpack the Bin inside), so `$a\dag b$` sets no space
//! where `$a\dagger b$` sets `\medmuskip` on each side. The compiler
//! rejected `\dag`/`\ddag` in math ("\dag is a text command"), dropping
//! the mark.
//! Line C pins the text forms (TS1 `\textdagger`) beside them.
//!
//! Expected numbers are pdfTeX 1.40.29 (TeX Live 2026),
//! `pdflatex -interaction=batchmode`, two passes, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, on exactly [`SOURCE`], read with
//! `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the page
//! top). Identity is the character FlashTeX paints for pdfTeX's slot:
//! cmsy `"79`/`"7A` and TS1's `\textdagger`/`\textdaggerdbl` are
//! U+2020/U+2021. The oracle never
//! runs here: the table is committed evidence.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $a\\dag b$, $a\\ddag b$, $a+\\dag$, $\\dag x$, $x^{\\dag}$, $A^\\dag$ X

B $a\\dagger b$, $a\\ddagger b$ X

C text \\dag{} and \\ddag{} X
\\end{document}
";

const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR10", 133.768, 134.765),
    ("a", "CMMI10", 144.557, 134.765),
    ("†", "CMSY10", 149.824, 134.765),
    ("b", "CMMI10", 154.251, 134.765),
    (",", "CMR10", 158.527, 134.765),
    ("a", "CMMI10", 164.622, 134.765),
    ("‡", "CMSY10", 169.888, 134.765),
    ("b", "CMMI10", 174.316, 134.765),
    (",", "CMR10", 178.592, 134.765),
    ("a", "CMMI10", 184.677, 134.765),
    ("+", "CMR10", 192.155, 134.765),
    ("†", "CMSY10", 202.115, 134.765),
    (",", "CMR10", 206.543, 134.765),
    ("†", "CMSY10", 212.638, 134.765),
    ("x", "CMMI10", 217.065, 134.765),
    (",", "CMR10", 222.759, 134.765),
    ("x", "CMMI10", 228.844, 134.765),
    ("†", "CMSY7", 234.54, 131.149),
    (",", "CMR10", 238.691, 134.765),
    ("A", "CMMI10", 244.776, 134.765),
    ("†", "CMSY7", 252.251, 131.149),
    ("X", "CMR10", 259.723, 134.765),
    ("B", "CMR10", 133.768, 146.72),
    ("a", "CMMI10", 144.142, 146.72),
    ("†", "CMSY10", 151.63, 146.72),
    ("b", "CMMI10", 158.269, 146.72),
    (",", "CMR10", 162.545, 146.72),
    ("a", "CMMI10", 168.63, 146.72),
    ("‡", "CMSY10", 176.108, 146.72),
    ("b", "CMMI10", 182.757, 146.72),
    ("X", "CMR10", 190.351, 146.72),
    ("C", "CMR10", 133.768, 158.675),
    ("t", "CMR10", 144.281, 158.675),
    ("e", "CMR10", 148.155, 158.675),
    ("x", "CMR10", 152.582, 158.675),
    ("t", "CMR10", 157.841, 158.675),
    ("†", "SFRM1000", 165.043, 158.675),
    ("a", "CMR10", 172.787, 158.675),
    ("n", "CMR10", 177.768, 158.675),
    ("d", "CMR10", 183.303, 158.675),
    ("‡", "SFRM1000", 192.156, 158.675),
    ("X", "CMR10", 199.91, 158.675),
];

#[test]
fn dag_in_math_is_an_ordinary_dagger() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, EXPECTED, 0.5);
}
