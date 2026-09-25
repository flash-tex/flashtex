//! An `array`'s `\hline`/`\cline` (compiler `Nucleus::Matrix::rules`,
//! 75c876176) drawn by `mathgrid::layout_grid_ruled`.
//!
//! Oracle (measured): TeX Live 2026 pdfTeX, `article`, PyMuPDF on
//! ```tex
//! \[ \begin{array}{cc} \hline a & b \\ \hline c & d \\ \hline\hline e & f \\
//!    \cline{2-2} g & h \\ \hline \end{array} \]
//! ```
//! Rule centre lines (bp): 135.960, 148.314, 160.667, 162.660, the cline at
//! 175.014 over x 305.282..321.195 (column 2 only), 186.969; the hlines span
//! x 290.053..321.194. Row baselines `a` 144.528, `c` 156.882, `e` 171.228,
//! `g` 183.183. So a rule sits 8.568 bp above the next baseline, `\hline\hline`
//! puts the second rule `\doublerulesep` (1.993 bp) below the first, and a
//! `\cline` takes no height (`e`→`g` is 11.955 bp, the plain row pitch).

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

const SRC: &str = "\\documentclass{article}\\pagestyle{empty}\\begin{document}Before\n\\[ \\begin{array}{cc} \\hline a & b \\\\ \\hline c & d \\\\ \\hline\\hline e & f \\\\ \\cline{2-2} g & h \\\\ \\hline \\end{array} \\]\n\\end{document}";

#[test]
fn array_rules_sit_where_pdflatex_puts_them() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let r = render_one(SRC);
    let mut rules: Vec<(f64, f64, f64)> = Vec::new(); // (centre y, x0, x1)
    let baseline = |ch: &str| -> f64 {
        for item in r.v2.pages[0].resident_items() {
            if let Item::GlyphRun(run) = item {
                if run.role != RunRole::Math {
                    continue;
                }
                for g in &run.glyphs {
                    let c = &run.clusters[g.cluster as usize];
                    if &run.text[c.text_start_byte..c.text_end_byte] == ch {
                        return g.baseline_y.to_bp();
                    }
                }
            }
        }
        panic!("no {ch}");
    };
    let (a, c, e, g) = (baseline("a"), baseline("c"), baseline("e"), baseline("g"));
    for item in r.v2.pages[0].resident_items() {
        if let Item::Rule(rule) = item {
            rules.push((rule.top.to_bp() + rule.height.to_bp() / 2.0, rule.x.to_bp(), rule.x.to_bp() + rule.width.to_bp()));
        }
    }
    rules.sort_by(|p, q| p.0.total_cmp(&q.0));
    assert_eq!(rules.len(), 6, "{rules:?}");
    let near = |got: f64, want: f64, what: &str| assert!((got - want).abs() < 0.02, "{what}: {got} vs pdflatex {want}");
    // Positions relative to row `a`'s baseline (pdflatex 144.528).
    let rel = [135.960, 148.314, 160.667, 162.660, 175.014, 186.969].map(|y| y - 144.528);
    for (i, want) in rel.iter().enumerate() {
        near(rules[i].0 - a, *want, &format!("rule {i}"));
    }
    near(c - a, 156.882 - 144.528, "c");
    near(e - a, 171.228 - 144.528, "e");
    near(g - e, 183.183 - 171.228, "g (the cline takes no height)");
    // The cline spans column 2 only; the hlines the whole alignment.
    near(rules[4].1 - rules[0].1, 305.282 - 290.053, "cline start");
    near(rules[4].2 - rules[0].1, 321.195 - 290.053, "cline end");
    near(rules[0].2 - rules[0].1, 321.194 - 290.053, "hline width");
}

/// A rule as pdfTeX paints it: (centre y from the page top, x0, x1,
/// thickness), bp.
type PaintedRule = (f64, f64, f64, f64);

/// The math rules on page 1, top to bottom then left to right, and the
/// baseline of the first math glyph spelling each of `chars`.
fn painted(src: &str, chars: &[&str]) -> (Vec<PaintedRule>, Vec<f64>) {
    let r = render_one(src);
    let items: &Vec<Item> = r.v2.pages[0].resident_items();
    let mut rules: Vec<PaintedRule> = items
        .iter()
        .filter_map(|item| match item {
            Item::Rule(rule) => Some((
                rule.top.to_bp() + rule.height.to_bp() / 2.0,
                rule.x.to_bp(),
                rule.x.to_bp() + rule.width.to_bp(),
                rule.height.to_bp(),
            )),
            _ => None,
        })
        .collect();
    rules.sort_by(|p, q| ((p.0 * 100.0).round(), p.1).partial_cmp(&((q.0 * 100.0).round(), q.1)).expect("finite"));
    let baselines = chars
        .iter()
        .map(|ch| {
            items
                .iter()
                .find_map(|item| match item {
                    Item::GlyphRun(run) if run.role == RunRole::Math => run.glyphs.iter().find_map(|g| {
                        let c = &run.clusters[g.cluster as usize];
                        (&run.text[c.text_start_byte..c.text_end_byte] == *ch).then(|| g.baseline_y.to_bp())
                    }),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no math {ch}"))
        })
        .collect();
    (rules, baselines)
}

/// Compares `got` with pdfTeX's rules and baselines, all relative to the
/// first rule's centre and left edge (the page offsets differ only by where
/// the display sits, which is not what these tests are about).
fn assert_like_pdftex(src: &str, want_rules: &[PaintedRule], chars: &[&str], want_baselines: &[f64]) {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let (got, baselines) = painted(src, chars);
    assert_eq!(got.len(), want_rules.len(), "rule count: {got:?}");
    let (gy, gx) = (got[0].0, got[0].1);
    let (wy, wx) = (want_rules[0].0, want_rules[0].1);
    let near = |got: f64, want: f64, what: String| assert!((got - want).abs() < 0.02, "{what}: {got:.3} vs pdflatex {want:.3}");
    for (i, (g, w)) in got.iter().zip(want_rules).enumerate() {
        near(g.0 - gy, w.0 - wy, format!("rule {i} centre"));
        near(g.1 - gx, w.1 - wx, format!("rule {i} x0"));
        near(g.2 - gx, w.2 - wx, format!("rule {i} x1"));
        // pdfTeX writes the thickness to 3 decimals, truncating.
        assert!((g.3 - w.3).abs() < 0.002, "rule {i} thickness: {:.4} vs pdflatex {:.3}", g.3, w.3);
    }
    for ((ch, g), w) in chars.iter().zip(&baselines).zip(want_baselines) {
        near(g - gy, w - wy, format!("baseline of {ch}"));
    }
}

/// Default `\arrayrulewidth` (0.4pt) and `\doublerulesep` (2pt) with the
/// shapes the first test does not reach: a `\cline` above the first row,
/// two `\cline`s at one boundary, a triple `\hline` and a `\cline` below
/// the last row, in three columns.
///
/// Oracle (measured): TeX Live 2026 pdfTeX, the source below, rules read
/// from the page content stream (`0.398 w ... l S`, centre-line strokes).
#[test]
fn array_rule_shapes_and_thickness_match_pdflatex() {
    let src = "\\documentclass{article}\\pagestyle{empty}\\begin{document}Before\n\\[ \\begin{array}{ccc} \\cline{1-2} a & b & c \\\\ \\hline\\hline d & e & f \\\\ \\cline{1-1}\\cline{3-3} g & h & i \\\\ \\hline\\hline\\hline j & k & l \\\\ \\cline{2-3} \\end{array} \\]\n\\end{document}";
    #[rustfmt::skip]
    let rules = [
        (135.960, 282.202, 313.133, 0.398), // \cline{1-2}, above row 1: no height
        (147.915, 282.202, 329.046, 0.398), // \hline
        (149.908, 282.202, 329.046, 0.398), // \hline: \doublerulesep below the first's top
        (162.261, 282.202, 297.431, 0.398), // \cline{1-1}
        (162.261, 313.133, 329.046, 0.398), // \cline{3-3}, same boundary
        (174.217, 282.202, 329.046, 0.398), // \hline\hline\hline
        (176.209, 282.202, 329.046, 0.398),
        (178.202, 282.202, 329.046, 0.398),
        (190.555, 297.431, 329.046, 0.398), // \cline{2-3}, below the last row
    ];
    assert_like_pdftex(src, &rules, &["a", "d", "g", "j"], &[144.129, 158.476, 170.431, 186.770]);
}

/// A document's own `\arrayrulewidth` (1.2pt) and `\doublerulesep` (3pt):
/// every rule is that thick, takes that much height, and `\hline\hline`
/// puts the second rule's top `\doublerulesep` below the first's.
///
/// Oracle (measured): TeX Live 2026 pdfTeX (rules painted as `re f`
/// rectangles 1.196bp tall).
#[test]
fn array_rules_use_the_documents_arrayrulewidth_and_doublerulesep() {
    let src = "\\documentclass{article}\\pagestyle{empty}\\setlength{\\arrayrulewidth}{1.2pt}\\setlength{\\doublerulesep}{3pt}\\begin{document}Before\n\\[ \\begin{array}{cc} \\hline a & b \\\\ \\hline\\hline c & d \\\\ \\cline{2-2} e & f \\\\ \\hline \\end{array} \\]\n\\end{document}";
    #[rustfmt::skip]
    let rules = [
        (136.358, 290.053, 321.194, 1.196),
        (149.509, 290.053, 321.194, 1.196),
        (152.498, 290.053, 321.194, 1.196),
        (165.649, 305.282, 321.195, 1.196), // \cline{2-2}
        (177.604, 290.053, 321.194, 1.196),
    ];
    assert_like_pdftex(src, &rules, &["a", "c", "e"], &[145.325, 161.464, 173.420]);
}

/// `\arrayrulewidth` set inside a group reaches only the array in that
/// group: the next array's rules are the default 0.4pt again.
///
/// Oracle (measured): TeX Live 2026 pdfTeX.
#[test]
fn a_grouped_arrayrulewidth_ends_with_its_group() {
    let src = "\\documentclass{article}\\pagestyle{empty}\\begin{document}Before\n{\\setlength{\\arrayrulewidth}{2pt}\\[ \\begin{array}{cc} \\hline a & b \\\\ \\hline \\end{array} \\]}\n\\[ \\begin{array}{cc} \\hline c & d \\\\ \\hline \\end{array} \\]\n\\end{document}";
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    // Each array against its own first rule: the gap *between* the two
    // displays is display spacing, not what this test is about.
    #[rustfmt::skip]
    let arrays: [([PaintedRule; 2], &str, f64); 2] = [
        ([(137.255, 290.890, 320.357, 1.993), (151.202, 290.890, 320.357, 1.993)], "a", 146.620),
        ([(167.940, 290.913, 320.335, 0.398), (180.294, 290.913, 320.335, 0.398)], "c", 176.508),
    ];
    let (got, baselines) = painted(src, &["a", "c"]);
    assert_eq!(got.len(), 4, "rule count: {got:?}");
    let near = |got: f64, want: f64, what: String| assert!((got - want).abs() < 0.02, "{what}: {got:.3} vs pdflatex {want:.3}");
    for (k, (want, ch, base)) in arrays.iter().enumerate() {
        let mine = &got[2 * k..2 * k + 2];
        for (i, (g, w)) in mine.iter().zip(want).enumerate() {
            near(g.0 - mine[0].0, w.0 - want[0].0, format!("array {k} rule {i} centre"));
            near(g.2 - g.1, w.2 - w.1, format!("array {k} rule {i} width"));
            assert!((g.3 - w.3).abs() < 0.002, "array {k} rule {i} thickness: {:.4} vs pdflatex {:.3}", g.3, w.3);
        }
        near(baselines[k] - mine[0].0, base - want[0].0, format!("baseline of {ch}"));
    }
}
