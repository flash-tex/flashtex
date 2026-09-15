//! `\cancel`, `\bcancel` and `\xcancel` strike the argument with diagonal
//! rule(s) transcribed from `cancel.sty` v2.2's `\@can@slash`: a quantized
//! slope from a small table (not corner-to-corner), centred on the body's
//! centre horizontally and the math axis vertically, drawn 0.4pt in every
//! style. No diagnostics; the marker rule carrying the slash rect never
//! leaks as a painted rectangle.
//!
//! None of these tests can run until the crate builds again (the vendor
//! re-pin past the sibling compiler PR has not landed), so every numeric
//! expectation below is hand-derived in the slice-2 checkin from the
//! embedded CM metrics (`cmmi12`/`cmsy10` scaled to 12pt). Position asserts
//! allow +-2 ticks: each endpoint and its anchor rounds once through
//! `Tick::from_tex_pt` on top of a shared page offset, so a same-render
//! difference can sit 1 tick off the offset-free value.

mod common;

use common::*;
use flashtex_render_pipeline::display::{
    Diagnostic, GlyphRun, Item, LineCap, PathCmd, PathItem, PathPaintOp, Rule, RunRole, Severity,
    Tick,
};

/// `Tick::from_tex_pt` for one expectation; the call sites name the point
/// value they came from so the checkin arithmetic can be followed.
fn pt_ticks(pt: f64) -> Tick {
    Tick::from_tex_pt(pt)
}

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

/// The strike is a fixed 0.4pt stroke (`cancel.sty`'s `\thinlines` at
/// `\unitlength 1pt`), in every style and size.
fn assert_solid_strike(path: &PathItem, src: &str) {
    let PathPaintOp::Stroke(stroke) = &path.op else {
        panic!("{src}: strike is not stroked: {:?}", path.op);
    };
    assert_eq!(stroke.width, pt_ticks(0.4), "{src}: thickness");
    assert_eq!(stroke.cap, LineCap::Butt, "{src}: cap");
    assert!(stroke.dash.is_empty(), "{src}: dash");
    assert!(path.clips.is_empty(), "{src}: clips");
}

/// Same-render endpoint difference against a hand-derived offset, in ticks.
/// See the module docs for the +-2 tick tolerance.
fn assert_offset(actual: Tick, anchor: Tick, expected: Tick, what: &str) {
    let got = actual.0 - anchor.0;
    assert!(
        (got - expected.0).abs() <= 2,
        "{what}: offset {got} ticks, expected {} +- 2",
        expected.0
    );
}

/// The strike's slope (rise over run from its tick endpoints) pins the
/// quantized `cancel.sty` slope table without needing any anchor.
fn strike_slope(cmds: &[PathCmd]) -> f64 {
    let (x0, y0, x1, y1) = match cmds {
        [PathCmd::Move(x0, y0), PathCmd::Line(x1, y1)] => (x0.0, y0.0, x1.0, y1.0),
        _ => panic!("expected one diagonal: {cmds:?}"),
    };
    ((y0 - y1).abs() as f64) / ((x1 - x0).abs() as f64)
}

fn assert_slope(cmds: &[PathCmd], expected: f64, src: &str) {
    let got = strike_slope(cmds);
    assert!(
        (got - expected).abs() / expected < 1e-3,
        "{src}: slope {got}, expected {expected}"
    );
}

#[test]
fn cancel_strikes_quantized_slope_over_x() {
    if !lm_available() {
        return;
    }
    // Body `x` at 12pt: width 6.67703pt, total 5.16667pt. Clamped total 6
    // < width, so the wide branch: level max(6.67703, 8) = 8,
    // k = floor(5*6/8) = 3, slope (4,3), run 8+2 = 10pt, rise 7.5pt.
    // Marker centred on the body centre ((6.67703-10)/2 = -1.66148376pt)
    // and on the 3pt text axis (top 3+3.75 = 6.75pt, depth 3.75-3 = 0.75pt).
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
    // The slash sticks out past the narrow body on both sides (the 8pt
    // floor dominates) and past it vertically (the box grows to fit).
    assert_offset(*bl_x, g.origin_x, pt_ticks(-1.6614837646484375), "left edge");
    assert_offset(*tr_x, g.origin_x, pt_ticks(8.338516235351562), "right edge");
    assert_offset(*tr_y, g.baseline_y, pt_ticks(-6.75), "top edge");
    assert_offset(*bl_y, g.baseline_y, pt_ticks(0.75), "bottom edge");
    assert_slope(path.commands.as_slice(), 0.75, r"\cancel{x}");
    assert!(tr_x.0 > bl_x.0 && tr_y.0 < bl_y.0, "rises to the right");
}

#[test]
fn bcancel_strikes_quantized_slope_over_y() {
    if !lm_available() {
        return;
    }
    // Body `y` at 12pt: width 5.72910 + 0.43056 italic = 6.15965pt, total
    // 5.16667 + 2.33331 = 7.5pt. Clamped total 7.5 >= width, so the tall
    // branch: extent max(7.5, 8) + 2 = 10pt, k = floor(5*6.15965/10) = 3,
    // slope (3,4), run 0.75*10 = 7.5pt, rise 10pt. Marker x offset
    // (6.15965-7.5)/2 = -0.67017365pt; top 3+5 = 8pt, depth 5-3 = 2pt
    // (the body's own 2.33331pt depth wins, so the box keeps it).
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
    assert_offset(*tl_x, g.origin_x, pt_ticks(-0.6701736450195312), "left edge");
    assert_offset(*br_x, g.origin_x, pt_ticks(6.829826354980469), "right edge");
    assert_offset(*tl_y, g.baseline_y, pt_ticks(-8.0), "top edge");
    assert_offset(*br_y, g.baseline_y, pt_ticks(2.0), "bottom edge");
    assert_slope(path.commands.as_slice(), 4.0 / 3.0, r"\bcancel{y}");
    assert!(tl_y.0 < g.baseline_y.0, "top above baseline");
    assert!(br_y.0 > g.baseline_y.0, "bottom below baseline");
}

#[test]
fn xcancel_strikes_both_diagonals_over_the_whole_body() {
    if !lm_available() {
        return;
    }
    // Body `xy`: width 6.67703 + 5.72910 + 0.43056 = 12.83669pt, total
    // 7.5pt. Wide branch: k = floor(5*7.5/12.83669) = 2, slope (2,1),
    // run 12.83669+2 = 14.83669pt (exactly 1pt past each side),
    // rise 7.41834pt. Marker top 3+3.70917130 = 6.70917130pt, depth
    // 0.70917130pt (body depth 2.33331pt wins).
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
    // Both diagonals share the one marker rect.
    assert_eq!(*bl_x, *tl_x, "shared left edge");
    assert_eq!(*tr_x, *br_x, "shared right edge");
    assert_eq!(*bl_y, *br_y, "shared bottom edge");
    assert_eq!(*tl_y, *tr_y, "shared top edge");
    let anchor_x = run.glyphs[0].origin_x;
    let anchor_y = run.glyphs[0].baseline_y;
    assert_offset(*bl_x, anchor_x, pt_ticks(-1.0), "left edge");
    assert_offset(*tr_x, anchor_x, pt_ticks(13.836685180664062), "right edge");
    assert_offset(*tl_y, anchor_y, pt_ticks(-6.709171295166016), "top edge");
    assert_offset(*bl_y, anchor_y, pt_ticks(0.7091712951660156), "bottom edge");
    assert_slope(&path.commands[0..2], 0.5, r"\xcancel{xy} /");
    assert_slope(&path.commands[2..4], 0.5, r"\xcancel{xy} \");
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

#[test]
fn cancel_strikes_an_empty_body() {
    if !lm_available() {
        return;
    }
    // Degenerate body: width clamps to 2pt, total to 6pt, so the tall
    // branch runs with extent max(6, 8) + 2 = 10pt, k = floor(5*2/10) = 1,
    // slope (1,4), run 0.25*10 = 2.5pt, rise 10pt. A slash always appears.
    let (diagnostics, f) = render_formula(r"\cancel{}");
    assert_no_strike_diagnostics(&diagnostics, r"\cancel{}");
    assert!(f.rules.is_empty(), "marker must not paint as a rule: {:?}", f.rules);
    assert_eq!(f.paths.len(), 1, "{:?}", f.paths);
    let path = &f.paths[0];
    assert_solid_strike(path, r"\cancel{}");
    let [PathCmd::Move(x0, y0), PathCmd::Line(x1, y1)] = path.commands.as_slice() else {
        panic!("expected one diagonal: {:?}", path.commands)
    };
    assert!((x1.0 - x0.0 - pt_ticks(2.5).0).abs() <= 2, "run 2.5pt: {:?}", path.commands);
    assert!((y0.0 - y1.0 - pt_ticks(10.0).0).abs() <= 2, "rise 10pt: {:?}", path.commands);
    assert_slope(path.commands.as_slice(), 4.0, r"\cancel{}");
}

#[test]
fn cancel_body_uses_the_ambient_style() {
    if !lm_available() {
        return;
    }
    // `cancel.sty` reaches the body through `\mathpalette`, so a subscript
    // body is laid out script-size, not display-size: with the old
    // always-DISPLAY layout both strikes below would be equally wide.
    // Script `x` (cmmi8 at 8pt): width 4.78477pt, total 3.44444pt; tall
    // branch with extent 10pt, k = floor(5*4.78477/10) = 2, slope (1,2),
    // run 0.5*10 = 5pt — half the top-level 10pt run.
    let (_, top) = render_formula(r"\cancel{x}");
    let (_, sub) = render_formula(r"a_{\cancel{x}}");
    assert_eq!(top.paths.len(), 1, "{:?}", top.paths);
    assert_eq!(sub.paths.len(), 1, "{:?}", sub.paths);
    let width_of = |p: &PathItem| match p.commands.as_slice() {
        [PathCmd::Move(x0, _), PathCmd::Line(x1, _)] => x1.0 - x0.0,
        cmds => panic!("expected one diagonal: {cmds:?}"),
    };
    let (top_run, sub_run) = (width_of(&top.paths[0]), width_of(&sub.paths[0]));
    assert!(
        sub_run < top_run,
        "subscript strike ({sub_run} ticks) must be narrower than top-level ({top_run} ticks)"
    );
    assert!((sub_run - pt_ticks(5.0).0).abs() <= 2, "script run 5pt: {sub_run}");
    assert_slope(&sub.paths[0].commands, 2.0, r"a_{\cancel{x}}");
}
