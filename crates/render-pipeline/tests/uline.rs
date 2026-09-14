//! GH-312: with ulem loaded, `\uline{...}` paints a 0.4pt underline rule
//! under the argument. The first step is a single-line (unbreakable)
//! fragment; ulem's leaders can break across lines, which is not required
//! here. Depth follows ulem.sty `\UL@setULdepth`: `\dp` of `\hbox{{(j}}`
//! is max(`(`, `j`) = 0.25em for cmr/lmr. pdflatex (TeX Live 2026) at
//! 10pt: `rule(-2.5+2.9)`; at 12pt: `rule(-3.0+3.4)`. The pipeline rule
//! top/height must match those TeX points within 0.01pt.
mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, Tick};

fn uline_doc(class_opt: &str) -> String {
    format!(
        r"\documentclass{class_opt}{{article}}
\usepackage[normalem]{{ulem}}
\begin{{document}}
Text \uline{{underlined words}} here.
\end{{document}}"
    )
}

#[test]
fn uline_with_ulem_is_not_an_unsupported_error() {
    if !lm_available() {
        return;
    }
    let rendered = render_one(&uline_doc("[10pt]"));
    let uline = rendered
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("\\uline is not supported"))
        .collect::<Vec<_>>();
    assert!(
        uline.is_empty(),
        "\\uline should be implemented when ulem is loaded: {:?}",
        rendered.v2.diagnostics
    );
}

fn assert_ulem_rule(class_opt: &str, size_pt: f64) {
    let rendered = render_one(&uline_doc(class_opt));
    let page = &rendered.v2.pages[0];
    let words: Vec<_> = page
        .items
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run)
                if run.text.contains("underlined") || run.text.contains("words") =>
            {
                Some(run)
            }
            _ => None,
        })
        .collect();
    let summary: Vec<String> = page
        .items
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
        .collect();
    assert!(!words.is_empty(), "argument glyphs missing: {summary:?}");
    let first = words[0].glyphs.first().expect("glyphs");
    let last = words
        .last()
        .and_then(|run| run.glyphs.last())
        .expect("glyphs");
    let text_left = first.origin_x.to_bp();
    let text_right = last.origin_x.to_bp() + last.advance_x.to_bp();
    let baseline = first.baseline_y.to_bp();
    let want_h = Tick::from_tex_pt(0.4).to_bp();
    // pdflatex `\hrule height -0.25em`: rule top 0.25em below the baseline.
    let want_top = Tick::from_tex_pt(0.25 * size_pt).to_bp();
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
        "expected an underline rule under the argument: {summary:?}"
    );
    let rule = rules[0];
    let got_h = rule.height.to_bp();
    let got_top = rule.top.to_bp() - baseline;
    assert!(
        (got_h - want_h).abs() < 0.01,
        "ulem \\ULthickness is 0.4pt ({want_h}bp), got {got_h}bp at {size_pt}pt"
    );
    assert!(
        (got_top - want_top).abs() < 0.01,
        "pdflatex rule top is {want_top}bp below baseline at {size_pt}pt, got {got_top}bp (page top {}, baseline {baseline})"
        ,
        rule.top.to_bp()
    );
}

#[test]
fn uline_paints_a_rule_under_the_argument() {
    if !lm_available() {
        return;
    }
    assert_ulem_rule("[10pt]", 10.0);
    assert_ulem_rule("[12pt]", 12.0);
}
