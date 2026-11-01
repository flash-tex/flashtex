//! Through-parsing tests for TikZ stroke/fill style keys.
//!
//! Gap: `tests/primitives.rs` checks `LineCap`/`LineJoin` only on a
//! hand-built `Style`, and no test fed raw `rounded corners` (or `line
//! cap`, `line join`, `miter limit`, `dash pattern`, `even odd rule`)
//! through TikZ key parsing. These tests render the exact user-level
//! strings and assert each key reaches the emitted stroke/fill fields.
//!
//! pdflatex oracles (TeX Live 2026, `pdflatex -interaction=nonstopmode`,
//! `\documentclass[tikz,border=0pt]{standalone}`; class/package located via
//! `/Library/TeX/texbin/kpsewhich standalone.cls tikz.sty`):
//! - `\draw[rounded corners=4pt] (0,0) rectangle (2,2);` compiles clean;
//!   log special `papersize=57.30548pt,57.30548pt` (= 2cm + 0.4pt per side
//!   allowance), page `/MediaBox [0 0 57.091 57.091]`; content stream draws
//!   `0.3985 w`, a `56.69362`bp square span with 4 `c` corner arcs of radius
//!   `3.9851`bp (= 4pt).
//! - `\draw[line cap=round,line join=bevel,miter limit=2,
//!   dash pattern=on 4pt off 2pt,even odd rule] (0,0) -- (2,0);` compiles
//!   clean; stream has `1 J` (round cap), `2 j` (bevel join), `2 M`
//!   (miter limit), `[ 3.9851 1.99255 ] 0.0 d` (on 4pt off 2pt).
//! - `\fill[even odd rule] ...` two rectangles compiles clean and fills
//!   with `f*` (even-odd).

use flashtex_vector_graphics::item::Item;
use flashtex_vector_graphics::path::{FillRule, LineCap, LineJoin, PathCommand};
use flashtex_vector_graphics::tikz::{ApproxMeasurer, Tikz};

/// Big points per TeX point (matches `tikz::expr::BP_PER_PT`).
const K: f64 = 72.0 / 72.27;
const PT_PER_CM: f64 = 72.27 / 2.54;
/// PGF's quarter-circle control-arm factor (see `tikz::interp::KAPPA`).
const KAPPA: f64 = 0.5523;

fn render(body: &str) -> flashtex_vector_graphics::tikz::Picture {
    Tikz::new(10.0).render_body("", body, &ApproxMeasurer)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn rounded_corners_4pt_rectangle_has_four_arcs_and_pdflatex_bbox() {
    let p = render(r"\draw[rounded corners=4pt] (0,0) rectangle (2,2);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);

    // Picture size: 2cm plus half the default 0.4pt line width on each side.
    // pdflatex: papersize 57.30548pt, MediaBox 57.091bp.
    let span_pt = 2.0 * PT_PER_CM + 0.4;
    assert!(close(p.width_bp, span_pt * K, 1e-6), "{}", p.width_bp);
    assert!(close(p.height_bp, span_pt * K, 1e-6), "{}", p.height_bp);

    assert_eq!(p.items.len(), 1);
    let Item::PathStroke(s) = &p.items[0] else {
        panic!("expected a stroke, got {:?}", p.items[0])
    };
    assert!(close(s.style.width, 0.4 * K, 1e-12));

    // One closed subpath: M, then per corner (L into the arc, C arc), then Z.
    let cmds = s.path.commands();
    assert_eq!(cmds.len(), 10, "{cmds:?}");
    let cubics = cmds
        .iter()
        .filter(|c| matches!(c, PathCommand::CubicTo(..)))
        .count();
    assert_eq!(cubics, 4, "four corner arcs, pdflatex has 4 `c` ops");
    assert!(matches!(cmds[0], PathCommand::MoveTo(_)));
    assert!(matches!(cmds[9], PathCommand::Close));

    // The curve itself still spans exactly 2cm; the stroke outset is only in
    // the picture bbox above. pdflatex draws a 56.69362bp span.
    let b = s.path.bounds().expect("non-empty path");
    assert!(close(b.width, 2.0 * PT_PER_CM * K, 1e-6), "{b:?}");
    assert!(close(b.height, 2.0 * PT_PER_CM * K, 1e-6), "{b:?}");

    // Each corner arc trims exactly 4pt from the sharp corner along both
    // edges (pdflatex radius 3.9851bp), with PGF control arms trim*KAPPA.
    let r = 4.0 * K;
    let corners = [(b.x, b.y), (b.max_x(), b.y), (b.max_x(), b.max_y()), (b.x, b.max_y())];
    let mut found = 0;
    let mut i = 0;
    while i + 1 < cmds.len() {
        let (PathCommand::LineTo(p1), PathCommand::CubicTo(c1, c2, p2)) =
            (cmds[i], cmds[i + 1])
        else {
            i += 1;
            continue;
        };
        // The sharp corner is the bounds corner nearest to both arc ends.
        let corner = corners
            .iter()
            .min_by(|a, b2| {
                let da = (a.0 - p1.x).hypot(a.1 - p1.y) + (a.0 - p2.x).hypot(a.1 - p2.y);
                let db = (b2.0 - p1.x).hypot(b2.1 - p1.y) + (b2.0 - p2.x).hypot(b2.1 - p2.y);
                da.partial_cmp(&db).unwrap()
            })
            .unwrap();
        let t1 = ((corner.0 - p1.x).powi(2) + (corner.1 - p1.y).powi(2)).sqrt();
        let t2 = ((corner.0 - p2.x).powi(2) + (corner.1 - p2.y).powi(2)).sqrt();
        assert!(close(t1, r, 1e-6), "trim {t1} != 4pt at {corner:?}");
        assert!(close(t2, r, 1e-6), "trim {t2} != 4pt at {corner:?}");
        // Control arms run along the edges with length trim * KAPPA.
        let arm1 = ((c1.x - p1.x).powi(2) + (c1.y - p1.y).powi(2)).sqrt();
        let arm2 = ((c2.x - p2.x).powi(2) + (c2.y - p2.y).powi(2)).sqrt();
        assert!(close(arm1, r * KAPPA, 1e-6), "arm {arm1}");
        assert!(close(arm2, r * KAPPA, 1e-6), "arm {arm2}");
        found += 1;
        i += 2;
    }
    assert_eq!(found, 4, "all four arcs have 4pt radius");
}

#[test]
fn stroke_style_keys_reach_the_emitted_stroke() {
    let p = render(
        r"\draw[line cap=round,line join=bevel,miter limit=2,dash pattern=on 4pt off 2pt,even odd rule] (0,0) -- (2,0);",
    );
    // `even odd rule` is a fill property: accepted on a stroke-only path
    // (pdflatex compiles it clean) without changing the stroke item.
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert_eq!(p.items.len(), 1);
    let Item::PathStroke(s) = &p.items[0] else {
        panic!("expected a stroke, got {:?}", p.items[0])
    };
    // pdflatex stream: `1 J`, `2 j`, `2 M`, `[ 3.9851 1.99255 ] 0.0 d`.
    assert_eq!(s.style.cap, LineCap::Round);
    assert_eq!(s.style.join, LineJoin::Bevel);
    assert!(close(s.style.miter_limit, 2.0, 1e-12));
    let dash = s.style.dash.as_ref().expect("dash pattern");
    assert_eq!(dash.array.len(), 2);
    assert!(close(dash.array[0], 4.0 * K, 1e-9), "{:?}", dash.array);
    assert!(close(dash.array[1], 2.0 * K, 1e-9), "{:?}", dash.array);
    assert!(close(dash.phase, 0.0, 1e-12));
}

#[test]
fn even_odd_rule_reaches_the_emitted_fill() {
    let p = render(r"\fill[even odd rule] (0,0) rectangle (2,2) (0.5,0.5) rectangle (1.5,1.5);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert_eq!(p.items.len(), 1);
    let Item::PathFill(f) = &p.items[0] else {
        panic!("expected a fill, got {:?}", p.items[0])
    };
    // pdflatex fills the same shape with `f*`.
    assert_eq!(f.rule, FillRule::EvenOdd);
    // Outer plus inner rectangle as two closed subpaths.
    let closes = f
        .path
        .commands()
        .iter()
        .filter(|c| matches!(c, PathCommand::Close))
        .count();
    assert_eq!(closes, 2, "{:?}", f.path.commands());

    // Default and explicit nonzero rule stay NonZero.
    let p = render(r"\fill (0,0) rectangle (1,1);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let Item::PathFill(f) = &p.items[0] else {
        panic!("expected a fill, got {:?}", p.items[0])
    };
    assert_eq!(f.rule, FillRule::NonZero);
    let p = render(r"\fill[even odd rule,nonzero rule] (0,0) rectangle (1,1);");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let Item::PathFill(f) = &p.items[0] else {
        panic!("expected a fill, got {:?}", p.items[0])
    };
    assert_eq!(f.rule, FillRule::NonZero);
}
