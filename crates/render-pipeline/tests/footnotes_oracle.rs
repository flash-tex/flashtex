//! Footnotes against pdflatex: marks in the text, the `\footins` area
//! (`\skip\footins`, `\footnoterule`, `\footnotesize` notes with the
//! `\@makefntext` mark box), note splitting and hold-over, two-column
//! footnotes.
//!
//! Expected data: `fixtures/footnotes/expected/*.txt`, every word's origin
//! and baseline and every rule read from the content streams of pdflatex's
//! own PDF (MacTeX 2026) by `tools/footnotes-oracle/generate.py`. No TeX
//! runs here.
//!
//! Per fixture: the page count; every word (matched on its page by text,
//! nearest position) within 0.5bp in x and baseline; every rule within
//! 0.1bp. Fixtures listed in `REPORTED` (constructs not implemented yet)
//! are measured and printed, not gated.

mod common;

use common::*;
use std::collections::BTreeMap;

const WORD_TOL_BP: f64 = 0.5;
const RULE_TOL_BP: f64 = 0.1;

/// Not implemented yet, measured and printed only:
/// * `13-minipage`, `39-minipage-notes`: `minipage` footnotes (the box
///   layout is not on main; `footnotes::MinipageNotes` is the hook);
/// * `15-math-note`: the formula is one rigid box in the justified line,
///   while TeX also stretches its `\thickmuskip`/`\medmuskip` (the words
///   after it are up to 1.7bp off; the 8pt math itself is exact);
/// * `36-math-note-11pt`: an 11pt class's notes are 9pt, whose math fonts
///   (cmr9/cmmi9/cmsy9) math-layout does not embed: set with the body's;
/// * `38-math-note-fraction`: the radical sign and `\sum` are painted from
///   Latin Modern Math, whose glyph origins are not lmsy8's/lmex10's (every
///   other glyph of the note is exact).
const REPORTED: &[&str] = &["13-minipage", "15-math-note", "36-math-note-11pt", "38-math-note-fraction", "39-minipage-notes"];

/// Need the compiler of the footnote-counters change (`\thanks` as symbol
/// footnotes, `\chapter` counter resets, a `\long` note argument): gated
/// once `vendor/compiler` has it (probed), measured until then.
const NEEDS_COMPILER: &[&str] = &["10-report-chapter-reset", "12-thanks", "14-multipar-note", "32-thanks-authors", "33-thanks-body-notes", "34-report-chapters", "35-book-chapters"];

fn compiler_has_footnote_counters() -> bool {
    use flashtex_compiler::parser::{parse, Block, Inline};
    fn notes(inlines: &[Inline], out: &mut Vec<(String, usize)>) {
        for i in inlines {
            if let Inline::Footnote { number, text, .. } = i {
                out.push((number.clone(), text.as_ref().map_or(0, |t| t.iter().filter(|x| matches!(x, Inline::LineBreak { .. })).count())));
            }
        }
    }
    let mut found = Vec::new();
    for block in parse("\\documentclass{report}\\title{T\\thanks{t}}\\author{A}\\begin{document}\\maketitle\\chapter{A}a\\footnote{x}\\chapter{B}b\\footnote{y\n\nz}\\end{document}").blocks {
        match block {
            Block::Paragraph(i) => notes(&i, &mut found),
            Block::TitleBlock { title, .. } => notes(&title, &mut found),
            _ => {}
        }
    }
    found == [("\u{2217}".to_string(), 0), ("1".to_string(), 0), ("1".to_string(), 1)]
}

#[derive(Debug, Clone)]
struct W {
    page: u32,
    text: String,
    x: f64,
    baseline: f64,
}

type Rules = Vec<Vec<(f64, f64, f64, f64)>>;

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/footnotes")
}

fn oracle(data: &str) -> (Vec<W>, Rules) {
    let mut words = Vec::new();
    let mut rules: Rules = Vec::new();
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.splitn(7, ' ').collect();
        let num = |i: usize| f[i].parse::<f64>().unwrap();
        match f[0] {
            "page" => rules.push(Vec::new()),
            "word" => words.push(W { page: f[1].parse().unwrap(), text: f[6].to_string(), x: num(2), baseline: num(3) }),
            "rule" => rules.last_mut().unwrap().push((num(2), num(3), num(4), num(5))),
            other => panic!("unknown record {other}"),
        }
    }
    (words, rules)
}

/// [`words_of`] with each word's font, and a run split where its glyphs
/// change baseline (a formula's scripts) or leave a gap (math spacing), as
/// the PDF's text positioning splits it.
fn glyph_words(r: &flashtex_render_pipeline::Rendered) -> Vec<(Word, String)> {
    let tick = flashtex_render_pipeline::display::TICKS_PER_BP;
    let mut words = Vec::new();
    for page in &r.v2.pages {
        for it in &page.items {
            let flashtex_render_pipeline::display::Item::GlyphRun(run) = it else { continue };
            if run.glyphs.is_empty() {
                continue;
            }
            let font = format!("{:?}", run.font_id);
            let chars: Vec<char> = run.text.chars().collect();
            let split = chars.len() == run.glyphs.len()
                && run.glyphs.windows(2).any(|w| w[0].baseline_y != w[1].baseline_y || ((w[1].origin_x.0 - w[0].origin_x.0 - w[0].advance_x.0) as f64 / tick).abs() > 0.5);
            if !split {
                let (first, last) = (run.glyphs[0], run.glyphs[run.glyphs.len() - 1]);
                let word = Word {
                    page: page.number,
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    width: (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64 / tick,
                };
                words.push((word, font));
                continue;
            }
            for (g, c) in run.glyphs.iter().zip(chars) {
                let word = Word { page: page.number, text: c.to_string(), x: g.origin_x.to_bp(), baseline: g.baseline_y.to_bp(), width: g.advance_x.0 as f64 / tick };
                words.push((word, font.clone()));
            }
        }
    }
    words
}

/// Glyph runs joined into words the way the oracle splits them (a run that
/// starts where the previous one ended, on the same baseline, continues
/// the word).
fn ours(r: &flashtex_render_pipeline::Rendered) -> (Vec<W>, Rules) {
    let mut out: Vec<W> = Vec::new();
    let mut prev_end: Option<(u32, f64, f64, String)> = None;
    for (w, font) in glyph_words(r) {
        match (out.last_mut(), prev_end.clone()) {
            // pdfTeX moves of less than 150/1000 em stay inside a word (the
            // `\scriptspace` between adjacent marks).
            (Some(last), Some((p, end, base, f))) if p == w.page && f == font && w.x - end > -0.01 && w.x - end < 1.0 && (w.baseline - base).abs() < 0.01 => last.text.push_str(&w.text),
            _ => out.push(W { page: w.page, text: w.text.clone(), x: w.x, baseline: w.baseline }),
        }
        prev_end = Some((w.page, w.x + w.width, w.baseline, font));
    }
    // pdfTeX's text extraction spells TS1 `\textasteriskcentered` `*`.
    for w in &mut out {
        w.text = w.text.replace('\u{2217}', "*");
    }
    let rules = r
        .v2
        .pages
        .iter()
        .map(|p| {
            p.items
                .iter()
                .filter_map(|it| match it {
                    flashtex_render_pipeline::display::Item::Rule(rule) => Some((rule.x.to_bp(), rule.top.to_bp(), rule.width.to_bp(), rule.height.to_bp())),
                    _ => None,
                })
                .collect()
        })
        .collect();
    (out, rules)
}

#[derive(Debug, Default)]
struct Report {
    pages: (usize, usize),
    words: usize,
    words_ok: usize,
    worst: f64,
    rules: usize,
    rules_ok: usize,
    fails: Vec<String>,
}

impl Report {
    fn clean(&self) -> bool {
        self.pages.0 == self.pages.1 && self.words_ok == self.words && self.rules_ok == self.rules
    }
}

fn measure(name: &str) -> Report {
    let tex = std::fs::read_to_string(dir().join(format!("{name}.tex"))).unwrap();
    let expected = std::fs::read_to_string(dir().join(format!("expected/{name}.txt"))).unwrap();
    let (want, want_rules) = oracle(&expected);
    let rendered = render_one(&tex);
    let (got, got_rules) = ours(&rendered);
    if std::env::var("FOOTNOTES_DUMP").is_ok_and(|n| n == name) {
        for g in &got {
            eprintln!("  ours p{} {:?} ({:.3}, {:.3})", g.page, g.text, g.x, g.baseline);
        }
        for d in &rendered.v2.diagnostics {
            eprintln!("  diagnostic {}: {}", d.code, d.message);
        }
    }
    let mut rep = Report { pages: (want_rules.len(), got_rules.len()), ..Report::default() };
    let mut by_page: BTreeMap<(u32, &str), Vec<(usize, &W)>> = BTreeMap::new();
    for (i, g) in got.iter().enumerate() {
        by_page.entry((g.page, g.text.as_str())).or_default().push((i, g));
    }
    let mut used = vec![false; got.len()];
    for w in &want {
        rep.words += 1;
        // The oracle writes `?` for a character outside its T1 table (an
        // OMS `\textbullet` label): any one-character word matches it.
        let wild: Vec<(usize, &W)>;
        let cands = if w.text == "?" {
            wild = got.iter().enumerate().filter(|(_, g)| g.page == w.page && g.text.chars().count() == 1).collect();
            Some(&wild)
        } else {
            by_page.get(&(w.page, w.text.as_str()))
        };
        let best = cands.and_then(|c| {
            c.iter()
                .filter(|(i, _)| !used[*i])
                .map(|(i, g)| (*i, (g.x - w.x).abs().max((g.baseline - w.baseline).abs())))
                .min_by(|a, b| a.1.total_cmp(&b.1))
        });
        match best {
            Some((i, d)) if d <= WORD_TOL_BP => {
                used[i] = true;
                rep.words_ok += 1;
                rep.worst = rep.worst.max(d);
            }
            Some((i, d)) => {
                if rep.fails.len() < 12 {
                    let g = &got[i];
                    rep.fails.push(format!("p{} {:?}: pdflatex ({:.3}, {:.3}) ours ({:.3}, {:.3}) off {d:.3}", w.page, w.text, w.x, w.baseline, g.x, g.baseline));
                }
            }
            None => {
                if rep.fails.len() < 12 {
                    rep.fails.push(format!("p{} {:?} at ({:.3}, {:.3}): not on our page", w.page, w.text, w.x, w.baseline));
                }
            }
        }
    }
    for (pi, wr) in want_rules.iter().enumerate() {
        let gr = got_rules.get(pi).cloned().unwrap_or_default();
        for r in wr {
            rep.rules += 1;
            let close = |a: f64, b: f64| (a - b).abs() <= RULE_TOL_BP;
            if gr.iter().any(|q| close(r.0, q.0) && close(r.1, q.1) && close(r.2, q.2) && close(r.3, q.3)) {
                rep.rules_ok += 1;
            } else if rep.fails.len() < 16 {
                rep.fails.push(format!("p{} rule {r:?}: ours {gr:?}", pi + 1));
            }
        }
    }
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
fn footnotes_against_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let names = fixtures();
    assert!(names.len() >= 20, "expected at least 20 footnote fixtures, found {}", names.len());
    let mut failures = Vec::new();
    let compiler = compiler_has_footnote_counters();
    if !compiler {
        eprintln!("vendor/compiler predates the footnote-counters change: {NEEDS_COMPILER:?} are measured only");
    }
    for name in &names {
        let r = measure(name);
        let line = format!(
            "{name}: pages {}/{} words {}/{} (worst {:.3}bp) rules {}/{}",
            r.pages.1, r.pages.0, r.words_ok, r.words, r.worst, r.rules_ok, r.rules
        );
        eprintln!("{line}");
        if std::env::var_os("FOOTNOTES_VERBOSE").is_some() {
            for f in &r.fails {
                eprintln!("    {f}");
            }
        }
        if !r.clean() && !REPORTED.contains(&name.as_str()) && (compiler || !NEEDS_COMPILER.contains(&name.as_str())) {
            failures.push(format!("{line}\n  {}", r.fails.join("\n  ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
