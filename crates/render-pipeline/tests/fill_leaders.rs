//! `\hrulefill` and `\dotfill` leaders against the TeX Live 2026 oracle.
//!
//! The pdflatex measurements below are from article 10pt with a 200pt
//! horizontal box: `A\hrulefill B` has a 0.4pt x 185.41663pt rule, while
//! `A\dotfill B` has 42 cleader boxes of 4.40002pt, with a 0.6156pt leftover
//! split into 0.3078pt at each end.  The dot glyph is 2.77779pt wide; its
//! first and last origins are 1.11891pt and 181.51991pt from the glue start.

mod common;

use common::render_one;
use flashtex_render_pipeline::display::{GlyphRun, Item, Provenance, Rule, Severity};

const PT_PER_BP: f64 = 72.27 / 72.0;
const TOL: f64 = 0.01;

fn bp(pt: f64) -> f64 {
    pt / PT_PER_BP
}

fn doc(body: &str) -> String {
    format!(
        "\\documentclass[10pt]{{article}}\\usepackage[textwidth=200pt,textheight=600pt]{{geometry}}\\begin{{document}}\\noindent{{{body}}}\\end{{document}}"
    )
}

fn rendered(body: &str) -> flashtex_render_pipeline::Rendered {
    let rendered = render_one(&doc(body));
    let errors: Vec<_> = rendered.v2.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "unexpected diagnostics: {errors:?}");
    rendered
}

fn text_run<'a>(items: &'a [Item], text: &str) -> &'a GlyphRun {
    items.iter().find_map(|item| match item {
        Item::GlyphRun(run) if run.text == text => Some(run),
        _ => None,
    }).unwrap_or_else(|| panic!("no glyph run {text:?} in {items:?}"))
}

fn synthetic_rule<'a>(items: &'a [Item], name: &str) -> &'a Rule {
    items.iter().find_map(|item| match item {
        Item::Rule(rule) if matches!(&rule.provenance, Provenance::Synthetic(s) if s == name) => Some(rule),
        _ => None,
    }).unwrap_or_else(|| panic!("no synthetic rule {name:?} in {items:?}"))
}

fn synthetic_run<'a>(items: &'a [Item], name: &str) -> &'a GlyphRun {
    items.iter().find_map(|item| match item {
        Item::GlyphRun(run) if run.clusters.iter().any(|c| matches!(&c.provenance, Provenance::Synthetic(s) if s == name)) => Some(run),
        _ => None,
    }).unwrap_or_else(|| panic!("no synthetic run {name:?} in {items:?}"))
}

fn pt(tick: flashtex_render_pipeline::display::Tick) -> f64 {
    tick.to_bp() * PT_PER_BP
}

#[test]
fn rule_fill_matches_pdflatex_geometry() {
    if !common::lm_available() {
        return;
    }
    let rendered = rendered("A\\hrulefill B");
    let items = &rendered.v2.pages[0].items;
    let rule = synthetic_rule(items, "\\hrulefill");
    let a = text_run(items, "A");
    let baseline = pt(a.glyphs[0].baseline_y);
    assert!((pt(rule.width) - 185.41663).abs() <= TOL, "rule width: {}pt", pt(rule.width));
    assert!((pt(rule.height) - 0.4).abs() <= TOL, "rule height: {}pt", pt(rule.height));
    assert!((pt(rule.top) - baseline + 0.4).abs() <= TOL, "rule top: {}pt, baseline: {baseline}pt", pt(rule.top));
}

#[test]
fn dot_fill_matches_pdflatex_cleaders() {
    if !common::lm_available() {
        return;
    }
    let rendered = rendered("A\\dotfill B");
    let items = &rendered.v2.pages[0].items;
    let dots = synthetic_run(items, "\\dotfill");
    let a = text_run(items, "A");
    let b = text_run(items, "B");
    let glue_start = a.glyphs[0].origin_x.0 + a.glyphs[0].advance_x.0;
    let first = (dots.glyphs[0].origin_x.0 - glue_start) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP * PT_PER_BP;
    let last = (dots.glyphs.last().unwrap().origin_x.0 - glue_start) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP * PT_PER_BP;
    let b_start = (b.glyphs[0].origin_x.0 - glue_start) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP * PT_PER_BP;
    assert_eq!(dots.text, ".".repeat(42));
    assert_eq!(dots.glyphs.len(), 42);
    assert_eq!(dots.clusters.len(), 42);
    assert!((pt(dots.glyphs[0].advance_x) - 2.77779).abs() <= TOL, "dot width: {}pt", pt(dots.glyphs[0].advance_x));
    assert!((first - 1.11891).abs() <= TOL, "first dot: {first}pt");
    assert!((last - 181.51991).abs() <= TOL, "last dot: {last}pt");
    assert!((b_start - 185.41663).abs() <= TOL, "glue width: {b_start}pt");
}

#[test]
fn leader_inside_text_keeps_both_sides() {
    if !common::lm_available() {
        return;
    }
    let rendered = rendered("left\\hrulefill right");
    let items = &rendered.v2.pages[0].items;
    assert!(items.iter().any(|item| matches!(item, Item::GlyphRun(run) if run.text == "left")));
    assert!(items.iter().any(|item| matches!(item, Item::GlyphRun(run) if run.text == "right")));
    synthetic_rule(items, "\\hrulefill");
}

#[test]
fn plain_hfill_has_no_leader_paint() {
    if !common::lm_available() {
        return;
    }
    let rendered = rendered("A\\hfill B");
    let items = &rendered.v2.pages[0].items;
    assert!(items.iter().any(|item| matches!(item, Item::GlyphRun(run) if run.text == "A")));
    assert!(items.iter().any(|item| matches!(item, Item::GlyphRun(run) if run.text == "B")));
    assert!(items.iter().all(|item| match item {
        Item::Rule(rule) => !matches!(rule.provenance, Provenance::Synthetic(_)),
        Item::GlyphRun(run) => !run.clusters.iter().any(|c| matches!(c.provenance, Provenance::Synthetic(_))),
        _ => true,
    }));
}
