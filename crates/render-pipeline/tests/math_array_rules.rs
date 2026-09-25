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
    let mut baseline = |ch: &str| -> f64 {
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
