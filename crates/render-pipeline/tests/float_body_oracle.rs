//! Float *bodies* that are not pictures, against pdflatex.
//!
//! `\@xfloat` sets the float in `\vbox{\hsize\columnwidth
//! \@parboxrestore \@floatboxreset <body>}`, so the body is ordinary
//! vertical material: a `tabular` is an hbox on a line of that box's
//! vertical list, an `itemize` is a `\list`, a display carries its own
//! `\abovedisplayskip`, and a paragraph is broken at `\columnwidth` with
//! `\parindent` and `\parskip` zero and `\sloppy` in force.
//! `\@makecaption` adds `\vskip\abovecaptionskip` and the numbered
//! caption wherever `\caption` stands among it.
//!
//! The fixtures are written by hand (`fixtures/float-body/*.tex`): a ruled
//! multi-column `tabular` in a `table` and in a `figure`, with and without
//! `\centering`, with the caption above and below; prose; a list; a
//! display; a `p{}` column; a picture and a table in one float; `[t]` and
//! `[b]` bodies on one page.
//!
//! Expected data: `fixtures/float-body/expected/*.txt`, every word's origin
//! and baseline, every rule (the tabular's own included) and every image
//! rectangle read from the content streams of pdflatex's own PDF by
//! `tools/float-body-oracle/generate.py`. **reference_engine: pdfTeX
//! 3.141592653-2.6-1.40.27 (TeX Live 2025/nixos.org)** -- generated on this
//! Linux host, unlike the MacTeX 2026 data under `fixtures/footnotes/` and
//! `fixtures/floats/`, which this suite never touches. No TeX runs here.
//!
//! Per fixture: the page count; every word (matched on its page by text,
//! nearest position) within 0.5bp in x and baseline; every rule and image
//! rectangle within 0.1bp and 1bp, rules merged on both sides first (a `|`
//! is stroked once per row by pdfTeX and once per `\hline` span here).

mod common;

use common::lm_available;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};
use std::collections::BTreeMap;

const WORD_TOL_BP: f64 = 0.5;
const RULE_TOL_BP: f64 = 0.1;
const IMAGE_TOL_BP: f64 = 1.0;

#[derive(Debug, Clone)]
struct W {
    page: u32,
    text: String,
    x: f64,
    baseline: f64,
}

type Rects = Vec<Vec<(f64, f64, f64, f64)>>;

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/float-body")
}

fn oracle(data: &str) -> (Vec<W>, Rects, Rects) {
    let (mut words, mut rules, mut images): (Vec<W>, Rects, Rects) = (Vec::new(), Vec::new(), Vec::new());
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.splitn(7, ' ').collect();
        let num = |i: usize| f[i].parse::<f64>().unwrap();
        match f[0] {
            "page" => {
                rules.push(Vec::new());
                images.push(Vec::new());
            }
            "word" => words.push(W { page: f[1].parse().unwrap(), text: f[6].to_string(), x: num(2), baseline: num(3) }),
            "rule" => rules.last_mut().unwrap().push((num(2), num(3), num(4), num(5))),
            "image" => images.last_mut().unwrap().push((num(2), num(3), num(4), num(5))),
            other => panic!("unknown record {other}"),
        }
    }
    (words, rules, images)
}

/// The oracle reads the PDF's own T1 encoding and writes `?` for a code it
/// has no name for -- the `itemize` bullet, which comes from the symbol
/// font. Ours carries the character, so both are reduced to the same text.
fn ascii_or_query(text: &str) -> String {
    text.chars().map(|c| if c.is_ascii_graphic() { c } else { '?' }).collect()
}

/// Glyph runs joined into words the way the oracle splits them (a run that
/// starts where the previous one ended, on the same baseline, continues
/// the word).
fn ours(r: &flashtex_render_pipeline::Rendered) -> (Vec<W>, Rects, Rects) {
    let mut out: Vec<W> = Vec::new();
    let mut prev_end: Option<(u32, f64, f64)> = None;
    for page in &r.v2.pages {
        for it in &page.items {
            let Item::GlyphRun(run) = it else { continue };
            let (Some(first), Some(last)) = (run.glyphs.first(), run.glyphs.last()) else { continue };
            let (x, baseline) = (first.origin_x.to_bp(), first.baseline_y.to_bp());
            let width = (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP;
            let text = ascii_or_query(&run.text);
            match (out.last_mut(), prev_end) {
                // pdfTeX moves of less than 150/1000 em stay inside a word
                // (the `\scriptspace` between adjacent marks).
                (Some(prev), Some((p, end, base))) if p == page.number && x - end > -0.01 && x - end < 1.0 && (baseline - base).abs() < 0.01 => prev.text.push_str(&text),
                _ => out.push(W { page: page.number, text, x, baseline }),
            }
            prev_end = Some((page.number, x + width, baseline));
        }
    }
    let rules = r
        .v2
        .pages
        .iter()
        .map(|p| p.items.iter().filter_map(|it| match it {
            Item::Rule(rule) => Some((rule.x.to_bp(), rule.top.to_bp(), rule.width.to_bp(), rule.height.to_bp())),
            _ => None,
        }).collect())
        .collect();
    let images = r
        .v2
        .pages
        .iter()
        .map(|p| p.items.iter().filter_map(|it| match it {
            Item::Image(im) => Some((im.x.to_bp(), im.top.to_bp(), im.width.to_bp(), im.height.to_bp())),
            _ => None,
        }).collect())
        .collect();
    (out, rules, images)
}

#[derive(Debug, Default)]
struct Report {
    pages: (usize, usize),
    words: usize,
    words_ok: usize,
    worst: f64,
    rects: usize,
    rects_ok: usize,
    diagnostics: Vec<String>,
    fails: Vec<String>,
}

impl Report {
    fn clean(&self) -> bool {
        self.pages.0 == self.pages.1 && self.words_ok == self.words && self.rects_ok == self.rects && self.diagnostics.is_empty()
    }
}

/// Merges rules that continue one another, as `tabular_oracle` does:
/// pdfTeX strokes a `|` once per row while this pipeline paints one rule
/// down the rows between two `\hline`s. Applied to both sides, so the
/// comparison is of the ink, not of how it was cut into segments.
fn merge(mut rules: Vec<(f64, f64, f64, f64)>) -> Vec<(f64, f64, f64, f64)> {
    loop {
        let mut joined = None;
        'outer: for i in 0..rules.len() {
            for j in 0..rules.len() {
                if i == j {
                    continue;
                }
                let (a, b) = (rules[i], rules[j]);
                let vertical = (a.0 - b.0).abs() < 0.01 && (a.2 - b.2).abs() < 0.01 && (a.1 + a.3 - b.1).abs() < 0.01;
                let horizontal = (a.1 - b.1).abs() < 0.01 && (a.3 - b.3).abs() < 0.01 && (a.0 + a.2 - b.0).abs() < 0.01;
                if vertical || horizontal {
                    joined = Some((i, j, vertical));
                    break 'outer;
                }
            }
        }
        let Some((i, j, vertical)) = joined else { return rules };
        let b = rules[j];
        if vertical {
            rules[i].3 = b.1 + b.3 - rules[i].1;
        } else {
            rules[i].2 = b.0 + b.2 - rules[i].0;
        }
        rules.remove(j);
    }
}

fn rects(want: &Rects, got: &Rects, tol: f64, what: &str, rep: &mut Report) {
    for (pi, wr) in want.iter().enumerate() {
        let gr = got.get(pi).cloned().unwrap_or_default();
        for r in wr {
            rep.rects += 1;
            let close = |a: f64, b: f64| (a - b).abs() <= tol;
            if gr.iter().any(|q| close(r.0, q.0) && close(r.1, q.1) && close(r.2, q.2) && close(r.3, q.3)) {
                rep.rects_ok += 1;
            } else if rep.fails.len() < 16 {
                rep.fails.push(format!("p{} {what} {r:?}: ours {gr:?}", pi + 1));
            }
        }
    }
}

fn measure(name: &str) -> Report {
    let d = dir();
    let tex = std::fs::read_to_string(d.join(format!("{name}.tex"))).unwrap();
    let expected = std::fs::read_to_string(d.join(format!("expected/{name}.txt"))).unwrap();
    let (want, want_rules, want_images) = oracle(&expected);
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(d.clone()), ..RenderOptions::default() };
    let docs = [SourceDocument { path: "main.tex", text: &tex }];
    let rendered = render(&docs, "main.tex", 1, "float-notes", &fonts, &options);
    let (got, got_rules, got_images) = ours(&rendered);
    let mut rep = Report { pages: (want_rules.len(), rendered.v2.pages.len()), ..Report::default() };
    // A fallback run must fail, not pass: no font or metric may be missing,
    // and no footnote or float may be dropped.
    for diag in &rendered.v2.diagnostics {
        let bad = matches!(
            diag.code.as_ref(),
            "font_unavailable" | "required_metrics_unavailable" | "ec_metrics_unavailable" | "font_outline_substituted" | "image_unavailable" | "unsupported_block" | "float_content_unsupported" | "float_too_large"
        );
        if bad {
            rep.diagnostics.push(format!("{}: {}", diag.code, diag.message));
        }
    }
    let mut by_page: BTreeMap<(u32, &str), Vec<(usize, &W)>> = BTreeMap::new();
    for (i, g) in got.iter().enumerate() {
        by_page.entry((g.page, g.text.as_str())).or_default().push((i, g));
    }
    let mut used = vec![false; got.len()];
    for w in &want {
        rep.words += 1;
        let best = by_page.get(&(w.page, w.text.as_str())).and_then(|c| {
            c.iter()
                .filter(|(i, _)| !used[*i])
                .map(|(i, g)| (*i, (g.x - w.x).abs().max((g.baseline - w.baseline).abs())))
                .min_by(|a, b| a.1.total_cmp(&b.1))
        });
        match best {
            Some((i, dist)) if dist <= WORD_TOL_BP => {
                used[i] = true;
                rep.words_ok += 1;
                rep.worst = rep.worst.max(dist);
            }
            Some((i, dist)) => {
                if rep.fails.len() < 12 {
                    let g = &got[i];
                    rep.fails.push(format!("p{} {:?}: pdflatex ({:.3}, {:.3}) ours ({:.3}, {:.3}) off {dist:.3}", w.page, w.text, w.x, w.baseline, g.x, g.baseline));
                }
            }
            None => {
                if rep.fails.len() < 12 {
                    rep.fails.push(format!("p{} {:?} at ({:.3}, {:.3}): not on our page", w.page, w.text, w.x, w.baseline));
                }
            }
        }
    }
    let want_rules: Rects = want_rules.into_iter().map(merge).collect();
    let got_rules: Rects = got_rules.into_iter().map(merge).collect();
    rects(&want_rules, &got_rules, RULE_TOL_BP, "rule", &mut rep);
    rects(&want_images, &got_images, IMAGE_TOL_BP, "image", &mut rep);
    rep
}

fn fixtures() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".tex"))
        .map(|n| n.trim_end_matches(".tex").to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn float_bodies_match_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let names = fixtures();
    assert!(names.len() >= 14, "expected at least 14 float-body fixtures, found {}", names.len());
    let mut failures = Vec::new();
    for name in &names {
        let r = measure(name);
        let line = format!(
            "{name}: pages {}/{} words {}/{} (worst {:.3}bp) rects {}/{} diagnostics {}",
            r.pages.1, r.pages.0, r.words_ok, r.words, r.worst, r.rects_ok, r.rects, r.diagnostics.len()
        );
        eprintln!("{line}");
        if std::env::var_os("FLOAT_BODY_VERBOSE").is_some() {
            for f in r.diagnostics.iter().chain(&r.fails) {
                eprintln!("    {f}");
            }
        }
        if !r.clean() {
            let mut detail = r.diagnostics.clone();
            detail.extend(r.fails.clone());
            failures.push(format!("{line}\n  {}", detail.join("\n  ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
