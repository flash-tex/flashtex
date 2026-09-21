//! Issue #895: `\DeclareMathOperator*` and `\operatorname*` against pdfTeX,
//! glyph by glyph.
//!
//! amsopn's starred operators are `\mathop{\operator@font ...}` followed by
//! `\nmlimits@`, which amsopn.sty `\let`s to `\displaylimits`: limits over
//! and under in display style, ordinary scripts beside the word in text
//! style. The pipeline mapped the starred flag to `\limits` (limits in
//! every style) until `9cd84c1bc`, so `$\ord_m(u)$` centred its `m` under
//! `ord`; `tests/math_operator_limits.rs` pins that relationally. This pins
//! the issue's repro against the oracle: every glyph's identity and origin
//! within 0.5 bp -- the inline starred subscripts beside the word
//! (`5` 1.494 bp below the baseline), the unstarred one likewise, the
//! display limit 6.155 bp below and centred, `\operatorname*` as the
//! declared form.
//!
//! Expected numbers are pdfTeX 1.40.29 (TeX Live 2026, amsopn 2022/04/08),
//! `pdflatex -interaction=batchmode`, two passes, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, on exactly [`ISSUE_REPRO`], read with
//! `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the page
//! top). The oracle never runs here: the table is committed evidence.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const ISSUE_REPRO: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\DeclareMathOperator*{\\ord}{ord}
\\DeclareMathOperator{\\ordplain}{ord}
\\begin{document}
Inline starred: $\\ord_5(2)$ and $\\ord_m(u)$.

Inline unstarred: $\\ordplain_5(2)$.

Display starred:
\\[ \\ord_5(2) \\]

Inline operatorname*: $\\operatorname*{ord}_5(2)$.
\\end{document}
";

/// pdfTeX's glyphs: (painted text, pdfTeX font, x bp, baseline y bp).
const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("I", "CMR10", 148.712, 134.765),
    ("n", "CMR10", 152.309, 134.765),
    ("l", "CMR10", 157.845, 134.765),
    ("i", "CMR10", 160.612, 134.765),
    ("n", "CMR10", 163.38, 134.765),
    ("e", "CMR10", 168.915, 134.765),
    ("s", "CMR10", 176.66, 134.765),
    ("t", "CMR10", 180.589, 134.765),
    ("a", "CMR10", 184.464, 134.765),
    ("r", "CMR10", 189.445, 134.765),
    ("r", "CMR10", 193.347, 134.765),
    ("e", "CMR10", 197.25, 134.765),
    ("d", "CMR10", 201.677, 134.765),
    (":", "CMR10", 207.212, 134.765),
    ("o", "CMR10", 214.413, 134.765),
    ("r", "CMR10", 219.395, 134.765),
    ("d", "CMR10", 223.297, 134.765),
    ("(", "CMR10", 233.298, 134.765),
    ("2", "CMR10", 237.172, 134.765),
    (")", "CMR10", 242.154, 134.765),
    ("a", "CMR10", 249.346, 134.765),
    ("n", "CMR10", 254.327, 134.765),
    ("d", "CMR10", 259.862, 134.765),
    ("o", "CMR10", 268.725, 134.765),
    ("r", "CMR10", 273.706, 134.765),
    ("d", "CMR10", 277.609, 134.765),
    ("(", "CMR10", 290.707, 134.765),
    ("u", "CMMI10", 294.581, 134.765),
    (")", "CMR10", 300.285, 134.765),
    (".", "CMR10", 304.159, 134.765),
    ("5", "CMR7", 228.829, 136.259),
    ("m", "CMMI7", 283.139, 136.259),
    ("I", "CMR10", 148.712, 146.72),
    ("n", "CMR10", 152.309, 146.72),
    ("l", "CMR10", 157.845, 146.72),
    ("i", "CMR10", 160.612, 146.72),
    ("n", "CMR10", 163.38, 146.72),
    ("e", "CMR10", 168.915, 146.72),
    ("u", "CMR10", 176.66, 146.72),
    ("n", "CMR10", 182.195, 146.72),
    ("s", "CMR10", 187.731, 146.72),
    ("t", "CMR10", 191.66, 146.72),
    ("a", "CMR10", 195.534, 146.72),
    ("r", "CMR10", 200.516, 146.72),
    ("r", "CMR10", 204.418, 146.72),
    ("e", "CMR10", 208.32, 146.72),
    ("d", "CMR10", 212.748, 146.72),
    (":", "CMR10", 218.283, 146.72),
    ("o", "CMR10", 225.484, 146.72),
    ("r", "CMR10", 230.465, 146.72),
    ("d", "CMR10", 234.367, 146.72),
    ("(", "CMR10", 244.368, 146.72),
    ("2", "CMR10", 248.242, 146.72),
    (")", "CMR10", 253.224, 146.72),
    (".", "CMR10", 257.098, 146.72),
    ("5", "CMR7", 239.898, 148.214),
    ("D", "CMR10", 148.712, 158.675),
    ("i", "CMR10", 156.322, 158.675),
    ("s", "CMR10", 159.09, 158.675),
    ("p", "CMR10", 163.019, 158.675),
    ("l", "CMR10", 168.555, 158.675),
    ("a", "CMR10", 171.322, 158.675),
    ("y", "CMR10", 176.024, 158.675),
    ("s", "CMR10", 184.61, 158.675),
    ("t", "CMR10", 188.539, 158.675),
    ("a", "CMR10", 192.414, 158.675),
    ("r", "CMR10", 197.395, 158.675),
    ("r", "CMR10", 201.298, 158.675),
    ("e", "CMR10", 205.2, 158.675),
    ("d", "CMR10", 209.627, 158.675),
    (":", "CMR10", 215.163, 158.675),
    ("o", "CMR10", 292.05, 170.63),
    ("r", "CMR10", 297.031, 170.63),
    ("d", "CMR10", 300.934, 170.63),
    ("(", "CMR10", 306.468, 170.63),
    ("2", "CMR10", 310.342, 170.63),
    (")", "CMR10", 315.324, 170.63),
    ("5", "CMR7", 297.273, 176.785),
    ("I", "CMR10", 148.712, 192.227),
    ("n", "CMR10", 152.309, 192.227),
    ("l", "CMR10", 157.845, 192.227),
    ("i", "CMR10", 160.612, 192.227),
    ("n", "CMR10", 163.38, 192.227),
    ("e", "CMR10", 168.915, 192.227),
    ("o", "CMR10", 176.66, 192.227),
    ("p", "CMR10", 181.641, 192.227),
    ("e", "CMR10", 187.456, 192.227),
    ("r", "CMR10", 191.883, 192.227),
    ("a", "CMR10", 195.785, 192.227),
    ("t", "CMR10", 200.767, 192.227),
    ("o", "CMR10", 204.641, 192.227),
    ("r", "CMR10", 209.622, 192.227),
    ("n", "CMR10", 213.525, 192.227),
    ("a", "CMR10", 219.06, 192.227),
    ("m", "CMR10", 224.041, 192.227),
    ("e", "CMR10", 232.343, 192.227),
    ("*", "CMR10", 236.77, 192.227),
    (":", "CMR10", 241.752, 192.227),
    ("o", "CMR10", 248.943, 192.227),
    ("r", "CMR10", 253.934, 192.227),
    ("d", "CMR10", 257.826, 192.227),
    ("(", "CMR10", 267.835, 192.227),
    ("2", "CMR10", 271.709, 192.227),
    (")", "CMR10", 276.691, 192.227),
    (".", "CMR10", 280.565, 192.227),
    ("5", "CMR7", 263.366, 193.721),
    ("1", "CMR10", 303.133, 702.635),
];

const TOL_BP: f64 = 0.5;

#[test]
fn starred_operators_match_pdftex_glyph_by_glyph() {
    if !lm_available() {
        return;
    }
    let r = render_one(ISSUE_REPRO);
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
    for &(text, font, x, y) in EXPECTED {
        let hit = actual
            .iter()
            .enumerate()
            .filter(|(_, (t, ax, ay, used))| !used && t == text && (ax - x).abs() <= TOL_BP && (ay - y).abs() <= TOL_BP)
            .min_by(|a, b| {
                let d = |g: &(String, f64, f64, bool)| (g.1 - x).abs() + (g.2 - y).abs();
                d(a.1).total_cmp(&d(b.1))
            })
            .map(|(i, _)| i);
        match hit {
            Some(i) => actual[i].3 = true,
            None => {
                let nearest: Vec<_> = actual
                    .iter()
                    .filter(|(_, ax, ay, _)| (ax - x).abs() <= 3.0 && (ay - y).abs() <= 8.0)
                    .map(|(t, ax, ay, _)| format!("{t:?} ({ax:.3}, {ay:.3})"))
                    .collect();
                misses.push(format!("{text:?} ({font} at {x}, {y}): nearest {nearest:?}"));
            }
        }
    }
    assert!(misses.is_empty(), "{} of {} pdfTeX glyphs unmatched within {TOL_BP} bp:\n{}", misses.len(), EXPECTED.len(), misses.join("\n"));
}
