//! `\cancel`, `\bcancel` and `\xcancel` strike the argument's box with
//! diagonal rule(s): one bottom-left to top-right, one top-left to
//! bottom-right, or both. No diagnostics; the marker rule carrying the
//! body's extents never leaks as a painted rectangle.

mod common;

use common::*;
use flashtex_compiler::math::FRACTION_RULE_EM;
use flashtex_render_pipeline::display::{
    Diagnostic, GlyphRun, Item, LineCap, PathCmd, PathItem, PathPaintOp, Rule, RunRole, Severity,
    Tick,
};

/// The default article body size these renders set math at (12pt): the
/// strike thickness is `FRACTION_RULE_EM` of it, like the `Frame` rules'.
const BODY_PT: f64 = 12.0;

struct Formula {
    glyphs: Vec<GlyphRun>,
    paths: Vec<PathItem>,
    rules: Vec<Rule>,
}

fn render_formula(body: &str) -> (Vec<Diagnostic>, Formula) {
    let src = format!("\\begin{{document}}${body}$\\end{{document}}");
    let r = render_one(&src);
    let mut glyphs = Vec::new();
    let mut paths = Vec::new();
    let mut rules = Vec::new();
    for page in &r.v2.pages {
        for item in &page.items {
            match item {
                Item::GlyphRun(run) if run.role == RunRole::Math => glyphs.push(run.clone()),
                Item::Path(p) => paths.push(p.clone()),
                Item::Rule(rule) => rules.push(rule.clone()),
                _ => {}
            }
        }
    }
    (r.v2.diagnostics, Formula { glyphs, paths, rules })
}

/// No errors, and nothing mentioning the strike commands: the mere
/// `math_resource_profile` font warning (also present for `\boxed`) is fine.
fn assert_no_strike_diagnostics(diagnostics: &[Diagnostic], src: &str) {
    for d in diagnostics {
        assert!(!matches!(d.severity, Severity::Error), "{src}: unexpected error: {d:?}");
        assert!(
            !d.message.contains("cancel")
                && !d.message.contains("not supported")
                && !d.message.contains("unsupported"),
            "{src}: strike diagnostic: {d:?}"
        );
    }
}

fn assert_solid_strike(path: &PathItem, src: &str) {
    let PathPaintOp::Stroke(stroke) = &path.op else {
        panic!("{src}: strike is not stroked: {:?}", path.op);
    };
    assert_eq!(stroke.width, Tick::from_tex_pt(FRACTION_RULE_EM * BODY_PT), "{src}: thickness");
    assert_eq!(stroke.cap, LineCap::Butt, "{src}: cap");
    assert!(stroke.dash.is_empty(), "{src}: dash");
    assert!(path.clips.is_empty(), "{src}: clips");
}

#[test]
fn cancel_strikes_bottom_left_to_top_right() {
    if !lm_available() {
        return;
    }
    let (diagnostics, f) = render_formula(r"\cancel{x}");
    assert_no_strike_diagnostics(&diagnostics, r"\cancel{x}");
    assert!(f.rules.is_empty(), "marker must not paint as a rule: {:?}", f.rules);
    assert_eq!(f.paths.len(), 1, "{:?}", f.paths);
    assert_eq!(f.glyphs.len(), 1, "{:?}", f.glyphs);
    let path = &f.paths[0];
    assert_solid_strike(path, r"\cancel{x}");
    let run = &f.glyphs[0];
    assert_eq!(run.glyphs.len(), 1, "{run:?}");
    let g = &run.glyphs[0];
    let [PathCmd::Move(bl_x, bl_y), PathCmd::Line(tr_x, tr_y)] = path.commands.as_slice() else {
        panic!("expected one diagonal: {:?}", path.commands)
    };
    // Corner to corner of the argument's box: the body starts at the glyph's
    // own origin, the bottom sits on its baseline (`x` has no depth) and the
    // right edge lands exactly one advance over.
    assert_eq!(*bl_x, g.origin_x, "{run:?}");
    assert_eq!(*bl_y, g.baseline_y, "{run:?}");
    assert_eq!(tr_x.0, g.origin_x.0 + g.advance_x.0, "{run:?}");
    assert!(tr_y.0 < bl_y.0, "non-degenerate diagonal");
}

#[test]
fn bcancel_strikes_top_left_to_bottom_right() {
    if !lm_available() {
        return;
    }
    let (diagnostics, f) = render_formula(r"\bcancel{y}");
    assert_no_strike_diagnostics(&diagnostics, r"\bcancel{y}");
    assert!(f.rules.is_empty(), "marker must not paint as a rule: {:?}", f.rules);
    assert_eq!(f.paths.len(), 1, "{:?}", f.paths);
    assert_eq!(f.glyphs.len(), 1, "{:?}", f.glyphs);
    let path = &f.paths[0];
    assert_solid_strike(path, r"\bcancel{y}");
    let g = &f.glyphs[0].glyphs[0];
    let [PathCmd::Move(tl_x, tl_y), PathCmd::Line(br_x, br_y)] = path.commands.as_slice() else {
        panic!("expected one diagonal: {:?}", path.commands)
    };
    // Top-left corner at the body's own left edge, bottom-right below the
    // baseline (`y` has a descender).
    assert_eq!(*tl_x, g.origin_x, "{:?}", f.glyphs[0]);
    assert!(tl_y.0 < g.baseline_y.0, "top above baseline");
    assert!(br_x.0 > g.origin_x.0, "non-degenerate diagonal");
    assert!(br_y.0 > g.baseline_y.0, "bottom below baseline");
}

#[test]
fn xcancel_strikes_both_diagonals_over_the_whole_body() {
    if !lm_available() {
        return;
    }
    let (diagnostics, f) = render_formula(r"\xcancel{xy}");
    assert_no_strike_diagnostics(&diagnostics, r"\xcancel{xy}");
    assert!(f.rules.is_empty(), "marker must not paint as a rule: {:?}", f.rules);
    assert_eq!(f.paths.len(), 1, "{:?}", f.paths);
    assert_eq!(f.glyphs.len(), 1, "{:?}", f.glyphs);
    let path = &f.paths[0];
    assert_solid_strike(path, r"\xcancel{xy}");
    let run = &f.glyphs[0];
    assert_eq!(run.glyphs.len(), 2, "{run:?}");
    let [PathCmd::Move(bl_x, bl_y), PathCmd::Line(tr_x, tr_y), PathCmd::Move(tl_x, tl_y), PathCmd::Line(br_x, br_y)] =
        path.commands.as_slice()
    else {
        panic!("expected two diagonals: {:?}", path.commands)
    };
    // The strike spans the whole two-glyph body from the first glyph's own
    // origin: both diagonals share the same box corners.
    assert_eq!(*bl_x, run.glyphs[0].origin_x, "{run:?}");
    assert_eq!(*tl_x, run.glyphs[0].origin_x, "{run:?}");
    assert_eq!(*tr_x, *br_x, "shared right edge");
    assert_eq!(*bl_y, *br_y, "shared bottom edge");
    assert_eq!(*tl_y, *tr_y, "shared top edge");
    assert!(tl_y.0 < bl_y.0, "non-degenerate diagonals");
    assert!(bl_x.0 < tr_x.0, "non-degenerate diagonals");
    // The box composes: `x`'s advance plus `y`'s struck width from the
    // `\bcancel{y}` render above (the `y` box carries its italic correction).
    let (_, single) = render_formula(r"\bcancel{y}");
    let [PathCmd::Move(y_x0, _), PathCmd::Line(y_x1, _)] = single.paths[0].commands.as_slice()
    else {
        unreachable!()
    };
    assert_eq!(tr_x.0 - bl_x.0, run.glyphs[0].advance_x.0 + (y_x1.0 - y_x0.0), "{run:?}");
    // Same body height as the single-glyph strikes, same depth as `\bcancel{y}`.
    let (_, cancel) = render_formula(r"\cancel{x}");
    let [_, PathCmd::Line(_, cancel_top)] = cancel.paths[0].commands.as_slice() else {
        unreachable!()
    };
    assert_eq!(*tl_y, *cancel_top, "shared top");
    let [_, PathCmd::Line(_, single_bottom)] = single.paths[0].commands.as_slice() else {
        unreachable!()
    };
    assert_eq!(*bl_y, *single_bottom, "shared bottom");
}

#[test]
fn cancel_leaves_the_body_where_plain_math_puts_it() {
    if !lm_available() {
        return;
    }
    // The wrapper adds no shift or padding: the struck glyph sits exactly
    // where the same body without a strike sits.
    let (_, struck) = render_formula(r"\cancel{y}");
    let (_, plain) = render_formula("y");
    assert_eq!(struck.glyphs.len(), 1, "{:?}", struck.glyphs);
    assert_eq!(plain.glyphs.len(), 1, "{:?}", plain.glyphs);
    assert_eq!(
        struck.glyphs[0].glyphs[0].origin_x, plain.glyphs[0].glyphs[0].origin_x,
        "no horizontal shift"
    );
    assert_eq!(
        struck.glyphs[0].glyphs[0].baseline_y, plain.glyphs[0].glyphs[0].baseline_y,
        "no vertical shift"
    );
}
