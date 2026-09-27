//! Inline math in TikZ node text is laid out by math-layout, from the
//! compiler's parse of the formula, and sits on the node's baseline where
//! pdflatex puts it (feature `tikz-node-math`). Reference positions are
//! pdflatex's (TeX Live 2025, `\documentclass{article}` 10pt), read from the
//! PDF's glyph origins in bp.
#![cfg(feature = "tikz-node-math")]

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};
use flashtex_render_pipeline::Rendered;

const SRC: &str = "\\documentclass{article}\n\\usepackage{tikz}\n\\newcommand{\\vv}[1]{\\vec{#1}}\n\\begin{document}\n\\noindent\n\\begin{tikzpicture}\n  \\node[circle,draw] (a) at (0,0) {$v_1$};\n  \\node[circle,draw] (b) at (2,0) {$v_2$};\n  \\node[draw] (c) at (4,0) {Speed $v$ here};\n  \\node at (6,0) {$\\vv{x}$};\n  \\foreach \\i in {1,2} \\node at (6+\\i,0) {$y_\\i$};\n\\end{tikzpicture}\n\\end{document}\n";

/// Every glyph of the page as (text of its run, role, origin x, baseline y), bp.
fn glyphs(r: &Rendered) -> Vec<(String, RunRole, f64, f64)> {
    r.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|i| if let Item::GlyphRun(run) = i { Some(run) } else { None })
        .flat_map(|run| run.glyphs.iter().map(move |g| (run.text.clone(), run.role, g.origin_x.to_bp(), g.baseline_y.to_bp())))
        .collect()
}

#[test]
fn node_math_is_typeset_on_the_node_baseline() {
    if !lm_available() {
        return;
    }
    let r = render_one(SRC);
    let unsupported: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.message.contains("node text")).collect();
    assert!(unsupported.is_empty(), "{unsupported:#?}");
    let g = glyphs(&r);
    let math: Vec<_> = g.iter().filter(|x| x.1 == RunRole::Math).collect();
    // v 1 | v 2 | v | vec-accent x | y 1 | y 2: math glyphs, not italic text.
    assert!(math.len() >= 10, "{g:#?}");
    assert!(g.iter().all(|x| !x.0.contains('_')), "no raw `_` is printed: {g:#?}");
    let text: Vec<_> = g.iter().filter(|x| x.1 == RunRole::Text).collect();
    let speed = text.iter().find(|x| x.0 == "Speed").expect("Speed");
    let here = text.iter().find(|x| x.0 == "here").expect("here");
    let (v1, v2, v) = (math[0], math[2], math[4]);
    // pdflatex: `Speed` S at 247.57, `v` at 276.63, `here` h at 285.14, all
    // on one baseline.
    assert!((v.2 - speed.2 - 29.06).abs() < 0.05, "{speed:?} {v:?}");
    assert!((here.2 - v.2 - 8.51).abs() < 0.05, "{v:?} {here:?}");
    assert!((v.3 - speed.3).abs() < 0.05, "{speed:?} {v:?}");
    // The circled `v_1` (subscript depth) is centred on y = 0 like `Speed`:
    // its baseline is 1.09 bp higher (284.38 against 285.47); `v_2` is 2cm
    // further right; the subscripts sit 4.82 bp right, 1.49 bp below.
    assert!((speed.3 - v1.3 - 1.09).abs() < 0.05, "{speed:?} {v1:?}");
    assert!((v2.2 - v1.2 - 2.0 * 72.0 / 2.54).abs() < 0.05, "{v1:?} {v2:?}");
    let sub = math[1];
    assert!((sub.2 - v1.2 - 4.82).abs() < 0.05 && (sub.3 - v1.3 - 1.49).abs() < 0.05, "{v1:?} {sub:?}");
}
