//! The `frame` environment draws a rule-bordered box around its body:
//! four `\fboxrule` frame rules sized to the content, the content's glyphs
//! strictly inside them, and no `unsupported_feature` diagnostic.
//!
//! `frame` lowers to the same `Inline::ColorBox` that `\fcolorbox` makes,
//! so both tests below assert the identical rule geometry and containment
//! on the pipeline's shared bordered-box path (`BoxRec::ColorBox`).
//! The `\begin{frame}` test itself is ignored until `vendor/compiler` is
//! re-pinned past the compiler change that recognises the environment
//! (it still sees the old plain-text fallback, by design: lanes never
//! touch `vendor/`).

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, Paint, Tick};

fn near(a: i64, b: i64) -> bool {
    (a - b).abs() <= 2
}

fn fill_of(p: &Paint) -> String {
    p.device.map(|d| d.fill_operator()).unwrap_or_else(|| "0 g".to_string())
}

/// Four `\fboxrule` rules around the content, the content strictly inside
/// them, and no "not implemented" diagnostic.
fn border_and_content(src: &str) {
    let r = render_one(src);
    assert!(
        r.v2.diagnostics.iter().all(|d| !d.message.contains("not implemented")
            && !d.message.contains("not supported")),
        "{:#?}",
        r.v2.diagnostics
    );

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

    // One invisible fill plus the four frame rules, in `\fcolorbox` order:
    // top, left, right, bottom.
    assert_eq!(rules.len(), 5, "{rules:?}");
    assert_eq!(fill_of(&rules[0].paint), "1 g");
    let frame = &rules[1..];
    assert!(frame.iter().all(|r| fill_of(&r.paint) == "0 g"), "{rules:?}");
    let [top, left, right, bottom] = frame else { unreachable!() };
    let rule = Tick::from_tex_pt(0.4);
    let half_rule = Tick::from_tex_pt(0.2);
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

    // The fill sits exactly inside the frame, inset one rule on every side.
    let fill = rules[0];
    assert!(near(fill.x.0, top.x.0 + rule.0));
    assert!(near(fill.top.0, top.top.0 + rule.0));
    assert!(near(fill.width.0 + 2 * rule.0, top.width.0));
    assert!(near(fill.top.0 + fill.height.0 + rule.0, bottom.top.0 + bottom.height.0));

    // The body glyphs sit strictly inside the border (not overlapping it):
    // every content cluster's hit rect is contained in the frame's inner
    // rect, and no other glyph overlaps the frame rect at all.
    let inner = (
        left.x.0 + left.width.0,
        top.top.0 + top.height.0,
        right.x.0,
        bottom.top.0,
    );
    let outer = (top.x.0, top.top.0, top.x.0 + top.width.0, bottom.top.0 + bottom.height.0);
    let overlaps = |r: (i64, i64, i64, i64), s: (i64, i64, i64, i64)| {
        r.0 < s.2 && s.0 < r.2 && r.1 < s.3 && s.1 < r.3
    };
    let mut inside_text = String::new();
    for page in &r.v2.pages {
        for item in &page.items {
            if let Item::GlyphRun(run) = item {
                let rects: Vec<_> = run
                    .clusters
                    .iter()
                    .map(|c| {
                        (
                            c.hit_rect.x.0,
                            c.hit_rect.top.0,
                            c.hit_rect.x.0 + c.hit_rect.width.0,
                            c.hit_rect.top.0 + c.hit_rect.height.0,
                        )
                    })
                    .collect();
                if run.text.contains("Hi") || run.text.contains("Yo") {
                    inside_text.push_str(&run.text);
                    for rect in rects {
                        assert!(
                            rect.0 >= inner.0 - 2
                                && rect.1 >= inner.1 - 2
                                && rect.2 <= inner.2 + 2
                                && rect.3 <= inner.3 + 2,
                            "{run:?} escapes the frame"
                        );
                    }
                } else {
                    for rect in rects {
                        assert!(!overlaps(rect, outer), "{run:?} overlaps the frame");
                    }
                }
            }
        }
    }
    assert!(inside_text.contains("Hi"), "framed words found inside: {inside_text:?}");
    assert!(inside_text.contains("Yo"), "framed words found inside: {inside_text:?}");
}

#[test]
fn shared_colorbox_path_draws_border_with_content_inside() {
    if !lm_available() {
        return;
    }
    // The exact node `frame` lowers to, through the compiler the pipeline
    // pins today: exercises the shared drawing path end to end.
    border_and_content(
        "\\usepackage{xcolor}\\begin{document}A \\fcolorbox{black}{white}{Hi \\textbf{Yo}} B\\end{document}",
    );
}

#[test]
#[ignore = "needs vendor/compiler re-pinned past the frame environment change"]
fn frame_draws_a_rule_border_around_its_content() {
    if !lm_available() {
        return;
    }
    border_and_content("\\begin{document}A \\begin{frame}Hi \\textbf{Yo}\\end{frame} B\\end{document}");
}
