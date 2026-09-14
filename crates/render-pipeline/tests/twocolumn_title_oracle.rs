//! `\twocolumn[\@maketitle]` against pdflatex: the `\@topnewpage` page.
//!
//! `\maketitle` in a two-column document is `\twocolumn[\@maketitle]`
//! (article.cls), and `\@topnewpage` (latex.ltx 20466-20505) boxes the
//! optional argument at `\hsize\textwidth`, appends `\vskip
//! -\dbltextfloatsep` inside the box, and subtracts `\ht\@currbox +
//! \dbltextfloatsep` -- the material's natural height -- from `\@colht`,
//! `\vsize` and `\@colroom`. `\@combinedblfloats` (21019) then stacks the
//! box, `\vskip\dbltextfloatsep` and the two-column box in one `\vbox
//! to\textheight`, so the columns start exactly that natural height down.
//! Both columns of the page are shortened, because `\@outputdblcol` only
//! reaches `\@outputpage` (`\global\@colht\textheight`) after the second
//! one, and `\@opcol`'s trailing `\@floatplacement` therefore computes
//! `\@toproom`/`\@botroom`/`\@fpmin` from the shortened `\@colht` in both.
//!
//! Expected data: `fixtures/twocolumn-title/expected/*.txt`, every word's
//! origin and baseline, every rule and every image rectangle read from the
//! content streams of pdflatex's own PDF by
//! `tools/twocolumn-title-oracle/generate.py`. **reference_engine: pdfTeX
//! 3.141592653-2.6-1.40.27 (TeX Live 2025/nixos.org)** -- generated on this
//! Linux host, like `fixtures/float-notes/` and unlike the MacTeX 2026 data
//! under `fixtures/footnotes/` and `fixtures/floats/`. No TeX runs here.
//!
//! Per fixture: the page count; every word (matched on its page by text,
//! nearest position) within 0.5bp in x and baseline; every rule and image
//! rectangle within 0.1bp and 1bp. Fixtures listed in `REPORTED` are
//! measured and printed, not gated.

mod common;

use common::lm_available;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};
use std::collections::BTreeMap;

const WORD_TOL_BP: f64 = 0.5;
const RULE_TOL_BP: f64 = 0.1;
const IMAGE_TOL_BP: f64 = 1.0;

/// Not implemented yet, measured and printed only:
/// * `04-title-figure-star`: full-width floats. `figure*`/`table*` reach
///   the float placer as ordinary single-column floats (`floats.rs`), never
///   as `\@dbltoplist` entries, so `\@addtodblcol`, `\dblfigrule` and
///   `\@topnewpage`'s `\global\@dbltopnum\m@ne` have no counterpart.
const REPORTED: &[&str] = &["04-title-figure-star"];

#[derive(Debug, Clone)]
struct W {
    page: u32,
    text: String,
    x: f64,
    baseline: f64,
}

type Rects = Vec<Vec<(f64, f64, f64, f64)>>;

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/twocolumn-title")
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
            match (out.last_mut(), prev_end) {
                (Some(prev), Some((p, end, base))) if p == page.number && x - end > -0.01 && x - end < 1.0 && (baseline - base).abs() < 0.01 => prev.text.push_str(&run.text),
                _ => out.push(W { page: page.number, text: run.text.clone(), x, baseline }),
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
    let rendered = render(&docs, "main.tex", 1, "twocolumn-title", &fonts, &options);
    let (got, got_rules, got_images) = ours(&rendered);
    let mut rep = Report { pages: (want_rules.len(), rendered.v2.pages.len()), ..Report::default() };
    // A fallback run must fail, not pass: no font or metric may be missing,
    // no image dropped, and no column left overfull.
    for diag in &rendered.v2.diagnostics {
        let bad = matches!(
            diag.code.as_ref(),
            "font_unavailable" | "required_metrics_unavailable" | "ec_metrics_unavailable" | "font_outline_substituted" | "image_unavailable" | "unsupported_block" | "float_content_unsupported" | "float_too_large" | "overfull_vbox"
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
fn a_spanning_title_shortens_both_columns_of_its_page_like_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let names = fixtures();
    assert_eq!(names.len(), 4, "expected 4 twocolumn-title fixtures, found {}", names.len());
    let mut failures = Vec::new();
    for name in &names {
        let r = measure(name);
        let line = format!(
            "{name}: pages {}/{} words {}/{} (worst {:.3}bp) rects {}/{} diagnostics {}",
            r.pages.1, r.pages.0, r.words_ok, r.words, r.worst, r.rects_ok, r.rects, r.diagnostics.len()
        );
        eprintln!("{line}");
        for d in &r.diagnostics {
            eprintln!("    {d}");
        }
        if std::env::var_os("TWOCOLUMN_TITLE_VERBOSE").is_some() {
            for f in &r.fails {
                eprintln!("    {f}");
            }
        }
        if !r.clean() && !REPORTED.contains(&name.as_str()) {
            failures.push(format!("{line}\n  {}\n  {}", r.diagnostics.join("\n  "), r.fails.join("\n  ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
