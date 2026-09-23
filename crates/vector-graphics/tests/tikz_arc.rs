//! Regression tests for the three TikZ `arc` spellings.
//!
//! Golden gap: `\draw (0,0) arc (0:90:1);`,
//! `arc[start angle=0,end angle=90,radius=1]` and
//! `arc (0:90:2 and 1)` all compiled with 0 feature diagnostics but had no
//! dedicated regression test (only a curve-count assertion inside
//! `rounded_corners_arcs_grids_and_curves`).
//!
//! Oracle: `pdflatex` (TeX Live 2026, `standalone[tikz,border=0pt]`) over a
//! minimal `.tex` file with one picture per form. Each page's content stream
//! holds a single stroked cubic under a translation; the expected points
//! below are that stream mapped into picture space (x identical, y flipped
//! about the oracle page height). Bounding boxes are the oracle `papersize`
//! specials converted with K = 72/72.27.

use flashtex_vector_graphics::item::Item;
use flashtex_vector_graphics::path::PathCommand;
use flashtex_vector_graphics::tikz::{ApproxMeasurer, Picture, Tikz};

/// Acceptance tolerance: endpoints and bbox match pdflatex to 0.01bp.
const TOL_BP: f64 = 0.01;

fn render(body: &str) -> Picture {
    Tikz::new(10.0).render_body("", body, &ApproxMeasurer)
}

fn the_stroke(p: &Picture) -> &flashtex_vector_graphics::PathStroke {
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    let strokes: Vec<_> = p
        .items
        .iter()
        .filter_map(|i| {
            if let Item::PathStroke(s) = i {
                Some(s)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(strokes.len(), 1, "{:?}", p.items);
    strokes.into_iter().next().unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOL_BP
}

/// Oracle page 1/2 stream (bp, y up, under `1 0 0 1 28.546 0.199 cm`):
/// `0 0 m; 0 15.6557, -12.69109 28.3468, -28.3468 28.3468 c`.
/// Page size `papersize=28.85274pt,28.85274pt` = 28.744946bp square.
fn expect_circular(m: (f64, f64), c1: (f64, f64), c2: (f64, f64), b: (f64, f64)) {
    // Start: page (28.546, 0.199); end: page (0.1992, 28.5458).
    assert!(close(m.0, 28.546) && close(m.1, 28.545946), "{m:?}");
    assert!(close(c1.0, 28.546) && close(c1.1, 12.890246), "{c1:?}");
    assert!(close(c2.0, 15.85491) && close(c2.1, 0.199146), "{c2:?}");
    assert!(close(b.0, 0.1992) && close(b.1, 0.199146), "{b:?}");
}

fn cubic_points(cmds: &[PathCommand]) -> ((f64, f64), (f64, f64), (f64, f64), (f64, f64)) {
    assert_eq!(cmds.len(), 2, "{cmds:?}");
    match (cmds[0], cmds[1]) {
        (PathCommand::MoveTo(a), PathCommand::CubicTo(c1, c2, b)) => {
            ((a.x, a.y), (c1.x, c1.y), (c2.x, c2.y), (b.x, b.y))
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn arc_paren_form_matches_pdflatex() {
    let p = render(r"\draw (0,0) arc (0:90:1);");
    // Oracle bbox: 28.85274pt square = 28.744946bp.
    assert!(close(p.width_bp, 28.744946), "{}", p.width_bp);
    assert!(close(p.height_bp, 28.744946), "{}", p.height_bp);
    let s = the_stroke(&p);
    let (m, c1, c2, b) = cubic_points(s.path.commands());
    expect_circular(m, c1, c2, b);
}

#[test]
fn arc_keyval_form_matches_pdflatex_and_paren_form() {
    let p = render(r"\draw (0,0) arc[start angle=0,end angle=90,radius=1];");
    // pdflatex emits a byte-identical content stream for the keyval form
    // (same translation, same cubic), so the same oracle applies.
    assert!(close(p.width_bp, 28.744946), "{}", p.width_bp);
    assert!(close(p.height_bp, 28.744946), "{}", p.height_bp);
    let s = the_stroke(&p);
    let (m, c1, c2, b) = cubic_points(s.path.commands());
    expect_circular(m, c1, c2, b);

    // The two spellings must agree with each other exactly.
    let q = render(r"\draw (0,0) arc (0:90:1);");
    let t = the_stroke(&q);
    assert_eq!(s.path.commands(), t.path.commands());
    assert!((s.path.bounds().unwrap().width - t.path.bounds().unwrap().width).abs() < 1e-12);
}

#[test]
fn arc_elliptical_form_matches_pdflatex() {
    let p = render(r"\draw (0,0) arc (0:90:2 and 1);");
    // Oracle page 3 stream (bp, y up, under `1 0 0 1 56.892 0.199 cm`):
    // `0 0 m; 0 15.6557, -25.38219 28.3468, -56.69362 28.3468 c`.
    // Page size `papersize=57.30548pt,28.85274pt` = 57.091387 x 28.744946bp.
    assert!(close(p.width_bp, 57.091387), "{}", p.width_bp);
    assert!(close(p.height_bp, 28.744946), "{}", p.height_bp);
    let s = the_stroke(&p);
    let (m, c1, c2, b) = cubic_points(s.path.commands());
    // Start: page (56.892, 0.199); end: page (0.19838, 28.5458).
    assert!(close(m.0, 56.892) && close(m.1, 28.545946), "{m:?}");
    assert!(close(c1.0, 56.892) && close(c1.1, 12.890246), "{c1:?}");
    assert!(close(c2.0, 31.50981) && close(c2.1, 0.199146), "{c2:?}");
    assert!(close(b.0, 0.19838) && close(b.1, 0.199146), "{b:?}");
}
