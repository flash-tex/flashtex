//! GH-69: `\thefigure`/`\thetable` against pdfLaTeX (MacTeX 2026) references
//! committed in `fixtures/float-numbering/reference/*.json` (made by
//! `fixtures/float-numbering/oracle.py`, test-only; cargo never runs TeX).
//!
//! report.cls and book.cls number floats within the chapter
//! (`\ifnum\c@chapter>\z@\thechapter.\fi\@arabic\c@figure`, reset by
//! `\@addtoreset{figure}{chapter}`); `\chapter*`, book's front and back
//! matter, and `\appendix` each change that differently. Every caption must
//! print the reference label, on the reference page, within 1 pt (bp) of
//! the reference caption start, and every `\ref` must resolve to the same
//! value.
//!
//! A `NN-*/` directory is a multi-file project (`main.tex` plus the files
//! it `\include`s or `\input`s), handed over in file-name order rather
//! than reading order, so the numbers must follow the resolved source
//! order, `\includeonly` included.

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

/// `(page, x, baseline, text)` of every text run.
fn runs(r: &flashtex_render_pipeline::Rendered) -> Vec<(u32, f64, f64, String)> {
    let mut out = Vec::new();
    for p in &r.v2.pages {
        for it in &p.items {
            if let Item::GlyphRun(run) = it {
                if run.role == RunRole::Text && !run.glyphs.is_empty() {
                    let g = run.glyphs[0];
                    out.push((p.number, g.origin_x.to_bp(), g.baseline_y.to_bp(), run.text.clone()));
                }
            }
        }
    }
    out
}

#[test]
fn float_numbers_match_pdflatex() {
    if !common::lm_available() {
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/float-numbering");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.as_bytes()[0].is_ascii_digit() && (n.ends_with(".tex") || std::path::Path::new(&format!("{dir}/{n}")).is_dir()))
        .collect();
    names.sort();
    assert_eq!(names.len(), 7, "expected 7 float-numbering fixtures");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    let mut failures = Vec::new();
    for name in &names {
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/reference/{}.json", name.trim_end_matches(".tex"))).unwrap()).unwrap();
        let files: Vec<(String, String)> = if name.ends_with(".tex") {
            vec![("main.tex".to_string(), std::fs::read_to_string(format!("{dir}/{name}")).unwrap())]
        } else {
            let mut files: Vec<(String, String)> = std::fs::read_dir(format!("{dir}/{name}"))
                .unwrap()
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|f| f.ends_with(".tex"))
                .map(|f| (f.clone(), std::fs::read_to_string(format!("{dir}/{name}/{f}")).unwrap()))
                .collect();
            files.sort();
            files
        };
        let docs: Vec<SourceDocument<'_>> = files.iter().map(|(path, text)| SourceDocument { path, text }).collect();
        let r = render(&docs, "main.tex", 1, "float-numbering", &fonts, &options);
        let runs = runs(&r);
        let ref_pages = num(&reference, "pages") as usize;
        if r.v2.pages.len() != ref_pages {
            failures.push(format!("{name}: pages {} vs reference {ref_pages}", r.v2.pages.len()));
        }
        for c in reference.get("captions").and_then(|v| v.as_arr()).unwrap() {
            let label = c.get("label").and_then(|v| v.as_str()).unwrap();
            let (page, x, baseline) = (num(c, "page") as u32, num(c, "x"), num(c, "baseline"));
            let (word, number) = label.split_once(' ').unwrap();
            let on_line: Vec<&(u32, f64, f64, String)> = runs.iter().filter(|r| r.0 == page && (r.2 - baseline).abs() <= TOL).collect();
            let at = on_line.iter().position(|r| r.3 == word && (r.1 - x).abs() <= TOL);
            match at.and_then(|i| on_line.get(i + 1)) {
                Some(next) if next.3 == format!("{number}:") => {}
                _ => {
                    let seen: Vec<String> = runs.iter().filter(|r| r.3 == word).map(|r| format!("p{} ({:.2},{:.2})", r.0, r.1, r.2)).collect();
                    let line: Vec<&str> = on_line.iter().map(|r| r.3.as_str()).collect();
                    failures.push(format!("{name}: caption {label:?} at p{page} ({x:.2},{baseline:.2}) not found; line there {line:?}; `{word}` runs at {seen:?}"));
                }
            }
        }
        let refs = reference.get("refs").and_then(|v| v.as_str()).unwrap();
        let line = runs.iter().find(|r| r.3 == "Refs").map(|first| runs.iter().filter(|r| r.0 == first.0 && r.2 == first.2).map(|r| r.3.as_str()).collect::<Vec<_>>().join(" "));
        if line.as_deref() != Some(refs) {
            failures.push(format!("{name}: refs line {line:?} vs reference {refs:?}"));
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
