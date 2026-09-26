//! `\bigcup`, `\bigcap` and the other `largesymbols` operators
//! (`fontmath.ltx` 247-262: `\bigsqcup`, `\bigvee`, `\bigwedge`,
//! `\bigoplus`, `\bigotimes`, `\bigodot`, `\biguplus`) with their scripts,
//! pinned against pdfTeX's glyph origins.
//!
//! v0.1.8 did not render any of them: the compiler it pinned had no row for
//! the names, so `$\bigcup_{i=1}^n A_i$` came out as the literal text
//! `\bigcup` with an `unsupported_feature` error (#846 added the rows; this
//! file keeps them rendering, limits included). `DEFAULT` is every operator
//! in text style (scripts beside it), in `\displaystyle` and in a display
//! (limits over and under it). `SWITCHES` is TeX's limit controls after an
//! operator (TeX §1159: `\limits`, `\nolimits`, `\displaylimits` set the
//! tail Op noad), which must move the scripts exactly as in pdfTeX -- an
//! explicit `\displaylimits` included, which stacks the limits of `\int` and
//! `\log` (both `\nolimits` by default) in display style and leaves them
//! beside the operator in text style.
//!
//! Every expected number is pdfTeX 1.40.29 (TeX Live 2026),
//! `pdflatex -interaction=batchmode`, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, on exactly the documents below, read with
//! `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the page
//! top). The oracle never runs here: the tables are committed evidence. The
//! cmex operators are pinned in x only (pdfTeX hangs the Type 1 glyph from
//! the top of its TFM box, FlashTeX paints the OpenType variant from its
//! baseline; see `kernel_math_846.rs`); their scripts pin the vertical
//! placement, limits included.

mod common;

use common::*;

const TOL_BP: f64 = 0.5;

const DEFAULT: &str = "\\documentclass{article}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
$\\bigcup_{i=1}^n A_i \\cap \\bigcap_{i\\in I} B_i$

$\\bigsqcup_i \\bigvee_i \\bigwedge_i \\bigoplus_i \\bigotimes_i \\bigodot_i \\biguplus_i$

$\\displaystyle\\bigcup_{i=1}^n A_i \\bigcap_{i\\in I} B_i$
\\[ \\bigcup_{i=1}^n A_i \\cap \\bigcap_{i\\in I} B_i \\]
\\[ \\bigsqcup_{i} \\bigvee_{i} \\bigwedge_{i} \\bigoplus_{i} \\bigotimes_{i} \\bigodot_{i} \\biguplus_{i} \\]
\\end{document}
";

/// pdfTeX's glyphs of [`DEFAULT`]: (painted text, pdfTeX font, x bp,
/// baseline y bp; `0.0` = x only).
const DEFAULT_EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("⋃", "CMEX10", 133.768, 0.0),
    ("n", "CMMI7", 142.071, 129.756),
    ("i", "CMMI7", 142.071, 137.753),
    ("=", "CMR7", 144.890, 137.753),
    ("1", "CMR7", 151.006, 137.753),
    ("A", "CMMI10", 157.135, 134.765),
    ("i", "CMMI7", 164.607, 136.259),
    ("∩", "CMSY10", 170.138, 134.765),
    ("⋂", "CMEX10", 178.993, 0.0),
    ("i", "CMMI7", 187.296, 137.753),
    ("∈", "CMSY7", 190.115, 137.753),
    ("I", "CMMI7", 195.483, 137.753),
    ("B", "CMMI10", 201.715, 134.765),
    ("i", "CMMI7", 209.271, 136.259),
    ("⨆", "CMEX10", 133.768, 0.0),
    ("i", "CMMI7", 142.071, 149.709),
    ("⋁", "CMEX10", 147.048, 0.0),
    ("i", "CMMI7", 155.350, 149.709),
    ("⋀", "CMEX10", 160.327, 0.0),
    ("i", "CMMI7", 168.630, 149.709),
    ("⨁", "CMEX10", 173.607, 0.0),
    ("i", "CMMI7", 184.676, 149.709),
    ("⨂", "CMEX10", 189.654, 0.0),
    ("i", "CMMI7", 200.723, 149.709),
    ("⨀", "CMEX10", 205.701, 0.0),
    ("i", "CMMI7", 216.770, 149.709),
    ("⨄", "CMEX10", 221.747, 0.0),
    ("i", "CMMI7", 230.050, 149.709),
    ("n", "CMMI7", 137.759, 154.704),
    ("⋃", "CMEX10", 134.687, 0.0),
    ("i", "CMMI7", 133.768, 178.912),
    ("=", "CMR7", 136.587, 178.912),
    ("1", "CMR7", 142.703, 178.912),
    ("A", "CMMI10", 148.335, 167.157),
    ("i", "CMMI7", 155.807, 168.652),
    ("⋂", "CMEX10", 161.379, 0.0),
    ("i", "CMMI7", 160.784, 179.063),
    ("∈", "CMSY7", 163.603, 179.063),
    ("I", "CMMI7", 168.971, 179.063),
    ("B", "CMMI10", 174.705, 167.157),
    ("i", "CMMI7", 182.262, 168.652),
    ("n", "CMMI7", 279.005, 185.582),
    ("⋃", "CMEX10", 275.933, 0.0),
    ("i", "CMMI7", 275.014, 209.790),
    ("=", "CMR7", 277.833, 209.790),
    ("1", "CMR7", 283.949, 209.790),
    ("A", "CMMI10", 289.581, 198.035),
    ("i", "CMMI7", 297.053, 199.530),
    ("∩", "CMSY10", 302.584, 198.035),
    ("⋂", "CMEX10", 312.035, 0.0),
    ("i", "CMMI7", 311.439, 209.941),
    ("∈", "CMSY7", 314.258, 209.941),
    ("I", "CMMI7", 319.626, 209.941),
    ("B", "CMMI10", 325.360, 198.035),
    ("i", "CMMI7", 332.917, 199.530),
    ("⨆", "CMEX10", 255.922, 0.0),
    ("i", "CMMI7", 260.047, 240.654),
    ("⋁", "CMEX10", 268.652, 0.0),
    ("i", "CMMI7", 272.777, 240.654),
    ("⋀", "CMEX10", 281.382, 0.0),
    ("i", "CMMI7", 285.507, 240.654),
    ("⨁", "CMEX10", 294.112, 0.0),
    ("i", "CMMI7", 300.230, 240.654),
    ("⨂", "CMEX10", 310.827, 0.0),
    ("i", "CMMI7", 316.945, 240.654),
    ("⨀", "CMEX10", 327.542, 0.0),
    ("i", "CMMI7", 333.660, 240.654),
    ("⨄", "CMEX10", 344.257, 0.0),
    ("i", "CMMI7", 348.382, 240.654),
];

#[test]
fn big_operators_and_their_scripts_match_pdftex() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(DEFAULT, DEFAULT_EXPECTED, TOL_BP);
}

const SWITCHES: &str = "\\documentclass{article}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
$\\bigcup\\limits_{i=1}^n A_i \\bigcap\\limits_{i\\in I} B_i$

$\\sum\\limits_{k} \\int\\limits_0^1 \\oint\\limits_C \\mathop{X}\\limits_{k}^{m} \\bigoplus\\limits_i$

$\\bigcup\\displaylimits_i \\bigcap_{j}\\limits B$
\\[ \\bigcup\\nolimits_{i=1}^n A_i \\quad \\bigcap\\nolimits_{i} \\quad \\sum\\nolimits_i \\quad \\bigcup_{i}\\nolimits^{n} \\]
$\\int\\displaylimits_0^1 f \\quad \\log\\displaylimits_2 x$
\\[ \\int\\displaylimits_0^1 f \\quad \\log\\displaylimits_2 x \\quad \\oint\\displaylimits_C g \\]
\\end{document}
";

/// pdfTeX's glyphs of [`SWITCHES`], as [`DEFAULT_EXPECTED`].
const SWITCHES_EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("n", "CMMI7", 137.759, 128.801),
    ("⋃", "CMEX10", 136.070, 0.0),
    ("i", "CMMI7", 133.768, 147.031),
    ("=", "CMR7", 136.587, 147.031),
    ("1", "CMR7", 142.703, 147.031),
    ("A", "CMMI10", 148.335, 138.265),
    ("i", "CMMI7", 155.807, 139.760),
    ("⋂", "CMEX10", 162.763, 0.0),
    ("i", "CMMI7", 160.784, 147.182),
    ("∈", "CMSY7", 163.603, 147.182),
    ("I", "CMMI7", 168.971, 147.182),
    ("B", "CMMI10", 174.705, 138.265),
    ("i", "CMMI7", 182.262, 139.760),
    ("∑", "CMEX10", 133.768, 0.0),
    ("k", "CMMI7", 136.825, 174.205),
    ("1", "CMR7", 148.249, 155.193),
    ("∫", "CMEX10", 145.945, 0.0),
    ("0", "CMR7", 146.312, 174.410),
    ("∮", "CMEX10", 154.247, 0.0),
    ("C", "CMMI7", 153.498, 174.681),
    ("m", "CMMI7", 163.923, 157.324),
    ("X", "CMMI10", 162.549, 166.124),
    ("k", "CMMI7", 164.474, 172.627),
    ("⨁", "CMEX10", 173.245, 0.0),
    ("i", "CMMI7", 177.371, 173.977),
    ("⋃", "CMEX10", 133.768, 0.0),
    ("i", "CMMI7", 142.071, 187.134),
    ("⋂", "CMEX10", 147.048, 0.0),
    ("j", "CMMI7", 149.349, 192.911),
    ("B", "CMMI10", 157.011, 184.146),
    ("⋃", "CMEX10", 245.435, 0.0),
    ("n", "CMMI7", 256.504, 199.262),
    ("i", "CMMI7", 256.504, 213.238),
    ("=", "CMR7", 259.323, 213.238),
    ("1", "CMR7", 265.439, 213.238),
    ("A", "CMMI10", 271.569, 207.260),
    ("i", "CMMI7", 279.041, 208.755),
    ("⋂", "CMEX10", 293.981, 0.0),
    ("i", "CMMI7", 305.050, 213.238),
    ("∑", "CMEX10", 319.990, 0.0),
    ("i", "CMMI7", 334.381, 213.238),
    ("⋃", "CMEX10", 349.321, 0.0),
    ("n", "CMMI7", 360.390, 199.263),
    ("i", "CMMI7", 360.390, 213.238),
    ("∫", "CMEX10", 133.768, 0.0),
    ("1", "CMR7", 140.410, 224.706),
    ("0", "CMR7", 138.473, 233.811),
    ("f", "CMMI10", 146.540, 230.269),
    ("l", "CMR10", 164.110, 230.269),
    ("o", "CMR10", 166.878, 230.269),
    ("g", "CMR10", 171.859, 230.269),
    ("2", "CMR7", 176.981, 232.704),
    ("x", "CMMI10", 183.111, 230.269),
    ("1", "CMR7", 271.947, 240.298),
    ("∫", "CMEX10", 266.737, 0.0),
    ("0", "CMR7", 267.519, 270.584),
    ("f", "CMMI10", 278.360, 255.851),
    ("l", "CMR10", 295.930, 255.851),
    ("o", "CMR10", 298.698, 255.851),
    ("g", "CMR10", 303.679, 255.851),
    ("2", "CMR7", 300.382, 263.942),
    ("x", "CMMI10", 310.462, 255.851),
    ("∮", "CMEX10", 327.779, 0.0),
    ("C", "CMMI7", 327.445, 270.856),
    ("g", "CMMI10", 339.402, 255.851),
];

/// The compiler records a limit switch on a `largesymbols` operator or a
/// `\mathop{...}` (`MathAtom::limits`, compiler `takes_limit_switch`), and
/// the renderer applies it to the Op atom (`convert_math_classed`).
#[test]
fn limit_switches_after_big_operators_match_pdftex() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SWITCHES, SWITCHES_EXPECTED, TOL_BP);
}

/// A limit switch after material that only *spells* an operator:
/// `\mathrm{lim}`, `\mathrm{sin}` and amsmath's `\text{lim}` are ordinary
/// (a math alphabet's group, an hbox), so TeX §1159 reports "Limit controls
/// must follow a math operator" for each switch and ignores it -- the
/// scripts stay beside the word in text and display style alike. So does a
/// `\color` between an operator and its switch (its whatsit is the tail,
/// and the script goes on a new empty Ord noad, TeX §1176), and an operator
/// inside a math alphabet inside `\ensuremath`; `\ensuremath{\sum}\limits`
/// still reaches the `\sum`. pdfTeX reports exactly seven such errors on
/// this document.
const NOT_OPERATORS: &str = "\\documentclass{article}
\\usepackage{amsmath,xcolor}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
$\\mathrm{lim}\\limits_{n} x$ and $\\mathrm{sin}\\nolimits_a y$
\\[\\mathrm{sin}\\nolimits_a y\\]
\\[\\mathrm{lim}\\limits_{n} x \\quad \\text{lim}\\limits_{m} w\\]
$\\sum\\color{red}\\limits_{i} x$

$\\ensuremath{\\mathrm{\\sum}}\\limits_{j}$

$\\ensuremath{\\sum}\\limits_{k} z$
\\end{document}
";

/// pdfTeX's glyphs of [`NOT_OPERATORS`], as [`DEFAULT_EXPECTED`].
const NOT_OPERATORS_EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("l", "CMR10", 133.768, 134.765),
    ("i", "CMR10", 136.536, 134.765),
    ("m", "CMR10", 139.303, 134.765),
    ("n", "CMMI7", 147.605, 136.259),
    ("x", "CMMI10", 153.028, 134.765),
    ("a", "CMR10", 162.039, 134.765),
    ("n", "CMR10", 167.020, 134.765),
    ("d", "CMR10", 172.556, 134.765),
    ("s", "CMR10", 181.418, 134.765),
    ("i", "CMR10", 185.348, 134.765),
    ("n", "CMR10", 188.115, 134.765),
    ("a", "CMMI7", 193.647, 136.259),
    ("y", "CMMI10", 198.467, 134.765),
    ("s", "CMR10", 294.477, 146.720),
    ("i", "CMR10", 298.406, 146.720),
    ("n", "CMR10", 301.174, 146.720),
    ("a", "CMMI7", 306.709, 148.214),
    ("y", "CMMI10", 311.529, 146.720),
    ("l", "CMR10", 273.763, 164.653),
    ("i", "CMR10", 276.531, 164.653),
    ("m", "CMR10", 279.298, 164.653),
    ("n", "CMMI7", 287.600, 166.147),
    ("x", "CMMI10", 293.023, 164.653),
    ("l", "CMR10", 308.679, 164.653),
    ("i", "CMR10", 311.447, 164.653),
    ("m", "CMR10", 314.214, 164.653),
    ("m", "CMMI7", 322.517, 166.147),
    ("w", "CMMI10", 330.084, 164.653),
    ("∑", "CMEX10", 133.768, 0.0),
    ("i", "CMMI7", 145.945, 184.080),
    ("x", "CMMI10", 149.262, 182.585),
    ("∑", "CMEX10", 133.768, 0.0),
    ("j", "CMMI7", 144.284, 197.529),
    ("∑", "CMEX10", 133.768, 0.0),
    ("k", "CMMI7", 136.825, 215.490),
    ("z", "CMMI10", 145.945, 206.496),
];

#[test]
fn a_limit_switch_after_an_operator_spelling_is_ignored_like_pdftex() {
    if !lm_available() {
        return;
    }
    let r = render_one(NOT_OPERATORS);
    let errors: Vec<_> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_render_pipeline::display::Severity::Error)
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(errors, ["Limit controls must follow a math operator"; 7], "pdfTeX reports seven");
    assert_eq!(r.v2.pages.len(), 1);
    let mut actual: Vec<(String, f64, f64, bool)> = painted_glyphs(&r).into_iter().map(|(t, x, y)| (t, x, y, false)).collect();
    let mut misses = Vec::new();
    for &(text, font, x, y) in NOT_OPERATORS_EXPECTED {
        let hit = actual
            .iter()
            .position(|(t, ax, ay, used)| !used && t == text && (ax - x).abs() <= TOL_BP && (y == 0.0 || (ay - y).abs() <= TOL_BP));
        match hit {
            Some(i) => actual[i].3 = true,
            None => misses.push(format!("{text:?} ({font} at {x}, {y})")),
        }
    }
    assert!(misses.is_empty(), "unmatched within {TOL_BP} bp: {misses:?}\npainted: {actual:?}");
}
