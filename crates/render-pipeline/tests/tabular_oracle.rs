//! `tabular` gate: text and rule geometry against the pdfLaTeX (TeX Live
//! 2026) references pinned in `crates/compiler/tests/tabular_corpus/refs`
//! (made by that corpus's `oracle.py`, test-only; cargo never runs TeX).
//!
//! For every fixture listed in `PASSING`: one page; every reference word's
//! origin has a candidate glyph origin within 0.5 bp (x and y); after merging
//! rules that continue one another (pdfTeX strokes a `|` once per row), the
//! rule counts are equal and every reference rule has a distinct candidate
//! rule within 0.1 bp in x, top, width and height. The corpus lives outside
//! this crate, so the test skips (loudly) when the directory is absent, as in
//! a `git archive` of this crate alone, or when Latin Modern is not installed.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::display::Item;

const WORD_TOL: f64 = 0.5;
const RULE_TOL: f64 = 0.1;

/// Fixtures that match pdfLaTeX today. The others need constructs outside
/// the table layout: math `array`, text-mode `\,`, and justified `p{}`/`m{}`/
/// `b{}` entries whose loose lines paragraph-layout's badness rejects (41/42
/// match in every rule and baseline; only their stretched words' x differ).
const PASSING: &[&str] = &[
    "40-array-gtlt", "43-array-extrarowheight", "44-array-bang", "55-array-newcolumntype",
    "56-array-math-cells", "57-array-w-itshape", "58-array-hline-double", "59-array-m-b-ragged",
    "01-col-l", "02-col-c", "03-col-r", "04-col-lcr", "05-vrule-single", "06-vrule-double", "07-at-empty",
    "08-at-text", "09-at-rule", "10-p-short", "12-p-rules", "13-multicolumn-wide", "14-multicolumn-narrow",
    "15-multicolumn-rules", "16-multicolumn-realign", "17-hline", "18-hline-double", "19-cline", "20-cline-multi",
    "21-arraystretch-15", "22-arraystretch-08", "23-tabcolsep-12", "24-tabcolsep-0", "25-rowskip-pos",
    "26-rowskip-neg", "27-tabularstar-fill", "28-tabularstar-cv", "29-pos-top", "30-pos-bottom", "31-pos-center",
    "32-nested", "34-array-display", "35-booktabs-basic", "36-booktabs-cmidrule", "37-booktabs-cmidrule-lr",
    "38-booktabs-width", "45-empty-cells", "46-math-cells", "47-styled-cells", "48-arrayrulewidth",
    "49-doublerulesep", "50-small-font", "51-center-env", "53-star-no-fill", "54-two-tables-inline",
];

fn num(v: &Value) -> f64 {
    match v {
        Value::Num(n) => *n,
        _ => f64::NAN,
    }
}

/// Merges rules (x, top, width, height) that continue one another.
fn merge(mut rules: Vec<[f64; 4]>) -> Vec<[f64; 4]> {
    loop {
        let mut joined = None;
        'outer: for i in 0..rules.len() {
            for j in 0..rules.len() {
                let (a, b) = (rules[i], rules[j]);
                if i == j {
                    continue;
                }
                let vertical = (a[0] - b[0]).abs() < 0.01 && (a[2] - b[2]).abs() < 0.01 && (a[1] + a[3] - b[1]).abs() < 0.01;
                let horizontal = (a[1] - b[1]).abs() < 0.01 && (a[3] - b[3]).abs() < 0.01 && (a[0] + a[2] - b[0]).abs() < 0.01;
                if vertical || horizontal {
                    joined = Some((i, j, vertical));
                    break 'outer;
                }
            }
        }
        let Some((i, j, vertical)) = joined else { return rules };
        let b = rules[j];
        if vertical {
            rules[i][3] = b[1] + b[3] - rules[i][1];
        } else {
            rules[i][2] = b[0] + b[2] - rules[i][0];
        }
        rules.remove(j);
    }
}

#[test]
fn tabular_fixtures_match_pdflatex() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../compiler/tests/tabular_corpus");
    if !std::path::Path::new(dir).is_dir() {
        eprintln!("SKIP tabular_oracle: {dir} not present");
        return;
    }
    if !common::lm_available() {
        eprintln!("SKIP tabular_oracle: Latin Modern fonts not installed");
        return;
    }
    let mut failures = Vec::new();
    for name in PASSING {
        let tex = std::fs::read_to_string(format!("{dir}/fixtures/{name}.tex")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/refs/{name}.json")).unwrap()).unwrap();
        let rendered = common::render_one(&tex);
        let pages = &rendered.v2.pages;
        if pages.len() != 1 {
            failures.push(format!("{name}: {} pages", pages.len()));
            continue;
        }
        let mut glyphs = Vec::new();
        let mut rules = Vec::new();
        for item in pages[0].resident_items() {
            match item {
                Item::GlyphRun(run) => glyphs.extend(run.glyphs.iter().map(|g| (g.origin_x.to_bp(), g.baseline_y.to_bp()))),
                Item::Rule(r) => rules.push([r.x.to_bp(), r.top.to_bp(), r.width.to_bp(), r.height.to_bp()]),
                _ => {}
            }
        }
        let words = reference.get("pages").and_then(|p| p.as_arr()).and_then(|p| p.first()).and_then(|p| p.as_arr()).unwrap();
        for w in words {
            let (x, y) = (num(w.get("x").unwrap()), num(w.get("y_top").unwrap()));
            if !glyphs.iter().any(|(gx, gy)| (gx - x).abs() <= WORD_TOL && (gy - y).abs() <= WORD_TOL) {
                failures.push(format!("{name}: no glyph at word {:?} ({x:.3}, {y:.3})", w.get("text").and_then(|t| t.as_str()).unwrap_or("")));
            }
        }
        let ref_rules: Vec<[f64; 4]> = reference
            .get("rules")
            .and_then(|r| r.as_arr())
            .and_then(|r| r.first())
            .and_then(|r| r.as_arr())
            .unwrap()
            .iter()
            .map(|r| {
                let v = r.as_arr().unwrap();
                [num(&v[0]), num(&v[1]), num(&v[2]), num(&v[3])]
            })
            .collect();
        let mut ours = merge(rules);
        if ours.len() != ref_rules.len() {
            failures.push(format!("{name}: {} rules vs reference {}", ours.len(), ref_rules.len()));
        }
        for r in &ref_rules {
            let best = ours
                .iter()
                .enumerate()
                .map(|(i, o)| (i, (0..4).map(|k| (o[k] - r[k]).abs()).fold(0.0, f64::max)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match best {
                Some((i, d)) if d <= RULE_TOL => {
                    ours.remove(i);
                }
                _ => failures.push(format!("{name}: no rule within {RULE_TOL} bp of {r:?}")),
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
