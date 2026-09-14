//! Display placement in running text against pdfLaTeX (MacTeX 2026)
//! references pinned in `fixtures/display-placement/refs/*.json` (made by
//! `fixtures/display-placement/oracle.py refs`; cargo never runs TeX).
//!
//! Short vs normal display skips (`\predisplaysize`), equation numbers right
//! and left (`leqno`, primitive `\eqno`/`\leqno`), amsmath `fleqn`,
//! `\tag`/`\tag*`/`\notag`, displays in lists and `quote`, right after a
//! heading and at page breaks. Every reference word must start at a glyph
//! of the same character within 0.5 bp (the full word-by-word comparison is
//! `oracle.py check`). Too-wide formulas are squeezed by their math glue
//! with the number beside them or on a line of its own (09, 10, 33; needs
//! math-layout `MathBox::pack_to`), and `\numberwithin`/`subequations`
//! numbers come from the compiler (18; 17's numbers are right, its `\eqref`
//! misses LaTeX's `\sw@slant` italic correction before the space). Fixtures
//! not listed here do not pass yet: 17, `\tag{$..$}` math (15) and nested
//! list labels (20).

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 0.5;

const PASSING: &[&str] = &[
    "01-short-line-bracket",
    "02-long-line-bracket",
    "03-par-start-bracket",
    "04-equation-short",
    "05-equation-long",
    "06-equation-leqno",
    "07-equation-fleqn",
    "08-fleqn-leqno",
    "09-long-equation-number-below",
    "10-long-equation-leqno-above",
    "11-dollars-short",
    "12-dollars-eqno",
    "13-dollars-leqno",
    "14-equation-star",
    "16-gather-numbers",
    "18-subequations",
    "19-itemize-display",
    "21-after-heading",
    "22-page-bottom",
    "23-page-top",
    "24-11pt-short-long",
    "25-12pt-short-long",
    "26-multline-number",
    "27-display-then-blank-line",
    "28-line-ends-inline-math",
    "29-quote-display",
    "30-widow-display",
    "31-tag-equation",
    "32-consecutive-displays",
    "33-wide-tag-shifts-formula",
    "34-parindent-medium-line",
    "35-12pt-fleqn-leqno-align",
    "36-cm-default-fonts",
];

fn num(v: &Value, k: &str) -> f64 {
    match v.get(k) {
        Some(Value::Num(n)) => *n,
        _ => f64::NAN,
    }
}

#[test]
fn display_placement_matches_pdflatex() {
    if !common::lm_available() {
        eprintln!("SKIP display_placement: Latin Modern fonts not installed");
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/display-placement");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let mut failures = Vec::new();
    for name in PASSING {
        let tex = std::fs::read_to_string(format!("{dir}/fixtures/{name}.tex")).unwrap();
        let reference = json::parse(&std::fs::read_to_string(format!("{dir}/refs/{name}.json")).unwrap()).unwrap();
        let docs = [SourceDocument { path: "main.tex", text: &tex }];
        let r = render(&docs, "main.tex", 1, "display-placement", &fonts, &options);
        let pages = reference.get("pages").and_then(|v| v.as_arr()).unwrap();
        if pages.len() != r.v2.pages.len() {
            failures.push(format!("{name}: {} pages, reference {}", r.v2.pages.len(), pages.len()));
            continue;
        }
        for (page, words) in r.v2.pages.iter().zip(pages) {
            // (first char, x bp, baseline y bp) of every glyph on the page.
            let mut glyphs = Vec::new();
            for item in page.resident_items() {
                if let Item::GlyphRun(run) = item {
                    let mut chars = run.clusters.iter().map(|c| run.text[c.text_start_byte as usize..c.text_end_byte as usize].chars().next());
                    for g in &run.glyphs {
                        glyphs.push((chars.next().flatten(), g.origin_x.to_bp(), g.baseline_y.to_bp()));
                    }
                }
            }
            for w in words.as_arr().unwrap() {
                let text = match w.get("text") {
                    Some(Value::Str(s)) => s.clone(),
                    _ => continue,
                };
                // pdftext leaves the itemize bullet (an unmapped glyph name) as "?".
                let Some(first) = text.chars().next().filter(|c| *c != '?') else { continue };
                let (x, y) = (num(w, "x"), num(w, "y_top"));
                let hit = glyphs.iter().any(|(c, gx, gy)| *c == Some(first) && (gx - x).abs() <= TOL && (gy - y).abs() <= TOL);
                if !hit {
                    failures.push(format!("{name} p{}: {text:?} at ({x:.2}, {y:.2}) has no glyph within {TOL} bp", page.number));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
