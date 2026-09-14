//! A `tikzpicture` in a document becomes display-list-v2 path items plus
//! glyph runs for its node text; the compiler's "unknown environment" text
//! and diagnostics for the picture body are gone, the surrounding text stays.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, PathCmd, PathPaintOp, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const SRC: &str = "Before text.\n\n\\begin{tikzpicture}\n\\draw[->] (0,0) -- (2,1) node[midway,above] {hi};\n\\node[draw] at (0,0) {box};\n\\clip (0,0) circle (1);\n\\fill[red] (0,0) rectangle (1,1);\n\\end{tikzpicture}\n\nAfter text.\n";

#[test]
fn tikzpicture_becomes_paths_and_glyph_runs() {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let docs = [SourceDocument { path: "main.tex", text: SRC }];
    let out = render(&docs, "main.tex", 1, "tikz", &fonts, &RenderOptions::default());
    let page = &out.v2.pages[0];
    let items = page.to_items();
    let texts: Vec<&str> = items
        .iter()
        .filter_map(|i| if let Item::GlyphRun(r) = i { Some(r.text.as_str()) } else { None })
        .collect();
    assert_eq!(texts, ["Before", "text.", "hi", "box", "After", "text."], "{texts:?}");

    let paths: Vec<_> = items.iter().filter_map(|i| if let Item::Path(p) = i { Some(p) } else { None }).collect();
    let strokes = paths.iter().filter(|p| matches!(p.op, PathPaintOp::Stroke(_))).count();
    let fills = paths.iter().filter(|p| matches!(p.op, PathPaintOp::Fill { .. })).count();
    assert_eq!(strokes, 3, "shaft, arrow tip, node border");
    assert_eq!(fills, 1);
    let clipped = paths.iter().find(|p| matches!(p.op, PathPaintOp::Fill { .. })).unwrap();
    assert_eq!(clipped.clips.len(), 1);
    assert_eq!(clipped.paint.r, 1.0);

    // The picture sits between the two paragraphs, at the left margin.
    let ys = |i: &Item| -> Vec<i64> {
        match i {
            Item::Path(p) => p
                .commands
                .iter()
                .filter_map(|c| match *c {
                    PathCmd::Move(_, y) | PathCmd::Line(_, y) => Some(y.0),
                    _ => None,
                })
                .collect(),
            Item::GlyphRun(r) => r.glyphs.iter().map(|g| g.baseline_y.0).collect(),
            _ => vec![],
        }
    };
    let placed = page.to_items();
    let before_y = ys(&placed[0])[0];
    let after_y = *ys(placed.iter().rev().find(|i| matches!(i, Item::GlyphRun(_))).unwrap()).last().unwrap();
    for p in &placed {
        if let Item::Path(_) = p {
            for y in ys(p) {
                assert!(y > before_y && y < after_y, "path y {y} outside ({before_y}, {after_y})");
            }
        }
    }
    let min_x = paths
        .iter()
        .flat_map(|p| p.commands.iter())
        .filter_map(|c| if let PathCmd::Move(x, _) | PathCmd::Line(x, _) = *c { Some(x.0) } else { None })
        .min()
        .unwrap();
    assert!(min_x >= 72 * TICKS_PER_BP as i64 - 1, "{min_x}");

    // No compiler complaints about \draw or the unknown environment.
    for d in &out.v2.diagnostics {
        assert!(d.code != "compiler", "{d:?}");
    }
    let features = out.v2.required_features();
    assert!(features.contains(&"path_stroke") && features.contains(&"path_fill") && features.contains(&"clip"), "{features:?}");
    let json = flashtex_compiler::json::write(&out.v2.to_json("x"));
    assert!(json.contains("\"kind\":\"path_stroke\"") && json.contains("\"clips\""));
}
