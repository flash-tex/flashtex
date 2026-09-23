//! `\usepackage{microtype}` (pdfTeX character protrusion and font expansion)
//! and `\sloppy` against pdflatex: the documents in `fixtures/microtype/`
//! render with pdflatex's line breaks (every line's words, in order) and
//! with every word's left edge within 0.1pt of pdflatex's.
//!
//! `fixtures/microtype/<name>.expected` holds pdflatex's words per line
//! (`pdftotext -bbox` of the pdflatex PDF, written by `generate.py` there;
//! TeX Live 2026, pdfTeX 1.40.29). No TeX runs in the test.
//! Variants: microtype defaults, no microtype, protrusion only, expansion
//! only, `draft` (alone it leaves microtype on), `lmodern` at 12pt, and a narrow `\sloppy`
//! paragraph whose lines are looser than a stretch ratio of 1.29 (TeX's
//! integer badness keeps them below `inf_bad`).

mod common;

use common::*;
use flashtex_render_pipeline::adapter::microtype_setup;

/// Letters and digits only: how a pdflatex word and one of our runs are
/// matched (quotes, dashes and ligature code points differ in extraction).
fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

struct Line {
    page: u32,
    words: Vec<(f64, String)>,
}

impl Line {
    fn text(&self) -> String {
        self.words.iter().map(|(_, t)| norm(t)).collect()
    }
}

fn fixture(name: &str) -> (String, Vec<Line>) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/microtype");
    let tex = std::fs::read_to_string(dir.join(format!("{name}.tex"))).unwrap();
    let expected = std::fs::read_to_string(dir.join(format!("{name}.expected"))).unwrap();
    let mut lines: Vec<Line> = Vec::new();
    for l in expected.lines() {
        if let Some(rest) = l.strip_prefix("line ") {
            let page = rest.split_whitespace().next().unwrap().parse().unwrap();
            lines.push(Line { page, words: Vec::new() });
        } else if let Some(rest) = l.strip_prefix("word ") {
            let (x, text) = rest.split_once(' ').unwrap();
            lines.last_mut().unwrap().words.push((x.parse().unwrap(), text.to_string()));
        }
    }
    lines.retain(|l| l.text().len() >= 3);
    (tex, lines)
}

/// Our glyph runs grouped into lines by page and baseline.
fn our_lines(words: &[Word]) -> Vec<Line> {
    let mut sorted: Vec<&Word> = words.iter().collect();
    sorted.sort_by(|a, b| (a.page, a.baseline, a.x).partial_cmp(&(b.page, b.baseline, b.x)).unwrap());
    let mut lines: Vec<(f64, Line)> = Vec::new();
    for w in sorted {
        match lines.last_mut() {
            Some((y, l)) if l.page == w.page && (w.baseline - *y).abs() < 1.0 => l.words.push((w.x, w.text.clone())),
            _ => lines.push((w.baseline, Line { page: w.page, words: vec![(w.x, w.text.clone())] })),
        }
    }
    let mut out: Vec<Line> = lines.into_iter().map(|(_, mut l)| {
        l.words.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        l
    }).collect();
    out.retain(|l| l.text().len() >= 3);
    out
}

/// Asserts pdflatex's lines and word positions; returns (lines, max |dx| pt).
fn check(name: &str) -> Option<(usize, f64)> {
    if !lm_available() {
        eprintln!("skipping {name}: Latin Modern not installed");
        return None;
    }
    let (tex, want) = fixture(name);
    let r = render_one(&tex);
    let got = our_lines(&words_of(&r));
    let got_text: Vec<String> = got.iter().map(Line::text).collect();
    let want_text: Vec<String> = want.iter().map(Line::text).collect();
    assert_eq!(got_text, want_text, "{name}: line breaks differ from pdflatex");
    let mut max_dx = 0f64;
    for (g, w) in got.iter().zip(&want) {
        let mut from = 0;
        for (x, t) in &w.words {
            let n = norm(t);
            if n.is_empty() {
                continue;
            }
            let Some(j) = (from..g.words.len()).find(|&j| norm(&g.words[j].1) == n) else { continue };
            from = j + 1;
            // bp -> TeX points.
            let dx = (g.words[j].0 - x).abs() * 72.27 / 72.0;
            assert!(dx <= 0.1, "{name}: page {} word {t:?} at {:.3}bp, pdflatex {x:.3}bp ({dx:.3}pt)", w.page, g.words[j].0);
            max_dx = max_dx.max(dx);
        }
    }
    eprintln!("{name}: {} lines identical to pdflatex, max word dx {max_dx:.4}pt", want.len());
    Some((want.len(), max_dx))
}

#[test]
fn microtype_defaults_protrude_and_expand_like_pdflatex() {
    check("01-default");
}

#[test]
fn without_microtype_lines_match_pdflatex() {
    check("02-off");
}

#[test]
fn protrusion_only_matches_pdflatex() {
    check("03-protrusion-only");
}

#[test]
fn expansion_only_matches_pdflatex() {
    check("04-expansion-only");
}

#[test]
fn microtype_draft_option_alone_keeps_protrusion_and_expansion() {
    // microtype.sty: `draft` only matters to `disable=ifdraft`; pdflatex's
    // draft PDF is its default one, and ours is our default one.
    check("05-draft");
    if lm_available() {
        let (draft, _) = fixture("05-draft");
        let (default, _) = fixture("01-default");
        let words = |t: &str| words_of(&render_one(t)).into_iter().map(|w| (w.page, w.text, (w.x * 1e6).round() as i64)).collect::<Vec<_>>();
        assert_eq!(words(&draft), words(&default));
    }
}

#[test]
fn lmodern_12pt_microtype_matches_pdflatex() {
    check("06-lmodern-12pt");
}

#[test]
fn sloppy_loose_lines_match_pdflatex() {
    check("07-sloppy");
}

#[test]
fn microtype_options_map_to_pdftex_levels() {
    let s = |pre: &str| microtype_setup(pre).map(|m| (m.protrude_chars, m.adjust_spacing));
    assert_eq!(s("\\usepackage{amsmath}"), None);
    assert_eq!(s("\\usepackage{microtype}"), Some((2, 2)));
    assert_eq!(s("\\usepackage[expansion=false]{microtype}"), Some((2, 0)));
    assert_eq!(s("\\usepackage[protrusion=false]{microtype}"), Some((0, 2)));
    assert_eq!(s("\\usepackage[protrusion=compatibility]{microtype}"), Some((1, 2)));
    assert_eq!(s("\\usepackage[draft]{microtype}"), Some((2, 2)));
    assert_eq!(s("\\usepackage[disable]{microtype}"), Some((0, 0)));
    assert_eq!(s("\\usepackage[disable=ifdraft]{microtype}"), Some((2, 2)));
    assert_eq!(s("\\usepackage[draft,disable=ifdraft]{microtype}"), Some((0, 0)));
    assert_eq!(s("\\documentclass[draft]{article}\\usepackage[disable=ifdraft]{microtype}"), Some((0, 0)));
    let m = microtype_setup("\\usepackage[stretch=30, shrink=10,step=5,selected]{microtype}").unwrap();
    assert_eq!((m.options.stretch, m.options.shrink, m.options.step, m.options.selected), (30, 10, 5, true));
}
