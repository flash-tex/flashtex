//! Entry-document commands around `\input` files against pdflatex: a
//! `\maketitle` ahead of `\input{sections/intro}` is laid out before the
//! included file's material, sections of consecutive included files follow
//! in reading order, the entry document's own section resumes after them
//! and a trailing `\pagestyle` stays after the last included file.
//!
//! Expected data: `fixtures/input-order/expected.txt`, every word's origin
//! and baseline read from the content stream of pdflatex's own PDF (MacTeX
//! 2026, two runs) by `tools/visual-oracle/pdftext.py`. No TeX runs here.
//!
//! Gate (0.5bp): one page; every oracle word has a word of ours with the
//! same text on the same page within 0.5bp in x and baseline.

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

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/input-order")
}

fn oracle(data: &str) -> (usize, Vec<W>) {
    let mut pages = 0;
    let mut words = Vec::new();
    for line in data.lines().filter(|l| !l.starts_with('#')) {
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
/// starts where the previous one ended, on the same baseline, continues
/// the word).
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

#[test]
fn maketitle_and_sections_around_input_files_match_pdflatex() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let read = |p: &str| std::fs::read_to_string(dir().join(p)).unwrap();
    let (main, intro, method, outro) = (read("main.tex"), read("sections/intro.tex"), read("sections/method.tex"), read("sections/outro.tex"));
    let docs = [("main.tex", main.as_str()), ("sections/intro.tex", intro.as_str()), ("sections/method.tex", method.as_str()), ("sections/outro.tex", outro.as_str())];
    let r = render_docs(&docs, "main.tex");
    let (pages, expected) = oracle(&read("expected.txt"));
    assert_eq!(r.v2.pages.len(), pages, "page count");
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
            Some((g, d)) => misses.push(format!("{:?} at ({:.3},{:.3}) nearest ours ({:.3},{:.3}) off by {d:.3}bp", e.text, e.x, e.baseline, g.x, g.baseline)),
            None => misses.push(format!("{:?} at ({:.3},{:.3}) missing on page {}", e.text, e.x, e.baseline, e.page)),
        }
    }
    assert!(misses.is_empty(), "{} of {} oracle words off:\n{}", misses.len(), expected.len(), misses.join("\n"));
}
