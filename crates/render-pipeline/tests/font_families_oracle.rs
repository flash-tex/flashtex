//! NFSS font families, series and shapes against pdfLaTeX (MacTeX 2026)
//! references committed in `fixtures/font-families/reference/*.json` (made
//! by `fixtures/font-families/oracle.py`, test-only; cargo never runs TeX).
//!
//! Every fixture is compared segment by segment: a segment is a maximal
//! run of glyphs in one font with no interword gap. Text, the font design
//! (pdfLaTeX's Type 1 `CMSS10`/`SFSS1000`/`LMSans10-Regular` mapped to the
//! Latin Modern OpenType design the pipeline draws) and the left edge and
//! baseline (0.5 bp) must agree, and every "Font shape ..." warning
//! pdfLaTeX logs must be among the pipeline's diagnostics.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, RunRole, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL_BP: f64 = 0.5;

/// Fixtures that depend on work not on this branch, with the reason. They
/// must still differ (so the entry is removed when the dependency lands).
///
/// Empty on integration/2026-09-13k: PR #129 (text-logos-rule-symbols-pipeline)
/// and the vendor/compiler re-pin carrying PR #136 (font-families-compiler)
/// are both merged, so the `\LaTeX` logo and `\mathsf`/`\mathtt`/`\mathit`/
/// `\mathfrak` Unicode-alphanumeric fixtures below now match pdfLaTeX.
const PENDING: [(&str, &str); 0] = [];

#[derive(Debug, Clone)]
struct Seg {
    page: u32,
    text: String,
    design: String,
    x: f64,
    baseline: f64,
}

/// The Latin Modern design a pdfLaTeX font name stands for: Computer Modern
/// (`CMSSBX10`), cm-super EC (`SFSX1095`) or Latin Modern Type 1
/// (`LMSans10-Bold`), all written as the OpenType family without its
/// optical size (`LMSans-Bold`).
fn design_of_pdf_font(name: &str) -> String {
    if name.starts_with("LMMath") {
        return "LatinModernMath-Regular".to_string();
    }
    if name.starts_with("LM") {
        return name.chars().filter(|c| !c.is_ascii_digit()).collect();
    }
    let letters: String = name.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let design = match letters.as_str() {
        "CMR" | "SFRM" => "LMRoman-Regular",
        "CMBX" | "SFBX" => "LMRoman-Bold",
        "CMTI" | "SFTI" => "LMRoman-Italic",
        "CMBXTI" | "SFBI" => "LMRoman-BoldItalic",
        "CMSL" | "SFSL" => "LMRomanSlant-Regular",
        "CMBXSL" | "SFBL" => "LMRomanSlant-Bold",
        "CMCSC" | "SFCC" => "LMRomanCaps-Regular",
        // No bold small caps in Latin Modern: the medium design is drawn.
        "SFXC" => "LMRomanCaps-Regular",
        "SFSC" => "LMRomanCaps-Oblique",
        "CMB" | "SFRB" => "LMRomanDemi-Regular",
        "CMSS" | "SFSS" => "LMSans-Regular",
        "CMSSI" | "SFSI" => "LMSans-Oblique",
        "CMSSBX" | "SFSX" => "LMSans-Bold",
        "SFSO" => "LMSans-BoldOblique",
        "CMTT" | "SFTT" => "LMMono-Regular",
        "CMITT" | "SFIT" => "LMMono-Italic",
        "CMSLTT" | "SFST" => "LMMonoSlant-Regular",
        "CMTCSC" | "SFTC" => "LMMonoCaps-Regular",
        // Math: Latin Modern Math draws the math italic, symbol, extension
        // and (Euler) fraktur fonts.
        "CMMI" | "CMSY" | "CMEX" | "EUFM" => "LatinModernMath-Regular",
        other => return format!("unmapped:{other}"),
    };
    design.to_string()
}

fn num(v: &Value, k: &str) -> f64 {
    match v.get(k) {
        Some(Value::Num(n)) => *n,
        _ => f64::NAN,
    }
}

fn text(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::Str(s)) => s.clone(),
        _ => String::new(),
    }
}

fn ours(r: &flashtex_render_pipeline::Rendered) -> Vec<Seg> {
    let v2 = &r.v2;
    let mut segs: Vec<Seg> = Vec::new();
    let mut last_end = f64::NAN;
    let mut last_size = 0.0f64;
    for page in &v2.pages {
        for it in page.resident_items() {
            let Item::GlyphRun(run) = it else { continue };
            if !matches!(run.role, RunRole::Text | RunRole::Math) || run.glyphs.is_empty() {
                continue;
            }
            let face = v2.fonts.iter().find(|f| f.font_id == run.font_id).map(|f| f.postscript_name.clone()).unwrap_or_default();
            let design: String = face.chars().filter(|c| !c.is_ascii_digit()).collect();
            let first = &run.glyphs[0];
            let last = run.glyphs.last().expect("non-empty");
            let (x, baseline) = (first.origin_x.to_bp(), first.baseline_y.to_bp());
            let end = (last.origin_x.0 + last.advance_x.0) as f64 / TICKS_PER_BP;
            let size = run.font_size.to_bp();
            let joins = segs.last().is_some_and(|s| {
                s.page == page.number && s.design == design && (s.baseline - baseline).abs() < 0.01 && (x - last_end).abs() < 0.15 * last_size.max(size)
            });
            // Math alphabet characters carry their mathematical alphanumeric
            // code point; pdfLaTeX's ToUnicode spells the ASCII letter.
            let run_text: String = run
                .text
                .chars()
                .map(|c| flashtex_render_pipeline::mathalpha::classify(c).map_or(c, |(_, letter)| letter))
                .collect();
            if joins {
                segs.last_mut().expect("joined").text.push_str(&run_text);
            } else {
                segs.push(Seg { page: page.number, text: run_text, design, x, baseline });
            }
            last_end = end;
            last_size = size;
        }
    }
    segs.retain(|s| !s.text.trim().is_empty());
    segs
}

#[test]
fn font_family_fixtures_match_pdflatex() {
    if !common::lm_available() {
        eprintln!("SKIP font_families_oracle: Latin Modern fonts not installed");
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/font-families");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tex") && n.as_bytes()[0].is_ascii_digit())
        .collect();
    names.sort();
    assert!(names.len() >= 30, "expected at least 30 font-family fixtures, found {}", names.len());
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let mut report = String::new();
    let mut failed = Vec::new();
    for name in &names {
        let tex = std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/reference/{}", name.replace(".tex", ".json"))).unwrap()).unwrap();
        let docs = [SourceDocument { path: "main.tex", text: &tex }];
        let r = render(&docs, "main.tex", 1, "font-families", &fonts, &options);
        let theirs: Vec<Seg> = reference
            .get("segments")
            .and_then(|v| v.as_arr())
            .unwrap()
            .iter()
            .map(|s| Seg {
                page: num(s, "page") as u32,
                text: text(s, "text"),
                design: design_of_pdf_font(&text(s, "font")),
                x: num(s, "x"),
                baseline: num(s, "baseline"),
            })
            .collect();
        let ours = ours(&r);
        let mut problems = Vec::new();
        if ours.len() != theirs.len() {
            problems.push(format!("{} segments vs pdflatex {}", ours.len(), theirs.len()));
        }
        let (mut worst_x, mut worst_y) = (0.0f64, 0.0f64);
        for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
            if a.text != b.text || a.page != b.page {
                problems.push(format!("segment {i}: {:?} p{} vs pdflatex {:?} p{}", a.text, a.page, b.text, b.page));
                break;
            }
            if a.design != b.design {
                problems.push(format!("segment {i} {:?}: font {} vs pdflatex {}", a.text, a.design, b.design));
            }
            let (dx, dy) = ((a.x - b.x).abs(), (a.baseline - b.baseline).abs());
            worst_x = worst_x.max(dx);
            worst_y = worst_y.max(dy);
            if dx > TOL_BP || dy > TOL_BP {
                problems.push(format!("segment {i} {:?}: x {:.3} vs {:.3}, baseline {:.3} vs {:.3}", a.text, a.x, b.x, a.baseline, b.baseline));
            }
        }
        let messages: Vec<&str> = r.v2.diagnostics.iter().map(|d| d.message.as_str()).collect();
        for w in reference.get("warnings").and_then(|v| v.as_arr()).unwrap() {
            let Value::Str(w) = w else { continue };
            if !messages.iter().any(|m| m == w) {
                problems.push(format!("missing warning {w:?}"));
            }
        }
        for d in &r.v2.diagnostics {
            if d.code == "font_unavailable" || d.code == "tfm_missing" {
                problems.push(format!("diagnostic {}: {}", d.code, d.message));
            }
        }
        report.push_str(&format!(
            "{name}: {} segments, max |dx| {worst_x:.3} bp, max |dy| {worst_y:.3} bp, {}\n",
            theirs.len(),
            if problems.is_empty() { "ok".to_string() } else { format!("{} problem(s): {}", problems.len(), problems.iter().take(3).cloned().collect::<Vec<_>>().join("; ")) }
        ));
        match (PENDING.iter().find(|(n, _)| n == name), problems.is_empty()) {
            (None, false) => failed.push(name.clone()),
            (Some(_), true) => failed.push(format!("{name} now matches: remove it from PENDING")),
            _ => {}
        }
    }
    eprintln!("{report}");
    assert!(failed.is_empty(), "{} of {} font-family fixtures differ from pdflatex: {failed:?}\n{report}", failed.len(), names.len());
}
