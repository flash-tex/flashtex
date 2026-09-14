//! Where a page ends, and how tall the column that holds it is set.
//!
//! Both fixtures here are pages whose break TeX does *not* put at the
//! obvious place, and both were wrong in the same document
//! (`fixtures/real-world/lmodern-report`, whose page 1 sat 1.83bp low with
//! the offset growing to 110bp by page 4).
//!
//! 1. **`\brokenpenalty`** (tex.web 890). The penalty node TeX appends
//!    between two lines carries `\interlinepenalty` + `\clubpenalty` +
//!    `\widowpenalty` + `\brokenpenalty`, the last one whenever *this*
//!    line's break was a discretionary. LaTeX sets it to 100
//!    (`\showthe\brokenpenalty` in any standard class). Without it the
//!    page builder reads a hyphenated line as an ordinary breakpoint and
//!    ends the page there, one line early or late.
//!
//! 2. **`\@makecol`'s `\vbox to\@colht`.** The column is packed to
//!    `\@colht`, so a column carrying more material than fits shrinks its
//!    glue. The float column builder was laying its pages out at their
//!    natural size instead, which leaves every line below the first
//!    shrinkable skip low by the whole shrink.
//!
//! The two fixtures separate the behaviours. `page-break-hyphen` ends its
//! page at a different line once `\brokenpenalty` is charged — ten words
//! move to the next page. `page-column-shrink` keeps the same break either
//! way (every word aligns before and after) and only moves vertically: with
//! the column left at its natural size the page runs 4.01pt past the text
//! area and every line under the first shrinkable skip sits 1.47–1.87bp low.
//!
//! Expected data: each fixture's `expected.txt`, every word's origin and
//! baseline read from pdflatex's own PDF by `tools/visual-oracle/pdftext.py`.
//! No TeX runs here.
//!
//! Gate (0.5bp): the page count matches and every oracle word has a word of
//! ours with the same text on the same page within 0.5bp in x and baseline.

mod common;

use common::*;

const TOL_BP: f64 = 0.5;

#[derive(Debug, Clone)]
struct W {
    page: u32,
    text: String,
    x: f64,
    baseline: f64,
}

fn oracle(data: &str) -> (usize, Vec<W>) {
    let mut pages = 0;
    let mut words = Vec::new();
    for line in data.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let f: Vec<&str> = line.splitn(5, ' ').collect();
        match f[0] {
            "page" => pages += 1,
            "word" => words.push(W { page: f[1].parse().unwrap(), x: f[2].parse().unwrap(), baseline: f[3].parse().unwrap(), text: f[4].to_string() }),
            other => panic!("unknown record {other}"),
        }
    }
    (pages, words)
}

/// Glyph runs joined into words the way the oracle splits them (a run that
/// starts where the previous one ended, on the same baseline, continues the
/// word).
fn ours(r: &flashtex_render_pipeline::Rendered) -> Vec<W> {
    let mut out: Vec<W> = Vec::new();
    let mut prev_end: Option<(u32, f64, f64)> = None;
    for w in words_of(r) {
        match (out.last_mut(), prev_end) {
            (Some(last), Some((p, end, base))) if p == w.page && w.x - end > -0.01 && w.x - end < 1.0 && (w.baseline - base).abs() < 0.01 => last.text.push_str(&w.text),
            _ => out.push(W { page: w.page, text: w.text.clone(), x: w.x, baseline: w.baseline }),
        }
        prev_end = Some((w.page, w.x + w.width, w.baseline));
    }
    out
}

fn check(fixture: &str) {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(fixture);
    let main = std::fs::read_to_string(dir.join("main.tex")).unwrap();
    let r = render_docs(&[("main.tex", main.as_str())], "main.tex");
    let (pages, expected) = oracle(&std::fs::read_to_string(dir.join("expected.txt")).unwrap());
    assert_eq!(r.v2.pages.len(), pages, "{fixture}: page count");
    let got = ours(&r);
    let mut misses = Vec::new();
    for e in &expected {
        let best = got
            .iter()
            .filter(|g| g.page == e.page && g.text == e.text)
            .map(|g| (g, (g.x - e.x).abs().max((g.baseline - e.baseline).abs())))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match best {
            Some((_, d)) if d <= TOL_BP => {}
            Some((g, d)) => misses.push(format!("p{} {:?} at ({:.3},{:.3}) nearest ours ({:.3},{:.3}) off by {d:.3}bp", e.page, e.text, e.x, e.baseline, g.x, g.baseline)),
            None => misses.push(format!("p{} {:?} at ({:.3},{:.3}) missing", e.page, e.text, e.x, e.baseline)),
        }
    }
    assert!(misses.is_empty(), "{fixture}: {} of {} oracle words off:\n{}", misses.len(), expected.len(), misses.join("\n"));
}

/// `\brokenpenalty`: the page does not end at the hyphen pdflatex carried
/// past. Before the penalty was charged this page held ten words pdflatex
/// sets on the next one.
#[test]
fn a_hyphenated_line_is_not_the_cheapest_place_to_end_a_page() {
    check("page-break-hyphen");
}

/// `\@makecol`: a column taller than `\@colht` is set at its shrunk glue,
/// on the float page builder's path as well as the plain one. The page break
/// is the same either way; before, the page simply ran 4.01pt past the text
/// area with every line under the first shrinkable skip 1.47-1.87bp low.
#[test]
fn a_column_taller_than_colht_is_set_at_its_shrunk_glue() {
    check("page-column-shrink");
}
