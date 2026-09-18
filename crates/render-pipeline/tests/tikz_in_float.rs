//! GH-884: a `tikzpicture` inside a `figure`/`table` float renders its
//! nodes and draws and reserves its real height, exactly as outside a
//! float. The float body is blanked in the source the typesetter reads
//! (`floats::mask`), and the picture compiler re-read the body bytes from
//! that blanked text, so an in-float picture compiled to empty: no node
//! text and no height, and the caption rode up by the picture's height.
//!
//! Expected positions come from pdfTeX (TeX Live 2026) on the same
//! sources; the node-to-caption distances are asserted relatively so the
//! separately-claimed paragraph-mode `tikzpicture` row (#857: the
//! `INLINEC` picture after `After.` starts a new line instead of staying
//! inline, which also shifts absolute page positions) cannot break them.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, Severity};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// First glyph origin of every glyph run, by run text.
fn runs(text: &str) -> (Vec<(String, f64, f64)>, Vec<String>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let out = render(&docs, "main.tex", 1, "tikz-in-float", &fonts, &RenderOptions::default());
    let mut positions = Vec::new();
    for page in &out.v2.pages {
        for it in page.resident_items() {
            if let Item::GlyphRun(run) = it {
                if let Some(g) = run.glyphs.first() {
                    positions.push((run.text.clone(), g.origin_x.to_bp(), g.baseline_y.to_bp()));
                }
            }
        }
    }
    let errors: Vec<String> = out
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect();
    (positions, errors)
}

fn at(positions: &[(String, f64, f64)], text: &str) -> (f64, f64) {
    positions.iter().find(|(t, _, _)| t == text).map(|(_, x, y)| (*x, *y)).unwrap_or_else(|| panic!("{text:?} not rendered; runs: {positions:?}"))
}

const REPRO: &str = "\\documentclass{article}\\usepackage{tikz}\\begin{document}\nBefore.\n\\begin{figure}[h]\\centering\n\\begin{tikzpicture}\\node[draw] at (0,0) {NODEA};\\draw (0,-1) -- (2,-1);\\node at (2,0) {NODEB};\\end{tikzpicture}\n\\caption{Cap}\\end{figure}\nAfter.\n\\begin{tikzpicture}\\node[draw] at (0,0) {INLINEC};\\end{tikzpicture}\n\\end{document}\n";

#[test]
fn tikzpicture_in_a_figure_renders_and_reserves_height() {
    if !common::lm_available() {
        return;
    }
    let (positions, errors) = runs(REPRO);
    assert!(errors.is_empty(), "{errors:?}");
    // The picture's nodes are set, at pdflatex's x (258.94 / 315.84).
    let (ax, ay) = at(&positions, "NODEA");
    let (bx, by) = at(&positions, "NODEB");
    assert!((ax - 258.94).abs() <= 0.5, "NODEA x {ax}");
    assert!((bx - 315.84).abs() <= 0.5, "NODEB x {bx}");
    assert!((ay - by).abs() <= 0.5, "nodes share a baseline: {ay} vs {by}");
    // The float reserves the picture's height: pdflatex sets the nodes
    // 22.28bp below `Before.` and the caption 47.06bp below the nodes.
    let (_, before_y) = at(&positions, "Before.");
    let (_, caption_y) = at(&positions, "Figure");
    assert!((ay - before_y - 22.28).abs() <= 0.5, "nodes {ay} vs Before. {before_y}");
    assert!((caption_y - ay - 47.06).abs() <= 0.5, "caption {caption_y} vs nodes {ay}");
    for word in ["1:", "Cap"] {
        at(&positions, word);
    }
    // #857's row is untouched: the `tikzpicture` after `After.` still
    // starts a new line instead of staying inline with `Before. After.`.
    let (ix, iy) = at(&positions, "INLINEC");
    assert!((ix - 137.29).abs() <= 0.5, "INLINEC x {ix}");
    assert!(iy > caption_y, "INLINEC {iy} still on its own line below the float (caption {caption_y})");
}

#[test]
fn tikzpicture_in_a_table_renders_too() {
    if !common::lm_available() {
        return;
    }
    let src = REPRO.replace("\\begin{figure}[h]", "\\begin{table}[h]").replace("\\end{figure}", "\\end{table}");
    let (positions, errors) = runs(&src);
    assert!(errors.is_empty(), "{errors:?}");
    let (_, ay) = at(&positions, "NODEA");
    let (_, by) = at(&positions, "NODEB");
    assert!((ay - by).abs() <= 0.5, "nodes share a baseline: {ay} vs {by}");
    let (kind_x, _) = at(&positions, "Table");
    assert!((kind_x - 276.98).abs() <= 0.5, "Table caption x {kind_x}");
    let (_, caption_y) = at(&positions, "Table");
    assert!((caption_y - ay - 47.06).abs() <= 0.5, "caption {caption_y} vs nodes {ay}");
}
