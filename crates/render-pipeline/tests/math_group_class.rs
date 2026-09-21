//! Braced groups and `\bmod` in math take TeX's classes.
//!
//! TeX makes every `{...}` in a formula an ordinary atom (§1186; a group
//! holding a single ordinary atom is unpacked to it). The compiler
//! flattened every group into the surrounding list, which is the same only
//! when the group holds ordinary atoms: `a{=}b` and `a{+}b` spaced the
//! operator as a relation or binary (pdfTeX sets no space), `${}+1$` and
//! `${}={}$` lost the space against the empty group, `\mathbin{R}{=}b`
//! demoted the `R` before the (flattened) relation, `${}^{14}C$` was an
//! error ("script marker has no preceding math atom"), `${a+b}^2$` set the
//! `2` on the `b`, and `${\sum}_i x$` kept the operator's thin space. A
//! group whose atoms are not all ordinary now stays one `Group` atom.
//!
//! `\bmod` is `\mathbin{\operator@font mod}` between two `\mkern5mu`
//! and `\mskip-\medmuskip`s; the pipeline built its `mod` as an ordinary
//! text run, so every `\bmod` was 2.2 bp short on each side. The compiler
//! now states the class on the atom and the pipeline applies it.
//!
//! Row A pins `\mathbin{R}` itself (already right on main): Bin between
//! ordinary atoms, Ord at the start, at the end, after a relation and after
//! an opening delimiter (Rules 5 and 6).
//!
//! Expected numbers are pdfTeX 1.40.29 (TeX Live 2026, amsmath 2025/07/09
//! v2.17z), `pdflatex -interaction=batchmode`, two passes,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on exactly [`SOURCE`], read
//! with `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the
//! page top). Identity is the character FlashTeX paints for pdfTeX's slot
//! (cmmi10 `"3A` is `.`); the cmex `\sum` is pinned in x only. The oracle
//! never runs here: the table is committed evidence.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $a\\mathbin{R}b$, $\\mathbin{R}b$, $a\\mathbin{R}$, $a=\\mathbin{R}b$, $(\\mathbin{R}b)$ X

B $a\\mathbin{R}{=}b$, $a\\mathbin{R}=b$, $a\\mathbin{R}\\mathbin{R}b$, $a\\mathbin{:=}b$ X

C $a{=}b$, $a{+}b$, ${}+1$, ${}={}$, $a{}+b$ X

D ${}^{14}C$, ${}_nC_k$, ${a+b}^2$, ${\\sum}_i x$, $a{1\\over2}b$ X

E $a\\bmod b$, $(a)\\bmod b$, $a\\dots\\bmod b$, $a\\bmod{}b$ X
\\end{document}
";

const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR10", 133.768, 134.765),
    ("a", "CMMI10", 144.557, 134.765),
    ("R", "CMMI10", 152.045, 134.765),
    ("b", "CMMI10", 161.901, 134.765),
    (",", "CMR10", 166.167, 134.765),
    ("R", "CMMI10", 172.262, 134.765),
    ("b", "CMMI10", 179.907, 134.765),
    (",", "CMR10", 184.173, 134.765),
    ("a", "CMMI10", 190.268, 134.765),
    ("R", "CMMI10", 195.534, 134.765),
    (",", "CMR10", 203.178, 134.765),
    ("a", "CMMI10", 209.264, 134.765),
    ("=", "CMR10", 217.299, 134.765),
    ("R", "CMMI10", 227.808, 134.765),
    ("b", "CMMI10", 235.452, 134.765),
    (",", "CMR10", 239.728, 134.765),
    ("(", "CMR10", 245.813, 134.765),
    ("R", "CMMI10", 249.688, 134.765),
    ("b", "CMMI10", 257.332, 134.765),
    (")", "CMR10", 261.608, 134.765),
    ("X", "CMR10", 268.8, 134.765),
    ("B", "CMR10", 133.768, 146.72),
    ("a", "CMMI10", 144.142, 146.72),
    ("R", "CMMI10", 151.63, 146.72),
    ("=", "CMR10", 161.486, 146.72),
    ("b", "CMMI10", 169.235, 146.72),
    (",", "CMR10", 173.511, 146.72),
    ("a", "CMMI10", 179.596, 146.72),
    ("R", "CMMI10", 184.862, 146.72),
    ("=", "CMR10", 195.266, 146.72),
    ("b", "CMMI10", 205.785, 146.72),
    (",", "CMR10", 210.061, 146.72),
    ("a", "CMMI10", 216.146, 146.72),
    ("R", "CMMI10", 223.634, 146.72),
    ("R", "CMMI10", 233.48, 146.72),
    ("b", "CMMI10", 241.124, 146.72),
    (",", "CMR10", 245.4, 146.72),
    ("a", "CMMI10", 251.485, 146.72),
    (":", "CMR10", 258.973, 146.72),
    ("=", "CMR10", 261.741, 146.72),
    ("b", "CMMI10", 271.701, 146.72),
    ("X", "CMR10", 279.295, 146.72),
    ("C", "CMR10", 133.768, 158.675),
    ("a", "CMMI10", 144.281, 158.675),
    ("=", "CMR10", 149.547, 158.675),
    ("b", "CMMI10", 157.296, 158.675),
    (",", "CMR10", 161.572, 158.675),
    ("a", "CMMI10", 167.667, 158.675),
    ("+", "CMR10", 172.933, 158.675),
    ("b", "CMMI10", 180.682, 158.675),
    (",", "CMR10", 184.958, 158.675),
    ("+", "CMR10", 193.255, 158.675),
    ("1", "CMR10", 203.215, 158.675),
    (",", "CMR10", 208.197, 158.675),
    ("=", "CMR10", 217.051, 158.675),
    (",", "CMR10", 227.57, 158.675),
    ("a", "CMMI10", 233.655, 158.675),
    ("+", "CMR10", 241.143, 158.675),
    ("b", "CMMI10", 251.104, 158.675),
    ("X", "CMR10", 258.697, 158.675),
    ("D", "CMR10", 133.768, 170.63),
    ("1", "CMR7", 144.7, 167.015),
    ("4", "CMR7", 148.672, 167.015),
    ("C", "CMMI10", 153.14, 170.63),
    (",", "CMR10", 160.978, 170.63),
    ("n", "CMMI7", 167.062, 172.125),
    ("C", "CMMI10", 172.485, 170.63),
    ("k", "CMMI7", 179.605, 172.125),
    (",", "CMR10", 184.507, 170.63),
    ("a", "CMMI10", 190.592, 170.63),
    ("+", "CMR10", 198.08, 170.63),
    ("b", "CMMI10", 208.041, 170.63),
    ("2", "CMR7", 212.313, 166.175),
    (",", "CMR10", 216.783, 170.63),
    ("∑", "CMEX10", 222.871, 0.0),
    ("i", "CMMI7", 233.387, 173.619),
    ("x", "CMMI10", 236.704, 170.63),
    (",", "CMR10", 242.398, 170.63),
    ("a", "CMMI10", 248.483, 170.63),
    ("1", "CMR7", 254.948, 166.708),
    ("2", "CMR7", 254.948, 174.066),
    ("b", "CMMI10", 260.115, 170.63),
    ("X", "CMR10", 267.708, 170.63),
    ("E", "CMR10", 133.768, 182.585),
    ("a", "CMMI10", 143.866, 182.585),
    ("m", "CMR10", 151.902, 182.585),
    ("o", "CMR10", 160.204, 182.585),
    ("d", "CMR10", 165.464, 182.585),
    ("b", "CMMI10", 173.769, 182.585),
    (",", "CMR10", 178.045, 182.585),
    ("(", "CMR10", 184.13, 182.585),
    ("a", "CMMI10", 188.004, 182.585),
    (")", "CMR10", 193.271, 182.585),
    ("m", "CMR10", 199.915, 182.585),
    ("o", "CMR10", 208.217, 182.585),
    ("d", "CMR10", 213.467, 182.585),
    ("b", "CMMI10", 221.772, 182.585),
    (",", "CMR10", 226.048, 182.585),
    ("a", "CMMI10", 232.143, 182.585),
    (".", "CMMI10", 239.063, 182.585),
    (".", "CMMI10", 243.494, 182.585),
    (".", "CMMI10", 247.915, 182.585),
    ("m", "CMR10", 253.453, 182.585),
    ("o", "CMR10", 261.755, 182.585),
    ("d", "CMR10", 267.015, 182.585),
    ("b", "CMMI10", 275.32, 182.585),
    (",", "CMR10", 279.596, 182.585),
    ("a", "CMMI10", 285.681, 182.585),
    ("m", "CMR10", 293.717, 182.585),
    ("o", "CMR10", 302.018, 182.585),
    ("d", "CMR10", 307.279, 182.585),
    ("b", "CMMI10", 315.573, 182.585),
    ("X", "CMR10", 323.177, 182.585),
];

#[test]
fn groups_and_bmod_are_classed_as_in_pdftex() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, EXPECTED, 0.5);
}
