//! One pgfplots `\begin{axis}...\end{axis}` with a single `\addplot{x^2}`
//! renders end to end: a framed box plus the sampled blue parabola, with
//! no TikZ diagnostics. The only expected diagnostic besides the pipeline's
//! own display-list notice is the compiler's known
//! "packages pgfplots are recognised but not implemented" notice
//! (`packages.rs` still carries pgfplots as unimplemented).
//!
//! IGNORED until `vendor/vector-graphics` is re-pinned past the axis merge:
//! `render-pipeline` builds against the frozen vendor snapshot
//! (`VENDORING.md`; `scripts/check-vendor-pins.sh` fails the suite if the
//! snapshot is edited), which predates the `axis` environment, so the axis
//! is skipped with `tikz_unsupported` and no paths are emitted. The axis
//! itself is covered meanwhile by the live crate's
//! `tikz::tests::axis_with_addplot_draws_framed_parabola` unit test.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, PathCmd, PathPaintOp};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const SRC: &str = "\\documentclass{article}\n\\usepackage{pgfplots}\n\\begin{document}\nBefore text.\n\n\\begin{tikzpicture}\n\\begin{axis}\n\\addplot{x^2};\n\\end{axis}\n\\end{tikzpicture}\n\nAfter text.\n\\end{document}\n";

fn points(cmds: &[PathCmd]) -> Vec<(i64, i64)> {
    cmds.iter()
        .filter_map(|c| match *c {
            PathCmd::Move(x, y) | PathCmd::Line(x, y) => Some((x.0, y.0)),
            _ => None,
        })
        .collect()
}

#[test]
#[ignore = "needs vendor/vector-graphics re-pinned past the axis merge; run with --ignored after the re-pin"]
fn axis_with_addplot_renders_frame_and_parabola() {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let docs = [SourceDocument { path: "main.tex", text: SRC }];
    let out = render(&docs, "main.tex", 1, "axis", &fonts, &RenderOptions::default());
    assert_eq!(out.v2.pages.len(), 1);
    let page = &out.v2.pages[0];

    let paths: Vec<_> = page
        .resident_items()
        .iter()
        .filter_map(|i| if let Item::Path(p) = i { Some(p) } else { None })
        .collect();
    let strokes: Vec<_> = paths.iter().filter(|p| matches!(p.op, PathPaintOp::Stroke(_))).collect();
    let fills = paths.iter().filter(|p| matches!(p.op, PathPaintOp::Fill { .. })).count();
    assert_eq!(strokes.len(), 2, "axis frame + plot curve");
    assert_eq!(fills, 0);

    // The frame: a closed box with no clip; the plot: a 101-point polyline
    // clipped to the frame.
    let frame = strokes.iter().find(|p| p.clips.is_empty()).expect("unclipped frame stroke");
    let plot = strokes.iter().find(|p| !p.clips.is_empty()).expect("clipped plot stroke");
    assert_eq!(plot.clips.len(), 1);
    assert_eq!(frame.commands.len(), 5, "{:?}", frame.commands);
    assert_eq!(plot.commands.len(), 101, "move + 100 samples");

    assert_eq!((frame.paint.r, frame.paint.g, frame.paint.b), (0.0, 0.0, 0.0));
    assert_eq!((plot.paint.r, plot.paint.g, plot.paint.b), (0.0, 0.0, 1.0));

    let f = points(&frame.commands);
    let (fx0, fx1) = (f.iter().map(|p| p.0).min().unwrap(), f.iter().map(|p| p.0).max().unwrap());
    let (fy0, fy1) = (f.iter().map(|p| p.1).min().unwrap(), f.iter().map(|p| p.1).max().unwrap());
    let p = points(&plot.commands);
    assert_eq!(p.len(), 101);
    // Endpoints sit on the frame's top corners (y down: top is min y).
    assert!((p[0].0 - fx0).abs() <= 8 && (p[0].1 - fy0).abs() <= 8, "{:?} vs frame", p[0]);
    assert!((p[100].0 - fx1).abs() <= 8 && (p[100].1 - fy0).abs() <= 8, "{:?} vs frame", p[100]);
    // Vertex (sample 50, x = 0) at the bottom centre; x strictly increasing.
    assert!((p[50].0 - (fx0 + fx1) / 2).abs() <= 8 && (p[50].1 - fy1).abs() <= 8, "{:?}", p[50]);
    assert!(p.windows(2).all(|w| w[1].0 > w[0].0), "x increases");
    assert!(p[0].1 < p[50].1 && p[50].1 > p[100].1, "parabola opens downward on screen (y down)");

    // Surrounding text is still set; no TikZ complaints.
    let texts: Vec<&str> = page
        .resident_items()
        .iter()
        .filter_map(|i| if let Item::GlyphRun(r) = i { Some(r.text.as_str()) } else { None })
        .collect();
    assert!(texts.contains(&"Before") && texts.contains(&"After"), "{texts:?}");
    for d in &out.v2.diagnostics {
        assert!(
            !matches!(
                d.code.as_str(),
                "tikz_error" | "tikz_unsupported" | "compiler" | "unknown_command" | "unsupported_feature" | "syntax_error"
            ),
            "{d:?}"
        );
    }
    assert!(
        out.v2
            .diagnostics
            .iter()
            .any(|d| d.message.contains("pgfplots") && d.message.contains("recognised but not implemented")),
        "the known pgfplots package notice is still reported: {:?}",
        out.v2.diagnostics
    );
}
