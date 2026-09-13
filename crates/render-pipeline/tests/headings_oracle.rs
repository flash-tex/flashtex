//! Sectioning headings against pdflatex: `\@startsection` headings
//! (`\section` .. `\subparagraph`, run-in and display), `\chapter`, `\part`,
//! `\appendix`, `secnumdepth`, starred forms, headings at page ends and in
//! two-column documents, at 10/11/12pt in article/report/book.
//!
//! Expected data: `fixtures/headings/expected/*.txt`, every word's origin
//! and baseline read from pdflatex's own PDF (MacTeX 2026, pdfTeX 1.40.29) by
//! `tools/headings-oracle/generate.py`. No TeX runs here.
//!
//! Words are aligned by text (longest common subsequence); a matched word is
//! placed when it is on the same page within 0.5 bp in x and baseline.
//! Fixtures in [`EXACT`] must match every oracle word, placed, with the same
//! page count; the others are reported (`cargo test --release --test
//! headings_oracle -- --nocapture`, `HEADINGS_VERBOSE=<name>` lists misses).

mod common;

use common::*;

const TOL_BP: f64 = 0.5;

const FIXTURES: &[&str] = &[
    "01-article-10pt-levels",
    "01-article-11pt-levels",
    "01-article-12pt-levels",
    "02-report-10pt-levels",
    "02-report-11pt-levels",
    "02-report-12pt-levels",
    "03-book-10pt-levels",
    "03-book-11pt-levels",
    "03-book-12pt-levels",
    "04-article-runin",
    "05-article-consecutive",
    "06-article-heading-list",
    "07-article-heading-display",
    "08-article-long-headings",
    "09-article-secnumdepth-1",
    "10-article-secnumdepth-5",
    "11-article-starred",
    "12-article-appendix",
    "13-report-appendix",
    "14-article-twocolumn",
    "15-article-page-bottom-a",
    "16-article-page-bottom-b",
    "17-article-page-bottom-c",
    "18-article-page-top",
    "19-article-many-10pt",
    "20-article-many-11pt",
    "21-article-many-12pt",
    "22-report-chapter-star",
    "23-book-sections-twoside",
    "24-article-part",
    "25-article-secnumdepth-body",
    "26-article-12pt-runin-subpar",
    "27-article-11pt-title-ligatures",
    "28-report-11pt-many",
    "29-article-noindent-after",
    "30-article-twocolumn-12pt",
    "31-article-after-list-end",
    "32-book-11pt-appendix",
    "33-article-math-headings",
    "34-report-twocolumn-chapter",
    "35-book-twocolumn-chapter",
];

/// Fixtures not yet exact (reported, not gated):
/// * `06-article-heading-list`: list glue — pdflatex shrinks page 1 to fit
///   one page, the pipeline's list skips are taller and it breaks (lists
///   lane, not the heading);
/// * `32-book-11pt-appendix`: the running head `APPENDIX A. TABLES` spaces
///   `A.` as a sentence end (mark text, not the heading);
/// * `33-article-math-headings`: `\sum` limits in text style inside a
///   heading (math-layout placement); every other heading word is placed.
/// Every other fixture is gated at 100% (every word matched and placed,
/// same pages).
const NOT_EXACT: &[&str] = &["06-article-heading-list", "32-book-11pt-appendix", "33-article-math-headings"];

#[derive(Debug, Clone)]
struct W {
    page: u32,
    text: String,
    x: f64,
    baseline: f64,
}

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/headings")
}

fn oracle(data: &str) -> (Vec<W>, usize) {
    let mut words = Vec::new();
    let mut pages = 0;
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.splitn(7, ' ').collect();
        match f[0] {
            "page" => pages += 1,
            "word" => words.push(W {
                page: f[1].parse().unwrap(),
                text: f[6].to_string(),
                x: f[2].parse().unwrap(),
                baseline: f[3].parse().unwrap(),
            }),
            _ => {}
        }
    }
    (words, pages)
}

/// Glyph runs joined into words the way the oracle splits them.
fn ours(r: &flashtex_render_pipeline::Rendered) -> Vec<W> {
    let mut out: Vec<W> = Vec::new();
    let mut prev_end: Option<(u32, f64, f64)> = None;
    for mut w in words_of(r) {
        // The oracle reader writes every code outside printable ASCII as `?`.
        w.text = w.text.chars().map(|c| if c.is_ascii_graphic() { c } else { '?' }).collect();
        match (out.last_mut(), prev_end) {
            (Some(last), Some((p, end, base))) if p == w.page && (w.x - end).abs() < 0.01 && (w.baseline - base).abs() < 0.01 => last.text.push_str(&w.text),
            _ => out.push(W {
                page: w.page,
                text: w.text.clone(),
                x: w.x,
                baseline: w.baseline,
            }),
        }
        prev_end = Some((w.page, w.x + w.width, w.baseline));
    }
    out
}

/// Pairs of indices of the longest common subsequence of word texts.
fn align(a: &[W], b: &[W]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![0u32; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * (m + 1) + j] = if a[i].text == b[j].text { dp[(i + 1) * (m + 1) + j + 1] + 1 } else { dp[(i + 1) * (m + 1) + j].max(dp[i * (m + 1) + j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i].text == b[j].text {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if dp[(i + 1) * (m + 1) + j] >= dp[i * (m + 1) + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

#[derive(Debug, Default)]
struct Report {
    pages: (usize, usize),
    words: (usize, usize),
    matched: usize,
    placed: usize,
    misses: Vec<String>,
}

fn measure(name: &str) -> Report {
    let tex = std::fs::read_to_string(dir().join(format!("{name}.tex"))).unwrap();
    let expected = std::fs::read_to_string(dir().join(format!("expected/{name}.txt"))).unwrap();
    let (want, want_pages) = oracle(&expected);
    let r = render_one(&tex);
    let got = ours(&r);
    let pairs = align(&want, &got);
    let mut rep = Report {
        pages: (want_pages, r.v2.pages.len()),
        words: (want.len(), got.len()),
        matched: pairs.len(),
        ..Report::default()
    };
    let (mut wi, mut gj) = (0, 0);
    for &(i, j) in &pairs {
        rep.misses.extend(want[wi..i].iter().map(|a| format!("{:?}: pdflatex p{} ({:.2}, {:.2}) unmatched", a.text, a.page, a.x, a.baseline)));
        rep.misses.extend(got[gj..j].iter().map(|b| format!("{:?}: ours p{} ({:.2}, {:.2}) unmatched", b.text, b.page, b.x, b.baseline)));
        (wi, gj) = (i + 1, j + 1);
    }
    for (i, j) in pairs {
        let (a, b) = (&want[i], &got[j]);
        if a.page == b.page && (a.x - b.x).abs() <= TOL_BP && (a.baseline - b.baseline).abs() <= TOL_BP {
            rep.placed += 1;
        } else {
            rep.misses.push(format!("{:?}: pdflatex p{} ({:.2}, {:.2}) ours p{} ({:.2}, {:.2})", a.text, a.page, a.x, a.baseline, b.page, b.x, b.baseline));
        }
    }
    rep
}

#[test]
fn headings_against_pdflatex() {
    if !lm_available() {
        eprintln!("Latin Modern not available; skipping");
        return;
    }
    let verbose = std::env::var("HEADINGS_VERBOSE").unwrap_or_default();
    let mut failures = Vec::new();
    let (mut exact, mut total_words, mut total_placed) = (0, 0, 0);
    for name in FIXTURES {
        let rep = measure(name);
        let full = rep.pages.0 == rep.pages.1 && rep.placed == rep.words.0 && rep.words.0 == rep.words.1;
        exact += usize::from(full);
        total_words += rep.words.0;
        total_placed += rep.placed;
        eprintln!(
            "{name:40} pages {}/{} words {}/{} matched {} placed {} ({:.1}%){}",
            rep.pages.0,
            rep.pages.1,
            rep.words.0,
            rep.words.1,
            rep.matched,
            rep.placed,
            100.0 * rep.placed as f64 / rep.words.0.max(1) as f64,
            if full { "  EXACT" } else { "" }
        );
        if !verbose.is_empty() && (verbose == "all" || verbose == *name) {
            let cap = std::env::var("HEADINGS_VERBOSE_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
            for m in rep.misses.iter().take(cap) {
                eprintln!("    {m}");
            }
        }
        if !NOT_EXACT.contains(name) && !full {
            failures.push(format!("{name}: {rep:?}", rep = (rep.pages, rep.words, rep.matched, rep.placed, rep.misses.first())));
        }
    }
    eprintln!("exact {exact}/{}; words placed {total_placed}/{total_words}", FIXTURES.len());
    assert!(failures.is_empty(), "{failures:#?}");
}
