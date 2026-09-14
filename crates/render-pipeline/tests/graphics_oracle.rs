//! `\includegraphics` geometry against pdfLaTeX.
//!
//! `floats_oracle.rs` measures where a *float* lands. This one measures the
//! image box itself: in running text (01-08), and inside a `figure` for the
//! `\includegraphics*` and `[llx,lly][urx,ury]` forms the float scanner used
//! to miss entirely (09-10). References in
//! `fixtures/graphics/reference/*.json` were made by that directory's
//! `oracle.py` on **TeX Live 2025** (`reference_engine`), so cargo never runs
//! TeX. They are this lane's own data; the MacTeX-generated
//! `fixtures/floats/reference/*.json` are untouched.
//!
//! Checked per fixture, within 1 pt (bp):
//!
//! * every image's page, x, top, width and height;
//! * where the box sits relative to the baseline of the line it is on: its
//!   bottom edge (`top + height`) must land `depth` below the reference
//!   baseline. That is the height/depth split of an inline image, which is
//!   what TeX's line height and depth — and so `\baselineskip` against
//!   `\lineskip` — are computed from. A rotated box has real depth.
//! * the first-line baseline of every body paragraph, which is where a tall
//!   inline image's effect on interline spacing shows up: it accumulates the
//!   `\baselineskip`/`\lineskip` decision of every line above it.
//!
//! A font-failure diagnostic fails the gate: two builds that share a fallback
//! face agree perfectly on the wrong geometry.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, RunRole};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 1.0;

/// Diagnostics that mean a face or its metrics were not the real ones.
const FONT_FAILURES: [&str; 6] = [
    "font_unavailable",
    "required_metrics_unavailable",
    "ec_metrics_unavailable",
    "font_outline_substituted",
    "tfm_missing",
    "math_font_unavailable",
];

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
fn graphics_boxes_match_pdflatex_within_one_point() {
    if !common::lm_available() {
        eprintln!("SKIP graphics_oracle: Latin Modern fonts not installed");
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/graphics");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tex") && n.as_bytes()[0].is_ascii_digit())
        .collect();
    names.sort();
    assert_eq!(names.len(), 10, "expected 10 graphics fixtures");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    let mut failures = Vec::new();
    let mut report = String::new();
    for name in &names {
        let tex = std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/reference/{}", name.replace(".tex", ".json"))).unwrap()).unwrap();
        assert_eq!(
            reference.get("reference_engine").and_then(|v| v.as_str()),
            Some("TeX Live 2025"),
            "{name}: reference data is not labelled with its engine"
        );
        let docs = [SourceDocument { path: "main.tex", text: &tex }];
        let r = render(&docs, "main.tex", 1, "graphics", &fonts, &options);
        let v2 = &r.v2;
        let mut worst: f64 = 0.0;
        let mut checks = 0;
        let fail = |what: String, failures: &mut Vec<String>| failures.push(format!("{name}: {what}"));
        let ref_pages = num(&reference, "pages") as usize;
        if v2.pages.len() != ref_pages {
            fail(format!("pages {} vs reference {ref_pages}", v2.pages.len()), &mut failures);
        }
        for d in &v2.diagnostics {
            if FONT_FAILURES.contains(&d.code.as_str()) {
                fail(format!("font failure {}: {}", d.code, d.message), &mut failures);
            }
            if d.code.starts_with("image") || d.code.starts_with("graphics") {
                fail(format!("diagnostic {}: {}", d.code, d.message), &mut failures);
            }
        }
        // Images, ordered by (page, top, x) on both sides.
        let mut ours: Vec<(u32, f64, f64, f64, f64)> = v2
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
        let mut theirs: Vec<(u32, f64, f64, f64, f64, f64)> = reference
            .get("images")
            .and_then(|v| v.as_arr())
            .unwrap()
            .iter()
            .map(|i| (num(i, "page") as u32, num(i, "x"), num(i, "top"), num(i, "width"), num(i, "height"), num(i, "baseline") + num(i, "depth")))
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
                fail(
                    format!("image ours p{} x{:.3} top{:.3} {:.3}x{:.3} vs ref p{} x{:.3} top{:.3} {:.3}x{:.3}", o.0, o.1, o.2, o.3, o.4, t.0, t.1, t.2, t.3, t.4),
                    &mut failures,
                );
            }
            // The box's bottom edge, which pdfTeX puts `depth` below the line's
            // baseline (`t.5` is the reference `baseline + depth`).
            checks += 1;
            let bottom = o.2 + o.4;
            worst = worst.max((bottom - t.5).abs());
            if (bottom - t.5).abs() > TOL {
                fail(format!("image bottom edge ours {bottom:.3} vs ref {:.3}", t.5), &mut failures);
            }
        }
        // Text anchors: the first glyph of the run whose first cluster starts
        // at a source byte.
        let find = |byte: usize| -> Option<Found> {
            for p in &v2.pages {
                for it in &p.items {
                    if let Item::GlyphRun(run) = it {
                        if run.role != RunRole::Text {
                            continue;
                        }
                        let Some(c) = run.clusters.first() else { continue };
                        if c.provenance.sources().first().is_some_and(|s| s.start_byte == byte) {
                            let g = run.glyphs[0];
                            return Some(Found { page: p.number, x: g.origin_x.to_bp(), y: g.baseline_y.to_bp() });
                        }
                    }
                }
            }
            None
        };
        for para in reference.get("paragraphs").and_then(|v| v.as_arr()).unwrap() {
            let byte = num(para, "source_byte") as usize;
            checks += 1;
            match find(byte) {
                Some(f) => {
                    let d = (f.x - num(para, "x")).abs().max((f.y - num(para, "baseline")).abs());
                    worst = worst.max(d);
                    if f.page != num(para, "page") as u32 || d > TOL {
                        fail(
                            format!("paragraph at byte {byte} ours p{} ({:.3},{:.3}) vs ref p{} ({:.3},{:.3})", f.page, f.x, f.y, num(para, "page"), num(para, "x"), num(para, "baseline")),
                            &mut failures,
                        );
                    }
                }
                None => fail(format!("paragraph at byte {byte} not rendered"), &mut failures),
            }
        }
        report.push_str(&format!("{name}: {checks} checks, worst {worst:.3}bp\n"));
    }
    assert!(failures.is_empty(), "{report}\n{}", failures.join("\n"));
    eprintln!("{report}");
}
