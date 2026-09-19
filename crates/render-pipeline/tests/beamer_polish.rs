//! beamer polish pass (issue #944 leftovers): the navigation symbol
//! strip, covered formulas / tables / graphics, Madrid ball items and
//! `\setbeamercovered{transparent}`, against pdflatex.
//!
//! Oracle: pdflatex 3.141592653-2.6-1.40.29 (TeX Live 2026), the pinned
//! `reference.pdf` of `fixtures/real-world/beamer-default` and
//! `beamer-madrid` (their content streams: pgf's `cm`, `m`/`l`/`c`/`re`
//! operators and the ball XObject's `/BBox`), and probe decks run through
//! `tools/visual-oracle/pdftext.py` (bp from the paper's top-left corner).
//! Every number below is one of those readings.
//!
//! Needs the bundled Latin Modern faces and their metrics
//! (`FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` at TeX Live's
//! `lm`/`ec`/`amsfonts/symbols` TFMs), like every oracle test in this crate.

mod common;

use common::{lm_available, render_one, words_of, Word};
use flashtex_render_pipeline::display::{Item, PathCmd, PathItem, PathPaintOp};
use flashtex_render_pipeline::Rendered;

/// Every `Item::Path` of `page` (1-based).
fn paths_of(r: &Rendered, page: usize) -> Vec<&PathItem> {
    r.v2.pages[page - 1]
        .resident_items()
        .iter()
        .filter_map(|it| match it {
            Item::Path(p) => Some(p),
            _ => None,
        })
        .collect()
}

/// The bounding box `(x0, y0, x1, y1)` in bp (y from the page top) of the
/// points of `paths` (control points included).
fn bbox(paths: &[&PathItem]) -> (f64, f64, f64, f64) {
    let mut b = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut add = |x: f64, y: f64| {
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
    };
    for p in paths {
        for c in &p.commands {
            match *c {
                PathCmd::Move(x, y) | PathCmd::Line(x, y) => add(x.to_bp(), y.to_bp()),
                PathCmd::Cubic(a, bb, c, d, e, f) => {
                    add(a.to_bp(), bb.to_bp());
                    add(c.to_bp(), d.to_bp());
                    add(e.to_bp(), f.to_bp());
                }
                PathCmd::Close => {}
            }
        }
    }
    b
}

fn rgb(p: &PathItem) -> (f64, f64, f64) {
    (p.paint.r, p.paint.g, p.paint.b)
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn default_deck() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-default/main.tex")).expect("corpus deck")
}

fn madrid_deck() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-madrid/main.tex")).expect("corpus deck")
}

/// Some run reading exactly `text` on `page` sits within `tol` bp of
/// `(x, baseline)`.
fn at(words: &[Word], page: u32, text: &str, x: f64, baseline: f64, tol: f64) {
    let same: Vec<&Word> = words.iter().filter(|w| w.page == page && w.text == text).collect();
    assert!(!same.is_empty(), "no word {text:?} on page {page}: {:?}", words.iter().filter(|w| w.page == page).map(|w| &w.text).collect::<Vec<_>>());
    let hit = same.iter().any(|w| near(w.x, x, tol) && near(w.baseline, baseline, tol));
    assert!(hit, "page {page} {text:?}: ours {:?}, pdflatex ({x:.3}, {baseline:.3}), tolerance {tol}", same.iter().map(|w| (w.x, w.baseline)).collect::<Vec<_>>());
}

/// Item 1: the default theme's navigation symbols. beamer-default p2's
/// content stream places six pgf pictures at `1 0 0 1 233.391 3.487 cm`
/// then `21.337 0 cm` steps (paper 362.835 x 272.126bp), strong paths in
/// `0.68 0.68 0.88` and light ones in `0.84 0.84 0.94`; the first
/// operator is the slide symbol's `8.26909 0.79701 3.38733 2.39105 re S`
/// and the last picture's rightmost point is the forward arrow's
/// `19.20023 2.60002`.
#[test]
fn navigation_symbols_sit_in_the_reference_strip() {
    if !lm_available() {
        return;
    }
    let r = render_one(&default_deck());
    assert_eq!(r.v2.pages.len(), 7);
    for page in 1..=7 {
        let paths = paths_of(&r, page);
        // 2 + 2 + 3 + 3 + 1 + 3 paint operations, as the reference's.
        assert_eq!(paths.len(), 14, "page {page}: {} paths", paths.len());
        let (x0, y0, x1, y1) = bbox(&paths);
        // Strip from the slide symbol's left triangle (233.391 + 2) to the
        // forward arrow (340.075 + 19.2); y from 4bp above the baseline
        // (272.126 - 3.487 - 4 = 264.64) to the baseline (268.64).
        assert!(near(x0, 233.391 + 2.0, 0.02), "page {page}: strip left {x0}");
        assert!(near(x1, 340.075 + 19.2, 0.02), "page {page}: strip right {x1}");
        assert!(near(y0, 272.126 - 3.487 - 4.0, 0.02), "page {page}: strip top {y0}");
        assert!(near(y1, 272.126 - 3.487, 0.02), "page {page}: strip bottom {y1}");
        let strong: Vec<_> = paths.iter().filter(|p| rgb(p) == (0.68, 0.68, 0.88)).collect();
        let light: Vec<_> = paths.iter().filter(|p| rgb(p) == (0.84, 0.84, 0.94)).collect();
        assert_eq!((strong.len(), light.len()), (8, 6), "page {page}");
        // The slide symbol's rectangle: stroked 0.4pt at the reference's
        // corner.
        let first = paths[0];
        assert!(matches!(&first.op, PathPaintOp::Stroke(s) if near(s.width.to_bp(), 0.3985, 0.001)));
        assert!(matches!(first.commands[0], PathCmd::Move(x, y) if near(x.to_bp(), 233.391 + 8.26909, 0.01) && near(y.to_bp(), 272.126 - 3.487 - 0.79701, 0.01)), "{:?}", first.commands[0]);
        // The light objects: the four triangle fills, plus the subsection
        // and section symbols' dimmed 0.6pt lines.
        assert_eq!(light.iter().filter(|p| matches!(p.op, PathPaintOp::Fill { .. })).count(), 4);
        assert!(light.iter().all(|p| matches!(&p.op, PathPaintOp::Stroke(s) if near(s.width.to_bp(), 0.59776, 0.001)) || matches!(p.op, PathPaintOp::Fill { .. })));
    }
}

/// `\setbeamertemplate{navigation symbols}{}` /
/// `\beamertemplatenavigationsymbolsempty` remove the strip; `[plain]`
/// frames never have one (`\thispagestyle{empty}`).
#[test]
fn navigation_symbols_can_be_switched_off() {
    if !lm_available() {
        return;
    }
    let off = render_one("\\documentclass{beamer}\n\\setbeamertemplate{navigation symbols}{}\n\\begin{document}\n\\begin{frame}{T}\nText\n\\end{frame}\n\\end{document}\n");
    assert!(paths_of(&off, 1).is_empty());
    let empty = render_one("\\documentclass{beamer}\n\\beamertemplatenavigationsymbolsempty\n\\begin{document}\n\\begin{frame}{T}\nText\n\\end{frame}\n\\end{document}\n");
    assert!(paths_of(&empty, 1).is_empty());
    let plain = render_one("\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}[plain]\nText\n\\end{frame}\n\\begin{frame}{T}\nText\n\\end{frame}\n\\end{document}\n");
    assert!(paths_of(&plain, 1).is_empty());
    assert_eq!(paths_of(&plain, 2).len(), 14);
}

/// Madrid keeps the strip: above the infolines footline, at `233.391
/// 12.121` (Madrid p3), the footline box being `8.66663pt` tall.
#[test]
fn madrid_navigation_symbols_sit_above_the_footline() {
    if !lm_available() {
        return;
    }
    let r = render_one(&madrid_deck());
    assert_eq!(r.v2.pages.len(), 7);
    let words = words_of(&r);
    at(&words, 3, "problem", 36.211, 20.061, 0.5);
    let paths: Vec<_> = paths_of(&r, 3).into_iter().filter(|p| rgb(p) == (0.68, 0.68, 0.88) || rgb(p) == (0.84, 0.84, 0.94)).collect();
    assert_eq!(paths.len(), 14);
    let (x0, y0, x1, y1) = bbox(&paths);
    assert!(near(x0, 233.391 + 2.0, 0.02) && near(x1, 340.075 + 19.2, 0.02), "{x0} {x1}");
    assert!(near(y1, 272.126 - 12.121, 0.02) && near(y0, 272.126 - 12.121 - 4.0, 0.02), "{y0} {y1}");
}
