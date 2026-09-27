//! A `tikzpicture` environment whose inlines reach a horizontal list -- in a
//! `tabular` cell, or inside a box argument (`\mbox`, `\colorbox`, and
//! `\raisebox`/`\fbox`/`\makebox` once those land) -- is one `\hbox` there,
//! like the `\tikz` shorthand: `\pgfpicture` ends with
//! `\leavevmode\box\pgfpic` in both forms. Before, the environment's body
//! was set as words ("(0,0) rectangle (1,1);") and the picture was lost.
//!
//! Expected positions are pdflatex's (MacTeX 2026, oracle only; read back
//! with tools/visual-oracle/pdftext.py from the PDF of each source), in bp
//! from the page's top-left corner. The cargo test never runs TeX.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const PREAMBLE: &str = "\\documentclass{article}\n\\usepackage{xcolor}\n\\usepackage{tikz}\n\\begin{document}\n";

const TABULAR: &str = "Before the table.\n\n\\begin{tabular}{|c|c|}\n\\hline\na & \\begin{tikzpicture}\\draw (0,0) -- (1,.5);\\end{tikzpicture} \\\\\n\\hline\nb & c \\begin{tikzpicture}[baseline=(n.base)]\\node[draw] (n) {d};\\end{tikzpicture} e \\\\\n\\hline\n\\end{tabular}\n\nAfter the table.\n\\end{document}\n";

const BOXES: &str = "First line of text before the boxes.\n\nCc \\mbox{\\begin{tikzpicture}\\draw (0,0) rectangle (1,1);\\end{tikzpicture}} Dd.\n\nOo \\colorbox{yellow}{\\begin{tikzpicture}\\draw (0,0) rectangle (1,.5);\\end{tikzpicture}} Pp.\n\nQq \\mbox{\\tikz\\draw (0,0) circle (3pt);} Rr \\mbox{A \\tikz[baseline=-3pt]\\fill (0,0) circle (2pt); B} Ss.\n\nThe node \\mbox{\\begin{tikzpicture}[baseline=(n.base)]\\node[draw] (n) {x};\\end{tikzpicture}} sits on the baseline, and this sentence is long enough to wrap onto a second line of the paragraph so line spacing is checked too.\n\nLast line of text after the boxes.\n\\end{document}\n";

/// Renders `body` under [`PREAMBLE`] and checks every expected word once,
/// in order, within `tol` bp of pdflatex, with no picture source leaked as
/// text and at least `paths` painted paths.
fn check(body: &str, expected: &[(&str, f64, f64)], tol: f64, paths: usize) {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let src = format!("{PREAMBLE}{body}");
    let docs = [SourceDocument { path: "main.tex", text: &src }];
    let out = render(&docs, "main.tex", 1, "tikz-in-box", &fonts, &RenderOptions::default());
    for d in &out.v2.diagnostics {
        assert!(d.severity != flashtex_render_pipeline::display::Severity::Error, "error diagnostic: {d:?}");
    }
    let page = &out.v2.pages[0];
    let runs: Vec<(String, f64, f64)> = page
        .resident_items()
        .iter()
        .filter_map(|i| match i {
            Item::GlyphRun(r) if !r.glyphs.is_empty() => Some((r.text.clone(), r.glyphs[0].origin_x.0 as f64 / TICKS_PER_BP, r.glyphs[0].baseline_y.0 as f64 / TICKS_PER_BP)),
            _ => None,
        })
        .collect();
    let texts: Vec<&str> = runs.iter().map(|r| r.0.as_str()).collect();
    assert!(!texts.iter().any(|t| t.contains("draw") || t.contains("rectangle") || t.contains(';')), "picture source leaked as text: {texts:?}");
    let mut from = 0;
    for &(word, x, y) in expected {
        let Some(at) = runs[from..].iter().position(|r| r.0 == word) else {
            panic!("word {word:?} not found after index {from}: {texts:?}");
        };
        let r = &runs[from + at];
        assert!((r.1 - x).abs() <= tol, "{word:?}: x {:.3} vs pdflatex {x:.3}", r.1);
        assert!((r.2 - y).abs() <= tol, "{word:?}: baseline {:.3} vs pdflatex {y:.3}", r.2);
        from += at + 1;
    }
    let found = page.resident_items().iter().filter(|i| matches!(i, Item::Path(_))).count();
    assert!(found >= paths, "expected {paths} painted pictures' paths, found {found}");
}

#[test]
fn a_tikzpicture_in_a_tabular_cell_is_a_box_in_the_cell() {
    // The rows' heights come from the pictures: a 0.5cm line in row one, a
    // drawn node on the baseline in row two. The 0.055 bp is the pinned
    // TikZ reader's node box (0.055 pt shorter than pgf's), the same
    // residual `tikzpicture` blocks and `\tikz` nodes have.
    check(
        TABULAR,
        &[
            ("Before", 148.712, 134.765),
            ("a", 154.967, 150.731),
            ("b", 154.690, 165.154),
            ("c", 172.517, 165.154),
            ("d", 183.785, 165.154),
            ("e", 196.160, 165.154),
            ("After", 148.712, 177.054),
        ],
        0.06,
        2,
    );
}

/// Needs a `vendor/compiler` whose box-argument scan steps over a whole
/// environment (crates/compiler `enclosed_environment_end`); the pinned one
/// closes `\mbox{` at the `\begin` and reports its brace as missing.
#[test]
#[cfg_attr(not(feature = "compiler-box-environments"), ignore = "needs vendor/compiler re-pinned past enclosed_environment_end")]
fn a_tikzpicture_inside_mbox_and_colorbox_is_the_box_content() {
    check(
        BOXES,
        &[
            ("Cc", 148.712, 164.506),
            // After a 1cm square in an `\mbox`.
            ("Dd.", 195.722, 164.506),
            ("Oo", 148.712, 183.063),
            // After the same picture's box plus `\fboxsep` on both sides.
            ("Pp.", 202.807, 183.063),
            ("Qq", 148.712, 195.018),
            ("Rr", 174.737, 195.018),
            ("A", 189.290, 195.018),
            ("B", 207.392, 195.018),
            ("Ss.", 217.766, 195.018),
            // A node picture whose baseline is the node text's.
            ("The", 148.712, 206.973),
            ("x", 196.953, 206.973),
            ("sits", 209.134, 206.973),
            ("onto", 133.768, 218.928),
            ("Last", 148.712, 230.883),
        ],
        0.01,
        5,
    );
}
