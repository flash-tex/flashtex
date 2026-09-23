//! `\tikz` shorthand pictures in running text are one `\hbox` in the
//! paragraph, sized by the bounding box and split at the `baseline` key,
//! exactly where pdflatex puts them.
//!
//! Expected positions are pdflatex's (MacTeX 2026, oracle only; read back
//! with tools/visual-oracle/pdftext.py from the PDF of `SRC`), in bp from
//! the page's top-left corner. The cargo test never runs TeX.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const SRC: &str = "\\documentclass{article}\n\\usepackage{tikz}\n\\begin{document}\nAlpha \\tikz[baseline=-0.5ex] \\draw[fill=gray] (0,0) rectangle (1em,2ex); beta \\tikz[baseline=(c.base)]{\\node[draw,circle,inner sep=2pt] (c) {1};} gamma \\tikz[baseline]{\\draw (0,-1ex) -- (2em,2ex);} delta.\n\nOvl \\tikz[overlay]{\\draw (0,0) -- (3cm,3cm);} after overlay text.\n\nBig \\tikz{\\draw (0,0) rectangle (2em,3em);} sets the line tall and \\tikz[baseline=(current bounding box.center)]{\\draw (0,0) rectangle (2em,3em);} centred.\n\n\\begin{figure}[h]\n\\centering\nFig \\tikz \\node[draw] {box}; text\n\\caption{A caption with a dot.}\n\\end{figure}\n\nLast line.\n\\end{document}\n";

/// pdflatex's word origins (x, baseline y) in bp.
const EXPECTED: &[(&str, f64, f64)] = &[
    ("Alpha", 148.712, 135.725),
    // After a 1em x 2ex rectangle with `baseline=-0.5ex`.
    ("beta", 192.006, 135.725),
    // Inside a circle node whose base is the line's baseline.
    ("1", 218.996, 135.725),
    ("gamma", 231.872, 135.725),
    // After `baseline` alone (the origin): the line from -1ex hangs below.
    ("delta.", 290.386, 135.725),
    ("Ovl", 148.712, 147.680),
    // An `overlay` picture takes no space.
    ("after", 171.128, 147.680),
    ("overlay", 194.676, 147.680),
    // A 3em-tall box on the baseline pushes this line down.
    ("Big", 148.712, 180.900),
    ("sets", 190.484, 180.900),
    ("and", 263.650, 180.900),
    // The same box centred on the baseline: half above, half below.
    ("centred.", 306.670, 180.900),
    // Inside a figure body: `\centering`, a node box in the line.
    ("Fig", 275.053, 221.956),
    ("box", 296.146, 218.437),
    ("text", 318.760, 221.956),
    ("Last", 148.712, 265.681),
];

#[test]
fn inline_tikz_boxes_match_pdflatex() {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let docs = [SourceDocument { path: "main.tex", text: SRC }];
    let out = render(&docs, "main.tex", 1, "tikz-inline", &fonts, &RenderOptions::default());
    for d in &out.v2.diagnostics {
        assert!(d.severity != flashtex_render_pipeline::display::Severity::Error, "error diagnostic: {d:?}");
        assert!(!d.message.contains("not supported by this compiler"), "compiler complaint survived: {d:?}");
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
    // Every expected word once, in order, within 0.15 bp of pdflatex. The
    // residuals are the pinned TikZ reader's node geometry (a circle node
    // 0.053 bp narrower per side, a drawn box 0.055 pt shorter), the same
    // for `tikzpicture` blocks.
    let mut from = 0;
    for &(word, x, y) in EXPECTED {
        let Some(at) = runs[from..].iter().position(|r| r.0 == word) else {
            panic!("word {word:?} not found after index {from}: {texts:?}");
        };
        let r = &runs[from + at];
        assert!((r.1 - x).abs() <= 0.15, "{word:?}: x {:.3} vs pdflatex {x:.3}", r.1);
        assert!((r.2 - y).abs() <= 0.15, "{word:?}: baseline {:.3} vs pdflatex {y:.3}", r.2);
        from += at + 1;
    }
    // The pictures' paths are in the display list, between the words.
    let paths = page.resident_items().iter().filter(|i| matches!(i, Item::Path(_))).count();
    assert!(paths >= 5, "expected the five painted pictures' paths, found {paths}");
}
