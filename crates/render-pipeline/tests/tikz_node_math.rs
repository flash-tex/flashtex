//! Math in TikZ node text is typeset as math (`crate::tikz::nodes`): the
//! node is sized by the formula's `\hbox` and the formula's glyphs are
//! painted at the text's origin, exactly where pdflatex puts them.
//!
//! Expected positions are pdflatex's (MacTeX 2026, oracle only; read back
//! with tools/visual-oracle/pdftext.py from the PDF of `SRC`), in bp from
//! the page's top-left corner. The cargo test never runs TeX.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const SRC: &str = "\\documentclass{article}\n\\usepackage{tikz}\n\\begin{document}\nStart here.\n\n\\begin{tikzpicture}\n\\node[draw] (a) at (0,0) {$\\alpha + \\beta$};\n\\node (b) at (3,0) {text $x^2$ more};\n\\draw (a) -- (b) node[midway,above] {$\\gamma$};\n\\end{tikzpicture}\n\nInline \\tikz{\\node[draw] {$x^2$};} and \\tikz[baseline=(m.base)]{\\node (m) {$\\frac{1}{2}$};} done.\n\\end{document}\n";

/// pdflatex's glyph-run origins (x, baseline y) in bp, in page order.
const EXPECTED: &[(&str, f64, f64)] = &[
    ("Start", 148.712, 134.765),
    // `$\alpha + \beta$` in a drawn node: three math runs.
    ("α", 152.232, 151.318),
    ("+", 160.857, 151.318),
    ("β", 170.817, 151.318),
    // Text and math mixed in one node: `text $x^2$ more`.
    ("text", 221.719, 152.883),
    ("x", 242.471, 152.883),
    ("2", 248.168, 149.267),
    ("more", 255.958, 152.883),
    // A path node above the line between the two.
    ("γ", 196.493, 143.371),
    // Inline pictures whose nodes hold math; the second on `(m.base)`.
    ("Inline", 148.712, 172.921),
    ("x", 180.183, 169.401),
    ("2", 185.877, 165.786),
    ("and", 197.187, 172.921),
    // `\frac{1}{2}`: the numerator's origin (the run holds both digits).
    ("12", 221.075, 168.998),
    ("done.", 232.883, 172.921),
];

#[test]
fn math_in_node_text_matches_pdflatex() {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let docs = [SourceDocument { path: "main.tex", text: SRC }];
    let out = render(&docs, "main.tex", 1, "tikz-node-math", &fonts, &RenderOptions::default());
    for d in &out.v2.diagnostics {
        assert!(d.severity != flashtex_render_pipeline::display::Severity::Error, "error diagnostic: {d:?}");
        assert!(!d.message.contains("math in TikZ node text"), "the reader's italic fallback ran: {d:?}");
        assert!(!d.message.contains("is not supported; omitted"), "a math command was dropped: {d:?}");
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
    assert!(!texts.iter().any(|t| t.contains('$') || t.contains('^') || t.contains("frac")), "formula source leaked as text: {texts:?}");
    let mut from = 0;
    for &(word, x, y) in EXPECTED {
        let Some(at) = runs[from..].iter().position(|r| r.0 == word) else {
            panic!("run {word:?} not found after index {from}: {texts:?}");
        };
        let r = &runs[from + at];
        assert!((r.1 - x).abs() <= 0.05, "{word:?}: x {:.3} vs pdflatex {x:.3}", r.1);
        assert!((r.2 - y).abs() <= 0.05, "{word:?}: baseline {:.3} vs pdflatex {y:.3}", r.2);
        from += at + 1;
    }
}
