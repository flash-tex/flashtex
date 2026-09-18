//! The cancel package's `\cancel`/`\bcancel`/`\xcancel` (#874): the body is
//! set exactly as it is undecorated and the strikes are the picture-mode
//! `\line`s cancel.sty draws through it, as `path_stroke` items.
//!
//! The geometry is cancel.sty's, not corner to corner of the body: the
//! slope is quantised to picture mode's, the length is the body's width plus
//! 2pt (wide bodies) or a fraction of its total height plus 2pt (tall ones),
//! and the line is centred on the body's box. Measured with `\showbox` under
//! pdfTeX (TeX Live 2026, 10pt) -- see `mathtext::cancelled_math_box` -- and
//! against the `line10` glyph origins pdftex writes:
//!
//! | formula | body box | line box | atom |
//! |---|---|---|---|
//! | `$\cancel{x}$` | `4.30554+0.0 x 5.71527` | `10.0+0.0 x 5.0` (1,2) | `7.15277+0.34723 x 5.71527` |
//! | `$\bcancel{x+y}$` | `5.83333+1.94444 x 23.199` | `0.0+6.29749 x 25.199` (4,-1) | `5.09319+1.94444 x 23.199` |
//! | `$\xcancel{\frac{a}{b}}$` | `6.9512+3.44841 x 6.73764` | `12.39+0.0 x 6.1998` (1,2) | `7.9464+3.44841 x 6.73764` |

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, LineCap, PathCmd, PathPaintOp, RunRole};

const BP_PER_PT: f64 = 72.0 / 72.27;

/// `(x, top)` in bp of every math glyph on page 1, in item order.
fn math_glyphs(r: &flashtex_render_pipeline::Rendered) -> Vec<(f64, f64, String)> {
    r.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run) if run.role == RunRole::Math => Some(run),
            _ => None,
        })
        .flat_map(|run| run.glyphs.iter().map(move |g| (g.origin_x.to_bp(), g.baseline_y.to_bp(), run.text.clone())))
        .collect()
}

/// Every stroked path on page 1: `((x0, y0), (x1, y1), width)` in bp.
fn strokes(r: &flashtex_render_pipeline::Rendered) -> Vec<((f64, f64), (f64, f64), f64)> {
    r.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|item| match item {
            Item::Path(p) => match (&p.op, p.commands.as_slice()) {
                (PathPaintOp::Stroke(s), [PathCmd::Move(x0, y0), PathCmd::Line(x1, y1)]) => {
                    assert_eq!(s.cap, LineCap::Round);
                    assert!(s.dash.is_empty());
                    Some(((x0.to_bp(), y0.to_bp()), (x1.to_bp(), y1.to_bp()), s.width.to_bp()))
                }
                other => panic!("a strike is one Move/Line stroke, got {other:?}"),
            },
            _ => None,
        })
        .collect()
}

fn doc(body: &str, cancel: bool) -> String {
    let package = if cancel { "\\usepackage{cancel}\n" } else { "" };
    format!("\\documentclass{{article}}\n{package}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Diagnostics other than the font-resource profile notice every math
/// formula raises in this test environment.
fn diagnostics(r: &flashtex_render_pipeline::Rendered) -> Vec<&flashtex_render_pipeline::display::Diagnostic> {
    r.v2.diagnostics.iter().filter(|d| d.code != "math_resource_profile").collect()
}

/// The three acceptance cases: body glyphs where the undecorated formula
/// puts them, and the strike(s) where cancel.sty's `\line` lands.
#[test]
fn cancel_strikes_match_cancel_sty() {
    if !lm_available() {
        return;
    }
    // (cancelled source, plain source, expected strikes as
    // (len, rise, forward?) in pt from the `\showbox` table above)
    let cases: [(&str, &str, &[(f64, f64, bool)]); 3] = [
        ("$\\cancel{x}$", "$x$", &[(5.0, 10.0, true)]),
        ("$\\bcancel{x+y}$", "$x+y$", &[(25.199, 6.29749, false)]),
        ("$\\xcancel{\\frac{a}{b}}$", "$\\frac{a}{b}$", &[(6.1998, 12.39, true), (6.1998, 12.39, false)]),
    ];
    for (cancelled, plain, expected) in cases {
        let r = render_one(&doc(cancelled, true));
        assert!(diagnostics(&r).is_empty(), "{cancelled}: {:#?}", diagnostics(&r));
        let p = render_one(&doc(plain, false));
        assert!(diagnostics(&p).is_empty(), "{plain}: {:#?}", diagnostics(&p));

        // The body is set undecorated: every glyph in the same place.
        let (got, want) = (math_glyphs(&r), math_glyphs(&p));
        assert_eq!(got.len(), want.len(), "{cancelled}: {got:?} vs {want:?}");
        for (g, w) in got.iter().zip(&want) {
            assert!(near(g.0, w.0, 0.001) && near(g.1, w.1, 0.001), "{cancelled}: glyph {g:?} moved from {w:?}");
        }
        assert!(strokes(&p).is_empty());

        // The strikes: one Move/Line stroke each, 0.4pt, the `\line` extent
        // centred on the body's box.
        let got = strokes(&r);
        assert_eq!(got.len(), expected.len(), "{cancelled}: {got:?}");
        // The body's box from its glyphs is not observable here (advance
        // and ink differ), so the centre is checked against the plain
        // formula's laid-out box through the cancelled one's own leaves:
        // the strike centre must be the same for every strike of an
        // `\xcancel`, and the extent must be the table's.
        let mut centres = Vec::new();
        for (((x0, y0), (x1, y1), width), (len, rise, forward)) in got.iter().zip(expected) {
            assert!(near(*width, 0.4 * BP_PER_PT, 1e-6), "{cancelled}: pen {width}");
            assert!(near(x1 - x0, len * BP_PER_PT, 0.001), "{cancelled}: length {} bp, want {} pt", x1 - x0, len);
            let dy = y1 - y0;
            assert!(near(dy.abs(), rise * BP_PER_PT, 0.001), "{cancelled}: rise {dy} bp, want {} pt", rise);
            // y grows downward: a forward strike (`/`) ends higher.
            assert_eq!(dy < 0.0, *forward, "{cancelled}: direction of {got:?}");
            centres.push(((x0 + x1) / 2.0, (y0 + y1) / 2.0));
        }
        for c in &centres[1..] {
            assert!(near(c.0, centres[0].0, 1e-6) && near(c.1, centres[0].1, 1e-6), "{cancelled}: strikes share a centre: {centres:?}");
        }
    }
}

/// The strike centre against the body: `$\cancel{x}$` at 10pt has the
/// `x` at the formula's left edge, 5.71527pt wide and 4.30554pt tall with
/// no depth, so the line's centre is 2.857635pt right of the glyph origin
/// and 2.15277pt above its baseline (pdfTeX: the (1,2) `line10` glyph sits
/// at x + 0.35764pt, its 10pt box from 2.84723pt below the baseline to
/// 7.15277pt above it).
#[test]
fn cancel_x_strike_is_centred_on_the_body() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc("$\\cancel{x}$", true));
    let [(x, baseline, _)] = math_glyphs(&r)[..] else { panic!("one glyph") };
    let [((x0, y0), (x1, y1), _)] = strokes(&r)[..] else { panic!("one strike") };
    let bp = |pt: f64| pt * BP_PER_PT;
    assert!(near(x0, x + bp(0.357635), 0.001), "left end {x0} vs {}", x + bp(0.357635));
    assert!(near(x1, x + bp(5.357635), 0.001), "right end {x1} vs {}", x + bp(5.357635));
    // Forward: bottom-left to top-right.
    assert!(near(y0, baseline + bp(2.84723), 0.001), "bottom {y0} vs {}", baseline + bp(2.84723));
    assert!(near(y1, baseline - bp(7.15277), 0.001), "top {y1} vs {}", baseline - bp(7.15277));
}

/// A `\cancel` atom keeps cancel.sty's box, not the body's: `\showbox` says
/// `$\cancel{x}$` is 7.15277pt tall and 0.34723pt deep where `$x$` is
/// 4.30554pt tall with no depth. On a page that is invisible until the
/// atom exceeds the line's leading, which a 10pt line box does at
/// `\baselineskip` 12pt only with a deep line above; the `\showbox`
/// measurement is the unit test in `mathtext`.
#[test]
fn cancel_atom_width_is_the_body_width() {
    if !lm_available() {
        return;
    }
    // Two cancelled atoms in a row: the second starts exactly one body
    // width (plus nothing) after the first, as the plain pair does.
    let r = render_one(&doc("$\\cancel{x}\\cancel{x}$", true));
    let p = render_one(&doc("$xx$", false));
    let (got, want) = (math_glyphs(&r), math_glyphs(&p));
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(want.len(), 2, "{want:?}");
    assert!(near(got[1].0 - got[0].0, want[1].0 - want[0].0, 0.001), "{got:?} vs {want:?}");
    assert_eq!(strokes(&r).len(), 2);
}

/// The display-list-v2 wire form: a strike is a `path_stroke` item, the
/// TikZ route's primitive, so the Mac consumer and the PDF writer already
/// draw it.
#[test]
fn cancel_strike_serialises_as_a_path_stroke() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc("$\\cancel{x}$", true));
    let json = flashtex_compiler::json::write(&r.v2.to_json("cancel"));
    assert!(json.contains("\"kind\":\"path_stroke\""), "{json}");
    assert!(!json.contains("\"kind\":\"rule\""), "no axis-aligned rule stands in for the strike: {json}");
}
