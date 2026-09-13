//! Pseudocode gate: `algorithm` floats (algorithm.sty on float.sty) and
//! `algorithmic`/`algpseudocode` statement lists, against the pdfLaTeX
//! references committed in `fixtures/algorithms/refs` (made by
//! `fixtures/algorithms/oracle.py`, test-only; cargo never runs TeX).
//!
//! Words and rules are extracted exactly as the float-body and tabular
//! corpora do, and compared the same way: the same page count; on every page
//! each reference word's origin has a candidate glyph origin within 0.5 bp
//! (x and y); after merging rules that continue one another the rule counts
//! are equal and every reference rule has a distinct candidate within 0.1 bp
//! in x, top, width and height; no `algorithm_*`/`float_*` diagnostic and no
//! `unsupported_block`.
//!
//! `KNOWN` lists fixtures whose residual is understood and owned elsewhere. A
//! `KNOWN` fixture that starts passing fails the test too, so the list cannot
//! quietly outlive its cause.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::display::{Item, PathCmd, PathPaintOp};

const WORD_TOL: f64 = 0.5;
const RULE_TOL: f64 = 0.1;

/// Fixtures with a documented residual, and the lane that owns it. Every
/// entry below was reproduced *outside* any pseudocode before being listed,
/// so none of them is this module's own layout.
const KNOWN: &[(&str, &str)] = &[
    // `\bmod` is `\mkern5mu\mathbin{mod}\mkern5mu` (amsmath/plain 1223); we
    // set it with no mkern on either side. Control `$r \gets a \bmod b$` in
    // an ordinary paragraph: `mod` 2.77 bp left, everything after 5.54 bp
    // left. Math-layout lane, not pseudocode.
    ("09-algpseudocode-basic", "math-layout: `\\bmod` is missing its `\\mkern5mu` on both sides (2.77 bp each; reproduced with no pseudocode)"),
    ("12-algpseudocode-procedures", "math-layout: `\\bmod`, as 09"),
    // `\dots` between commas is `\ldots` as `\mathinner`: pdflatex's gaps are
    // 1.66 bp wider per dot. Control `$1, 2, \dots, n$` in an ordinary
    // paragraph reproduces it exactly.
    ("10-algpseudocode-loops", "math-layout: `\\dots` inter-dot spacing is 1.66 bp narrow per gap (reproduced with no pseudocode)"),
    // algpseudocode's comment marker is `\hfill\(\triangleright\) #1`, and
    // LaTeX declares `\triangleright` in the `letters` family: cmmi10 slot
    // "2F, 0.5 em. We paint U+25B7 from Latin Modern Math at its OpenType
    // advance (0.858 em) instead, so the marker sits 3.56 bp left. Math-font
    // metrics lane (the same machinery as `\varnothing`/`\mathcal`).
    ("13-algpseudocode-comments", "math fonts: `\\triangleright` is painted at Latin Modern Math's advance, not cmmi10 slot \"2F's 0.5 em (3.56 bp)"),
    // The line right after a float.sty `\hrule` takes no interline glue, so
    // the next rule's position is exactly that line's height+depth. Ours
    // comes from the glyph boxes rather than their TFM heights, so the rules
    // below a rule drift; it accumulates over a float's three rules.
    ("17-algorithm-ref", "font metrics: a line right after an `\\hrule` is as tall as its glyph boxes, not their TFM heights; the rules below drift 0.24 bp"),
    ("18-algorithm-top", "font metrics: the same post-rule line height (0.10 bp)"),
    ("23-algorithm-figure-mix", "font metrics: the same post-rule line height (0.16 bp)"),
    // pdflatex itself reports `Overfull \hbox (124.26015pt too wide) in
    // paragraph at lines 4--5` here and sets that line at maximum shrink
    // (every interword space 1.109 bp narrow, cumulative). We end the
    // paragraph cleanly at natural width. #162 explained this as the
    // pipeline setting interrupted lines at natural width generally; that is
    // not so — the control `\begin{itemize}` interrupting the same sentence
    // matches pdflatex within 0.004 bp. It is specific to what
    // algorithmic.sty leaves in horizontal mode.
    ("20-algorithmic-bare", "pdflatex reports a 124.26pt overfull hbox for this paragraph and sets the line at maximum shrink; we end it at natural width"),
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

/// A path that pdfTeX would paint as a rule: a straight horizontal or
/// vertical stroke (the float.sty rules reach the display list as
/// `Item::Rule`, but a fixture may also carry drawn material).
fn path_rule(path: &flashtex_render_pipeline::display::PathItem) -> Option<[f64; 4]> {
    if path.commands.iter().any(|c| matches!(c, PathCmd::Cubic(..))) {
        return None;
    }
    let pts: Vec<(f64, f64)> = path
        .commands
        .iter()
        .filter_map(|c| match c {
            PathCmd::Move(x, y) | PathCmd::Line(x, y) => Some((x.to_bp(), y.to_bp())),
            _ => None,
        })
        .collect();
    let (x0, x1) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.0), b.max(p.0)));
    let (y0, y1) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.1), b.max(p.1)));
    match &path.op {
        PathPaintOp::Stroke(st) if pts.len() == 2 => {
            let w = st.width.to_bp();
            if (y1 - y0).abs() < 1e-3 {
                Some([x0, y0 - w / 2.0, x1 - x0, w])
            } else if (x1 - x0).abs() < 1e-3 {
                Some([x0 - w / 2.0, y0, w, y1 - y0])
            } else {
                None
            }
        }
        _ => None,
    }
}

#[test]
fn algorithm_fixtures_match_pdflatex() {
    if !common::lm_available() {
        eprintln!("SKIP algorithms_oracle: Latin Modern fonts not installed");
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/algorithms");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tex") && n.as_bytes()[0].is_ascii_digit())
        .map(|n| n.trim_end_matches(".tex").to_string())
        .collect();
    names.sort();
    assert!(names.len() >= 25, "expected at least 25 pseudocode fixtures, found {}", names.len());
    let only = std::env::var("ALGORITHMS_ONLY").ok();
    let mut problems = Vec::new();
    let mut report = String::new();
    for name in names.iter().filter(|n| only.as_deref().is_none_or(|o| n.contains(o))) {
        let tex = std::fs::read_to_string(format!("{dir}/{name}.tex")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/refs/{name}.json")).unwrap()).unwrap();
        let rendered = common::render_one(&tex);
        let mut failures: Vec<String> = Vec::new();
        let (mut word_worst, mut rule_worst, mut words_n, mut rules_n) = (0.0f64, 0.0f64, 0usize, 0usize);
        for d in &rendered.v2.diagnostics {
            if d.code.starts_with("algorithm") || d.code.starts_with("float") || d.code == "unsupported_block" {
                failures.push(format!("{name}: diagnostic {}: {}", d.code, d.message));
            }
        }
        let ref_pages = reference.get("pages").and_then(|p| p.as_arr()).unwrap();
        let ref_rules = reference.get("rules").and_then(|p| p.as_arr()).unwrap();
        let pages = &rendered.v2.pages;
        if pages.len() != ref_pages.len() {
            failures.push(format!("{name}: {} pages vs reference {}", pages.len(), ref_pages.len()));
        }
        for (pi, (page, (words, rules))) in pages.iter().zip(ref_pages.iter().zip(ref_rules)).enumerate() {
            let mut glyphs = Vec::new();
            let mut ours = Vec::new();
            for item in &page.items {
                match item {
                    Item::GlyphRun(run) => glyphs.extend(run.glyphs.iter().map(|g| (g.origin_x.to_bp(), g.baseline_y.to_bp()))),
                    Item::Rule(r) => ours.push([r.x.to_bp(), r.top.to_bp(), r.width.to_bp(), r.height.to_bp()]),
                    Item::Path(path) => ours.extend(path_rule(path)),
                    _ => {}
                }
            }
            for w in words.as_arr().unwrap() {
                let (x, y) = (num(w.get("x").unwrap()), num(w.get("y_top").unwrap()));
                words_n += 1;
                let best = glyphs.iter().map(|(gx, gy)| (gx - x).abs().max((gy - y).abs())).fold(f64::INFINITY, f64::min);
                word_worst = word_worst.max(best);
                if best > WORD_TOL {
                    failures.push(format!(
                        "{name} p{}: no glyph at word {:?} ({x:.3}, {y:.3}); nearest {best:.3}",
                        pi + 1,
                        w.get("text").and_then(|t| t.as_str()).unwrap_or("")
                    ));
                }
            }
            let reference_rules: Vec<[f64; 4]> = rules
                .as_arr()
                .unwrap()
                .iter()
                .map(|r| {
                    let v = r.as_arr().unwrap();
                    [num(&v[0]), num(&v[1]), num(&v[2]), num(&v[3])]
                })
                .collect();
            let mut ours = merge(ours);
            if ours.len() != reference_rules.len() {
                failures.push(format!("{name} p{}: {} rules vs reference {}", pi + 1, ours.len(), reference_rules.len()));
            }
            for r in &reference_rules {
                rules_n += 1;
                let best = ours
                    .iter()
                    .enumerate()
                    .map(|(i, o)| (i, (0..4).map(|k| (o[k] - r[k]).abs()).fold(0.0, f64::max)))
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                match best {
                    Some((i, d)) => {
                        rule_worst = rule_worst.max(d);
                        if d <= RULE_TOL {
                            ours.remove(i);
                        } else {
                            failures.push(format!("{name} p{}: no rule within {RULE_TOL} bp of {r:?}; nearest {:?} ({d:.3})", pi + 1, ours[i]));
                        }
                    }
                    None => failures.push(format!("{name} p{}: no rule for {r:?}", pi + 1)),
                }
            }
        }
        let known = KNOWN.iter().find(|(n, _)| *n == name.as_str());
        let status = match (failures.is_empty(), known) {
            (true, None) => "pass",
            (true, Some((_, why))) => {
                problems.push(format!("{name} now matches pdflatex; remove it from KNOWN ({why})"));
                "PASS (stale KNOWN)"
            }
            (false, Some(_)) => "known",
            (false, None) => {
                problems.extend(failures.iter().take(12).cloned());
                if failures.len() > 12 {
                    problems.push(format!("{name}: and {} more", failures.len() - 12));
                }
                "FAIL"
            }
        };
        report.push_str(&format!(
            "| {name} | {status} | {}/{} | {words_n} | {word_worst:.3} | {rules_n} | {rule_worst:.3} |\n",
            pages.len(),
            ref_pages.len()
        ));
    }
    println!("| fixture | result | pages ours/ref | words | worst word (bp) | rules | worst rule (bp) |\n|---|---|---|---|---|---|---|\n{report}");
    assert!(problems.is_empty(), "{} problems:\n{}", problems.len(), problems.join("\n"));
}
