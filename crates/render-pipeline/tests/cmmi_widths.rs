//! Every cmmi glyph a LaTeX command reaches, boxed alone in math, is as
//! wide as pdfTeX makes it, in all three math sizes (cmmi10/cmmi7/cmmi5).
//!
//! #710 found Latin Modern Math's `\ell` 0.09331 pt wider than cmmi10's (a
//! wider advance plus an italic correction cmmi10 does not have), putting
//! `\sqrt{\frac{\ell}{\ell}}`'s bar 0.093 bp off. The math list now boxes
//! every cmmi character from the cmmi TFM, so the whole family is pinned
//! here rather than one glyph at a time. A radical's bar spans its radicand,
//! so the bar over `\sqrt{X}` is X's math box width -- the TFM width plus
//! the italic correction tex.web §755 appends -- and measures it directly.
//!
//! The reference widths are pdfTeX 3.141592653 (TeX Live 2026),
//! `\setbox4\hbox{$<style> X$}\the\wd4` in a 10 pt `article`, for every
//! kernel `letters` declaration (fontmath.ltx) plus the Latin letters. Not
//! listed: the cmmi uppercase Greek and old-style digits, which only
//! `\mathnormal{...}` reaches and which this engine still sets from the
//! roman family, and `\lhook`/`\rhook`, which do not typeset alone.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const BP: f64 = 72.0 / 72.27;
/// pdfTeX prints widths to 1e-5 pt and the TFM route reproduces them to
/// that digit. The project's 0.1 bp rule gate (0.0996 pt) is deliberately
/// not used: #710's own 0.0933 pt error sat inside it.
const TFM_GATE: f64 = 0.0005;

/// (cmmi slot, source, pdfTeX box width in text, script, scriptscript style)
const CMMI: &[(u8, &str, [f64; 3])] = &[
    (0x0B, "\\alpha", [6.43404, 5.19876, 4.56659]),
    (0x0C, "\\beta", [6.18402, 4.89413, 4.21011]),
    (0x0D, "\\gamma", [5.73285, 4.63107, 4.08173]),
    (0x0E, "\\delta", [4.8229, 3.88855, 3.4323]),
    (0x0F, "\\epsilon", [4.05904, 3.333, 3.06772]),
    (0x10, "\\zeta", [5.11285, 4.09288, 3.5981]),
    (0x11, "\\eta", [5.32408, 4.37134, 3.89818]),
    (0x12, "\\theta", [4.97223, 4.02428, 3.55905]),
    (0x13, "\\iota", [3.53937, 2.95952, 2.74887]),
    (0x14, "\\kappa", [5.76158, 4.68175, 4.13779]),
    (0x15, "\\lambda", [5.83336, 4.74307, 4.20143]),
    (0x16, "\\mu", [6.02548, 4.86232, 4.24889]),
    (0x17, "\\nu", [5.57639, 4.47572, 3.91324]),
    (0x18, "\\xi", [4.83508, 3.87761, 3.4245]),
    (0x19, "\\pi", [6.05905, 4.92747, 4.33511]),
    (0x1A, "\\rho", [5.17015, 4.14899, 3.63023]),
    (0x1B, "\\sigma", [6.0729, 4.8851, 4.25523]),
    (0x1C, "\\tau", [5.50348, 4.4969, 3.98788]),
    (0x1D, "\\upsilon", [5.76158, 4.68175, 4.13776]),
    (0x1E, "\\phi", [5.95834, 4.80833, 4.20837]),
    (0x1F, "\\chi", [6.25691, 4.99654, 4.32297]),
    (0x20, "\\psi", [6.8727, 5.54286, 4.83223]),
    (0x21, "\\omega", [6.58331, 5.28893, 4.61115]),
    (0x22, "\\varepsilon", [4.66318, 3.77814, 3.37328]),
    (0x23, "\\vartheta", [5.9144, 4.82898, 4.27667]),
    (0x24, "\\varpi", [8.55908, 6.86497, 5.89764]),
    (0x25, "\\varrho", [5.17015, 4.14899, 3.63023]),
    (0x26, "\\varsigma", [4.42706, 3.61284, 3.25522]),
    (0x27, "\\varphi", [6.54167, 5.25975, 4.59032]),
    (0x28, "\\leftharpoonup", [10.00002, 7.97224, 6.80565]),
    (0x29, "\\leftharpoondown", [10.00002, 7.97224, 6.80565]),
    (0x2A, "\\rightharpoonup", [10.00002, 7.97224, 6.80565]),
    (0x2B, "\\rightharpoondown", [10.00002, 7.97224, 6.80565]),
    (0x2E, "\\triangleright", [5.00002, 4.09723, 3.68059]),
    (0x2F, "\\triangleleft", [5.00002, 4.09723, 3.68059]),
    (0x3A, ".", [2.77779, 2.375, 2.29169]),
    (0x3B, ",", [2.77779, 2.375, 2.29169]),
    (0x3C, "<", [7.7778, 6.25002, 5.41673]),
    (0x3D, "/", [5.00002, 4.09723, 3.68059]),
    (0x3E, ">", [7.7778, 6.25002, 5.41673]),
    (0x3F, "\\star", [5.00002, 4.09723, 3.68059]),
    (0x40, "\\partial", [5.86458, 4.70103, 4.09203]),
    (0x41, "A", [7.50002, 6.01392, 5.18063]),
    (0x42, "B", [8.0868, 6.383, 5.41846]),
    (0x43, "C", [7.86249, 6.22598, 5.3202]),
    (0x44, "D", [8.55695, 6.75377, 5.72299]),
    (0x45, "E", [7.95831, 6.25557, 5.29173]),
    (0x46, "F", [7.81946, 6.14795, 5.20493]),
    (0x47, "G", [7.86249, 6.22598, 5.3202]),
    (0x48, "H", [9.12497, 7.08473, 5.92366]),
    (0x49, "I", [5.18054, 4.08821, 3.51738]),
    (0x4A, "J", [6.50694, 5.08472, 4.3403]),
    (0x4B, "K", [9.20833, 7.20976, 6.03479]),
    (0x4C, "L", [6.80557, 5.48615, 4.77783]),
    (0x4D, "M", [10.79166, 8.3764, 6.96533]),
    (0x4E, "N", [9.12497, 7.08473, 5.92366]),
    (0x4F, "O", [7.90555, 6.29776, 5.41118]),
    (0x50, "P", [7.80904, 6.16771, 5.24484]),
    (0x51, "Q", [7.90555, 6.29778, 5.41116]),
    (0x52, "R", [7.67015, 6.06009, 5.15805]),
    (0x53, "S", [6.70831, 5.29308, 4.53476]),
    (0x54, "T", [7.23265, 5.79965, 5.02959]),
    (0x55, "U", [7.91803, 6.1982, 5.2646]),
    (0x56, "V", [8.05556, 6.44447, 5.52786]),
    (0x57, "W", [10.83334, 8.59724, 7.264]),
    (0x58, "X", [9.06943, 7.10211, 5.948]),
    (0x59, "Y", [8.02779, 6.42296, 5.51048]),
    (0x5A, "Z", [7.54167, 5.93889, 5.05562]),
    (0x5B, "\\flat", [3.8889, 3.23611, 2.98615]),
    (0x5C, "\\natural", [3.8889, 3.23611, 2.98615]),
    (0x5D, "\\sharp", [3.8889, 3.23611, 2.98615]),
    (0x5E, "\\smile", [10.00002, 7.97224, 6.80565]),
    (0x5F, "\\frown", [10.00002, 7.97224, 6.80565]),
    (0x60, "\\ell", [4.16669, 3.34726, 2.98613]),
    (0x61, "a", [5.28589, 4.33765, 3.87215]),
    (0x62, "b", [4.29166, 3.51666, 3.16667]),
    (0x63, "c", [4.32756, 3.57375, 3.24713]),
    (0x64, "d", [5.20486, 4.16287, 3.69852]),
    (0x65, "e", [4.65627, 3.79411, 3.38509]),
    (0x66, "f", [5.97226, 4.68408, 4.01045]),
    (0x67, "g", [5.12846, 4.15245, 3.68231]),
    (0x68, "h", [5.76158, 4.68175, 4.13779]),
    (0x69, "i", [3.44513, 2.82928, 2.66785]),
    (0x6A, "j", [4.69049, 3.71356, 3.23586]),
    (0x6B, "k", [5.52084, 4.42017, 3.87157]),
    (0x6C, "l", [3.18057, 2.56946, 2.40164]),
    (0x6D, "m", [8.78014, 7.09612, 6.14014]),
    (0x6E, "n", [6.00235, 4.94333, 4.40399]),
    (0x6F, "o", [4.84723, 3.94722, 3.51392]),
    (0x70, "p", [5.03125, 4.12234, 3.69855]),
    (0x71, "q", [4.8229, 3.91634, 3.47397]),
    (0x72, "r", [4.78937, 3.92825, 3.53014]),
    (0x73, "s", [4.6875, 3.77432, 3.35072]),
    (0x74, "t", [3.61111, 3.02084, 2.81831]),
    (0x75, "u", [5.72458, 4.72806, 4.2304]),
    (0x76, "v", [5.20601, 4.25119, 3.79053]),
    (0x77, "w", [7.42825, 5.9734, 5.17947]),
    (0x78, "x", [5.71527, 4.53473, 3.95836]),
    (0x79, "y", [5.2616, 4.30675, 3.8322]),
    (0x7A, "z", [5.0903, 4.10768, 3.62848]),
    (0x7B, "\\imath", [3.22456, 2.79054, 2.66785]),
    (0x7C, "\\jmath", [3.8403, 3.0938, 2.76738]),
    (0x7D, "\\wp", [6.3646, 5.11078, 4.44624]),
];

const STYLES: [&str; 3] = ["", "\\scriptstyle ", "\\scriptscriptstyle "];

/// The widths of the rules in `body`'s rendering, in document order (page,
/// then top), in TeX pt.
fn rule_widths(body: &str) -> Vec<f64> {
    let r = render_one(&format!(
        "\\documentclass[10pt]{{article}}\\begin{{document}}{body}\\end{{document}}"
    ));
    let mut out = Vec::new();
    for (p, page) in r.v2.pages.iter().enumerate() {
        for it in page.resident_items() {
            if let Item::Rule(rl) = it {
                out.push((p, rl.top.to_bp() / BP, rl.width.to_bp() / BP));
            }
        }
    }
    out.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).expect("finite"));
    out.into_iter().map(|(_, _, w)| w).collect()
}

#[test]
fn every_cmmi_glyph_boxes_at_its_pdftex_width() {
    if !lm_available() {
        return;
    }
    let cases: Vec<(u8, String, f64)> = CMMI
        .iter()
        .flat_map(|(slot, src, widths)| {
            STYLES
                .iter()
                .zip(widths)
                .map(move |(style, want)| (*slot, format!("{style}{src}"), *want))
        })
        .collect();
    // One radical per line, all in one document: one render, not 309.
    let body: String = cases
        .iter()
        .map(|(_, src, _)| format!("\\noindent$\\sqrt{{{src}}}$\\par\n"))
        .collect();
    let got = rule_widths(&body);
    assert_eq!(got.len(), cases.len(), "one bar per radical");
    let bad: Vec<String> = cases
        .iter()
        .zip(&got)
        .filter(|((_, _, want), w)| (*w - want).abs() >= TFM_GATE)
        .map(|((slot, src, want), w)| format!("cmmi 0x{slot:02X} {src}: bar {w} pt, pdfTeX {want} pt"))
        .collect();
    assert!(
        bad.is_empty(),
        "{} of {} mismatch:\n{}",
        bad.len(),
        cases.len(),
        bad.join("\n")
    );
}
