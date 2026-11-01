//! Issue #894: a mid-formula `\displaystyle` and amsmath's `\cfrac`,
//! against pdfTeX glyph by glyph.
//!
//! `\cfrac` (`amsmath.sty` 912-914) is
//! `{\displaystyle\frac{\strut#2}{#3}}\kern-\nulldelimiterspace`: every
//! level a display-style fraction, a `\strut` (`.7`/`.3` of the text size's
//! `\baselineskip`) heading every numerator, and a 1.2pt kern taking back
//! the null delimiter space. Setting it as `\dfrac` got the sizes right but
//! each level 2.68 bp too short and 1.2pt too wide at 10pt; the strut and
//! the kern close both. A style switch lasts to the end of its group
//! (TeX §1171): a mid-formula `\displaystyle` sets the rest of the formula,
//! a later `\textstyle` ends it for what follows only (the run before it
//! keeps display limits, which `split_at_spaces` used to hand to the new
//! style), and `{\displaystyle ...}` sets only its own group.
//!
//! Every expected number is pdfTeX 1.40.29 (TeX Live 2026, amsmath
//! 2025/07/09 v2.17z), `pdflatex -interaction=batchmode`, two passes,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on exactly the documents
//! below (the first is the issue's repro), read with
//! `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the page
//! top). The oracle never runs here: the tables are committed evidence.
//! Identity is the character FlashTeX paints for pdfTeX's slot; the cmex
//! operators are pinned in x only (pdfTeX's Type 1 cmex origin is the top
//! of the TFM box), as in `kernel_math_846.rs` -- their x is where the
//! display (`∑` 14.39 pt) or text (10.52 pt) variant starts, so a wrong
//! style still misses.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const ISSUE_REPRO: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\begin{document}
A: leading displaystyle: $\\displaystyle\\sum_{d\\mid n}\\varphi(d)$.

B: mid-formula displaystyle: $L(n)=\\displaystyle\\sum_{d\\mid n}\\varphi(d)$.

C: mid-formula prod: $m=\\displaystyle\\prod_{k=1}p_k$.

D: no displaystyle: $\\sum_{d\\mid n}\\varphi(d)$.

E: cfrac: $[3,5,2,3]=3+\\cfrac{1}{5+\\cfrac{1}{2+\\cfrac{1}{3}}}$.

F: frac: $[3,5,2,3]=3+\\frac{1}{5+\\frac{1}{2+\\frac{1}{3}}}$.

G: display cfrac:
\\[ 3+\\cfrac{1}{5+\\cfrac{1}{2+\\cfrac{1}{3+\\cfrac{1}{4}}}} \\]
\\end{document}
";

const TWELVE_POINT: &str = "\\documentclass[12pt]{article}
\\usepackage{amsmath}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $x=\\cfrac{1}{1+\\cfrac{1}{x}}$ Y and more text.

B $x^{\\cfrac{a}{b}} + y$ Y

C $y + \\cfrac{-1}{2}$ Y

E \\[ a_0+\\cfrac{b_1}{a_1+\\cfrac{b_2}{a_2+\\dotsb}} \\]
F $L(n)=\\displaystyle\\sum_{d\\mid n}\\varphi(d) = \\textstyle\\sum_d 1$ Y

G ${\\displaystyle\\sum_i} + \\sum_j a$ Y

H $a+{\\displaystyle\\sum_i b}+\\sum_j c$ Y
\\end{document}
";

/// pdfTeX's glyphs for [`ISSUE_REPRO`]: (painted text, pdfTeX font, x bp,
/// baseline y bp); `y = 0.0` means x only (a cmex operator).
const ISSUE_REPRO_EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("∑", "CMEX10", 254.261, 0.0),
    ("A", "CMR10", 148.712, 135.263),
    (":", "CMR10", 156.184, 135.263),
    ("l", "CMR10", 162.269, 135.263),
    ("e", "CMR10", 165.037, 135.263),
    ("a", "CMR10", 169.464, 135.263),
    ("d", "CMR10", 174.445, 135.263),
    ("i", "CMR10", 179.981, 135.263),
    ("n", "CMR10", 182.748, 135.263),
    ("g", "CMR10", 188.283, 135.263),
    ("d", "CMR10", 196.592, 135.263),
    ("i", "CMR10", 202.127, 135.263),
    ("s", "CMR10", 204.895, 135.263),
    ("p", "CMR10", 208.824, 135.263),
    ("l", "CMR10", 214.36, 135.263),
    ("a", "CMR10", 217.127, 135.263),
    ("y", "CMR10", 221.83, 135.263),
    ("s", "CMR10", 227.088, 135.263),
    ("t", "CMR10", 231.017, 135.263),
    ("y", "CMR10", 234.613, 135.263),
    ("l", "CMR10", 239.871, 135.263),
    ("e", "CMR10", 242.638, 135.263),
    (":", "CMR10", 247.066, 135.263),
    ("φ", "CMMI10", 270.312, 135.263),
    ("(", "CMR10", 276.83, 135.263),
    ("d", "CMMI10", 280.704, 135.263),
    (")", "CMR10", 285.89, 135.263),
    (".", "CMR10", 289.764, 135.263),
    ("d", "CMMI7", 255.737, 147.633),
    ("∣", "CMSY7", 259.884, 147.633),
    ("n", "CMMI7", 262.251, 147.633),
    ("∑", "CMEX10", 309.805, 0.0),
    ("B", "CMR10", 148.712, 161.83),
    (":", "CMR10", 155.769, 161.83),
    ("m", "CMR10", 161.854, 161.83),
    ("i", "CMR10", 170.156, 161.83),
    ("d", "CMR10", 172.923, 161.83),
    ("-", "CMR10", 178.458, 161.83),
    ("f", "CMR10", 181.779, 161.83),
    ("o", "CMR10", 184.823, 161.83),
    ("r", "CMR10", 189.805, 161.83),
    ("m", "CMR10", 193.707, 161.83),
    ("u", "CMR10", 201.74, 161.83),
    ("l", "CMR10", 207.275, 161.83),
    ("a", "CMR10", 210.043, 161.83),
    ("d", "CMR10", 218.342, 161.83),
    ("i", "CMR10", 223.877, 161.83),
    ("s", "CMR10", 226.644, 161.83),
    ("p", "CMR10", 230.574, 161.83),
    ("l", "CMR10", 236.109, 161.83),
    ("a", "CMR10", 238.877, 161.83),
    ("y", "CMR10", 243.579, 161.83),
    ("s", "CMR10", 248.837, 161.83),
    ("t", "CMR10", 252.766, 161.83),
    ("y", "CMR10", 256.362, 161.83),
    ("l", "CMR10", 261.62, 161.83),
    ("e", "CMR10", 264.388, 161.83),
    (":", "CMR10", 268.815, 161.83),
    ("L", "CMMI10", 276.016, 161.83),
    ("(", "CMR10", 282.797, 161.83),
    ("n", "CMMI10", 286.671, 161.83),
    (")", "CMR10", 292.651, 161.83),
    ("=", "CMR10", 299.285, 161.83),
    ("φ", "CMMI10", 325.856, 161.83),
    ("(", "CMR10", 332.374, 161.83),
    ("d", "CMMI10", 336.248, 161.83),
    (")", "CMR10", 341.434, 161.83),
    (".", "CMR10", 345.308, 161.83),
    ("d", "CMMI7", 311.281, 174.2),
    ("∣", "CMSY7", 315.428, 174.2),
    ("n", "CMMI7", 317.795, 174.2),
    ("∏", "CMEX10", 268.815, 0.0),
    ("C", "CMR10", 148.712, 188.397),
    (":", "CMR10", 155.907, 188.397),
    ("m", "CMR10", 161.992, 188.397),
    ("i", "CMR10", 170.294, 188.397),
    ("d", "CMR10", 173.062, 188.397),
    ("-", "CMR10", 178.597, 188.397),
    ("f", "CMR10", 181.917, 188.397),
    ("o", "CMR10", 184.962, 188.397),
    ("r", "CMR10", 189.943, 188.397),
    ("m", "CMR10", 193.846, 188.397),
    ("u", "CMR10", 201.878, 188.397),
    ("l", "CMR10", 207.414, 188.397),
    ("a", "CMR10", 210.181, 188.397),
    ("p", "CMR10", 218.48, 188.397),
    ("r", "CMR10", 224.015, 188.397),
    ("o", "CMR10", 227.918, 188.397),
    ("d", "CMR10", 233.178, 188.397),
    (":", "CMR10", 238.713, 188.397),
    ("m", "CMMI10", 245.904, 188.397),
    ("=", "CMR10", 257.421, 188.397),
    ("p", "CMMI10", 284.086, 188.397),
    (".", "CMR10", 294.0, 188.397),
    ("k", "CMMI7", 289.098, 189.891),
    ("k", "CMMI7", 267.934, 200.38),
    ("=", "CMR7", 272.337, 200.38),
    ("1", "CMR7", 278.453, 200.38),
    ("∑", "CMEX10", 233.921, 0.0),
    ("D", "CMR10", 148.712, 209.845),
    (":", "CMR10", 156.322, 209.845),
    ("n", "CMR10", 162.408, 209.845),
    ("o", "CMR10", 167.943, 209.845),
    ("d", "CMR10", 176.252, 209.845),
    ("i", "CMR10", 181.787, 209.845),
    ("s", "CMR10", 184.554, 209.845),
    ("p", "CMR10", 188.484, 209.845),
    ("l", "CMR10", 194.019, 209.845),
    ("a", "CMR10", 196.787, 209.845),
    ("y", "CMR10", 201.489, 209.845),
    ("s", "CMR10", 206.747, 209.845),
    ("t", "CMR10", 210.676, 209.845),
    ("y", "CMR10", 214.272, 209.845),
    ("l", "CMR10", 219.53, 209.845),
    ("e", "CMR10", 222.298, 209.845),
    (":", "CMR10", 226.725, 209.845),
    ("φ", "CMMI10", 258.034, 209.845),
    ("(", "CMR10", 264.552, 209.845),
    ("d", "CMMI10", 268.426, 209.845),
    (")", "CMR10", 273.612, 209.845),
    (".", "CMR10", 277.486, 209.845),
    ("d", "CMMI7", 244.437, 212.833),
    ("∣", "CMSY7", 248.584, 212.833),
    ("n", "CMMI7", 250.951, 212.833),
    ("1", "CMR10", 278.293, 223.942),
    ("E", "CMR10", 148.712, 231.414),
    (":", "CMR10", 155.493, 231.414),
    ("c", "CMR10", 161.578, 231.414),
    ("f", "CMR10", 166.005, 231.414),
    ("r", "CMR10", 169.05, 231.414),
    ("a", "CMR10", 172.952, 231.414),
    ("c", "CMR10", 177.933, 231.414),
    (":", "CMR10", 182.361, 231.414),
    ("[", "CMR10", 189.562, 231.414),
    ("3", "CMR10", 192.329, 231.414),
    (",", "CMMI10", 197.311, 231.414),
    ("5", "CMR10", 201.732, 231.414),
    (",", "CMMI10", 206.713, 231.414),
    ("2", "CMR10", 211.145, 231.414),
    (",", "CMMI10", 216.126, 231.414),
    ("3", "CMR10", 220.557, 231.414),
    ("]", "CMR10", 225.539, 231.414),
    ("=", "CMR10", 231.066, 231.414),
    ("3", "CMR10", 241.584, 231.414),
    ("+", "CMR10", 248.777, 231.414),
    (".", "CMR10", 301.628, 231.414),
    ("1", "CMR10", 287.47, 238.686),
    ("5", "CMR10", 259.94, 246.158),
    ("+", "CMR10", 267.133, 246.158),
    ("1", "CMR10", 296.646, 253.431),
    ("2", "CMR10", 278.293, 260.903),
    ("+", "CMR10", 285.486, 260.903),
    ("3", "CMR10", 296.646, 267.737),
    ("1", "CMR7", 266.634, 273.227),
    ("F", "CMR10", 148.712, 277.15),
    (":", "CMR10", 155.216, 277.15),
    ("f", "CMR10", 161.301, 277.15),
    ("r", "CMR10", 164.345, 277.15),
    ("a", "CMR10", 168.248, 277.15),
    ("c", "CMR10", 173.229, 277.15),
    (":", "CMR10", 177.656, 277.15),
    ("[", "CMR10", 184.857, 277.15),
    ("3", "CMR10", 187.625, 277.15),
    (",", "CMMI10", 192.606, 277.15),
    ("5", "CMR10", 197.028, 277.15),
    (",", "CMMI10", 202.009, 277.15),
    ("2", "CMR10", 206.44, 277.15),
    (",", "CMMI10", 211.422, 277.15),
    ("3", "CMR10", 215.853, 277.15),
    ("]", "CMR10", 220.834, 277.15),
    ("=", "CMR10", 226.362, 277.15),
    ("3", "CMR10", 236.88, 277.15),
    ("+", "CMR10", 244.073, 277.15),
    (".", "CMR10", 283.2, 277.15),
    ("1", "CMR5", 271.968, 278.467),
    ("5", "CMR7", 255.235, 281.144),
    ("+", "CMR7", 259.207, 281.144),
    ("1", "CMR5", 276.223, 283.119),
    ("2", "CMR5", 266.518, 285.049),
    ("+", "CMR5", 269.908, 285.049),
    ("3", "CMR5", 276.223, 287.699),
    ("G", "CMR10", 148.712, 295.614),
    (":", "CMR10", 156.53, 295.614),
    ("d", "CMR10", 162.615, 295.614),
    ("i", "CMR10", 168.15, 295.614),
    ("s", "CMR10", 170.918, 295.614),
    ("p", "CMR10", 174.847, 295.614),
    ("l", "CMR10", 180.382, 295.614),
    ("a", "CMR10", 183.15, 295.614),
    ("y", "CMR10", 187.862, 295.614),
    ("c", "CMR10", 196.438, 295.614),
    ("f", "CMR10", 200.865, 295.614),
    ("r", "CMR10", 203.91, 295.614),
    ("a", "CMR10", 207.812, 295.614),
    ("c", "CMR10", 212.793, 295.614),
    (":", "CMR10", 217.221, 295.614),
    ("1", "CMR10", 312.31, 306.916),
    ("3", "CMR10", 266.427, 314.388),
    ("+", "CMR10", 273.62, 314.388),
    ("1", "CMR10", 321.487, 321.66),
    ("5", "CMR10", 284.78, 329.132),
    ("+", "CMR10", 291.973, 329.132),
    ("1", "CMR10", 330.663, 336.405),
    ("2", "CMR10", 303.133, 343.877),
    ("+", "CMR10", 310.326, 343.877),
    ("1", "CMR10", 339.84, 351.149),
    ("3", "CMR10", 321.487, 358.621),
    ("+", "CMR10", 328.68, 358.621),
    ("4", "CMR10", 339.84, 365.455),
    ("1", "CMR10", 303.133, 702.635),
];

/// pdfTeX's glyphs for [`TWELVE_POINT`].
const TWELVE_POINT_EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("1", "CMR12", 158.258, 135.91),
    ("A", "CMR12", 110.854, 144.907),
    ("x", "CMMI12", 123.527, 144.907),
    ("=", "CMR12", 133.502, 144.907),
    ("Y", "CMR12", 179.146, 144.907),
    ("a", "CMR12", 191.819, 144.907),
    ("n", "CMR12", 197.672, 144.907),
    ("d", "CMR12", 204.175, 144.907),
    ("m", "CMR12", 214.588, 144.907),
    ("o", "CMR12", 224.344, 144.907),
    ("r", "CMR12", 230.197, 144.907),
    ("e", "CMR12", 234.75, 144.907),
    ("t", "CMR12", 243.85, 144.907),
    ("e", "CMR12", 248.402, 144.907),
    ("x", "CMR12", 253.605, 144.907),
    ("t", "CMR12", 259.784, 144.907),
    (".", "CMR12", 264.336, 144.907),
    ("1", "CMR12", 168.992, 153.703),
    ("1", "CMR12", 147.126, 162.7),
    ("+", "CMR12", 155.633, 162.7),
    ("x", "CMMI12", 168.592, 170.9),
    ("a", "CMMI12", 130.894, 182.009),
    ("b", "CMMI12", 131.477, 199.205),
    ("B", "CMR12", 110.854, 200.492),
    ("x", "CMMI12", 123.041, 200.492),
    ("+", "CMR12", 140.193, 200.492),
    ("y", "CMMI12", 151.952, 200.492),
    ("Y", "CMR12", 161.987, 200.492),
    ("−", "CMSY10", 147.617, 213.925),
    ("1", "CMR12", 159.57, 213.925),
    ("C", "CMR12", 110.854, 222.921),
    ("y", "CMMI12", 123.206, 222.921),
    ("+", "CMR12", 131.998, 222.921),
    ("Y", "CMR12", 169.328, 222.921),
    ("2", "CMR12", 152.267, 231.122),
    ("E", "CMR12", 110.854, 240.288),
    ("b", "CMMI12", 313.516, 251.396),
    ("1", "CMR8", 318.494, 253.189),
    ("a", "CMMI12", 259.014, 260.392),
    ("+", "CMR12", 272.548, 260.392),
    ("0", "CMR8", 265.159, 262.186),
    ("b", "CMMI12", 326.762, 269.189),
    ("2", "CMR8", 331.739, 270.982),
    ("a", "CMMI12", 285.504, 278.185),
    ("+", "CMR12", 299.038, 278.185),
    ("1", "CMR8", 291.649, 279.979),
    ("a", "CMMI12", 311.995, 286.386),
    ("+", "CMR12", 325.529, 286.386),
    ("⋅", "CMSY10", 337.288, 286.386),
    ("⋅", "CMSY10", 342.606, 286.386),
    ("⋅", "CMSY10", 347.912, 286.386),
    ("2", "CMR8", 318.14, 288.179),
    ("∑", "CMEX10", 162.199, 0.0),
    ("∑", "CMEX10", 220.062, 0.0),
    ("F", "CMR12", 110.854, 308.204),
    ("L", "CMMI12", 122.391, 308.204),
    ("(", "CMR12", 130.355, 308.204),
    ("n", "CMMI12", 134.908, 308.204),
    (")", "CMR12", 141.896, 308.204),
    ("=", "CMR12", 149.772, 308.204),
    ("φ", "CMMI12", 181.46, 308.204),
    ("(", "CMR12", 189.128, 308.204),
    ("d", "CMMI12", 193.681, 308.204),
    (")", "CMR12", 199.763, 308.204),
    ("=", "CMR12", 207.639, 308.204),
    ("1", "CMR12", 239.529, 308.204),
    ("Y", "CMR12", 249.28, 308.204),
    ("d", "CMMI8", 232.681, 311.691),
    ("d", "CMMI8", 164.909, 322.75),
    ("∣", "CMSY8", 169.266, 322.75),
    ("n", "CMMI8", 171.618, 322.75),
    ("∑", "CMEX10", 123.941, 0.0),
    ("∑", "CMEX10", 155.628, 0.0),
    ("G", "CMR12", 110.854, 339.487),
    ("+", "CMR12", 143.866, 339.487),
    ("a", "CMMI12", 174.622, 339.487),
    ("Y", "CMR12", 184.664, 339.487),
    ("j", "CMMI8", 168.247, 342.974),
    ("i", "CMMI8", 131.134, 353.324),
    ("∑", "CMEX10", 144.094, 0.0),
    ("∑", "CMEX10", 182.751, 0.0),
    ("H", "CMR12", 110.854, 368.069),
    ("a", "CMMI12", 123.527, 368.069),
    ("+", "CMR12", 132.337, 368.069),
    ("b", "CMMI12", 163.356, 368.069),
    ("+", "CMR12", 170.987, 368.069),
    ("c", "CMMI12", 201.745, 368.069),
    ("Y", "CMR12", 210.68, 368.069),
    ("j", "CMMI8", 195.37, 371.556),
    ("i", "CMMI8", 151.287, 381.906),
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
            .filter(|(_, (t, ax, ay, used))| !used && t == text && (ax - x).abs() <= TOL_BP && (y == 0.0 || (ay - y).abs() <= TOL_BP))
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
fn issue_repro_matches_pdftex_glyph_by_glyph() {
    if !lm_available() {
        return;
    }
    assert_matches(ISSUE_REPRO, ISSUE_REPRO_EXPECTED);
}

#[test]
fn twelve_point_cfrac_and_scoped_switches_match_pdftex() {
    if !lm_available() {
        return;
    }
    assert_matches(TWELVE_POINT, TWELVE_POINT_EXPECTED);
}
