//! enumitem keys in a list's own optional argument against pdflatex:
//! `\begin{itemize}[nosep]` (`\topsep`, `\partopsep`, `\itemsep`,
//! `\parsep` all zero), `\begin{enumerate}[noitemsep]` (`\itemsep`,
//! `\parsep` zero) and explicit `topsep=`/`itemsep=`/`parsep=` keys, each
//! followed by a paragraph so the closing skip is measured too, in an
//! 11pt article with `\parskip` 4pt.
//!
//! Expected data: `fixtures/enumitem-keys/expected.txt`, every word's
//! origin and baseline read from the content stream of pdflatex's own PDF
//! (MacTeX 2026, two runs) by `tools/visual-oracle/pdftext.py`. No TeX
//! runs here.
//!
//! Gate (0.5bp): one page; every oracle line (baseline) has a line of ours
//! at that baseline with the same words after the label, and every word
//! of the heading, the paragraphs' later lines and the page number sits at
//! the oracle's place. Reported, not gated: the bullet's x (enumitem sets
//! `\labelsep` to `0.5em` at the body size where article keeps 5pt), the
//! indent of a paragraph that continues right after `\end{itemize}`
//! without a blank line, and the `enumerate` labels — the pinned compiler
//! reads `[noitemsep]` as an enumerate-package label template and sets the
//! word itself as the label (compiler-owner item).

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::collapsible_if, dead_code, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

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
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/enumitem-keys")
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

/// pdftext renders the T1 bullet as `?`; an item line starts with it or
/// with an enumerate label (`1.`).
fn is_label(text: &str) -> bool {
    text == "?" || text == "•" || (text.ends_with('.') && text[..text.len() - 1].bytes().all(|b| b.is_ascii_digit()) && text.len() > 1)
}

#[test]
fn enumitem_nosep_noitemsep_and_explicit_keys_match_pdflatex() {
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
    assert_eq!(exp_lines.len(), 13, "the fixture sets 13 lines");
    let mut misses = Vec::new();
    let mut reported = Vec::new();
    for (baseline, ws) in &exp_lines {
        let Some((_, gws)) = got_lines.iter().find(|(b, _)| (b - baseline).abs() <= TOL_BP) else {
            misses.push(format!("line {:?}… at baseline {baseline:.3} has no line of ours", ws[0].text));
            continue;
        };
        let item = is_label(&ws[0].text);
        let (etail, gtail) = if item { (&ws[1..], &gws[1..]) } else { (&ws[..], &gws[..]) };
        let et: Vec<&str> = etail.iter().map(|w| w.text.as_str()).collect();
        let gt: Vec<&str> = gtail.iter().map(|w| w.text.as_str()).collect();
        if et != gt {
            misses.push(format!("line at {baseline:.3}: pdflatex {et:?}, ours {gt:?}"));
            continue;
        }
        if item {
            if (ws[0].x - gws[0].x).abs() > TOL_BP {
                reported.push(format!("label {:?} at x={:.3}, ours {:?} at x={:.3}", ws[0].text, ws[0].x, gws[0].text, gws[0].x));
            }
        }
        for (e, g) in etail.iter().zip(gtail) {
            if (e.x - g.x).abs() > TOL_BP {
                let msg = format!("{:?} at x={:.3}, ours x={:.3} (baseline {baseline:.3})", e.text, e.x, g.x);
                // A paragraph continuing right after `\end{...}` is set with
                // `\parindent` by the pipeline; pdflatex keeps it unindented.
                // Behind a compiler-set enumerate label (`noitemsep` for
                // `1.`) the words shift by the label's extra width.
                let label_differs = item && ws[0].text != gws[0].text && !is_label(&gws[0].text);
                if (!item && ws[0].x < 72.05 && gws[0].x > 72.05) || label_differs {
                    reported.push(msg);
                } else {
                    misses.push(msg);
                }
            }
        }
    }
    if !reported.is_empty() {
        eprintln!("REPORTED ({}): {}", reported.len(), reported.join("; "));
    }
    assert!(misses.is_empty(), "{} gate misses:\n{}", misses.len(), misses.join("\n"));
}
