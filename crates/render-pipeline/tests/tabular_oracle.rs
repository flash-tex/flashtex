//! `tabular` gate: text and rule geometry against the pdfLaTeX (TeX Live
//! 2026) references pinned in `crates/compiler/tests/tabular_corpus/refs`
//! (made by that corpus's `oracle.py`, test-only; cargo never runs TeX).
//!
//! For every fixture in the corpus: the page count equals the reference's (a
//! `longtable` runs over several); on each page every reference word's origin
//! has a candidate glyph origin within 0.5 bp (x and y); and, after merging
//! rules that continue one another *on both sides* (pdfTeX strokes a `|` once
//! per row while this pipeline may paint one rule down the table), the rule
//! counts are equal and every reference rule has a distinct candidate rule
//! within 0.1 bp in x, top, width and height whose colour matches to 0.001.
//! The corpus lives outside this crate, so the test skips (loudly) when the
//! directory is absent, as in a `git archive` of this crate alone, or when
//! Latin Modern is not installed.
//!
//! This mirrors `oracle.py check`, which is the corpus's own harness but is
//! not run by CI. Keeping the two in step is the point: CI runs `cargo test`,
//! so whatever this file does not measure is not measured at all.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::display::Item;

const WORD_TOL: f64 = 0.5;
const RULE_TOL: f64 = 0.1;
/// Colour components (sRGB, 0-1) of rules and colortbl fills; pdfTeX writes
/// xcolor's decimals, so they must agree to this.
const COLOR_TOL: f64 = 0.001;

/// `[x, top, width, height, r, g, b]`.
type Rule = [f64; 7];

/// Every fixture in the corpus is expected to match: the corpus is the gate,
/// so a fixture is added only once it does. There is deliberately no opt-out
/// list -- one used to sit here naming 53 of the then 59 fixtures, and it was
/// never extended when the longtable/multirow/colortbl fixtures arrived, so
/// `cargo test` went on measuring the same 53 while the new ones rode along
/// unchecked. The names are read from the directory instead.
fn fixtures(dir: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(format!("{dir}/fixtures"))
        .expect("corpus fixtures directory")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().to_str()?.strip_suffix(".tex").map(str::to_string))
        .filter(|name| std::path::Path::new(&format!("{dir}/refs/{name}.json")).is_file())
        .collect();
    names.sort();
    names
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Num(n) => *n,
        _ => f64::NAN,
    }
}

fn same_colour(a: &Rule, b: &Rule) -> bool {
    (4..7).all(|k| (a[k] - b[k]).abs() <= COLOR_TOL)
}

/// Merges rules of one colour that continue one another (`oracle.py`'s
/// `merge_rules`). Rules of different colours are never joined: a colortbl
/// fill and the black rule abutting it are two different paints.
fn merge(mut rules: Vec<Rule>) -> Vec<Rule> {
    loop {
        let mut joined = None;
        'outer: for i in 0..rules.len() {
            for j in 0..rules.len() {
                if i == j {
                    continue;
                }
                let (a, b) = (rules[i], rules[j]);
                if !same_colour(&a, &b) {
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

/// One page of the reference's `rules`. A rule pinned before colours were
/// recorded has only its four geometry entries and is black.
fn reference_rules(page: &Value) -> Vec<Rule> {
    page.as_arr()
        .unwrap()
        .iter()
        .map(|rule| {
            let v = rule.as_arr().unwrap();
            let mut out = [0.0; 7];
            for (k, slot) in out.iter_mut().enumerate() {
                *slot = v.get(k).map_or(0.0, num);
            }
            out
        })
        .collect()
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
    let names = fixtures(dir);
    assert!(!names.is_empty(), "no fixtures with references under {dir}");
    let mut failures = Vec::new();
    let mut matched = 0usize;
    for name in &names {
        let tex = std::fs::read_to_string(format!("{dir}/fixtures/{name}.tex")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/refs/{name}.json")).unwrap()).unwrap();
        let rendered = common::render_one(&tex);
        let pages = &rendered.v2.pages;
        let ref_pages = reference.get("pages").and_then(|p| p.as_arr()).unwrap();
        let ref_rule_pages = reference.get("rules").and_then(|r| r.as_arr()).unwrap();
        if pages.len() != ref_pages.len() {
            failures.push(format!("{name}: {} pages, reference {}", pages.len(), ref_pages.len()));
            continue;
        }
        let before = failures.len();
        for (page_no, page) in pages.iter().enumerate() {
            let at = format!("{name} p{}", page_no + 1);
            let mut glyphs = Vec::new();
            let mut rules: Vec<Rule> = Vec::new();
            for item in &page.items {
                match item {
                    Item::GlyphRun(run) => {
                        glyphs.extend(run.glyphs.iter().map(|g| (g.origin_x.to_bp(), g.baseline_y.to_bp())))
                    }
                    Item::Rule(r) => rules.push([
                        r.x.to_bp(),
                        r.top.to_bp(),
                        r.width.to_bp(),
                        r.height.to_bp(),
                        r.paint.r,
                        r.paint.g,
                        r.paint.b,
                    ]),
                    _ => {}
                }
            }
            for w in ref_pages[page_no].as_arr().unwrap() {
                let (x, y) = (num(w.get("x").unwrap()), num(w.get("y_top").unwrap()));
                if !glyphs.iter().any(|(gx, gy)| (gx - x).abs() <= WORD_TOL && (gy - y).abs() <= WORD_TOL) {
                    let text = w.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    failures.push(format!("{at}: no glyph at word {text:?} ({x:.3}, {y:.3})"));
                }
            }
            let ref_rules = merge(reference_rules(&ref_rule_pages[page_no]));
            let mut ours = merge(rules);
            if ours.len() != ref_rules.len() {
                failures.push(format!("{at}: {} rules vs reference {}", ours.len(), ref_rules.len()));
            }
            for r in &ref_rules {
                let best = ours
                    .iter()
                    .enumerate()
                    .map(|(i, o)| (i, (0..4).map(|k| (o[k] - r[k]).abs()).fold(0.0, f64::max)))
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                match best {
                    Some((i, d)) if d <= RULE_TOL && same_colour(&ours[i], r) => {
                        ours.remove(i);
                    }
                    _ => failures.push(format!("{at}: no rule within {RULE_TOL} bp of {r:?}")),
                }
            }
        }
        matched += usize::from(failures.len() == before);
    }
    assert!(
        failures.is_empty(),
        "{} mismatches over {} fixtures:\n{}",
        failures.len(),
        names.len(),
        failures.join("\n")
    );
    eprintln!("tabular_oracle: {matched}/{} fixtures match pdflatex", names.len());
}
