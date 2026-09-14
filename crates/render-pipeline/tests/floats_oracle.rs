//! FT-063 gate: float placement and `\includegraphics` sizing against
//! pdfLaTeX (MacTeX 2026) references committed in
//! `fixtures/floats/reference/*.json` (made by `fixtures/floats/oracle.py`,
//! test-only; cargo never runs TeX). Every image box, caption start and body
//! paragraph start must agree on the page and within 1 pt (bp) in x and y.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, RunRole};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 1.0;

fn num(v: &Value, k: &str) -> f64 {
    match v.get(k) {
        Some(Value::Num(n)) => *n,
        _ => f64::NAN,
    }
}

struct Found {
    page: u32,
    x: f64,
    y: f64,
}

#[test]
fn float_fixtures_match_pdflatex_within_one_point() {
    if !common::lm_available() {
        eprintln!("SKIP floats_oracle: Latin Modern fonts not installed");
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/floats");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tex") && n.as_bytes()[0].is_ascii_digit())
        .collect();
    names.sort();
    assert_eq!(names.len(), 10, "expected 10 float fixtures");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    let mut failures = Vec::new();
    let mut report = String::new();
    for name in &names {
        let tex = std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/reference/{}", name.replace(".tex", ".json"))).unwrap()).unwrap();
        let docs = [SourceDocument { path: "main.tex", text: &tex }];
        let r = render(&docs, "main.tex", 1, "floats", &fonts, &options);
        let v2 = &r.v2;
        let mut worst: f64 = 0.0;
        let mut checks = 0;
        let mut fail = |what: String, failures: &mut Vec<String>| failures.push(format!("{name}: {what}"));
        let ref_pages = num(&reference, "pages") as usize;
        if v2.pages.len() != ref_pages {
            fail(format!("pages {} vs reference {ref_pages}", v2.pages.len()), &mut failures);
        }
        for d in &v2.diagnostics {
            if d.code.starts_with("image") || d.code.starts_with("float") || d.code.starts_with("graphics") {
                fail(format!("diagnostic {}: {}", d.code, d.message), &mut failures);
            }
        }
        // Images: ordered by (page, top, x) on both sides.
        let mut ours: Vec<(u32, f64, f64, f64, f64)> = v2
            .pages
            .iter()
            .flat_map(|p| p.items.iter().filter_map(move |it| match it {
                Item::Image(i) => Some((p.number, i.x.to_bp(), i.top.to_bp(), i.width.to_bp(), i.height.to_bp())),
                _ => None,
            }))
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
            fail(format!("{} images vs reference {}", ours.len(), theirs.len()), &mut failures);
        }
        for (o, t) in ours.iter().zip(&theirs) {
            checks += 1;
            let d = [(o.1 - t.1).abs(), (o.2 - t.2).abs(), (o.3 - t.3).abs(), (o.4 - t.4).abs()].into_iter().fold(0.0, f64::max);
            worst = worst.max(d);
            if o.0 != t.0 || d > TOL {
                fail(format!("image ours p{} x{:.3} top{:.3} {:.3}x{:.3} vs ref p{} x{:.3} top{:.3} {:.3}x{:.3}", o.0, o.1, o.2, o.3, o.4, t.0, t.1, t.2, t.3, t.4), &mut failures);
            }
        }
        // Text anchors: the first glyph of the run whose first cluster
        // starts at a source byte.
        let find = |byte: usize| -> Option<Found> {
            for p in &v2.pages {
                for it in &p.items {
                    if let Item::GlyphRun(run) = it {
                        if run.role != RunRole::Text {
                            continue;
                        }
                        let Some(c) = run.clusters.first() else { continue };
                        if c.provenance.sources().first().is_some_and(|s| s.start() == byte) {
                            let g = run.glyphs[0];
                            return Some(Found { page: p.number, x: g.origin_x.to_bp(), y: g.baseline_y.to_bp() });
                        }
                    }
                }
            }
            None
        };
        // Captions: `\caption` commands in source order, numbered per kind.
        let mut caption_bytes = Vec::new();
        let (mut figs, mut tabs) = (0, 0);
        let mut env_kind = "";
        let mut at = 0;
        while at < tex.len() {
            let rest = &tex[at..];
            if rest.starts_with("\\begin{figure}") {
                env_kind = "Figure";
            } else if rest.starts_with("\\begin{table}") {
                env_kind = "Table";
            } else if rest.starts_with("\\caption{") {
                let n = if env_kind == "Figure" {
                    figs += 1;
                    figs
                } else {
                    tabs += 1;
                    tabs
                };
                caption_bytes.push((format!("{env_kind} {n}"), at));
            }
            at += rest.chars().next().map_or(1, char::len_utf8);
        }
        for c in reference.get("captions").and_then(|v| v.as_arr()).unwrap() {
            let label = c.get("label").and_then(|v| v.as_str()).unwrap();
            let Some((_, byte)) = caption_bytes.iter().find(|(l, _)| l == label) else {
                fail(format!("caption {label} not in source"), &mut failures);
                continue;
            };
            checks += 1;
            match find(*byte) {
                Some(f) => {
                    let d = (f.x - num(c, "x")).abs().max((f.y - num(c, "baseline")).abs());
                    worst = worst.max(d);
                    if f.page != num(c, "page") as u32 || d > TOL {
                        fail(format!("caption {label} ours p{} ({:.3},{:.3}) vs ref p{} ({:.3},{:.3})", f.page, f.x, f.y, num(c, "page"), num(c, "x"), num(c, "baseline")), &mut failures);
                    }
                }
                None => fail(format!("caption {label} not rendered"), &mut failures),
            }
        }
        for para in reference.get("paragraphs").and_then(|v| v.as_arr()).unwrap() {
            let byte = num(para, "source_byte") as usize;
            checks += 1;
            match find(byte) {
                Some(f) => {
                    let d = (f.x - num(para, "x")).abs().max((f.y - num(para, "baseline")).abs());
                    worst = worst.max(d);
                    if f.page != num(para, "page") as u32 || d > TOL {
                        fail(format!("paragraph @{byte} ours p{} ({:.3},{:.3}) vs ref p{} ({:.3},{:.3})", f.page, f.x, f.y, num(para, "page"), num(para, "x"), num(para, "baseline")), &mut failures);
                    }
                }
                None => fail(format!("paragraph @{byte} not rendered"), &mut failures),
            }
        }
        report.push_str(&format!("| {name} | {}/{ref_pages} | {checks} | {worst:.3} |\n", v2.pages.len()));
    }
    println!("| fixture | pages ours/ref | anchors checked | max abs diff (bp) |\n|---|---|---|---|\n{report}");
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn image_items_are_only_serialised_when_negotiated() {
    if !common::lm_available() {
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/floats");
    let tex = std::fs::read_to_string(format!("{dir}/01-here.tex")).unwrap();
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    let r = render(&[SourceDocument { path: "main.tex", text: &tex }], "main.tex", 1, "floats", &fonts, &options);
    assert!(r.v2.has_images());
    let frozen = json::write(&r.v2.to_json("x"));
    assert!(!frozen.contains("\"image\""), "frozen display-list-v2 line must not carry image items");
    let proposed = json::write(&r.v2.to_json_with("x", true));
    assert!(proposed.contains("\"kind\":\"image\"") && proposed.contains("\"required_features\":[\"glyph_run\",\"rgba-srgb\",\"cluster-actualtext\",\"image\"]"), "{}", &proposed[..400.min(proposed.len())]);
    // Without a project root the image is reported, never guessed.
    let r = render(&[SourceDocument { path: "main.tex", text: &tex }], "main.tex", 1, "floats", &fonts, &RenderOptions::default());
    assert!(!r.v2.has_images());
    assert!(r.v2.diagnostics.iter().any(|d| d.code == "image_unavailable"));
}
