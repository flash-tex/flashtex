//! GH-69: `\includegraphics` box sizes in floats against pdfLaTeX (MacTeX
//! 2026) references committed in `fixtures/float-graphics/reference/*.json`
//! (made by `fixtures/float-graphics/oracle.py`, test-only; cargo never runs
//! TeX).
//!
//! `01-png-keys`: `width=\linewidth`, `scale`, `keepaspectratio`, `angle`
//! before and after a size key, and `\textwidth` multiples. `02-objstm-pdf`:
//! PDFs written by pdfTeX with its default object streams, whose page tree
//! a byte scan cannot see, including a `/Rotate 90` page. Every image must be
//! on the reference page and within 1 pt (bp) in x, top, width and height,
//! with no image diagnostic.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 1.0;

fn num(v: &Value, k: &str) -> f64 {
    match v.get(k) {
        Some(Value::Num(n)) => *n,
        _ => f64::NAN,
    }
}

#[test]
fn float_graphics_match_pdflatex() {
    if !common::lm_available() {
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/float-graphics");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tex") && n.as_bytes()[0].is_ascii_digit())
        .collect();
    names.sort();
    assert_eq!(names.len(), 2, "expected 2 float-graphics fixtures");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    let mut failures = Vec::new();
    for name in &names {
        let tex = std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/reference/{}", name.replace(".tex", ".json"))).unwrap()).unwrap();
        let r = render(&[SourceDocument { path: "main.tex", text: &tex }], "main.tex", 1, "float-graphics", &fonts, &options);
        let ref_pages = num(&reference, "pages") as usize;
        if r.v2.pages.len() != ref_pages {
            failures.push(format!("{name}: pages {} vs reference {ref_pages}", r.v2.pages.len()));
        }
        for d in &r.v2.diagnostics {
            if d.code.starts_with("image") || d.code.starts_with("graphics") || d.code.starts_with("float") {
                failures.push(format!("{name}: diagnostic {}: {}", d.code, d.message));
            }
        }
        let mut ours: Vec<(u32, f64, f64, f64, f64)> = r
            .v2
            .pages
            .iter()
            .flat_map(|p| {
                p.items.iter().filter_map(move |it| match it {
                    Item::Image(i) => Some((p.number, i.x.to_bp(), i.top.to_bp(), i.width.to_bp(), i.height.to_bp())),
                    _ => None,
                })
            })
            .collect();
        ours.sort_by(|a, b| (a.0, a.2, a.1).partial_cmp(&(b.0, b.2, b.1)).unwrap());
        let mut theirs: Vec<(u32, f64, f64, f64, f64)> = reference
            .get("images")
            .and_then(|v| v.as_arr())
            .unwrap()
            .iter()
            .map(|i| (num(i, "page") as u32, num(i, "x"), num(i, "top"), num(i, "width"), num(i, "height")))
            .collect();
        theirs.sort_by(|a, b| (a.0, a.2, a.1).partial_cmp(&(b.0, b.2, b.1)).unwrap());
        if ours.len() != theirs.len() {
            failures.push(format!("{name}: {} images vs reference {}", ours.len(), theirs.len()));
        }
        for (o, t) in ours.iter().zip(&theirs) {
            let d = [(o.1 - t.1).abs(), (o.2 - t.2).abs(), (o.3 - t.3).abs(), (o.4 - t.4).abs()].into_iter().fold(0.0, f64::max);
            if o.0 != t.0 || d > TOL {
                failures.push(format!("{name}: image ours p{} x{:.3} top{:.3} {:.3}x{:.3} vs ref p{} x{:.3} top{:.3} {:.3}x{:.3}", o.0, o.1, o.2, o.3, o.4, t.0, t.1, t.2, t.3, t.4));
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
