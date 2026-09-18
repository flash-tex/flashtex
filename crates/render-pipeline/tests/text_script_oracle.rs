//! `\textsuperscript`/`\textsubscript` (compiler `Inline::TextScript`, #507)
//! against pdflatex. latex.ltx sets the argument in an `\mbox` at
//! `\sf@size` inside `\ensuremath{^{...}}` (or `_{...}`), so the box is the
//! script of an empty nucleus in text style: shifted up by `sup2` or down
//! by `sub1` of the symbol font at the text size, `\scriptspace` included
//! in its width.
//!
//! Expected values are pdflatex's own `\showbox` of
//! `\hbox{A\textsuperscript{th} B\textsubscript{2} ...}` (MacTeX 2026,
//! plain `article`):
//!
//! ```text
//! 10pt: \hbox(4.8611+0.0)x8.0417, shifted -3.62892    (cmr7 th)
//!       \hbox(4.51111+0.0)x4.48613, shifted 1.49998   (cmr7 2)
//!       \glue 3.33333 plus 1.66666 minus 1.11111
//! 12pt: \hbox(5.55556+0.0)x8.5279, shifted -4.3547    (cmr8 th)
//!       \hbox(5.15556+0.0)x4.75006, shifted 1.79999   (cmr8 2)
//!       \glue 3.91663 plus 1.95831 minus 1.30554
//! ```
//!
//! The paragraph below is one short last line, so its glue is natural: the
//! next word starts one box width plus one interword space after the
//! script's first glyph.
mod common;

use common::*;
use flashtex_render_pipeline::display::{GlyphRun, Item};

const TOL_PT: f64 = 0.02;

struct Expect {
    class_opt: &'static str,
    script_size: f64,
    sup_shift: f64,
    sup_width: f64,
    sub_shift: f64,
    sub_width: f64,
    space: f64,
}

fn pt(bp: f64) -> f64 {
    bp * 72.27 / 72.0
}

fn check(e: &Expect) {
    let doc = format!(
        "\\documentclass[{}]{{article}}\n\\begin{{document}}\nWord A\\textsuperscript{{th}} word B\\textsubscript{{2}} word.\n\\end{{document}}\n",
        e.class_opt
    );
    let rendered = render_one(&doc);
    let page = &rendered.v2.pages[0];
    let runs: Vec<&GlyphRun> = page
        .resident_items()
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run) if !run.glyphs.is_empty() => Some(run),
            _ => None,
        })
        .collect();
    let summary: Vec<String> = runs
        .iter()
        .map(|r| format!("{:?}@({:.3},{:.3})/{:.2}", r.text, pt(r.glyphs[0].origin_x.to_bp()), pt(r.glyphs[0].baseline_y.to_bp()), pt(r.font_size.to_bp())))
        .collect();
    let find = |text: &str| -> &GlyphRun {
        runs.iter().copied().find(|r| r.text == text).unwrap_or_else(|| panic!("{text:?} missing: {summary:?}"))
    };
    let x = |r: &GlyphRun| pt(r.glyphs[0].origin_x.to_bp());
    let y = |r: &GlyphRun| pt(r.glyphs[0].baseline_y.to_bp());
    // The first run strictly to the right of `after` on the body baseline.
    let next_word = |after: &GlyphRun, base: f64| -> &GlyphRun {
        runs.iter()
            .copied()
            .filter(|r| (y(r) - base).abs() < TOL_PT && x(r) > x(after) + 0.1)
            .min_by(|a, b| x(a).total_cmp(&x(b)))
            .unwrap_or_else(|| panic!("no word after {:?}: {summary:?}", after.text))
    };

    let a = find("A");
    let base = y(a);
    let sup = find("th");
    assert!((pt(sup.font_size.to_bp()) - e.script_size).abs() < TOL_PT, "{} th size: {summary:?}", e.class_opt);
    assert!((base - y(sup) - e.sup_shift).abs() < TOL_PT, "{} th shift {:.4} want {}: {summary:?}", e.class_opt, base - y(sup), e.sup_shift);
    let a_end = x(a) + pt(a.glyphs.iter().map(|g| g.advance_x.to_bp()).sum::<f64>());
    assert!((x(sup) - a_end).abs() < TOL_PT, "{} th starts at A's end: {summary:?}", e.class_opt);
    let after_sup = next_word(sup, base);
    assert!(
        (x(after_sup) - x(sup) - (e.sup_width + e.space)).abs() < TOL_PT,
        "{} word after th at +{:.4} want {:.4}: {summary:?}",
        e.class_opt,
        x(after_sup) - x(sup),
        e.sup_width + e.space
    );

    let b = find("B");
    let sub = find("2");
    assert!((pt(sub.font_size.to_bp()) - e.script_size).abs() < TOL_PT, "{} 2 size: {summary:?}", e.class_opt);
    assert!((y(sub) - y(b) - e.sub_shift).abs() < TOL_PT, "{} 2 shift {:.4} want {}: {summary:?}", e.class_opt, y(sub) - y(b), e.sub_shift);
    let after_sub = next_word(sub, y(b));
    assert!(
        (x(after_sub) - x(sub) - (e.sub_width + e.space)).abs() < TOL_PT,
        "{} word after 2 at +{:.4} want {:.4}: {summary:?}",
        e.class_opt,
        x(after_sub) - x(sub),
        e.sub_width + e.space
    );
}

#[test]
fn textsuperscript_and_textsubscript_match_pdflatex_10pt() {
    if !lm_available() {
        return;
    }
    check(&Expect { class_opt: "10pt", script_size: 7.0, sup_shift: 3.62892, sup_width: 8.0417, sub_shift: 1.49998, sub_width: 4.48613, space: 3.33333 });
}

#[test]
fn textsuperscript_and_textsubscript_match_pdflatex_12pt() {
    if !lm_available() {
        return;
    }
    check(&Expect { class_opt: "12pt", script_size: 8.0, sup_shift: 4.3547, sup_width: 8.5279, sub_shift: 1.79999, sub_width: 4.75006, space: 3.91663 });
}
