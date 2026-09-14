//! `\boxed` uses the same four-rule frame geometry as `\fcolorbox`, with
//! amsmath's fixed 3pt separation and 0.4pt rule width.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, Tick};

fn near(a: i64, b: i64) -> bool {
    (a - b).abs() <= 2
}

#[test]
fn boxed_math_draws_frames_in_inline_and_display_math() {
    if !lm_available() {
        return;
    }
    let src = "\\begin{document}Inline $\\boxed{x}$ here.\n\\[\\boxed{a+b}\\]\n\\end{document}";
    let r = render_one(src);
    let rules: Vec<_> = r
        .v2
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|item| match item {
            Item::Rule(rule) => Some(rule),
            _ => None,
        })
        .collect();

    assert_eq!(rules.len(), 8, "two boxed formulas need four rules each: {rules:?}");
    assert!(r.v2.diagnostics.iter().all(|d| !d.message.contains("\\boxed frame dropped")), "{:#?}", r.v2.diagnostics);

    let rule = Tick::from_tex_pt(0.4);
    let half_rule = Tick::from_tex_pt(0.2);
    let inset = Tick::from_tex_pt(3.4);
    for frame in rules.chunks_exact(4) {
        let [top, left, right, bottom] = frame else { unreachable!() };
        assert_eq!(top.height, rule);
        assert_eq!(bottom.height, rule);
        assert_eq!(left.width, rule);
        assert_eq!(right.width, rule);
        assert!(left.height.0 > rule.0 && right.height.0 > rule.0);
        assert_eq!(left.x, top.x);
        assert!(near(right.x.0 + right.width.0, top.x.0 + top.width.0));
        assert!(near(left.top.0, top.top.0 + half_rule.0));
        assert!(near(right.top.0, left.top.0));
        assert!(near(bottom.top.0, top.top.0 + left.height.0));
    }

    let glyph_x: Vec<_> = r
        .v2
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|item| match item {
            Item::GlyphRun(run) if run.role == flashtex_render_pipeline::display::RunRole::Math => run.glyphs.first().map(|g| g.origin_x),
            _ => None,
        })
        .collect();
    assert_eq!(glyph_x.len(), 2, "one first glyph per boxed formula: {glyph_x:?}");
    assert!(near(glyph_x[0].0, rules[0].x.0 + inset.0));
    assert!(near(glyph_x[1].0, rules[4].x.0 + inset.0));
}
