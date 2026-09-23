//! `\neq`, `\not`, `\notin`, `\mapsto` and `\longmapsto` against pdfTeX,
//! glyph by glyph.
//!
//! pdfTeX builds them from cmsy characters that paint over their
//! neighbours: `\not` (`"36`) and `\mapstochar` (`"37`) have no width and
//! ink to the right of their origin, `\notin` is latex.ltx's `\c@ncel` --
//! cmmi's `/` after a 1mu kern, centred on `\in` in an `\ooalign` as tall
//! as the slash -- and `\mapsto` is `\mapstochar\rightarrow`. FlashTeX
//! paints them from Latin Modern Math, whose nearest glyphs ink elsewhere
//! relative to their origin (U+0338 entirely left of it, the bar end of
//! U+21A6's assembly from 0), so those two overprints are placed to put
//! their ink where the cmsy design inks it. Their expected origin is
//! pdfTeX's plus that constant offset, stated below and derived from the
//! fonts, not measured from FlashTeX; every other glyph is pdfTeX's origin
//! as is.
//!
//! On main, `\notin` was `\not\in` (the cmsy slash, not cmmi's `/`, and a
//! box as tall as `\in`: its subscript sat 0.3 bp too high), `\mapsto` one
//! U+21A6 glyph 0.22 pt short of the arrowhead, `\longmapsto`'s bar the
//! full-height `\mid`, and `\not` centred on the relation after it (0.5 bp
//! off on `\in`).
//!
//! Expected numbers are pdfTeX 1.40.29 (TeX Live 2026, amsmath 2025/07/09
//! v2.17z), `pdflatex -interaction=batchmode`, two passes,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on exactly [`SOURCE`], read
//! with `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the
//! page top). The oracle never runs here: the table is committed evidence.

mod common;

use common::*;

const SOURCE: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
A $a\\neq b$, $a\\ne b$, $a\\not< b$, $a\\not\\in b$ X

B $a\\notin b$, $x_{a\\notin b}$ X

C $a\\mapsto b$, $x_{a\\mapsto b}$, $a\\longmapsto b$ X
\\end{document}
";

const BP_PER_PT: f64 = 1.0 / 1.00375;

/// `\not`: lmsy10's `negationslash` inks x = 139..638/1000 em (centre
/// 388.5), Latin Modern Math's U+0338 -458..-69 (centre -263.5), so its
/// origin sits 0.652 em right of pdfTeX's (6.52 pt at 10 pt).
const NOT_10: f64 = (388.5 + 263.5) / 1000.0 * 10.0 * BP_PER_PT;

/// `\mapstochar`: lmsy10's bar starts 56/1000 em right of the origin (lmsy7:
/// 78), the assembly part's at 0.
const MAPSTO_10: f64 = 56.0 / 1000.0 * 10.0 * BP_PER_PT;
const MAPSTO_7: f64 = 78.0 / 1000.0 * 7.0 * BP_PER_PT;

fn expected() -> Vec<(&'static str, &'static str, f64, f64)> {
    vec![
        ("A", "CMR10", 133.768, 134.765),
        ("a", "CMMI10", 144.557, 134.765),
        ("\u{0338}", "CMSY10", 152.593 + NOT_10, 134.765),
        ("=", "CMR10", 152.593, 134.765),
        ("b", "CMMI10", 163.112, 134.765),
        (",", "CMR10", 167.388, 134.765),
        ("a", "CMMI10", 173.473, 134.765),
        ("\u{0338}", "CMSY10", 181.509 + NOT_10, 134.765),
        ("=", "CMR10", 181.509, 134.765),
        ("b", "CMMI10", 192.027, 134.765),
        (",", "CMR10", 196.303, 134.765),
        ("a", "CMMI10", 202.388, 134.765),
        ("\u{0338}", "CMSY10", 210.424 + NOT_10, 134.765),
        ("<", "CMMI10", 210.424, 134.765),
        ("b", "CMMI10", 220.933, 134.765),
        (",", "CMR10", 225.209, 134.765),
        ("a", "CMMI10", 231.304, 134.765),
        ("\u{0338}", "CMSY10", 239.34 + NOT_10, 134.765),
        ("∈", "CMSY10", 239.34, 134.765),
        ("b", "CMMI10", 248.741, 134.765),
        ("X", "CMR10", 256.345, 134.765),
        ("B", "CMR10", 133.768, 146.72),
        ("a", "CMMI10", 144.142, 146.72),
        // `\notin`: one cluster, the cmmi `/` and the cmsy `\in`.
        ("∉", "CMMI10", 153.284, 146.72),
        ("∉", "CMSY10", 152.178, 146.72),
        ("b", "CMMI10", 161.59, 146.72),
        (",", "CMR10", 165.866, 146.72),
        ("x", "CMMI10", 171.951, 146.72),
        ("a", "CMMI7", 177.647, 148.519),
        ("∉", "CMMI7", 182.84, 148.519),
        ("∉", "CMSY7", 181.971, 148.519),
        ("b", "CMMI7", 187.339, 148.519),
        ("X", "CMR10", 194.659, 146.72),
        ("C", "CMR10", 133.768, 158.675),
        ("a", "CMMI10", 144.281, 158.675),
        // `\mapsto`: one cluster, the `\mapstochar` bar and the arrow.
        ("↦", "CMSY10", 152.316 + MAPSTO_10, 158.675),
        ("↦", "CMSY10", 152.316, 158.675),
        ("b", "CMMI10", 165.049, 158.675),
        (",", "CMR10", 169.325, 158.675),
        ("x", "CMMI10", 175.41, 158.675),
        ("a", "CMMI7", 181.106, 160.169),
        ("↦", "CMSY7", 185.428 + MAPSTO_7, 160.169),
        ("↦", "CMSY7", 185.428, 160.169),
        ("b", "CMMI7", 193.37, 160.169),
        (",", "CMR10", 197.371, 158.675),
        ("a", "CMMI10", 203.456, 158.675),
        ("↦", "CMSY10", 211.492 + MAPSTO_10, 158.675),
        ("−", "CMSY10", 211.492, 158.675),
        ("→", "CMSY10", 217.577, 158.675),
        ("b", "CMMI10", 230.309, 158.675),
        ("X", "CMR10", 237.913, 158.675),
    ]
}

#[test]
fn negations_and_mapsto_paint_pdftex_glyphs() {
    if !lm_available() {
        return;
    }
    assert_pdftex_glyphs(SOURCE, &expected(), 0.5);
}
