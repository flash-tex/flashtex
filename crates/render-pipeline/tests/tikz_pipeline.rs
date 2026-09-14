//! A `tikzpicture` in a document becomes display-list-v2 path items plus
//! glyph runs for its node text; the compiler's "unknown environment" text
//! and diagnostics for the picture body are gone, the surrounding text stays.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, Origin, PathCmd, PathPaintOp, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const SRC: &str = "Before text.\n\n\\begin{tikzpicture}\n\\draw[->] (0,0) -- (2,1) node[midway,above] {hi};\n\\node[draw] at (0,0) {box};\n\\clip (0,0) circle (1);\n\\fill[red] (0,0) rectangle (1,1);\n\\end{tikzpicture}\n\nAfter text.\n";

#[test]
fn tikzpicture_becomes_paths_and_glyph_runs() {
    // Via `common::lm_available`, not the raw probe: this was the last test in
    // the suite that still skipped silently on a fontless run.
    if !common::lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: SRC }];
    let out = render(&docs, "main.tex", 1, "tikz", &fonts, &RenderOptions::default());
    let page = &out.v2.pages[0];
    let texts: Vec<&str> = page
        .items
        .iter()
        .filter_map(|i| if let Item::GlyphRun(r) = i { Some(r.text.as_str()) } else { None })
        .collect();
    assert_eq!(texts, ["Before", "text.", "hi", "box", "After", "text."], "{texts:?}");

    let paths: Vec<_> = page.items.iter().filter_map(|i| if let Item::Path(p) = i { Some(p) } else { None }).collect();
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
    let before_y = ys(&page.items[0])[0];
    let after_y = *ys(page.items.iter().rev().find(|i| matches!(i, Item::GlyphRun(_))).unwrap()).last().unwrap();
    for p in &page.items {
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

    // No compiler complaints about \draw or the unknown environment: the
    // pipeline's own TikZ reader supersedes them, so `lib.rs`'s `in_picture`
    // filter must drop every compiler diagnostic spanned by the picture.
    //
    // Asks `origin`, not `code`. This used to read `d.code != "compiler"`,
    // which worked only because `from_compiler` stamped that literal on
    // everything; now that real codes are preserved, a leaked `\draw` would
    // arrive as `unsupported_feature` and the old form would pass silently --
    // going dark on exactly the regression it exists to catch.
    for d in &out.v2.diagnostics {
        assert_eq!(d.origin, Origin::Pipeline, "compiler diagnostic leaked out of the picture: {d:?}");
    }
    let features = out.v2.required_features();
    assert!(features.contains(&"path_stroke") && features.contains(&"path_fill") && features.contains(&"clip"), "{features:?}");
    let json = flashtex_compiler::json::write(&out.v2.to_json("x"));
    assert!(json.contains("\"kind\":\"path_stroke\"") && json.contains("\"clips\""));
}
