//! A `tikzpicture` inside a `figure` float (#884).
//!
//! The float environments are blanked to spaces before the compiler parses
//! the document (`floats::mask`), and `typeset::picture_block` used to
//! compile the picture from those masked bytes: an empty picture with no
//! nodes and no height, so the caption moved up by the picture's height.
//! The picture is now compiled from the unmasked sources
//! (`typeset::Context::set_sources`).
//!
//! Oracle: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, MacTeX), the
//! word origins of its own PDF read by `tools/visual-oracle/pdftext.py`
//! (x and baseline in bp from the page's top-left). Measurement only; no
//! TeX runs here.

mod common;

use common::lm_available;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const SRC: &str = "\\documentclass{article}\\usepackage{tikz}\\begin{document}\n\
Before.\n\
\\begin{figure}[h]\\centering\n\
\\begin{tikzpicture}\\node[draw] at (0,0) {NODEA};\\draw (0,-1) -- (2,-1);\\node at (2,0) {NODEB};\\end{tikzpicture}\n\
\\caption{Cap}\\end{figure}\n\
After.\n\
\\end{document}\n";

const TOL_BP: f64 = 0.5;

/// `(text, x, baseline)` of every glyph run on page 1, in bp from the top-left.
fn runs(src: &str) -> Vec<(String, f64, f64)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: src }];
    let out = render(&docs, "main.tex", 1, "float-tikz", &fonts, &RenderOptions::default());
    assert_eq!(out.v2.pages.len(), 1, "one page");
    out.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|i| match i {
            Item::GlyphRun(r) => {
                let g = r.glyphs.first()?;
                Some((r.text.clone(), g.origin_x.0 as f64 / TICKS_PER_BP, g.baseline_y.0 as f64 / TICKS_PER_BP))
            }
            _ => None,
        })
        .collect()
}

fn find<'a>(runs: &'a [(String, f64, f64)], text: &str) -> &'a (String, f64, f64) {
    runs.iter().find(|r| r.0 == text).unwrap_or_else(|| panic!("no glyph run {text:?} in {runs:?}"))
}

#[test]
fn a_tikzpicture_inside_a_figure_sets_its_nodes_and_takes_its_height() {
    if !lm_available() {
        return;
    }
    let runs = runs(SRC);
    // pdflatex, `tools/visual-oracle/pdftext.py` on its PDF:
    //   258.939 157.047 NODEA    315.840 157.047 NODEB
    //   274.961 204.107 Figure   148.712 134.765 Before.
    let oracle = [("Before.", 148.712, 134.765), ("NODEA", 258.939, 157.047), ("NODEB", 315.840, 157.047), ("Figure", 274.961, 204.107)];
    for (text, x, y) in oracle {
        let got = find(&runs, text);
        assert!((got.1 - x).abs() <= TOL_BP && (got.2 - y).abs() <= TOL_BP, "{text}: got ({:.3}, {:.3}), pdflatex ({x:.3}, {y:.3})", got.1, got.2);
    }
    // The picture's box, not just its text, is in the float: the caption
    // sits below the nodes by more than a line, and the `\draw` is a path.
    let nodes = find(&runs, "NODEA").2;
    let caption = find(&runs, "Figure").2;
    assert!(caption - nodes > 30.0, "caption baseline {caption:.3} vs node baseline {nodes:.3}");
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: SRC }];
    let out = render(&docs, "main.tex", 1, "float-tikz", &fonts, &RenderOptions::default());
    let paths = out.v2.pages[0].resident_items().iter().filter(|i| matches!(i, Item::Path(_))).count();
    assert!(paths >= 2, "node border and the \\draw line: {paths} paths");
}

#[test]
fn the_same_picture_outside_a_float_is_unchanged() {
    if !lm_available() {
        return;
    }
    // The float removed: the picture is a paragraph of its own, as before.
    let src = SRC.replace("\\begin{figure}[h]\\centering\n", "\n").replace("\\caption{Cap}\\end{figure}\n", "\n");
    let runs = runs(&src);
    assert!(runs.iter().any(|r| r.0 == "NODEA") && runs.iter().any(|r| r.0 == "NODEB"), "{runs:?}");
    assert!(!runs.iter().any(|r| r.0 == "Figure"));
}
