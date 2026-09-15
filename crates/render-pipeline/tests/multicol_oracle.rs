//! multicol (`multicols`, `multicols*`) against pdflatex.
//!
//! Expected data: `fixtures/multicol/expected/*.txt`, every word's origin
//! and baseline and every rule read from the content streams of pdflatex's
//! own PDF (MacTeX 2026, pdfTeX 1.40.29, multicol v2.0b) by
//! `tools/multicol-oracle/generate.py`. No TeX runs here.
//!
//! Per fixture: page count, words matched in reading order per page by
//! text (x and baseline within 0.5bp), rules within 0.1bp, and multicol's
//! own warnings. `EXACT` lists the fixtures gated at 100%; the others are
//! reported (`MULTICOL_REPORT=1` prints the table).

mod common;

use common::*;

const WORD_TOL_BP: f64 = 0.5;
const RULE_TOL_BP: f64 = 0.1;

const FIXTURES: &[&str] = &[
    "01-two-balanced",
    "02-three-columns",
    "03-four-columns",
    "04-uneven-content",
    "05-multicols-star",
    "06-columnbreak",
    "07-preface",
    "08-multipage",
    "09-columnseprule",
    "10-raggedcolumns",
    "11-itemize-inside",
    "12-display-inside",
    "13-heading-before",
    "14-twocolumn-document",
    "15-two-environments",
    "16-premulticols-newpage",
    "17-columnsep",
    "18-multicolsep",
    "19-three-multipage",
    "20-section-inside",
    "21-star-multipage",
    "22-twelve-point",
    "23-short-content",
    "24-columnbreak-inline",
    "25-mid-page-after-text",
];

/// Fixtures whose every word and rule must match. Not gated:
/// `11-itemize-inside` (the list's `\topsep`/`\partopsep` reach the page
/// builder without their stretch, so a flush column spreads its glue
/// differently: 0.8bp at the list) and `14-twocolumn-document` (multicols in
/// a two-column document is not implemented; a diagnostic says so).
const EXACT: &[&str] = &[
    "01-two-balanced",
    "02-three-columns",
    "03-four-columns",
    "04-uneven-content",
    "05-multicols-star",
    "06-columnbreak",
    "07-preface",
    "08-multipage",
    "09-columnseprule",
    "10-raggedcolumns",
    "12-display-inside",
    "13-heading-before",
    "15-two-environments",
    "16-premulticols-newpage",
    "17-columnsep",
    "18-multicolsep",
    "19-three-multipage",
    "20-section-inside",
    "21-star-multipage",
    "22-twelve-point",
    "23-short-content",
    "24-columnbreak-inline",
    "25-mid-page-after-text",
];

#[derive(Debug, Clone)]
struct W {
    page: u32,
    text: String,
    x: f64,
    baseline: f64,
}

struct Expected {
    pages: usize,
    words: Vec<W>,
    rules: Vec<(u32, f64, f64, f64, f64)>,
    warnings: Vec<String>,
}

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/multicol")
}

fn expected(name: &str) -> Expected {
    let data = std::fs::read_to_string(dir().join(format!("expected/{name}.txt"))).unwrap();
    let mut e = Expected { pages: 0, words: Vec::new(), rules: Vec::new(), warnings: Vec::new() };
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        if let Some(w) = line.strip_prefix("warning ") {
            e.warnings.push(w.to_string());
            continue;
        }
        let f: Vec<&str> = line.splitn(7, ' ').collect();
        let num = |i: usize| f[i].parse::<f64>().unwrap();
        match f[0] {
            "page" => e.pages += 1,
            "word" => e.words.push(W { page: f[1].parse().unwrap(), text: f[6].to_string(), x: num(2), baseline: num(3) }),
            "rule" => e.rules.push((f[1].parse().unwrap(), num(2), num(3), num(4), num(5))),
            other => panic!("unknown record {other}"),
        }
    }
    e
}

/// Glyph runs joined into words the way the oracle splits them.
fn our_words(r: &flashtex_render_pipeline::Rendered) -> Vec<W> {
    let mut out: Vec<W> = Vec::new();
    let mut prev_end: Option<(u32, f64, f64)> = None;
    for w in words_of(r) {
        match (out.last_mut(), prev_end) {
            (Some(last), Some((p, end, base))) if p == w.page && (w.x - end).abs() < 0.01 && (w.baseline - base).abs() < 0.01 => last.text.push_str(&w.text),
            _ => out.push(W { page: w.page, text: w.text.clone(), x: w.x, baseline: w.baseline }),
        }
        prev_end = Some((w.page, w.x + w.width, w.baseline));
    }
    out
}

fn our_rules(r: &flashtex_render_pipeline::Rendered) -> Vec<(u32, f64, f64, f64, f64)> {
    let mut out = Vec::new();
    for p in &r.v2.pages {
        for it in &p.items {
            if let flashtex_render_pipeline::display::Item::Rule(rule) = it {
                out.push((p.number, rule.x.to_bp(), rule.top.to_bp(), rule.width.to_bp(), rule.height.to_bp()));
            }
        }
    }
    out
}

#[derive(Debug, Default)]
struct Report {
    pages: (usize, usize),
    words: usize,
    matched: usize,
    ok: usize,
    rules: usize,
    rules_ok: usize,
    warnings_ok: bool,
    first_fail: Option<String>,
}

fn measure(name: &str) -> Report {
    let tex = std::fs::read_to_string(dir().join(format!("{name}.tex"))).unwrap();
    let e = expected(name);
    let r = render_one(&tex);
    let got = our_words(&r);
    if std::env::var("MULTICOL_DUMP").ok().as_deref() == Some(name) {
        let want1: Vec<&W> = e.words.iter().filter(|w| w.page == 1).take(60).collect();
        let got1: Vec<&W> = got.iter().filter(|w| w.page == 1).take(60).collect();
        for i in 0..want1.len().max(got1.len()) {
            eprintln!("{:<40} | {:?}", want1.get(i).map_or(String::new(), |w| format!("{} {:.3} {:.3}", w.text, w.x, w.baseline)), got1.get(i).map(|w| (&w.text, (w.x * 1000.0).round() / 1000.0, (w.baseline * 1000.0).round() / 1000.0)));
        }
        for d in &r.v2.diagnostics {
            eprintln!("diag {} {}", d.code, d.message);
        }
    }
    let mut rep = Report { pages: (e.pages, r.v2.pages.len()), words: e.words.len(), rules: e.rules.len(), ..Report::default() };
    // Words: per page, in content order; match each expected word to the
    // next unused word with the same text on the same page.
    for page in 1..=e.pages as u32 {
        let want: Vec<&W> = e.words.iter().filter(|w| w.page == page).collect();
        let ours: Vec<&W> = got.iter().filter(|w| w.page == page).collect();
        let mut used = vec![false; ours.len()];
        for w in want {
            let pos = ours.iter().enumerate().filter(|(i, g)| !used[*i] && g.text == w.text).min_by(|a, b| {
                let da = (a.1.x - w.x).abs() + (a.1.baseline - w.baseline).abs();
                let db = (b.1.x - w.x).abs() + (b.1.baseline - w.baseline).abs();
                da.total_cmp(&db)
            });
            let Some((i, g)) = pos else {
                rep.first_fail.get_or_insert_with(|| format!("p{page} {:?} missing", w.text));
                continue;
            };
            used[i] = true;
            rep.matched += 1;
            if (g.x - w.x).abs() <= WORD_TOL_BP && (g.baseline - w.baseline).abs() <= WORD_TOL_BP {
                rep.ok += 1;
            } else {
                rep.first_fail.get_or_insert_with(|| format!("p{page} {:?}: pdflatex ({:.3}, {:.3}) ours ({:.3}, {:.3})", w.text, w.x, w.baseline, g.x, g.baseline));
            }
        }
    }
    let ours = our_rules(&r);
    for rule in &e.rules {
        let close = |a: f64, b: f64| (a - b).abs() <= RULE_TOL_BP;
        if ours.iter().any(|q| q.0 == rule.0 && close(q.1, rule.1) && close(q.2, rule.2) && close(q.3, rule.3) && close(q.4, rule.4)) {
            rep.rules_ok += 1;
        } else {
            rep.first_fail.get_or_insert_with(|| format!("rule {rule:?} ours {ours:?}"));
        }
    }
    let diags: Vec<&str> = r.v2.diagnostics.iter().filter(|d| d.code == "multicol").map(|d| d.message.as_str()).collect();
    rep.warnings_ok = e.warnings.iter().all(|w| diags.iter().any(|d| w.starts_with(d) || d.starts_with(w.trim_end_matches(" on input line"))));
    rep
}

#[test]
fn multicol_fixtures_against_pdflatex() {
    if !lm_available() {
        eprintln!("SKIP multicol_oracle: Latin Modern fonts not installed");
        return;
    }
    let mut table = String::new();
    let mut failures = Vec::new();
    let (mut exact, mut words, mut ok) = (0, 0, 0);
    for name in FIXTURES {
        let rep = measure(name);
        let full = rep.pages.0 == rep.pages.1 && rep.ok == rep.words && rep.rules_ok == rep.rules && rep.warnings_ok;
        exact += usize::from(full);
        words += rep.words;
        ok += rep.ok;
        table.push_str(&format!(
            "{name}: pages {}/{} words {}/{} (matched {}) rules {}/{} warnings {} {}\n",
            rep.pages.1,
            rep.pages.0,
            rep.ok,
            rep.words,
            rep.matched,
            rep.rules_ok,
            rep.rules,
            if rep.warnings_ok { "ok" } else { "MISSING" },
            rep.first_fail.as_deref().unwrap_or("")
        ));
        if EXACT.contains(name) && !full {
            failures.push(format!("{name}: {}", rep.first_fail.unwrap_or_default()));
        }
    }
    table.push_str(&format!("exact fixtures {exact}/{}; words {ok}/{words}\n", FIXTURES.len()));
    if std::env::var_os("MULTICOL_REPORT").is_some() {
        eprintln!("{table}");
    }
    assert!(failures.is_empty(), "{}\n{table}", failures.join("\n"));
}
