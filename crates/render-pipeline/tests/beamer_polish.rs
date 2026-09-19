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
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, PathCmd, PathItem, PathPaintOp};
use flashtex_render_pipeline::{render, FontSet, RenderOptions, Rendered};

const BLOCKS_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-blocks-columns");

/// Renders `text` as `main.tex` of the blocks-columns corpus deck's
/// directory, so `\includegraphics{figure.png}` reads that fixture's file.
fn render_with_figure(text: &str) -> Rendered {
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(BLOCKS_FIXTURE.into()), ..RenderOptions::default() };
    let docs = [SourceDocument { path: "main.tex", text }];
    render(&docs, "main.tex", 1, "beamer-polish", &fonts, &options)
}

/// The visible words of `page` as `(text, x, baseline)`.
fn page_words(words: &[Word], page: u32) -> Vec<(String, f64, f64)> {
    words.iter().filter(|w| w.page == page).map(|w| (w.text.clone(), w.x, w.baseline)).collect()
}

fn images_of(r: &Rendered, page: usize) -> usize {
    r.v2.pages[page - 1].resident_items().iter().filter(|it| matches!(it, Item::Image(_))).count()
}

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

/// Item 2: covered formulas, graphics and tables keep their space and are
/// not painted. Probe deck (pdflatex, `pdftext.py`): on slide 1 `E`, `=`,
/// `mc`, `2`, `a`..`d` and the image are written 2000bp off the page
/// (`\pgfsys@begininvisible`), while `after` stays at x = 106.326,
/// `tail.` at 124.888 and `end.` at 95.257 (baselines 103.594 / 139.161 /
/// 156.434); slide 2 paints everything at the same places (`E` 61.165,
/// `a` (62.809, 149.642), `d` (80.006, 163.191), the image `1 0 0 1
/// 3.637 0 cm` after `Picture`).
#[test]
fn covered_math_graphics_and_tables_keep_their_space_unpainted() {
    if !lm_available() {
        return;
    }
    let deck = "\\documentclass{beamer}\n\\setbeamertemplate{navigation symbols}{}\n\\begin{document}\n\\begin{frame}{Covered material}\nBefore \\uncover<2->{$E=mc^2$} after the formula.\n\nPicture \\uncover<2->{\\includegraphics[width=2cm]{figure.png}} tail.\n\nTable \\uncover<2->{\\begin{tabular}{ll} a & b \\\\ c & d \\end{tabular}} end.\n\\end{frame}\n\\end{document}\n";
    let r = render_with_figure(deck);
    assert_eq!(r.v2.pages.len(), 2);
    let words = words_of(&r);
    // Slide 1: no glyph of the covered material, no image.
    let slide1 = page_words(&words, 1);
    for covered in ["E", "=", "mc", "2", "a", "b", "c", "d"] {
        assert!(!slide1.iter().any(|(t, _, _)| t == covered), "slide 1 paints covered {covered:?}: {slide1:?}");
    }
    assert_eq!(images_of(&r, 1), 0);
    assert!(paths_of(&r, 1).is_empty());
    // ... but the space is kept: the words after each covered box sit where
    // pdflatex puts them. (`after` is 0.74bp off on both slides because
    // beamer sets `$E=mc^2$` in its sans math fonts, CMSSI10/CMSS10, which
    // this pipeline sets in Latin Modern Math: a gap of its own, identical
    // on both slides.)
    at(&words, 1, "after", 106.326, 103.594, 0.8);
    at(&words, 1, "tail.", 124.888, 139.161, 0.01);
    at(&words, 1, "end.", 95.257, 156.434, 0.01);
    // Slide 2: everything painted, at the same places as an uncovered
    // render of the same frame.
    at(&words, 2, "a", 62.809, 149.642, 0.01);
    at(&words, 2, "d", 80.006, 163.191, 0.01);
    at(&words, 2, "tail.", 124.888, 139.161, 0.01);
    assert_eq!(images_of(&r, 2), 1);
    let uncovered = render_with_figure(&deck.replace("\\uncover<2->", "\\uncover<1->"));
    let plain = words_of(&uncovered);
    assert_eq!(page_words(&plain, 1), page_words(&words, 2));
    // The visible words of slide 1 sit exactly where slide 2 has them.
    for (t, x, y) in &slide1 {
        assert!(page_words(&words, 2).iter().any(|(t2, x2, y2)| t2 == t && near(*x, *x2, 0.001) && near(*y, *y2, 0.001)), "{t:?} moved between slides");
    }
}

/// The paint of the run reading `text` on `page` nearest `baseline`.
fn run_paint(r: &Rendered, page: u32, text: &str, baseline: f64) -> (f64, f64, f64) {
    let mut best: Option<(f64, (f64, f64, f64))> = None;
    for p in &r.v2.pages {
        if p.number != page {
            continue;
        }
        for it in p.resident_items() {
            if let Item::GlyphRun(run) = it {
                if run.text == text {
                    let d = (run.glyphs[0].baseline_y.to_bp() - baseline).abs();
                    if best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, (run.paint.r, run.paint.g, run.paint.b)));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| panic!("no run {text:?} on page {page}"))
}

/// Item 3: Madrid's `items[ball]` labels. The reference paints the
/// `bigsphere` shading as an XObject (`/BBox [0 0 5.139 5.139]`): on p3
/// at `1 0 0 1 22.133 162.312 cm` for each itemize item (its bottom
/// 0.2pt above the item baseline 110.013, its right edge `\labelsep`
/// before `Every` at 32.727), on p4 scaled 1.75 about the picture origin
/// `22.424 164.069` (a 8.993bp ball centred 3.152bp above the baseline
/// 111.209) under the white `\tiny` `1` at (20.837, 109.521). This
/// pipeline paints a flat disc (`class_geometry::beamer::ball`: radius
/// 0.491ex, colour 0.632 structure + 0.113 white + 0.256 black =
/// 0.239 0.239 0.555) inside each of those boxes.
#[test]
fn madrid_ball_items_are_flat_discs_under_the_labels() {
    if !lm_available() {
        return;
    }
    let r = render_one(&madrid_deck());
    let words = words_of(&r);
    let ball = |p: &&PathItem| near(p.paint.r, 0.2394, 0.001) && near(p.paint.b, 0.5554, 0.001);
    // p3: four itemize discs, each inside its XObject box.
    let discs: Vec<&PathItem> = paths_of(&r, 3).into_iter().filter(ball).collect();
    assert_eq!(discs.len(), 4, "p3 discs");
    assert!(discs.iter().all(|p| matches!(p.op, PathPaintOp::Fill { .. })));
    let (x0, y0, x1, y1) = bbox(&discs[..1]);
    let (side, cx, top) = (5.139, 22.133 + 5.139 / 2.0, 272.126 - 162.312 - 5.139);
    assert!(x0 >= 22.133 - 0.01 && x1 <= 22.133 + side + 0.01, "disc x {x0}..{x1}");
    assert!(y0 >= top - 0.01 && y1 <= top + side + 0.01, "disc y {y0}..{y1}");
    assert!(near((x0 + x1) / 2.0, cx, 0.01) && near((y0 + y1) / 2.0, top + side / 2.0, 0.01), "disc centre");
    assert!(near(x1 - x0, 2.0 * 2.38, 0.05), "disc diameter {}", x1 - x0);
    at(&words, 3, "Every", 32.727, 110.013, 0.01);
    // p4: three enumerate discs 1.75 x as large, centred on the picture
    // origin, under a white `1`/`2`/`3`.
    let discs: Vec<&PathItem> = paths_of(&r, 4).into_iter().filter(ball).collect();
    assert_eq!(discs.len(), 3, "p4 discs");
    let (x0, y0, x1, y1) = bbox(&discs[..1]);
    assert!(near((x0 + x1) / 2.0, 22.424, 0.01) && near((y0 + y1) / 2.0, 272.126 - 164.069, 0.01), "enumerate disc centre {:?}", (x0, y0, x1, y1));
    assert!(near(x1 - x0, 1.75 * 2.0 * 2.38, 0.05), "enumerate disc diameter {}", x1 - x0);
    at(&words, 4, "1", 20.837, 109.521, 0.05);
    assert_eq!(run_paint(&r, 4, "1", 109.521), (1.0, 1.0, 1.0));
    // The disc is painted before the number (the number sits on it).
    let items = r.v2.pages[3].resident_items();
    let disc_at = items.iter().position(|it| matches!(it, Item::Path(p) if ball(&p))).unwrap();
    let one_at = items.iter().position(|it| matches!(it, Item::GlyphRun(g) if g.text == "1")).unwrap();
    assert!(disc_at < one_at);
}

/// Item 4: `\setbeamercovered{transparent}`. pdflatex paints the covered
/// material of the item-2 probe deck on slide 1 in its colour mixed
/// `15!bg` -- `0.85 g` before `E`, `=`, `mc`, `2`, `a`..`d` (the
/// `\beamer@colorhook`), the image `/Im7 Do` at full strength with no
/// ExtGState -- at the same positions as slide 2 (`E` 61.169 / 61.165,
/// `a` (62.809, 149.642)); `transparent=30` mixes `30!bg`.
#[test]
fn transparent_covered_material_is_painted_mixed_with_the_background() {
    if !lm_available() {
        return;
    }
    let deck = "\\documentclass{beamer}\n\\setbeamertemplate{navigation symbols}{}\n\\setbeamercovered{transparent}\n\\begin{document}\n\\begin{frame}{Covered material}\nBefore \\uncover<2->{$E=mc^2$} after the formula.\n\nPicture \\uncover<2->{\\includegraphics[width=2cm]{figure.png}} tail.\n\nTable \\uncover<2->{\\begin{tabular}{ll} a & b \\\\ c & d \\end{tabular}} end.\n\\end{frame}\n\\end{document}\n";
    let r = render_with_figure(deck);
    assert_eq!(r.v2.pages.len(), 2);
    let words = words_of(&r);
    at(&words, 1, "a", 62.809, 149.642, 0.01);
    at(&words, 1, "d", 80.006, 163.191, 0.01);
    at(&words, 1, "E", 61.169, 103.594, 0.05);
    at(&words, 1, "end.", 95.257, 156.434, 0.01);
    assert_eq!(page_words(&words, 1), page_words(&words, 2));
    let grey = (0.85, 0.85, 0.85);
    for covered in ["E", "=", "mc", "2", "a", "b", "c", "d"] {
        let baseline = if covered == "2" { 99.635 } else if "abcd".contains(covered) { 150.0 } else { 103.594 };
        assert_eq!(run_paint(&r, 1, covered, baseline), grey, "slide 1 {covered:?}");
        assert_eq!(run_paint(&r, 2, covered, baseline), (0.0, 0.0, 0.0), "slide 2 {covered:?}");
    }
    assert_eq!(run_paint(&r, 1, "after", 103.594), (0.0, 0.0, 0.0));
    // The image is painted on both slides; a `transparent=30` deck mixes
    // 30% of the colour.
    assert_eq!((images_of(&r, 1), images_of(&r, 2)), (1, 1));
    let thirty = render_with_figure(&deck.replace("{transparent}", "{transparent=30}"));
    assert_eq!(run_paint(&thirty, 1, "a", 150.0), (0.7, 0.7, 0.7));
    // Madrid: a covered `\item<2->`'s ball disc and its text are mixed too.
    let madrid = render_one("\\documentclass{beamer}\n\\usetheme{Madrid}\n\\setbeamercovered{transparent}\n\\begin{document}\n\\begin{frame}{T}\n\\begin{itemize}\n\\item<1-> one\n\\item<2-> two\n\\end{itemize}\n\\end{frame}\n\\end{document}\n");
    assert_eq!(madrid.v2.pages.len(), 2);
    assert_eq!(run_paint(&madrid, 1, "two", 130.0), grey);
    // (The other fills on the page are the navigation strip's triangles,
    // 0.84 0.84 0.94.)
    let discs: Vec<&PathItem> = paths_of(&madrid, 1).into_iter().filter(|p| matches!(p.op, PathPaintOp::Fill { .. })).collect();
    let full = discs.iter().filter(|p| near(p.paint.r, 0.2394, 0.001)).count();
    let mixed = discs.iter().filter(|p| near(p.paint.r, 0.2394 * 0.15 + 0.85, 0.001) && near(p.paint.b, 0.5554 * 0.15 + 0.85, 0.001)).count();
    assert_eq!((full, mixed), (1, 1), "{:?}", discs.iter().map(|p| rgb(p)).collect::<Vec<_>>());
}
