//! Issue #846: the kernel and amsmath math commands real documents used to
//! drop — `\varrho`, `\bullet`, `\prec`/`\succ` (and `eq`), the big
//! operators, `\backslash`, `\Vert`/`\vert`, LaTeX 2.09's `\bf`/`\rm`/`\cal`
//! as `\@fontswitch` over the rest of the group, `\ensuremath` in both
//! modes, `\mkern`, `\longmapsto` — pinned against pdfTeX's glyph origins.
//!
//! Every expected number below is pdfTeX 1.40.29 (TeX Live 2026),
//! `pdflatex -interaction=batchmode`, two passes, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, on exactly the document `FIXTURE` below, read
//! with `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the
//! page top, the display-list-v2 convention). The oracle never runs here:
//! the table is committed evidence, as in `tests/amsmath_corpus`.
//!
//! What is pinned per glyph: its identity (the character FlashTeX paints,
//! from the family/slot pdfTeX set it in — cmmi for `\varrho`, cmsy for the
//! relations and `\backslash`, cmbx for `{\bf 1}`, cmr for `{\rm Tr}` and
//! `\rm SEP`) and its origin within 0.5 bp in x and y. The two `largesymbols`
//! operators are pinned in x only: pdfTeX draws the Type 1 cmex glyph from
//! the origin at the top of its TFM box while FlashTeX paints the OpenType
//! variant from the baseline, so their y legitimately differ (the same rule
//! `amsmath_corpus/oracle.py` applies to extension glyphs).

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const FIXTURE: &str = "\\documentclass{article}
\\usepackage{amsmath}
\\pagestyle{empty}
\\setlength{\\parindent}{0pt}
\\begin{document}
$a\\varrho b$

$a\\bullet b$

$a\\prec b\\succ c$

$\\bigcup_i A_i \\bigcap B$

$G\\backslash H$

$a\\Vert b\\vert c$

${\\bf 1}_S + {\\rm Tr}\\,\\varrho$

$\\inf_{\\sigma\\in \\rm SEP} x$

${\\cal H}_A$

$\\ensuremath{x}+1$ and \\ensuremath{y^2}

$a\\mkern18mu b$

$x\\longmapsto y$

$a\\preceq b\\succeq c$
\\end{document}
";

/// pdfTeX's glyphs: (painted text, pdfTeX font, x bp, baseline y bp).
/// `y = 0.0` means x only (a cmex operator, see the module comment).
const EXPECTED: &[(&str, &str, f64, f64)] = &[
    // $a\varrho b$ — \varrho is cmmi10 "25 (fontmath.ltx 201).
    ("a", "CMMI10", 133.768, 134.765),
    ("ϱ", "CMMI10", 139.034, 134.765),
    ("b", "CMMI10", 144.185, 134.765),
    // $a\bullet b$ — \mathbin cmsy10 "0F (283): 4mu each side.
    ("a", "CMMI10", 133.768, 146.72),
    ("∙", "CMSY10", 141.246, 146.72),
    ("b", "CMMI10", 148.439, 146.72),
    // $a\prec b\succ c$ — \mathrel cmsy10 "1E/"1F (321/320): 5mu each side.
    ("a", "CMMI10", 133.768, 158.675),
    ("≺", "CMSY10", 141.804, 158.675),
    ("b", "CMMI10", 152.322, 158.675),
    ("≻", "CMSY10", 159.358, 158.675),
    ("c", "CMMI10", 169.876, 158.675),
    // $\bigcup_i A_i \bigcap B$ — cmex10 "53/"54 (252/251), text style:
    // the limits go to the side as scripts.
    ("⋃", "CMEX10", 133.768, 0.0),
    ("i", "CMMI7", 142.071, 173.619),
    ("A", "CMMI10", 147.048, 170.63),
    ("i", "CMMI7", 154.52, 172.125),
    ("⋂", "CMEX10", 159.497, 0.0),
    ("B", "CMMI10", 169.46, 170.63),
    // $G\backslash H$ — \mathord over \setminus's cmsy10 "6E (483): no glue.
    ("G", "CMMI10", 133.768, 182.585),
    ("∖", "CMSY10", 141.602, 182.585),
    ("H", "CMMI10", 146.583, 182.585),
    // $a\Vert b\vert c$ — both \mathord (465-470): no glue at all.
    ("a", "CMMI10", 133.768, 194.54),
    ("‖", "CMSY10", 139.034, 194.54),
    ("b", "CMMI10", 144.016, 194.54),
    ("∣", "CMSY10", 148.291, 194.54),
    ("c", "CMMI10", 151.059, 194.54),
    // ${\bf 1}_S + {\rm Tr}\,\varrho$ — `\bf` is \mathbf for the rest of
    // its group (cmbx10 "31, painted as the bold digit U+1D7CF), the
    // subscript hangs off the group; `\rm Tr` is cmr10 — had `T` stayed
    // cmmi10 (5.84 pt wide), `r` would sit 1.35 bp to the left.
    ("𝟏", "CMBX10", 133.768, 206.496),
    ("S", "CMMI7", 139.497, 207.99),
    ("+", "CMR10", 147.482, 206.496),
    ("T", "CMR10", 157.443, 206.496),
    ("r", "CMR10", 163.811, 206.496),
    ("ϱ", "CMMI10", 169.377, 206.496),
    // $\inf_{\sigma\in \rm SEP} x$ — `\rm` inside the subscript group sets
    // `SEP` in cmr7 and nothing else.
    ("i", "CMR10", 133.768, 218.451),
    ("n", "CMR10", 136.536, 218.451),
    ("f", "CMR10", 142.071, 218.451),
    ("σ", "CMMI7", 145.89, 219.945),
    ("∈", "CMSY7", 150.758, 219.945),
    ("S", "CMR7", 156.126, 219.945),
    ("E", "CMR7", 160.527, 219.945),
    ("P", "CMR7", 165.882, 219.945),
    ("x", "CMMI10", 173.394, 218.451),
    // ${\cal H}_A$ — `\cal` is \mathcal for the rest of the group (cmsy10
    // "48, painted as the script capital U+210B).
    ("ℋ", "CMSY10", 133.768, 230.406),
    ("A", "CMMI7", 142.182, 231.9),
    // $\ensuremath{x}+1$ and \ensuremath{y^2} — the group in math, `$y^2$`
    // in text.
    ("x", "CMMI10", 133.768, 242.361),
    ("+", "CMR10", 141.673, 242.361),
    ("1", "CMR10", 151.634, 242.361),
    ("a", "CMR10", 159.943, 242.361),
    ("n", "CMR10", 164.924, 242.361),
    ("d", "CMR10", 170.459, 242.361),
    ("y", "CMMI10", 179.312, 242.361),
    ("2", "CMR7", 184.555, 238.746),
    // $a\mkern18mu b$ — 18mu = one quad of cmsy10, 10pt.
    ("a", "CMMI10", 133.768, 254.316),
    ("b", "CMMI10", 148.997, 254.316),
    // $x\longmapsto y$ — \mapstochar\longrightarrow (391): the bar piece at
    // the zero-advance \mapstochar's origin, then the arrow head.
    ("x", "CMMI10", 133.768, 266.272),
    ("−", "CMSY10", 142.231, 266.272),
    ("→", "CMSY10", 148.316, 266.272),
    ("y", "CMMI10", 161.049, 266.272),
    // $a\preceq b\succeq c$ — cmsy10 "16/"17 (324/323), \mathrel.
    ("a", "CMMI10", 133.768, 278.227),
    ("⪯", "CMSY10", 141.804, 278.227),
    ("b", "CMMI10", 152.322, 278.227),
    ("⪰", "CMSY10", 159.358, 278.227),
    ("c", "CMMI10", 169.876, 278.227),
];

const TOL_BP: f64 = 0.5;

#[test]
fn kernel_math_commands_match_pdftex_glyph_origins() {
    if !lm_available() {
        return;
    }
    let r = render_one(FIXTURE);
    let errors: Vec<_> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_render_pipeline::display::Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    assert!(errors.is_empty(), "no command may be dropped: {errors:?}");
    assert_eq!(r.v2.pages.len(), 1);
    // Every painted glyph: (cluster text, face, x bp, baseline y bp).
    let mut actual: Vec<(String, String, f64, f64)> = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        let face = r
            .v2
            .fonts
            .iter()
            .find(|f| f.font_id == run.font_id)
            .map(|f| f.postscript_name.clone())
            .unwrap_or_default();
        for g in &run.glyphs {
            let c = &run.clusters[g.cluster as usize];
            let text = run.text[c.text_start_byte..c.text_end_byte].to_string();
            actual.push((text, face.clone(), g.origin_x.to_bp(), g.baseline_y.to_bp()));
        }
    }
    let mut misses = Vec::new();
    for &(text, font, x, y) in EXPECTED {
        let hit = actual.iter().find(|(t, _, ax, ay)| {
            t == text && (ax - x).abs() <= TOL_BP && (y == 0.0 || (ay - y).abs() <= TOL_BP)
        });
        if hit.is_none() {
            let nearest: Vec<_> = actual
                .iter()
                .filter(|(_, _, ax, ay)| (ax - x).abs() <= 3.0 && (y == 0.0 || (ay - y).abs() <= 3.0))
                .map(|(t, f, ax, ay)| format!("{t:?} {f} ({ax:.3}, {ay:.3})"))
                .collect();
            misses.push(format!("{text:?} ({font} at {x}, {y}): nearest {nearest:?}"));
        }
    }
    assert!(
        misses.is_empty(),
        "{} of {} pdfTeX glyphs unmatched within {TOL_BP} bp:\n{}",
        misses.len(),
        EXPECTED.len(),
        misses.join("\n")
    );
}
