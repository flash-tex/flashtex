//! `thebibliography` against pdflatex: article.cls sets the list's
//! `\labelwidth` from the widest-label argument (`\settowidth\labelwidth
//! {\@biblabel{#1}}`, `\leftmargin\labelwidth \advance\leftmargin
//! \labelsep`), so `[1]` starts at the left margin for `{9}` and further
//! right for `{99}`, continuation lines hang by `\labelwidth + \labelsep`,
//! and the first `\bibitem` after the `References` heading takes
//! `\@nbitem`'s `\addvspace{\@outerparskip - \parskip}` — a negative
//! `\addvspace` that `\@xaddvskip` always applies to the heading's
//! after-skip, so the heading-to-entry gap equals the heading-to-paragraph
//! gap.
//!
//! Expected data: `fixtures/thebibliography/expected.txt`, every word's
//! origin and baseline read from the content stream of pdflatex's own PDF
//! (MacTeX 2026, two runs) by `tools/visual-oracle/pdftext.py`. No TeX
//! runs here.
//!
//! Gate (0.5bp): one page; every word outside the two lists (headings,
//! paragraphs, page number) has a word of ours with the same text at the
//! same place; every entry label sits at the oracle's place (matched by
//! position: the pinned compiler does not reset `enumiv` at
//! `\begin{thebibliography}`, setting `[4]`/`[5]` where pdflatex sets
//! `[1]`/`[2]` — a compiler-owner item); every line of every entry starts
//! at the oracle's left edge and baseline (the hanging indent and the
//! list's vertical glue). Not gated, printed: the words inside entry
//! lines — `\thebibliography` also sets `\sloppy` and `\sfcode`\.\@m`,
//! which the pipeline does not apply per list yet, so interword glue and
//! one line break differ.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(dead_code, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

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
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/thebibliography")
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

fn is_biblabel(text: &str) -> bool {
    text.len() >= 3 && text.starts_with('[') && text.ends_with(']') && text[1..text.len() - 1].bytes().all(|b| b.is_ascii_digit())
}

/// The words on each baseline, left to right.
fn lines(words: &[W]) -> Vec<(f64, Vec<&W>)> {
    let mut out: Vec<(f64, Vec<&W>)> = Vec::new();
    let mut sorted: Vec<&W> = words.iter().collect();
    sorted.sort_by(|a, b| a.baseline.total_cmp(&b.baseline).then(a.x.total_cmp(&b.x)));
    for w in sorted {
        match out.last_mut() {
            Some((b, ws)) if (*b - w.baseline).abs() < 0.05 => ws.push(w),
            _ => out.push((w.baseline, vec![w])),
        }
    }
    out
}

#[test]
fn thebibliography_labels_hang_and_gaps_match_pdflatex() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let read = |p: &str| std::fs::read_to_string(dir().join(p)).unwrap();
    let main = read("main.tex");
    let r = render_one(&main);
    let (pages, expected) = oracle(&read("expected.txt"));
    assert_eq!(r.v2.pages.len(), pages, "page count");
    let got = ours(&r);
    let exp_lines = lines(&expected);
    let got_lines = lines(&got);
    // Entry lines: the oracle lines from a label line up to the next
    // line that starts at the margin with a non-label word (a heading or
    // a paragraph) — labels start entries, hanging lines continue them.
    let mut in_entry = false;
    let mut misses = Vec::new();
    let mut reported = Vec::new();
    for (baseline, ws) in &exp_lines {
        let first = ws[0];
        if is_biblabel(&first.text) {
            in_entry = true;
        } else if (first.x - 72.0).abs() < 0.05 {
            in_entry = false;
        }
        let ours_line = got_lines.iter().find(|(b, _)| (b - baseline).abs() <= TOL_BP);
        if !in_entry {
            for e in ws.iter() {
                let hit = got.iter().any(|g| g.text == e.text && (g.x - e.x).abs() <= TOL_BP && (g.baseline - e.baseline).abs() <= TOL_BP);
                if !hit {
                    misses.push(format!("{:?} at ({:.3},{:.3}) not set there", e.text, e.x, e.baseline));
                }
            }
            continue;
        }
        let Some((_, gws)) = ours_line else {
            misses.push(format!("entry line {:?}… at baseline {baseline:.3} has no line of ours", first.text));
            continue;
        };
        let g0 = gws[0];
        // A `\TeX` logo's lowered `E` is its own oracle line inside the
        // entry; only lines that start at the label or the hanging indent
        // are gated.
        let starts_entry_line = first.x < 100.0;
        if starts_entry_line && (g0.x - first.x).abs() > TOL_BP {
            misses.push(format!("entry line {:?}… starts at x={:.3}, ours {:?} at x={:.3}", first.text, first.x, g0.text, g0.x));
        }
        if is_biblabel(&first.text) {
            if !is_biblabel(&g0.text) {
                misses.push(format!("label {:?} at ({:.3},{:.3}): ours starts the line with {:?}", first.text, first.x, baseline, g0.text));
            } else if g0.text != first.text {
                reported.push(format!("{} set where pdflatex sets {}", g0.text, first.text));
            }
        }
        for e in ws.iter().skip(usize::from(starts_entry_line)) {
            let hit = gws.iter().any(|g| g.text == e.text && (g.x - e.x).abs() <= TOL_BP);
            if !hit {
                reported.push(format!("{:?} at x={:.3} (\\sloppy/\\sfcode not applied)", e.text, e.x));
            }
        }
    }
    let entry_lines = exp_lines.iter().filter(|(_, ws)| is_biblabel(&ws[0].text)).count();
    assert_eq!(entry_lines, 5, "the fixture has five entries");
    if !reported.is_empty() {
        eprintln!("REPORTED ({}): {}", reported.len(), reported.join("; "));
    }
    assert!(misses.is_empty(), "{} gate misses:\n{}", misses.len(), misses.join("\n"));
}
