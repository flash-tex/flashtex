//! `color`/`xcolor` gate: glyph and rule geometry with their exact colour
//! operators against the pdfLaTeX (TeX Live 2026) references pinned in
//! `tests/xcolor_oracle/refs` (made by `tests/xcolor_oracle/oracle.py`,
//! test-only; cargo never runs TeX).
//!
//! For every fixture in `PASSING`: the same page count; every reference
//! glyph has a candidate glyph within 0.5 bp (x and y) whose run paints the
//! same fill operator (a math extension glyph, whose painted origin
//! legitimately differs, only needs the nearest glyph to paint it), with the
//! operator written as pdfTeX does (`1 0 0 rg`, `0 0 1 0 k`, `0.8 g`; the default colour
//! is pdfTeX's page-start `0 g`); the rule counts are equal and every
//! reference rule (fills, `\fcolorbox` frames, `\pagecolor`) has a distinct
//! candidate rule within 0.1 bp in x, top, width and height with the same
//! operator. Skips (loudly) without Latin Modern.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::display::{Item, Paint};

const TOL: f64 = 0.5;
const RULE_TOL: f64 = 0.1;

/// Fixtures that match pdfLaTeX today (the others are listed in the PR with
/// the construct they need).
const PASSING: &[&str] = &[
    "01-base-colours",
    "02-definecolor-models",
    "03-mix-expressions",
    "04-current-colour",
    "05-colorlet",
    "06-dvipsnames",
    "07-svgnames",
    "08-x11names",
    "09-target-rgb",
    "10-target-cmyk",
    "11-target-gray",
    "12-nested-scopes",
    "13-colour-across-lines",
    "14-colour-across-pages",
    "15-colorbox",
    "16-fcolorbox",
    "17-fboxsep-fboxrule",
    "18-colorbox-content-colour",
    "19-pagecolor",
    "20-inline-math-colour",
    "21-display-math-colour",
    "22-heading-colour",
    "23-color-sty",
    "24-macro-colour",
    "25-font-and-colour",
    "26-environment-colour",
    "27-undeclared-models",
    "28-colorbox-math",
];

fn num(v: &Value) -> f64 {
    match v {
        Value::Num(n) => *n,
        _ => f64::NAN,
    }
}

fn str_of(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn fill_of(p: &Paint) -> String {
    p.device.map(|d| d.fill_operator()).unwrap_or_else(|| "0 g".to_string())
}

#[test]
fn xcolor_fixtures_match_pdflatex() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/xcolor_oracle");
    if !common::lm_available() {
        eprintln!("SKIP xcolor_oracle: Latin Modern fonts not installed");
        return;
    }
    let mut failures = Vec::new();
    for name in PASSING {
        let tex = std::fs::read_to_string(format!("{dir}/fixtures/{name}.tex")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/refs/{name}.json")).unwrap()).unwrap();
        let ref_pages = reference.get("pages").and_then(|p| p.as_arr()).unwrap();
        let rendered = common::render_one(&tex);
        let pages = &rendered.v2.pages;
        if pages.len() != ref_pages.len() {
            failures.push(format!("{name}: {} pages vs reference {}", pages.len(), ref_pages.len()));
            continue;
        }
        for (pi, (page, rp)) in pages.iter().zip(ref_pages).enumerate() {
            let mut glyphs: Vec<(f64, f64, String)> = Vec::new();
            let mut rules: Vec<([f64; 4], String)> = Vec::new();
            for item in &page.items {
                match item {
                    Item::GlyphRun(run) => {
                        let fill = fill_of(&run.paint);
                        glyphs.extend(run.glyphs.iter().map(|g| (g.origin_x.to_bp(), g.baseline_y.to_bp(), fill.clone())));
                    }
                    Item::Rule(r) => rules.push(([r.x.to_bp(), r.top.to_bp(), r.width.to_bp(), r.height.to_bp()], fill_of(&r.paint))),
                    _ => {}
                }
            }
            for g in rp.get("glyphs").and_then(|g| g.as_arr()).unwrap() {
                let g = g.as_arr().unwrap();
                let (text, x, y, fill) = (str_of(&g[0]), num(&g[1]), num(&g[2]), str_of(&g[3]));
                let font = g.get(4).map(str_of).unwrap_or("").to_uppercase();
                let extension = font.contains("CMEX") || font.contains("LMEX") || font.contains("MATHEXTENSION");
                let nearest = glyphs
                    .iter()
                    .filter(|(gx, gy, _)| extension || ((gx - x).abs() <= TOL && (gy - y).abs() <= TOL))
                    .min_by(|a, b| {
                        let da = (a.0 - x).abs().max((a.1 - y).abs());
                        let db = (b.0 - x).abs().max((b.1 - y).abs());
                        da.total_cmp(&db)
                    });
                match nearest {
                    None => failures.push(format!("{name} p{}: no glyph at {text:?} ({x:.3}, {y:.3})", pi + 1)),
                    Some((_, _, f)) if f != fill => {
                        failures.push(format!("{name} p{}: glyph {text:?} painted {f}, pdfTeX {fill}", pi + 1))
                    }
                    Some(_) => {}
                }
            }
            let ref_rules = rp.get("rules").and_then(|r| r.as_arr()).unwrap();
            if ref_rules.len() != rules.len() {
                failures.push(format!("{name} p{}: {} rules vs reference {}", pi + 1, rules.len(), ref_rules.len()));
            }
            for r in ref_rules {
                let v = r.as_arr().unwrap();
                let (want, fill) = ([num(&v[0]), num(&v[1]), num(&v[2]), num(&v[3])], str_of(&v[4]));
                let best = rules
                    .iter()
                    .enumerate()
                    .map(|(i, (o, _))| (i, (0..4).map(|k| (o[k] - want[k]).abs()).fold(0.0, f64::max)))
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                match best {
                    Some((i, d)) if d <= RULE_TOL => {
                        let (_, f) = rules.remove(i);
                        if f != fill {
                            failures.push(format!("{name} p{}: rule {want:?} painted {f}, pdfTeX {fill}", pi + 1));
                        }
                    }
                    _ => failures.push(format!("{name} p{}: no rule within {RULE_TOL} bp of {want:?}", pi + 1)),
                }
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
