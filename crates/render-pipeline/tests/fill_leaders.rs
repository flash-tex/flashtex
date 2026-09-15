//! `\hrulefill` and `\dotfill` leaders against the TeX Live 2026 oracle.
//!
//! The pdflatex measurements below are from article 10pt with a 200pt
//! horizontal box: `A\hrulefill B` has a 0.4pt x 185.41663pt rule, while
//! `A\dotfill B` has 42 cleader boxes of 4.40002pt, with a 0.6156pt leftover
//! split into 0.3078pt at each end.  The dot glyph is 2.77779pt wide; its
//! first and last origins are 1.11891pt and 181.51991pt from the glue start.

mod common;

use flashtex_compiler::json;
use common::render_one;
use flashtex_render_pipeline::display::{GlyphRun, Item, Provenance, Rule, Severity};

const PT_PER_BP: f64 = 72.27 / 72.0;
const TOL: f64 = 0.01;
const BASE_HFILL_ITEMS: &str = r#"[{"clusters":[{"carets":[{"height":7196394,"text_byte":0,"top":84721021,"x":216398394},{"height":7196394,"text_byte":1,"top":84721021,"x":224233333}],"hit_rects":[{"height":7196394,"top":84721021,"width":7834939,"x":216398394}],"sources":[{"end_byte":138,"path":"main.tex","start_byte":137}],"text_end_byte":1,"text_start_byte":0}],"font_id":"1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852","font_size":10446585,"glyphs":[{"advance_x":7834939,"advance_y":0,"baseline_y":91917415,"cluster":0,"gid":27,"origin_x":216398394}],"kind":"glyph_run","paint":{"a":1,"b":0,"g":0,"r":0},"text":"A"},{"clusters":[{"carets":[{"height":7196394,"text_byte":0,"top":84721021,"x":417930788},{"height":7196394,"text_byte":1,"top":84721021,"x":425330100}],"hit_rects":[{"height":7196394,"top":84721021,"width":7399313,"x":417930788}],"sources":[{"end_byte":146,"path":"main.tex","start_byte":145}],"text_end_byte":1,"text_start_byte":0}],"font_id":"1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852","font_size":10446585,"glyphs":[{"advance_x":7399313,"advance_y":0,"baseline_y":91917415,"cluster":0,"gid":34,"origin_x":417930788}],"kind":"glyph_run","paint":{"a":1,"b":0,"g":0,"r":0},"text":"B"},{"clusters":[{"carets":[{"height":6578471,"text_byte":0,"top":733027233,"x":318252601},{"height":6578471,"text_byte":1,"top":733027233,"x":323475893}],"hit_rects":[{"height":6578471,"top":733027233,"width":5223293,"x":318252601}],"synthetic_reason":"page chrome","text_end_byte":1,"text_start_byte":0}],"font_id":"1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852","font_size":10446585,"glyphs":[{"advance_x":5223293,"advance_y":0,"baseline_y":739605704,"cluster":0,"gid":82,"origin_x":318252601}],"kind":"glyph_run","paint":{"a":1,"b":0,"g":0,"r":0},"text":"1"}]"#;

fn doc(body: &str) -> String {
    format!(
        "\\documentclass[10pt]{{article}}\\usepackage[textwidth=200pt,textheight=600pt]{{geometry}}\\setlength{{\\parindent}}{{0pt}}\\begin{{document}}\\noindent{{{body}}}\\end{{document}}"
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

fn item_json(rendered: &flashtex_render_pipeline::Rendered) -> String {
    let document = rendered.v2.to_json("hfill-base");
    let page = document
        .get("payload")
        .and_then(|payload| payload.get("pages"))
        .and_then(|pages| pages.as_arr())
        .and_then(|pages| pages.first())
        .expect("one page");
    json::write(page.get("items").expect("page items"))
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
    assert_eq!(item_json(&rendered), BASE_HFILL_ITEMS, "plain \\hfill changed from the #402 item list");
}
