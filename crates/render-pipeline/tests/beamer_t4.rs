//! beamer Tier 4 (issue #944): frame options (`\frame{...}`, `[plain]`,
//! `[fragile]`, `[allowframebreaks]`) and the Madrid theme, against
//! pdflatex.
//!
//! Oracle: pdflatex 3.141592653-2.6-1.40.29 (TeX Live 2026), the pinned
//! `reference.pdf` of `fixtures/real-world/beamer-fragile` and
//! `fixtures/real-world/beamer-madrid` (corpus PR #943), positions read
//! back with `tools/visual-oracle/pdftext.py` (bp from the paper's
//! top-left corner, baseline of the word's first glyph) and the filled
//! rectangles from the content streams (`re f`). Every number below is one
//! of those readings; the bodies are the fixtures' own frames.
//!
//! Needs the bundled Latin Modern faces and their metrics
//! (`FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` at TeX Live's
//! `lm`/`ec`/`amsfonts/symbols` TFMs), like every oracle test in this crate.

mod common;

use common::{lm_available, render_one, rules_of, words_of, Word};
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::Rendered;

/// Some run reading exactly `text` on `page` sits within `tol` bp of
/// `(x, baseline)` (a deck repeats words: the footline's author is on the
/// title page too, a listing has several `}`).
fn at(words: &[Word], page: u32, text: &str, x: f64, baseline: f64, tol: f64) {
    let same: Vec<&Word> = words.iter().filter(|w| w.page == page && w.text == text).collect();
    assert!(!same.is_empty(), "no word {text:?} on page {page}: {:?}", words.iter().filter(|w| w.page == page).map(|w| &w.text).collect::<Vec<_>>());
    let hit = same.iter().any(|w| (w.x - x).abs() <= tol && (w.baseline - baseline).abs() <= tol);
    assert!(
        hit,
        "page {page} {text:?}: ours {:?}, pdflatex ({x:.3}, {baseline:.3}), tolerance {tol}",
        same.iter().map(|w| (w.x, w.baseline)).collect::<Vec<_>>()
    );
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

/// A filled path of `page` (1-based) whose points' bounding box is
/// `(x, top, width, height)` within `tol` bp, painted `rgb` (a rounded
/// block's head or body fill: the arcs' control points lie on the box).
fn fill(r: &Rendered, page: usize, x: f64, top: f64, width: f64, height: f64, rgb: (f64, f64, f64), tol: f64) {
    use flashtex_render_pipeline::display::{PathCmd, PathPaintOp};
    let p = &r.v2.pages[page - 1];
    let mut seen = Vec::new();
    let found = p.resident_items().iter().any(|it| match it {
        Item::Path(path) if matches!(path.op, PathPaintOp::Fill { .. }) => {
            let mut b = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            let mut add = |px: f64, py: f64| {
                b.0 = b.0.min(px);
                b.1 = b.1.min(py);
                b.2 = b.2.max(px);
                b.3 = b.3.max(py);
            };
            for c in &path.commands {
                match *c {
                    PathCmd::Move(px, py) | PathCmd::Line(px, py) => add(px.to_bp(), py.to_bp()),
                    PathCmd::Cubic(_, _, _, _, px, py) => add(px.to_bp(), py.to_bp()),
                    PathCmd::Close => {}
                }
            }
            seen.push((b, (path.paint.r, path.paint.g, path.paint.b)));
            (b.0 - x).abs() <= tol
                && (b.1 - top).abs() <= tol
                && (b.2 - b.0 - width).abs() <= tol
                && (b.3 - b.1 - height).abs() <= tol
                && (path.paint.r - rgb.0).abs() < 1e-6
                && (path.paint.g - rgb.1).abs() < 1e-6
                && (path.paint.b - rgb.2).abs() < 1e-6
        }
        _ => false,
    });
    assert!(found, "page {page}: no fill at ({x}, {top}) {width}x{height} in {rgb:?}; fills: {seen:?}");
}

/// The filled rectangle of `page` (1-based) at `(x, top, width, height)`
/// within `tol` bp, painted `rgb`.
fn rect(r: &Rendered, page: usize, x: f64, top: f64, width: f64, height: f64, rgb: (f64, f64, f64), tol: f64) {
    let p = &r.v2.pages[page - 1];
    let found = p.resident_items().iter().any(|it| match it {
        Item::Rule(rule) => {
            (rule.x.to_bp() - x).abs() <= tol
                && (rule.top.to_bp() - top).abs() <= tol
                && (rule.width.to_bp() - width).abs() <= tol
                && (rule.height.to_bp() - height).abs() <= tol
                && (rule.paint.r - rgb.0).abs() < 1e-6
                && (rule.paint.g - rgb.1).abs() < 1e-6
                && (rule.paint.b - rgb.2).abs() < 1e-6
        }
        _ => false,
    });
    assert!(found, "page {page}: no rule at ({x}, {top}) {width}x{height} in {rgb:?}; rules: {:?}", rules_of(r)[page - 1]);
}

fn fragile_deck() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-fragile/main.tex")).expect("corpus deck")
}

fn madrid_deck() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-madrid/main.tex")).expect("corpus deck")
}

// ---------------------------------------------------------------------------
// beamer-fragile: 8 pages.

#[test]
fn fragile_deck_has_eight_pages_and_no_note_text() {
    if !lm_available() {
        return;
    }
    let r = render_one(&fragile_deck());
    assert_eq!(r.v2.pages.len(), 8, "pdflatex ships 8 pages");
    let w = words_of(&r);
    for leak in ["presenter", "hidden", "hidden."] {
        assert!(!w.iter().any(|x| x.text == leak), "{leak:?} was typeset: {:?}", w.iter().map(|x| &x.text).collect::<Vec<_>>());
    }
}

/// `[fragile]` with `verbatim`: the code lines are typeset as in an
/// article (cmtt10 at 10.95pt, `\verbatim@font`), at the frame's `[c]`
/// position.
#[test]
fn fragile_verbatim_lines_sit_where_pdflatex_puts_them() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&fragile_deck()));
    at(&w, 2, "Verbatim", 8.504, 21.057, 0.5);
    at(&w, 2, "A", 28.346, 99.358, 0.5);
    at(&w, 2, "\\begin{frame}[fragile]", 28.346, 121.874, 0.5);
    at(&w, 2, "needs", 188.710, 135.423, 0.5);
    at(&w, 2, "...", 39.801, 148.972, 0.5);
    at(&w, 2, "written.", 205.008, 171.488, 0.5);
}

/// `[fragile]` with `lstlisting` (`basicstyle=\ttfamily\small`): cmtt10 at
/// 10pt on 12pt, the listing's `\medskipamount` above and below inside the
/// frame's `\vbox to\textheight`, every code line `\strutbox`-tall.
#[test]
fn fragile_listing_lines_sit_where_pdflatex_puts_them() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&fragile_deck()));
    at(&w, 3, "listing", 22.463, 21.057, 0.5);
    at(&w, 3, "f", 29.044, 113.517, 0.5);
    at(&w, 3, "l", 54.237, 125.472, 0.5);
    at(&w, 3, "}", 28.870, 149.382, 0.5);
}

/// `[allowframebreaks]`: the 20-item list breaks after item 14 at
/// `0.95\textheight`; both pages carry the title with its ` I` / ` II`
/// continuation suffix and the finite autobreak glue (item pitch 16.63bp:
/// the `\itemsep` `plus 2pt` stretched by 0.04607).
#[test]
fn allowframebreaks_splits_the_list_after_item_fourteen() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&fragile_deck()));
    at(&w, 4, "I", 73.454, 21.057, 0.5);
    at(&w, 4, "one", 74.282, 44.432, 0.5);
    at(&w, 4, "two.", 74.282, 61.062, 0.5);
    at(&w, 4, "twelve.", 74.282, 227.360, 0.5);
    at(&w, 4, "fourteen.", 74.282, 260.619, 0.5);
    assert!(!w.iter().any(|x| x.page == 4 && x.text == "fifteen."), "item 15 belongs to the second page");
    at(&w, 5, "II", 73.454, 21.057, 0.5);
    at(&w, 5, "fifteen.", 74.282, 91.514, 0.5);
    at(&w, 5, "twenty,", 74.282, 179.254, 0.5);
    assert!(!w.iter().any(|x| x.page == 6 && x.text == "twenty,"), "the list ends on the second page");
}

/// `\frame{...}`: a page of its own, laid out like the environment.
#[test]
fn frame_command_form_is_its_own_page() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&fragile_deck()));
    at(&w, 6, "command", 36.211, 21.057, 0.5);
    at(&w, 6, "This", 28.346, 123.639, 0.5);
    at(&w, 6, "environment.", 28.346, 137.188, 0.5);
}

/// `[plain]`: the exit code's `\vspace*{-\footheight}` (4pt in the default
/// theme) widens the `[c]` free height and the body's first line has no
/// interline glue: 120.80 against 123.64 for the same two lines on an
/// ordinary frame.
#[test]
fn plain_frame_body_sits_higher() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&fragile_deck()));
    at(&w, 7, "Plain", 8.504, 21.057, 0.5);
    at(&w, 7, "A", 28.346, 120.800, 0.5);
    at(&w, 7, "margins.", 209.919, 134.350, 0.5);
    at(&w, 8, "frame.", 69.502, 129.059, 0.5);
}

// ---------------------------------------------------------------------------
// beamer-madrid: 7 pages.

#[test]
fn madrid_deck_has_seven_pages() {
    if !lm_available() {
        return;
    }
    let r = render_one(&madrid_deck());
    assert_eq!(r.v2.pages.len(), 7);
}

/// The infolines footline on every page: three `.333333\paperwidth` boxes
/// `ht=2.25ex dp=1ex` at `\tiny` (8.634bp) on the paper's bottom edge in
/// the whale palette (tertiary / secondary / primary), the short author
/// `(institute)` and the short title centred, the date and `n / N` between
/// `2ex` skips, all at baseline 269.469.
#[test]
fn madrid_footline_boxes_and_text() {
    if !lm_available() {
        return;
    }
    let r = render_one(&madrid_deck());
    let w = words_of(&r);
    for page in 1..=7 {
        rect(&r, page, 0.0, 263.492, 120.943, 8.634, (0.1, 0.1, 0.35), 0.5);
        rect(&r, page, 120.943, 263.492, 120.943, 8.634, (0.15, 0.15, 0.525), 0.5);
        rect(&r, page, 241.886, 263.492, 120.943, 8.634, (0.2, 0.2, 0.7), 0.5);
        let page = page as u32;
        at(&w, page, "Whitfield", 31.441, 269.469, 0.5);
        at(&w, page, "(FlashTeX)", 59.781, 269.469, 0.5);
        at(&w, page, "Incremental", 165.983, 269.469, 0.5);
        at(&w, page, "March", 280.782, 269.469, 0.5);
        at(&w, page, "2026", 299.589, 269.469, 0.5);
        at(&w, page, &page.to_string(), 345.868, 269.469, 0.5);
        at(&w, page, "/", 350.108, 269.469, 0.5);
        at(&w, page, "7", 354.342, 269.469, 0.5);
        assert_eq!(run_paint(&r, page, "Whitfield", 269.469), (1.0, 1.0, 1.0), "footline text is white");
    }
}

/// The frametitle bar: `\paperwidth` x 27.569bp from the page top in the
/// structure colour, the title in white at baseline 20.061 (no `\lineskip`
/// before the colour box); the body at the 1em margin (x 10.909) with
/// `\textheight` 260.48pt.
#[test]
fn madrid_frametitle_bar_and_body() {
    if !lm_available() {
        return;
    }
    let r = render_one(&madrid_deck());
    let w = words_of(&r);
    for page in 2..=7 {
        rect(&r, page, 0.0, 0.0, 362.835, 27.569, (0.2, 0.2, 0.7), 0.5);
    }
    at(&w, 2, "Outline", 8.504, 20.061, 0.5);
    assert_eq!(run_paint(&r, 2, "Outline", 20.061), (1.0, 1.0, 1.0));
    at(&w, 2, "A", 10.909, 102.163, 0.5);
    at(&w, 2, "reference.", 244.911, 169.909, 0.5);
    // p3: the itemize (ball labels: the number-less items keep their
    // text at 2em) and p4's enumerate numbers in white `\tiny`.
    at(&w, 3, "Every", 32.727, 110.013, 0.5);
    at(&w, 3, "seconds.", 244.758, 159.627, 0.5);
    at(&w, 4, "1", 20.837, 109.521, 0.5);
    at(&w, 4, "3", 20.836, 142.597, 0.5);
    at(&w, 4, "The", 32.727, 111.209, 0.5);
    at(&w, 7, "Caching", 10.909, 123.841, 0.5);
}

/// The rounded inner theme's title page: the `title` colour box inside a
/// `beamerboxesrounded` (painted 59.758..113.832bp from the top, 6.909..
/// 355.926 across, 4bp corner arcs, the `shadow=true` fade), the title
/// and subtitle in white and 2.23bp higher than under the default
/// template, the author 4.49bp lower.
#[test]
fn madrid_title_page_rounded_box() {
    if !lm_available() {
        return;
    }
    let r = render_one(&madrid_deck());
    let w = words_of(&r);
    fill(&r, 1, 6.909, 59.758, 349.021, 54.074, (0.2, 0.2, 0.7), 0.5);
    // The shadow: 4bp outside the right edge (355.926) and the bottom.
    shadow(&r, 1, 355.926, 113.832, 1);
    at(&w, 1, "Incremental", 111.113, 83.181, 0.5);
    at(&w, 1, "Why", 92.660, 100.242, 0.5);
    at(&w, 1, "J.", 154.796, 142.283, 0.5);
    at(&w, 1, "FlashTeX", 152.164, 164.754, 0.5);
    at(&w, 1, "March", 154.342, 190.816, 0.5);
    assert_eq!(run_paint(&r, 1, "Incremental", 83.181), (1.0, 1.0, 1.0));
    assert_eq!(run_paint(&r, 1, "Why", 100.242), (1.0, 1.0, 1.0));
    assert_eq!(run_paint(&r, 1, "J.", 142.283), (0.0, 0.0, 0.0));
}

/// `[plain]` under Madrid: no footline on that page (`\thispagestyle
/// {empty}`), the frametitle bar still painted, and the exit code's
/// `-\footheight` is the theme's 12.66663pt.
#[test]
fn madrid_plain_frame_has_no_footline() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{beamer}\n\\usetheme{Madrid}\n\\title[T]{T}\\author[A]{A}\\date{D}\n\\begin{document}\n\\begin{frame}[plain]\n\\frametitle{Plain}\nOne line.\n\\end{frame}\n\\begin{frame}\n\\frametitle{Full}\nOne line.\n\\end{frame}\n\\end{document}\n";
    let r = render_one(src);
    assert_eq!(r.v2.pages.len(), 2);
    let rules = rules_of(&r);
    assert_eq!(rules[0].len(), 1, "the bar only: {:?}", rules[0]);
    assert_eq!(rules[1].len(), 4, "the bar and three footline boxes: {:?}", rules[1]);
    let w = words_of(&r);
    assert!(!w.iter().any(|x| x.page == 1 && x.text == "1"), "no frame number on the plain page");
    at(&w, 2, "2", 345.868, 269.469, 0.5);
}

/// Madrid's rounded blocks (`blocks[rounded][shadow=true]`, orchid
/// colours): the `\large` white title on the block-title bg, the body on
/// its 10% tint, the box `\lineskip` + `\medskipamount` under the
/// previous material and `\smallskipamount` after. Measured with
/// pdflatex on the probe below (`\showoutput`: `\vbox(48.19664)`,
/// `\vbox(32.46747)`, `\vbox(33.30078)`; the fills from the content
/// stream: head 72.583..87.372bp, body ..118.600, x 6.909, 349.021
/// wide), with 4bp corner arcs, the 2pt head/body transition fade and
/// the `shadow=true` drop shadow.
#[test]
fn madrid_rounded_blocks() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{beamer}\n\\usetheme{Madrid}\n\\begin{document}\n\\begin{frame}\n  \\frametitle{Blocks}\n  \\begin{block}{Plain block}\n    The body of the block, which wraps onto a second line when it is long enough.\n  \\end{block}\n  \\begin{alertblock}{Alert}\n    One line.\n  \\end{alertblock}\n  \\begin{exampleblock}{Example}\n    Another line.\n  \\end{exampleblock}\n\\end{frame}\n\\end{document}\n";
    let r = render_one(src);
    let w = words_of(&r);
    at(&w, 1, "Plain", 10.909, 83.885, 0.5);
    at(&w, 1, "The", 10.909, 99.431, 0.5);
    at(&w, 1, "enough.", 10.909, 112.980, 0.5);
    at(&w, 1, "Alert", 10.909, 141.864, 0.5);
    at(&w, 1, "One", 10.909, 157.410, 0.5);
    at(&w, 1, "Example", 10.909, 184.173, 0.5);
    at(&w, 1, "Another", 10.909, 200.549, 0.5);
    assert_eq!(run_paint(&r, 1, "Plain", 83.885), (1.0, 1.0, 1.0));
    assert_eq!(run_paint(&r, 1, "The", 99.431), (0.0, 0.0, 0.0));
    fill(&r, 1, 6.909, 72.583, 349.021, 87.372 - 72.583, (0.15, 0.15, 0.525), 0.5);
    fill(&r, 1, 6.909, 87.372, 349.021, 118.600 - 87.372, (0.915, 0.915, 0.9525), 0.5);
    fill(&r, 1, 6.909, 130.562, 349.021, 145.351 - 130.562, (0.75, 0.0, 0.0), 0.5);
    fill(&r, 1, 6.909, 172.871, 349.021, 188.490 - 172.871, (0.0, 0.375, 0.0), 0.5);
    shadow(&r, 1, 355.926, 118.600, 3);
    // The `bmb@transition` fade over the first block's head/body seam:
    // `upper.bg` to `lower.bg` 1pt either side of the head fill's bottom edge (87.372),
    // fading into `upper.bg` -- strips strictly between the two colours.
    use flashtex_render_pipeline::display::{PathCmd, PathPaintOp};
    let strips = r.v2.pages[0]
        .resident_items()
        .iter()
        .filter(|it| match it {
            Item::Path(p) if matches!(p.op, PathPaintOp::Fill { .. }) => {
                let ys: Vec<f64> = p.commands.iter().filter_map(|c| match *c {
                    PathCmd::Move(_, y) | PathCmd::Line(_, y) => Some(y.to_bp()),
                    _ => None,
                }).collect();
                let (lo, hi) = ys.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &y| (a.0.min(y), a.1.max(y)));
                p.paint.r > 0.15 + 1e-6 && p.paint.r < 0.915 - 1e-6 && (p.paint.b - p.paint.r) > 0.0 && lo > 85.5 && hi < 89.5
            }
            _ => false,
        })
        .count();
    assert_eq!(strips, 16, "transition strips over the first block's seam");
}

/// A `shadow=true` fade: `blocks` sets of grey round-capped strokes whose
/// widest ring reaches 4bp past the box's right edge `right` and bottom
/// edge `bottom` (bp from the page top), lightest outermost.
fn shadow(r: &Rendered, page: usize, right: f64, bottom: f64, blocks: usize) {
    use flashtex_render_pipeline::display::{LineCap, PathCmd, PathPaintOp};
    let p = &r.v2.pages[page - 1];
    let mut widest = Vec::new();
    let mut greys = Vec::new();
    for it in p.resident_items() {
        let Item::Path(path) = it else { continue };
        let PathPaintOp::Stroke(st) = &path.op else { continue };
        // Ring greys are `1 - 0.5 x (1 - (k + 0.5)/32)`: odd multiples of
        // 1/128 under white (the navigation symbols' strokes are not).
        let k = (1.0 - path.paint.r) * 128.0;
        if st.cap != LineCap::Round || path.paint.r != path.paint.g || (k - k.round()).abs() > 1e-6 || k.round() as i64 % 2 != 1 {
            continue;
        }
        greys.push(path.paint.r);
        if (st.width.to_bp() - 8.0).abs() < 0.01 {
            let pts: Vec<(f64, f64)> = path.commands.iter().filter_map(|c| match *c {
                PathCmd::Move(x, y) | PathCmd::Line(x, y) => Some((x.to_bp(), y.to_bp())),
                PathCmd::Cubic(_, _, _, _, x, y) => Some((x.to_bp(), y.to_bp())),
                PathCmd::Close => None,
            }).collect();
            let max_x = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let max_y = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            widest.push((max_x, max_y));
        }
    }
    assert_eq!(greys.len(), 32 * blocks, "shadow rings on page {page}: {greys:?}");
    assert!(widest.iter().any(|&(x, y)| (x - right).abs() < 0.5 && (y - bottom).abs() < 0.5), "no shadow on ({right}, {bottom}): {widest:?}");
    // Widest (lightest) first within each shadow.
    assert!(greys[0] > greys[31], "{greys:?}");
}

/// `\logo{..}` on every frame page but `[plain]` ones, right-aligned
/// 0.1cm from the paper edge above the navigation symbols in the `logo`
/// colour, at `\Tiny` (4pt). Positions from the pdflatex content streams
/// (TeX Live 2026): `\logo{\textbf{FT}}` at (354.642, 260.669) under the
/// default theme, (354.642, 252.035) under Madrid (white: whale's
/// `palette secondary` fg); with the navigation symbols emptied and a
/// descender, `\logo{\textbf{Fgy} \color{red}R}` at (349.168, 263.160)
/// and the red `R` at 357.262.
#[test]
fn logo_sits_above_the_navigation_symbols() {
    if !lm_available() {
        return;
    }
    let deck = |preamble: &str| {
        format!("\\documentclass{{beamer}}\n{preamble}\n\\begin{{document}}\n\\begin{{frame}}\n  \\frametitle{{First}}\n  Some text on the frame.\n\\end{{frame}}\n\\begin{{frame}}[plain]\n  Plain.\n\\end{{frame}}\n\\end{{document}}\n")
    };
    let r = render_one(&deck("\\logo{\\textbf{FT}}"));
    let w = words_of(&r);
    at(&w, 1, "FT", 354.642, 260.669, 0.01);
    assert_eq!(run_paint(&r, 1, "FT", 260.669), (0.15, 0.15, 0.525));
    assert!(!w.iter().any(|x| x.page == 2 && x.text == "FT"), "a [plain] frame has no sidebar");

    let r = render_one(&deck("\\usetheme{Madrid}\n\\logo{\\textbf{FT}}"));
    let w = words_of(&r);
    at(&w, 1, "FT", 354.642, 252.035, 0.01);
    assert_eq!(run_paint(&r, 1, "FT", 252.035), (1.0, 1.0, 1.0));

    let r = render_one(&deck("\\setbeamertemplate{navigation symbols}{}\n\\logo{\\textbf{Fgy} \\color{red}R}"));
    let w = words_of(&r);
    at(&w, 1, "Fgy", 349.168, 263.160, 0.01);
    at(&w, 1, "R", 357.262, 263.160, 0.01);
    assert_eq!(run_paint(&r, 1, "R", 263.160), (1.0, 0.0, 0.0));
}

/// `\tableofcontents` with subsections, and `\AtBeginSection[]` putting
/// an outline frame with `\tableofcontents[currentsection]` before every
/// section: pdflatex ships 6 pages (the contents, an outline, two
/// frames, an outline, a frame). Positions from its content stream,
/// identical on the three contents pages: sections at x 28.346,
/// subsections at 44.710 (`\leftskip` 1.5em), baselines 96.679 /
/// 110.228 / 123.777 / 170.225 / 183.774. On the outlines the other
/// section and its subsections are shaded (`\showoutput`: `0.84 0.84
/// 0.94 rg` and `0.8 g`); the current section's subsections are black.
#[test]
fn beamer_toc_subsections_and_outline_frames() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{beamer}\n\\AtBeginSection[]{\\begin{frame}\\frametitle{Outline}\\tableofcontents[currentsection]\\end{frame}}\n\\begin{document}\n\\begin{frame}\\frametitle{Contents}\\tableofcontents\\end{frame}\n\\section{Intro}\n\\subsection{Why}\n\\begin{frame}\\frametitle{Why}Text.\\end{frame}\n\\subsection{How}\n\\begin{frame}\\frametitle{How}Text.\\end{frame}\n\\section{Results}\n\\subsection{Speed}\n\\begin{frame}\\frametitle{Speed}Text.\\end{frame}\n\\end{document}\n";
    let r = render_one(src);
    assert_eq!(r.v2.pages.len(), 6);
    let w = words_of(&r);
    for page in [1, 2, 5] {
        at(&w, page, "Intro", 28.346, 96.679, 0.01);
        at(&w, page, "Why", 44.710, 110.228, 0.01);
        at(&w, page, "How", 44.710, 123.777, 0.01);
        at(&w, page, "Results", 28.346, 170.225, 0.01);
        at(&w, page, "Speed", 44.710, 183.774, 0.01);
    }
    at(&w, 2, "Outline", 8.504, 21.057, 0.01);
    at(&w, 3, "Why", 8.504, 21.057, 0.01);
    at(&w, 6, "Speed", 8.504, 21.057, 0.01);
    let shaded_section = (0.84, 0.84, 0.94);
    let close = |a: (f64, f64, f64), b: (f64, f64, f64)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6 && (a.2 - b.2).abs() < 1e-6;
    // The full list: every section in the structure colour, subsections black.
    assert_eq!(run_paint(&r, 1, "Results", 170.225), (0.2, 0.2, 0.7));
    assert_eq!(run_paint(&r, 1, "Speed", 183.774), (0.0, 0.0, 0.0));
    // Page 2 (at `Intro`) shades `Results` and `Speed`.
    assert_eq!(run_paint(&r, 2, "Intro", 96.679), (0.2, 0.2, 0.7));
    assert_eq!(run_paint(&r, 2, "Why", 110.228), (0.0, 0.0, 0.0));
    assert!(close(run_paint(&r, 2, "Results", 170.225), shaded_section), "{:?}", run_paint(&r, 2, "Results", 170.225));
    assert!(close(run_paint(&r, 2, "Speed", 183.774), (0.8, 0.8, 0.8)));
    // Page 5 (at `Results`) shades `Intro`, `Why`, `How`.
    assert!(close(run_paint(&r, 5, "Intro", 96.679), shaded_section));
    assert!(close(run_paint(&r, 5, "How", 123.777), (0.8, 0.8, 0.8)));
    assert_eq!(run_paint(&r, 5, "Speed", 183.774), (0.0, 0.0, 0.0));
}
