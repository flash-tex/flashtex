//! Page frame against pdflatex: two-sided left edges, two-column frames,
//! page numbers and running heads, report/book chapter openers, `\noindent`.
//!
//! Expected data: `fixtures/page-frame/expected/*.txt`, every word's origin
//! and baseline read from the content streams of pdflatex's own PDF
//! (MacTeX 2026, pdfTeX 1.40.29) by `tools/page-frame-oracle/generate.py`.
//! No TeX runs here.
//!
//! Gates (0.1pt = 0.0996bp):
//! * chrome: every header/footer word (page numbers, running heads) matches
//!   text, x and baseline;
//! * left edge: per page (per column for two-column pages) the leftmost
//!   body word;
//! * lines: line starts matched by their first words; x always, baseline
//!   when the line sits at the same position in its column (so a line break
//!   or page break that differs for other reasons is reported, not failed).

mod common;

use common::*;
use flashtex_class_geometry::{resolve, DocumentSetup};
use std::collections::BTreeMap;

const TOL_BP: f64 = 0.1 * 72.0 / 72.27;

pub const FIXTURES: &[&str] = &[
    "01-article-plain",
    "02-article-empty",
    "03-article-twoside-plain",
    "04-article-twoside-headings",
    "05-article-oneside-headings",
    "06-article-twoside-myheadings",
    "07-article-oneside-myheadings",
    "08-article-twocolumn",
    "09-article-twocolumn-rule",
    "10-article-twocolumn-twoside-headings",
    "11-report-chapter",
    "12-report-chapter-headings",
    "13-book-chapter",
    "14-article-noindent",
    "15-article-11pt-twoside-plain",
    "16-article-12pt-a4-geometry-plain",
    "17-article-thispagestyle-empty",
    "18-article-geometry-twoside-asymmetric",
    "19-article-twocolumn-geometry-12pt",
    "20-article-twoside-headings-subsections",
    "21-report-twoside-headings",
    "22-article-pagestyle-in-body",
    "23-article-roman-then-arabic",
    "24-article-Roman-twoside-headings",
    "25-article-setcounter-page",
    "26-article-twoside-setcounter-even",
    "27-article-alph-Alph",
    "28-book-openright",
    "29-book-openright-headings",
    "30-report-twoside-openright",
    "31-article-maketitle-headings",
    "32-article-maketitle-empty",
    "33-article-maketitle-and",
    "34-article-maketitle-author-lines-nodate",
    "35-article-maketitle-thanks",
    "36-article-titlepage",
    "37-report-maketitle",
    "38-book-maketitle",
    "39-report-notitlepage",
    "40-article-twocolumn-maketitle",
    "41-article-11pt-twoside-maketitle-nodate",
    "42-article-12pt-maketitle-long-title",
    "43-book-frontmatter-mainmatter-backmatter",
    "44-article-twoside-titlepage",
];

/// `\thanks` footnotes (marks and the page-bottom `\footins` material)
/// are not implemented: the first page's text area and glue differ, so
/// body baselines there are reported, not gated.
const MAKETITLE_Y_UNGATED: &[&str] = &["35-article-maketitle-thanks"];

#[derive(Debug, Clone)]
struct W {
    page: u32,
    text: String,
    x: f64,
    baseline: f64,
}

#[derive(Debug, Default)]
pub struct Report {
    pub pages: (usize, usize),
    pub chrome_ok: usize,
    pub chrome_total: usize,
    pub chrome_fail: Vec<String>,
    pub rules_ok: usize,
    pub rules_total: usize,
    pub rules_fail: Vec<String>,
    pub edge_ok: usize,
    pub edge_total: usize,
    pub edge_fail: Vec<String>,
    pub lines_total: usize,
    pub lines_matched: usize,
    pub lines_x_ok: usize,
    pub lines_y_checked: usize,
    pub lines_y_ok: usize,
    pub lines_fail: Vec<String>,
}

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/page-frame")
}

/// `expected/<name>.txt`: `page <n> <w> <h>`, `word <page> <x> <baseline>
/// <font> <size> <text>`, `rule <page> <x> <top> <width> <height>` (bp,
/// top-left origin).
fn oracle_words(data: &str) -> (Vec<W>, Vec<Vec<(f64, f64, f64, f64)>>) {
    let mut words = Vec::new();
    let mut rules: Vec<Vec<(f64, f64, f64, f64)>> = Vec::new();
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.splitn(7, ' ').collect();
        let num = |i: usize| f[i].parse::<f64>().unwrap();
        match f[0] {
            "page" => rules.push(Vec::new()),
            "word" => words.push(W {
                page: f[1].parse().unwrap(),
                text: f[6].to_string(),
                x: num(2),
                baseline: num(3),
            }),
            "rule" => rules.last_mut().unwrap().push((num(2), num(3), num(4), num(5))),
            other => panic!("unknown record {other}"),
        }
    }
    (words, rules)
}

/// Our words: glyph runs joined into words the way the oracle splits them
/// (a run that starts where the previous one ended continues the word).
fn our_words(r: &flashtex_render_pipeline::Rendered) -> (Vec<W>, Vec<Vec<(f64, f64, f64, f64)>>) {
    let mut out: Vec<W> = Vec::new();
    let mut prev_end: Option<(u32, f64, f64)> = None;
    for w in words_of(r) {
        match (out.last_mut(), prev_end) {
            (Some(last), Some((p, end, base))) if p == w.page && (w.x - end).abs() < 0.01 && (w.baseline - base).abs() < 0.01 => {
                last.text.push_str(&w.text);
            }
            _ => out.push(W {
                page: w.page,
                text: w.text.clone(),
                x: w.x,
                baseline: w.baseline,
            }),
        }
        prev_end = Some((w.page, w.x + w.width, w.baseline));
    }
    let rules = r
        .v2
        .pages
        .iter()
        .map(|p| {
            p.resident_items()
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

struct Frame {
    head: f64,
    foot: f64,
    text_top: f64,
    text_bottom: f64,
    twoside: bool,
    odd_left: f64,
    even_left: f64,
    col2: Option<f64>,
    colwidth: f64,
}

fn frame_of(tex: &str) -> Frame {
    let doc = resolve(&DocumentSetup::from_preamble(tex).expect("standard class"));
    let f = &doc.frame;
    Frame {
        head: f.head_baseline.to_bp(),
        foot: f.foot_baseline.to_bp(),
        text_top: f.text_top.to_bp(),
        text_bottom: (f.text_top + f.text_height).to_bp(),
        twoside: f.twoside,
        odd_left: f.odd_text_left.to_bp(),
        even_left: f.even_text_left.to_bp(),
        col2: f.columns.get(1).map(|c| c.offset.to_bp()),
        colwidth: f.columns[0].width.to_bp(),
    }
}

impl Frame {
    fn left(&self, page: u32) -> f64 {
        if self.twoside && page % 2 == 0 {
            self.even_left
        } else {
            self.odd_left
        }
    }
    fn is_chrome(&self, w: &W) -> bool {
        (w.baseline - self.head).abs() < 0.5 || (w.baseline - self.foot).abs() < 0.5
    }
    fn column(&self, w: &W) -> usize {
        match self.col2 {
            Some(off) if w.x >= self.left(w.page) + (self.colwidth + off) / 2.0 => 1,
            _ => 0,
        }
    }
}

/// Line starts per (page, column): (first word, second word, x, baseline, ordinal).
fn line_starts(words: &[W], frame: &Frame) -> BTreeMap<(u32, usize), Vec<(String, f64, f64, usize)>> {
    let mut lines: BTreeMap<(u32, usize, i64), Vec<&W>> = BTreeMap::new();
    for w in words.iter().filter(|w| !frame.is_chrome(w)) {
        lines.entry((w.page, frame.column(w), (w.baseline * 100.0).round() as i64)).or_default().push(w);
    }
    let mut out: BTreeMap<(u32, usize), Vec<(String, f64, f64, usize)>> = BTreeMap::new();
    for ((page, col, _), mut ws) in lines {
        ws.sort_by(|a, b| a.x.total_cmp(&b.x));
        let key = ws.iter().take(3).map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
        let v = out.entry((page, col)).or_default();
        let ord = v.len();
        v.push((key, ws[0].x, ws[0].baseline, ord));
    }
    out
}

pub fn measure(name: &str) -> Report {
    let tex = std::fs::read_to_string(dir().join(format!("{name}.tex"))).unwrap();
    let expected = std::fs::read_to_string(dir().join(format!("expected/{name}.txt"))).unwrap();
    let frame = frame_of(&tex);
    let (want, want_rules) = oracle_words(&expected);
    let rendered = render_one(&tex);
    let (got, got_rules) = our_words(&rendered);
    let mut rep = Report {
        pages: (want_rules.len(), got_rules.len()),
        ..Report::default()
    };
    let close = |a: f64, b: f64| (a - b).abs() <= TOL_BP;
    // Chrome.
    let n_pages = want_rules.len().max(got_rules.len()) as u32;
    for page in 1..=n_pages {
        let mut w: Vec<&W> = want.iter().filter(|w| w.page == page && frame.is_chrome(w)).collect();
        let mut g: Vec<&W> = got.iter().filter(|w| w.page == page && frame.is_chrome(w)).collect();
        w.sort_by(|a, b| (a.baseline, a.x).partial_cmp(&(b.baseline, b.x)).unwrap());
        g.sort_by(|a, b| (a.baseline, a.x).partial_cmp(&(b.baseline, b.x)).unwrap());
        rep.chrome_total += w.len().max(g.len());
        for i in 0..w.len().max(g.len()) {
            match (w.get(i), g.get(i)) {
                (Some(a), Some(b)) if a.text == b.text && close(a.x, b.x) && close(a.baseline, b.baseline) => rep.chrome_ok += 1,
                (a, b) => rep.chrome_fail.push(format!(
                    "p{page}: pdflatex {:?} ours {:?}",
                    a.map(|a| (&a.text, a.x, a.baseline)),
                    b.map(|b| (&b.text, b.x, b.baseline))
                )),
            }
        }
        // Rules (\columnseprule).
        let wr = want_rules.get(page as usize - 1).cloned().unwrap_or_default();
        let gr = got_rules.get(page as usize - 1).cloned().unwrap_or_default();
        rep.rules_total += wr.len();
        for r in &wr {
            if gr.iter().any(|q| close(r.0, q.0) && close(r.1, q.1) && close(r.2, q.2) && close(r.3, q.3)) {
                rep.rules_ok += 1;
            } else {
                rep.rules_fail.push(format!("p{page}: pdflatex rule {r:?} ours {gr:?}"));
            }
        }
    }
    // Left edges per page and column.
    let cols = if frame.col2.is_some() { 2 } else { 1 };
    for page in 1..=n_pages {
        for col in 0..cols {
            let edge = |ws: &[W]| {
                ws.iter()
                    .filter(|w| w.page == page && !frame.is_chrome(w) && frame.column(w) == col && w.baseline > frame.text_top && w.baseline < frame.text_bottom + 10.0)
                    .map(|w| w.x)
                    .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.min(x))))
            };
            if let Some(a) = edge(&want) {
                rep.edge_total += 1;
                match edge(&got) {
                    Some(b) if close(a, b) => rep.edge_ok += 1,
                    b => rep.edge_fail.push(format!("p{page} col{col}: pdflatex {a} ours {b:?}")),
                }
            }
        }
    }
    // Line starts.
    let wl = line_starts(&want, &frame);
    let gl = line_starts(&got, &frame);
    for (k, lines) in &wl {
        rep.lines_total += lines.len();
        let Some(ours) = gl.get(k) else { continue };
        let mut from = 0;
        for (key, x, y, ord) in lines {
            let Some(j) = (from..ours.len()).find(|&j| ours[j].0 == *key) else { continue };
            from = j + 1;
            rep.lines_matched += 1;
            let (_, gx, gy, gord) = &ours[j];
            if close(*x, *gx) {
                rep.lines_x_ok += 1;
            } else {
                rep.lines_fail.push(format!("p{} col{} {key:?}: x pdflatex {x} ours {gx}", k.0, k.1));
            }
            if ord == gord {
                rep.lines_y_checked += 1;
                if close(*y, *gy) {
                    rep.lines_y_ok += 1;
                } else {
                    rep.lines_fail.push(format!("p{} col{} {key:?}: baseline pdflatex {y} ours {gy}", k.0, k.1));
                }
            }
        }
    }
    rep
}

fn summary(name: &str, r: &Report) -> String {
    format!(
        "{name}: pages {}/{} chrome {}/{} rules {}/{} edges {}/{} lines matched {}/{} x {}/{} y {}/{}",
        r.pages.1, r.pages.0, r.chrome_ok, r.chrome_total, r.rules_ok, r.rules_total, r.edge_ok, r.edge_total, r.lines_matched, r.lines_total, r.lines_x_ok, r.lines_matched, r.lines_y_ok, r.lines_y_checked
    )
}

fn all() -> Vec<(&'static str, Report)> {
    FIXTURES.iter().map(|n| (*n, measure(n))).collect()
}

#[test]
fn page_frame_against_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let mut failures = Vec::new();
    for (name, r) in all() {
        eprintln!("{}", summary(name, &r));
        if std::env::var_os("PAGE_FRAME_VERBOSE").is_some() {
            for f in r.chrome_fail.iter().chain(&r.rules_fail).chain(&r.edge_fail).chain(&r.lines_fail).take(40) {
                eprintln!("    {f}");
            }
        }
        if r.chrome_ok != r.chrome_total || r.rules_ok != r.rules_total || r.edge_ok != r.edge_total {
            failures.push(format!("{}\n  {}", summary(name, &r), r.chrome_fail.iter().chain(&r.rules_fail).chain(&r.edge_fail).take(6).cloned().collect::<Vec<_>>().join("\n  ")));
        }
        // `\thanks` (fixture 35): its footnote marks and `\footins` text are
        // not implemented, so body baselines there are reported, not gated
        // (chrome, edges and x still are).
        let y_gated = !MAKETITLE_Y_UNGATED.contains(&name);
        if r.lines_x_ok != r.lines_matched || (y_gated && r.lines_y_ok != r.lines_y_checked) {
            failures.push(format!("{}\n  {}", summary(name, &r), r.lines_fail.iter().take(6).cloned().collect::<Vec<_>>().join("\n  ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
