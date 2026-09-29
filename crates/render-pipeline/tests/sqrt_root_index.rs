//! `\sqrt[n]{x}`'s root index (compiler 3cce14260): the compiler hands it
//! over as a zero-width `Space` carrier whose superscript is the index,
//! directly before the `Radical`; the pipeline sets it as math-layout's
//! radical degree (LaTeX `\r@@t`: scriptscript style, `\mkern5mu` in, raised
//! `.6(ht-dp)` of the radical, `\mkern-10mu` back to the sign). Before this
//! the carrier was dropped: no index was painted and everything after the
//! radical sat 1.62 bp to the left.
//!
//! The tables are pdfTeX 3.141592653 (TeX Live 2026) glyph origins in bp
//! (x, baseline from the page top) for these exact sources, `article` 10pt
//! on US letter; pdflatex never runs here.

mod common;

use common::*;

const TOL_BP: f64 = 0.1;

#[test]
fn amsmath_root_index_sits_where_pdftex_puts_it() {
    if !lm_available() {
        return;
    }
    let src = r"\documentclass{article}\usepackage{amsmath}\begin{document}$\sqrt[n]{x}$ A $\frac{\sqrt[3]{a}}{2}$ B $\sqrt[\leftroot{2}\uproot{3}m]{y}$ C\end{document}";
    assert_pdftex_glyphs(
        src,
        &[
            ("n", "CMMI5", 151.480, 131.852),
            ("x", "CMMI10", 158.635, 135.203),
            ("A", "CMR10", 167.651, 135.203),
            // An index inside a text-style fraction's numerator: the
            // radical is script style, its index still scriptscript.
            ("3", "CMR5", 181.906, 128.048),
            ("a", "CMMI7", 187.305, 130.510),
            ("2", "CMR7", 183.647, 138.638),
            ("B", "CMR10", 196.143, 135.203),
            // `\leftroot`/`\uproot` only move the index; the radicand and
            // what follows are where pdfTeX puts them.
            ("y", "CMMI10", 218.173, 135.203),
            ("C", "CMR10", 226.731, 135.203),
        ],
        TOL_BP,
    );
}

#[test]
fn kernel_root_index_in_text_scripts_and_display() {
    if !lm_available() {
        return;
    }
    let src = r"\documentclass{article}\begin{document}$\sqrt[3]{x}$ A $y_{\sqrt[p+q]{z}}$ B\[\sqrt[k]{\frac{u}{v}}\]\end{document}";
    assert_pdftex_glyphs(
        src,
        &[
            ("3", "CMR5", 151.480, 131.414),
            ("x", "CMMI10", 157.637, 134.765),
            ("A", "CMR10", 166.653, 134.765),
            // In a subscript the index is still scriptscript (5pt).
            ("y", "CMMI10", 177.453, 134.765),
            ("p", "CMMI5", 184.597, 134.654),
            ("+", "CMR5", 188.283, 134.654),
            ("q", "CMMI5", 193.404, 134.654),
            ("z", "CMMI7", 198.872, 137.161),
            ("B", "CMR10", 206.784, 134.765),
            // Display: over the tall cmex sign.
            ("k", "CMMI5", 298.818, 151.233),
            ("u", "CMMI10", 308.299, 148.371),
            ("v", "CMMI10", 308.557, 161.944),
        ],
        TOL_BP,
    );
}

/// math-layout's radical has no index shift, so amsmath's `\leftroot{2}
/// \uproot{3}` index is set at the unshifted place (pdfTeX: `m` at
/// (208.181, 131.354), 2mu left and 3mu up of where this draws it) -- painted,
/// and reported once, never silently dropped.
#[test]
fn shifted_root_index_is_painted_and_reported_once() {
    if !lm_available() {
        return;
    }
    let src = r"\documentclass{article}\usepackage{amsmath}\begin{document}$\sqrt[n]{x}$ A $\frac{\sqrt[3]{a}}{2}$ B $\sqrt[\leftroot{2}\uproot{3}m]{y}$ C\end{document}";
    let r = render_one(src);
    let limits: Vec<&str> = r.v2.diagnostics.iter().filter(|d| d.code == "math_limitation").map(|d| d.message.as_str()).collect();
    assert_eq!(limits.len(), 1, "{limits:?}");
    assert!(limits[0].contains("\\leftroot{2}\\uproot{3}"), "{limits:?}");
    let glyphs = painted_glyphs(&r);
    assert!(glyphs.iter().any(|(t, x, _)| t == "m" && (x - 209.288).abs() < TOL_BP), "{glyphs:?}");
}
