//! GH-330: text-mode `\underline{under}` and ulem `\sout{struck}` must keep
//! their argument glyphs and paint a rule. Builds after the vendor/compiler
//! re-pin past the compiler commits on this branch (same split as #329).
mod common;

use common::*;
use flashtex_compiler::parser::{
    CMR_EX_PER_EM, MATH_RULE_THETA_PT, SOUT_RAISE_EX, UL_THICKNESS_PT,
};
use flashtex_render_pipeline::display::{Item, Tick};

fn underline_doc(class_opt: &str) -> String {
    format!(
        r"\documentclass{class_opt}{{article}}
\begin{{document}}
Text \underline{{under}} here.
\end{{document}}"
    )
}

fn sout_doc(class_opt: &str) -> String {
    format!(
        r"\documentclass{class_opt}{{article}}
\usepackage[normalem]{{ulem}}
\begin{{document}}
Text \sout{{struck}} here.
\end{{document}}"
    )
}

fn page_summary(page: &flashtex_render_pipeline::display::Page) -> Vec<String> {
    page.items
        .iter()
        .map(|item| match item {
            Item::GlyphRun(run) => format!("text:{:?}", run.text),
            Item::Rule(rule) => format!(
                "rule:{:.3}x{:.3}+{:.3}x{:.3}",
                rule.x.to_bp(),
                rule.top.to_bp(),
                rule.width.to_bp(),
                rule.height.to_bp()
            ),
            _ => "other".into(),
        })
        .collect()
}

fn assert_text_and_rule(source: &str, needle: &str, want_height_pt: f64, want_top_pt: f64) {
    let rendered = render_one(source);
    let page = &rendered.v2.pages[0];
    let summary = page_summary(page);
    let words: Vec<_> = page
        .items
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run) if run.text.contains(needle) => Some(run),
            _ => None,
        })
        .collect();
    assert!(
        !words.is_empty(),
        "argument glyphs missing ({needle}): {summary:?}"
    );
    let first = words[0].glyphs.first().expect("glyphs");
    let last = words
        .last()
        .and_then(|run| run.glyphs.last())
        .expect("glyphs");
    let text_left = first.origin_x.to_bp();
    let text_right = last.origin_x.to_bp() + last.advance_x.to_bp();
    let baseline = first.baseline_y.to_bp();
    let want_h = Tick::from_tex_pt(want_height_pt).to_bp();
    let want_top = Tick::from_tex_pt(want_top_pt).to_bp();
    let rules: Vec<_> = page
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Rule(rule) => Some(rule),
            _ => None,
        })
        .filter(|rule| {
            let left = rule.x.to_bp();
            let right = left + rule.width.to_bp();
            right > text_left && left < text_right
        })
        .collect();
    assert!(
        !rules.is_empty(),
        "expected a rule spanning {needle}: {summary:?}"
    );
    let rule = rules[0];
    let got_h = rule.height.to_bp();
    let got_top = rule.top.to_bp() - baseline;
    assert!(
        (got_h - want_h).abs() < 0.01,
        "rule height {want_h}bp, got {got_h}bp"
    );
    assert!(
        (got_top - want_top).abs() < 0.01,
        "rule top {want_top}bp from baseline, got {got_top}bp (page top {}, baseline {baseline})",
        rule.top.to_bp()
    );
}

#[test]
fn text_mode_underline_is_not_an_unsupported_error() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&underline_doc("[10pt]"));
    let hits: Vec<_> = rendered
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("\\underline is not supported"))
        .collect();
    assert!(
        hits.is_empty(),
        "text-mode \\underline should be implemented: {:?}",
        rendered.v2.diagnostics
    );
}

#[test]
fn sout_with_ulem_is_not_an_unsupported_error() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&sout_doc("[10pt]"));
    let hits: Vec<_> = rendered
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("\\sout is not supported"))
        .collect();
    assert!(
        hits.is_empty(),
        "\\sout should be implemented when ulem is loaded: {:?}",
        rendered.v2.diagnostics
    );
}

#[test]
fn text_mode_underline_paints_a_rule() {
    if !lm_available() {
        return;
    }
    assert_text_and_rule(
        &underline_doc("[10pt]"),
        "under",
        MATH_RULE_THETA_PT,
        3.0 * MATH_RULE_THETA_PT,
    );
    assert_text_and_rule(
        &underline_doc("[12pt]"),
        "under",
        MATH_RULE_THETA_PT,
        3.0 * MATH_RULE_THETA_PT,
    );
}

#[test]
fn sout_paints_a_rule() {
    if !lm_available() {
        return;
    }
    let top_at = |size: f64| -(SOUT_RAISE_EX * CMR_EX_PER_EM * size + UL_THICKNESS_PT);
    assert_text_and_rule(&sout_doc("[10pt]"), "struck", UL_THICKNESS_PT, top_at(10.0));
    assert_text_and_rule(&sout_doc("[12pt]"), "struck", UL_THICKNESS_PT, top_at(12.0));
}
