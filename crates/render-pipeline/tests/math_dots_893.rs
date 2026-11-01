//! Issue #893: math `\dots` against pdfTeX, glyph by glyph.
//!
//! amsmath's `\dots` (`amsmath.sty` 497-629, `\mdots@@`) looks at the
//! token after it: before a binary operator or relation (`+ = < > - * :`,
//! `\not`, a Bin/Rel `\mathchar`, a `\DOTSB` macro such as `\sum`) it
//! sets the centred `\cdots`, before a `\DOTSI` integral `\!\cdots`,
//! and anything else the low `\ldots`; before a right delimiter or the
//! closing `$` it adds a thin space (`\extra@`), and `\cdots`/`\dotsc`
//! add one before punctuation (`\extrap@`). The kernel's `\dots` is
//! `\mathellipsis`, low wherever it is. The compiler decides the form
//! (`MathParser::ellipsis`); the pipeline paints the dot that decision chose
//! (`convert_math_classed`), where it used to re-read the control word and
//! paint every bare `\dots` low.
//!
//! Every expected number is pdfTeX 1.40.29 (TeX Live 2026, amsmath
//! 2025/07/09 v2.17z), `pdflatex -interaction=batchmode`, two passes,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on exactly the documents
//! below, read with `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y
//! from the page top). The oracle never runs here: the tables are committed
//! evidence. Identity is the character FlashTeX paints for pdfTeX's slot:
//! cmsy10 `"01` (`\cdotp`) is U+22C5, cmmi10 `"3A` (`\ldotp`) is `.`. The
//! cmex operators (`\sum`, `\int`) are pinned in x only, as in
//! `kernel_math_846.rs`. The trailing `X` of lines G-N pins the width the
//! dots and their thin spaces take: 1.66 bp each, three times the tolerance.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const AMSMATH: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $\\dots = \\gcd(a,b)$.

B $a_1 + \\dots + a_n$.

C $a_1, \\dots, a_n$.

D $\\cdots = \\gcd(a,b)$.

E $\\ldots = \\gcd(a,b)$.

F $x \\dots \\le y$.

G $a\\dots\\sum b$ X

H $a\\dots\\int b$ X

I $(a,\\dots)$ X

J $a,\\dots$ X

K $a\\cdots, b$ X

L $a\\dotsc; b$ X

M $a\\dots\\quad b$ X

N $\\{a\\dots\\}$ X
\\end{document}
";

const KERNEL: &str = "\\documentclass{article}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $\\dots = b$ X

B $a_1 + \\dots + a_n$ X

F $x \\dots \\le y$ X

J $a,\\dots$ X
\\end{document}
";

/// pdfTeX's glyphs for [`AMSMATH`]: (painted text, pdfTeX font, x bp,
/// baseline y bp); `y = 0.0` means x only (a cmex operator).
const AMSMATH_EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR10", 133.768, 134.765),
    ("⋅", "CMSY10", 144.557, 134.765),
    ("⋅", "CMSY10", 148.989, 134.765),
    ("⋅", "CMSY10", 153.42, 134.765),
    ("=", "CMR10", 158.947, 134.765),
    ("g", "CMR10", 169.466, 134.765),
    ("c", "CMR10", 174.447, 134.765),
    ("d", "CMR10", 178.875, 134.765),
    ("(", "CMR10", 184.41, 134.765),
    ("a", "CMMI10", 188.284, 134.765),
    (",", "CMMI10", 193.551, 134.765),
    ("b", "CMMI10", 197.982, 134.765),
    (")", "CMR10", 202.258, 134.765),
    (".", "CMR10", 206.132, 134.765),
    ("B", "CMR10", 133.768, 146.72),
    ("a", "CMMI10", 144.142, 146.72),
    ("+", "CMR10", 156.095, 146.72),
    ("⋅", "CMSY10", 166.056, 146.72),
    ("⋅", "CMSY10", 170.487, 146.72),
    ("⋅", "CMSY10", 174.918, 146.72),
    ("+", "CMR10", 179.898, 146.72),
    ("a", "CMMI10", 189.858, 146.72),
    (".", "CMR10", 200.547, 146.72),
    ("1", "CMR7", 149.412, 148.214),
    ("n", "CMMI7", 195.124, 148.214),
    ("C", "CMR10", 133.768, 158.675),
    ("a", "CMMI10", 144.281, 158.675),
    (",", "CMMI10", 154.02, 158.675),
    (".", "CMMI10", 158.451, 158.675),
    (".", "CMMI10", 162.873, 158.675),
    (".", "CMMI10", 167.304, 158.675),
    (",", "CMMI10", 171.735, 158.675),
    ("a", "CMMI10", 176.157, 158.675),
    (".", "CMR10", 186.848, 158.675),
    ("1", "CMR7", 149.551, 160.169),
    ("n", "CMMI7", 181.425, 160.169),
    ("D", "CMR10", 133.768, 170.63),
    ("⋅", "CMSY10", 144.696, 170.63),
    ("⋅", "CMSY10", 149.127, 170.63),
    ("⋅", "CMSY10", 153.559, 170.63),
    ("=", "CMR10", 159.086, 170.63),
    ("g", "CMR10", 169.604, 170.63),
    ("c", "CMR10", 174.586, 170.63),
    ("d", "CMR10", 179.013, 170.63),
    ("(", "CMR10", 184.548, 170.63),
    ("a", "CMMI10", 188.423, 170.63),
    (",", "CMMI10", 193.689, 170.63),
    ("b", "CMMI10", 198.12, 170.63),
    (")", "CMR10", 202.396, 170.63),
    (".", "CMR10", 206.271, 170.63),
    ("E", "CMR10", 133.768, 182.585),
    (".", "CMMI10", 143.866, 182.585),
    (".", "CMMI10", 148.297, 182.585),
    (".", "CMMI10", 152.729, 182.585),
    ("=", "CMR10", 158.256, 182.585),
    ("g", "CMR10", 168.775, 182.585),
    ("c", "CMR10", 173.756, 182.585),
    ("d", "CMR10", 178.183, 182.585),
    ("(", "CMR10", 183.718, 182.585),
    ("a", "CMMI10", 187.593, 182.585),
    (",", "CMMI10", 192.859, 182.585),
    ("b", "CMMI10", 197.291, 182.585),
    (")", "CMR10", 201.566, 182.585),
    (".", "CMR10", 205.441, 182.585),
    ("F", "CMR10", 133.768, 194.541),
    ("x", "CMMI10", 143.589, 194.541),
    ("⋅", "CMSY10", 150.947, 194.541),
    ("⋅", "CMSY10", 155.378, 194.541),
    ("⋅", "CMSY10", 159.799, 194.541),
    ("≤", "CMSY10", 165.336, 194.541),
    ("y", "CMMI10", 175.855, 194.541),
    (".", "CMR10", 181.098, 194.541),
    ("∑", "CMEX10", 165.117, 0.0),
    ("G", "CMR10", 133.768, 206.496),
    ("a", "CMMI10", 144.903, 206.496),
    ("⋅", "CMSY10", 151.833, 206.496),
    ("⋅", "CMSY10", 156.265, 206.496),
    ("⋅", "CMSY10", 160.686, 206.496),
    ("b", "CMMI10", 177.294, 206.496),
    ("X", "CMR10", 184.887, 206.496),
    ("∫", "CMEX10", 163.111, 0.0),
    ("H", "CMR10", 133.768, 218.451),
    ("a", "CMMI10", 144.557, 218.451),
    ("⋅", "CMSY10", 149.824, 218.451),
    ("⋅", "CMSY10", 154.255, 218.451),
    ("⋅", "CMSY10", 158.686, 218.451),
    ("b", "CMMI10", 171.413, 218.451),
    ("X", "CMR10", 179.006, 218.451),
    ("I", "CMR10", 133.768, 230.406),
    ("(", "CMR10", 140.683, 230.406),
    ("a", "CMMI10", 144.557, 230.406),
    (",", "CMMI10", 149.824, 230.406),
    (".", "CMMI10", 154.255, 230.406),
    (".", "CMMI10", 158.686, 230.406),
    (".", "CMMI10", 163.108, 230.406),
    (")", "CMR10", 167.539, 230.406),
    ("X", "CMR10", 174.731, 230.406),
    ("J", "CMR10", 133.768, 242.361),
    ("a", "CMMI10", 142.205, 242.361),
    (",", "CMMI10", 147.472, 242.361),
    (".", "CMMI10", 151.903, 242.361),
    (".", "CMMI10", 156.334, 242.361),
    (".", "CMMI10", 160.756, 242.361),
    ("X", "CMR10", 168.505, 242.361),
    ("K", "CMR10", 133.768, 254.316),
    ("a", "CMMI10", 144.834, 254.316),
    ("⋅", "CMSY10", 151.764, 254.316),
    ("⋅", "CMSY10", 156.196, 254.316),
    ("⋅", "CMSY10", 160.617, 254.316),
    (",", "CMMI10", 166.712, 254.316),
    ("b", "CMMI10", 171.134, 254.316),
    ("X", "CMR10", 178.737, 254.316),
    ("L", "CMR10", 133.768, 266.272),
    ("a", "CMMI10", 143.312, 266.272),
    (".", "CMMI10", 150.242, 266.272),
    (".", "CMMI10", 154.674, 266.272),
    (".", "CMMI10", 159.095, 266.272),
    (";", "CMR10", 165.19, 266.272),
    ("b", "CMMI10", 169.611, 266.272),
    ("X", "CMR10", 177.215, 266.272),
    ("M", "CMR10", 133.768, 278.227),
    ("a", "CMMI10", 146.218, 278.227),
    (".", "CMMI10", 153.148, 278.227),
    (".", "CMMI10", 157.58, 278.227),
    (".", "CMMI10", 162.001, 278.227),
    ("b", "CMMI10", 176.395, 278.227),
    ("X", "CMR10", 183.988, 278.227),
    ("N", "CMR10", 133.768, 290.182),
    ("{", "CMSY10", 144.557, 290.182),
    ("a", "CMMI10", 149.539, 290.182),
    (".", "CMMI10", 156.469, 290.182),
    (".", "CMMI10", 160.9, 290.182),
    (".", "CMMI10", 165.322, 290.182),
    ("}", "CMSY10", 169.753, 290.182),
    ("X", "CMR10", 178.052, 290.182),
];

/// pdfTeX's glyphs for [`KERNEL`].
const KERNEL_EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR10", 133.768, 134.765),
    (".", "CMMI10", 144.557, 134.765),
    (".", "CMMI10", 148.989, 134.765),
    (".", "CMMI10", 153.42, 134.765),
    ("=", "CMR10", 158.947, 134.765),
    ("b", "CMMI10", 169.466, 134.765),
    ("X", "CMR10", 177.059, 134.765),
    ("B", "CMR10", 133.768, 146.72),
    ("a", "CMMI10", 144.142, 146.72),
    ("+", "CMR10", 156.095, 146.72),
    (".", "CMMI10", 166.056, 146.72),
    (".", "CMMI10", 170.487, 146.72),
    (".", "CMMI10", 174.918, 146.72),
    ("+", "CMR10", 179.898, 146.72),
    ("a", "CMMI10", 189.858, 146.72),
    ("X", "CMR10", 203.868, 146.72),
    ("1", "CMR7", 149.412, 148.214),
    ("n", "CMMI7", 195.124, 148.214),
    ("F", "CMR10", 133.768, 158.675),
    ("x", "CMMI10", 143.589, 158.675),
    (".", "CMMI10", 150.947, 158.675),
    (".", "CMMI10", 155.378, 158.675),
    (".", "CMMI10", 159.799, 158.675),
    ("≤", "CMSY10", 165.336, 158.675),
    ("y", "CMMI10", 175.855, 158.675),
    ("X", "CMR10", 184.416, 158.675),
    ("J", "CMR10", 133.768, 170.63),
    ("a", "CMMI10", 142.205, 170.63),
    (",", "CMMI10", 147.472, 170.63),
    (".", "CMMI10", 151.903, 170.63),
    (".", "CMMI10", 156.334, 170.63),
    (".", "CMMI10", 160.756, 170.63),
    ("X", "CMR10", 166.851, 170.63),
];

const TOL_BP: f64 = 0.5;

/// Matches every expected glyph to a distinct painted glyph with the same
/// text within [`TOL_BP`], nearest first.
fn assert_matches(source: &str, expected: &[(&str, &str, f64, f64)]) {
    let r = render_one(source);
    let errors: Vec<_> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_render_pipeline::display::Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(r.v2.pages.len(), 1);
    let mut actual: Vec<(String, f64, f64, bool)> = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        for g in &run.glyphs {
            let c = &run.clusters[g.cluster as usize];
            let text = run.text[c.text_start_byte..c.text_end_byte].to_string();
            actual.push((text, g.origin_x.to_bp(), g.baseline_y.to_bp(), false));
        }
    }
    let mut misses = Vec::new();
    for &(text, font, x, y) in expected {
        let hit = actual
            .iter()
            .enumerate()
            .filter(|(_, (t, ax, ay, used))| {
                !used && t == text && (ax - x).abs() <= TOL_BP && (y == 0.0 || (ay - y).abs() <= TOL_BP)
            })
            .min_by(|a, b| {
                let d = |g: &(String, f64, f64, bool)| (g.1 - x).abs() + if y == 0.0 { 0.0 } else { (g.2 - y).abs() };
                d(a.1).total_cmp(&d(b.1))
            })
            .map(|(i, _)| i);
        match hit {
            Some(i) => actual[i].3 = true,
            None => {
                let nearest: Vec<_> = actual
                    .iter()
                    .filter(|(_, ax, ay, _)| (ax - x).abs() <= 3.0 && (y == 0.0 || (ay - y).abs() <= 3.0))
                    .map(|(t, ax, ay, _)| format!("{t:?} ({ax:.3}, {ay:.3})"))
                    .collect();
                misses.push(format!("{text:?} ({font} at {x}, {y}): nearest {nearest:?}"));
            }
        }
    }
    assert!(
        misses.is_empty(),
        "{} of {} pdfTeX glyphs unmatched within {TOL_BP} bp:\n{}",
        misses.len(),
        expected.len(),
        misses.join("\n")
    );
}

#[test]
fn amsmath_dots_match_pdftex_glyph_by_glyph() {
    if !lm_available() {
        return;
    }
    assert_matches(AMSMATH, AMSMATH_EXPECTED);
}

#[test]
fn kernel_dots_are_always_low() {
    if !lm_available() {
        return;
    }
    assert_matches(KERNEL, KERNEL_EXPECTED);
}
