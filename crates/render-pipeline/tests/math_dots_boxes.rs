//! `\vdots` and `\ddots` are box constructions over *text*-font periods.
//!
//! ```tex
//! \def\vdots{\vbox{\baselineskip4\p@ \lineskiplimit\z@
//!   \kern6\p@\hbox{.}\hbox{.}\hbox{.}}}
//! \def\ddots{\mathinner{\mkern1mu\raise7\p@\vbox{\kern7\p@\hbox{.}}\mkern2mu
//!   \raise4\p@\hbox{.}\mkern2mu\raise\p@\vbox{\kern7\p@\hbox{.}}\mkern1mu}}
//! ```
//!
//! (plain.tex 350-354.) Two things follow that a single `⋮`/`⋱` math glyph
//! does not give. The periods are the *text* font's and the kerns are absolute
//! points, so both constructions are `14pt + height(.)` tall — 15.156 pt for
//! cmr10 at 10.95 pt — whatever the math style. And that height is larger than
//! an array row's `\@arstrut` (9.52 pt at 11 pt), so a matrix row holding one
//! is *taller than the others*.
//!
//! The height is what this file is really about. `pmatrix`/`bmatrix` is a
//! `\vcenter`ed array: its box is centred on the math axis, so two matrices in
//! one display share that axis however many rows each has. Understate the
//! height of the `\vdots` row and the matrix holding it is centred on the
//! wrong point — and, because it is the tallest thing in the display, it holds
//! its own place while *everything else in the display and every line below it*
//! moves by half the missing height.
//!
//! That is the GH-MATRIX-REGISTER finding on `fixtures/real-world/math-sheet`
//! page 2 (the corpus-fidelity sweep of #750, F6): `±2.778 bp` four times,
//! −5.55 bp net, carrying the last 102 reference baselines of the page. The
//! 2.778 bp is exactly half the height a `\vdots` row was missing —
//! `(14pt + height(.)) − 9.52pt` over two, in a T1 document whose period is
//! 0.1 em tall.
//!
//! **Oracle**: `/Library/TeX/texbin/pdflatex` (pdfTeX 1.40.29, TeX Live 2026),
//! two passes, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on the document in
//! [`MATRICES`] at each of the three class sizes. pdflatex is an oracle only
//! and is never in the product path; nothing here is a parity claim. Every
//! number below is a glyph origin read out of its PDF with
//! `tools/visual-oracle/pdftext.py`.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

/// A glyph of page 1: its text, and its origin in bp from the page's top-left.
#[derive(Debug, Clone)]
struct G {
    text: String,
    x: f64,
    y: f64,
}

fn glyphs(body: &str) -> Vec<G> {
    let r = render_one(body);
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        for g in &run.glyphs {
            // `Glyph::cluster` indexes `clusters`, which partition `text`; a
            // run is not one glyph per cluster (a matrix row arrives as one
            // run of several), so the two cannot be zipped.
            let c = &run.clusters[g.cluster as usize];
            let text = run.text[c.text_start_byte..c.text_end_byte].to_string();
            out.push(G { text, x: g.origin_x.to_bp(), y: g.baseline_y.to_bp() });
        }
    }
    out
}

/// The baseline of the single glyph spelled `text`.
fn only(gs: &[G], text: &str) -> f64 {
    let hits: Vec<&G> = gs.iter().filter(|g| g.text == text).collect();
    assert_eq!(hits.len(), 1, "expected exactly one {text:?}, got {hits:?}");
    hits[0].y
}

fn close(what: &str, got: f64, want: f64) {
    // The project's glyph-position gate.
    assert!((got - want).abs() < 0.5, "{what}: {got:.4} bp, pdflatex {want:.4} bp (off by {:+.4})", got - want);
}

/// Two matrices in one display with **different row counts**, the second
/// holding a `\vdots`/`\ddots` row: the shape of `math-sheet` page 2, with
/// every cell a distinct letter so each row can be named.
fn matrices(size: u32) -> String {
    format!(
        "\\documentclass[{size}pt]{{article}}\n\
         \\usepackage[T1]{{fontenc}}\n\
         \\usepackage{{amsmath}}\n\
         \\usepackage[margin=1in]{{geometry}}\n\
         \\pagestyle{{empty}}\n\
         \\begin{{document}}\n\
         \\noindent Oo.\n\
         \\[\n\
         \x20 A = \\begin{{bmatrix}} a & b & c \\\\ d & e & f \\\\ g & h & k \\end{{bmatrix}}, \\qquad\n\
         \x20 M = \\begin{{bmatrix}}\n\
         \x20   p & q & r \\\\\n\
         \x20   s & t & u \\\\\n\
         \x20   \\vdots & \\ddots & \\vdots \\\\\n\
         \x20   w & x & z\n\
         \x20 \\end{{bmatrix}}.\n\
         \\]\n\
         \\noindent Ii.\n\
         \\end{{document}}\n"
    )
}

/// pdflatex glyph baselines for [`matrices`], bp from the page top:
/// `(class size, A's three rows, M's rows 1, 2 and 4, the line after the
/// display)`.
///
/// `A` has three rows and `M` four, so their rows interleave 6.8 pt apart at
/// 11 pt and nothing lines up by accident: every one of these is a separate
/// witness that the two matrices hang from the same axis.
const MATRICES: [(u32, [f64; 3], [f64; 3], f64); 3] = [
    (10, [100.620, 112.575, 124.531], [91.327, 103.283, 133.823], 151.245),
    (11, [102.992, 116.541, 130.091], [93.440, 106.989, 139.643], 158.693),
    (12, [105.674, 120.120, 134.566], [95.952, 110.398, 144.287], 165.216),
];

/// The three periods of the left `\vdots` of the same document: pdflatex
/// baselines, bp from the page top. 3.9851 bp apart at every class size,
/// because `\vdots` sets its own absolute `\baselineskip4\p@`.
const DOTS: [(u32, [f64; 3]); 3] = [
    (10, [113.898, 117.883, 121.868]),
    (11, [118.124, 122.109, 126.094]),
    (12, [121.871, 125.856, 129.841]),
];

/// The finding itself. On `main` before this change the `\vdots` row was no
/// taller than the array strut, `M` was centred 2.778 bp off the axis at
/// 11 pt, and `A`, the `=` signs and every line below the display came out
/// that far high; `M`'s own rows, being the tallest box in the display, stayed
/// put. All three class sizes, because the error is *not* proportional to the
/// body size — the kerns inside `\vdots` are absolute points, so the shortfall
/// is 3.315 bp at 10 pt, 2.778 at 11 and 3.388 at 12.
#[test]
fn two_matrices_of_different_row_counts_hang_from_one_axis() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    for (size, a_rows, m_rows, after) in MATRICES {
        let gs = glyphs(&matrices(size));
        for (row, (cell, want)) in [("a", a_rows[0]), ("d", a_rows[1]), ("g", a_rows[2])].into_iter().enumerate() {
            close(&format!("{size}pt: A row {}", row + 1), only(&gs, cell), want);
        }
        for (cell, want) in [("p", m_rows[0]), ("s", m_rows[1]), ("w", m_rows[2])] {
            close(&format!("{size}pt: M row {cell}"), only(&gs, cell), want);
        }
        // The rest of each row sits on its row's baseline.
        for (l, r) in [("a", "b"), ("a", "c"), ("d", "e"), ("g", "h"), ("p", "q"), ("w", "z")] {
            let (yl, yr) = (only(&gs, l), only(&gs, r));
            assert!((yl - yr).abs() < 0.01, "{size}pt: {l} at {yl:.4} but {r} at {yr:.4}");
        }
        // The display's own height, and so every baseline under it.
        close(&format!("{size}pt: the line after the display"), only(&gs, "I"), after);
    }
}

/// The periods of the dotted matrix row, split into the two `\vdots` columns
/// (three periods sharing one x, ys ascending) and the single periods of
/// `\ddots` between them, left to right. The closing `.` of the display and
/// the periods of the two text lines are singletons outside the columns and
/// are dropped.
fn dot_columns(gs: &[G]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut by_x: Vec<(f64, Vec<f64>)> = Vec::new();
    for g in gs.iter().filter(|g| g.text == ".") {
        match by_x.iter_mut().find(|(x, _)| (x - g.x).abs() < 0.01) {
            Some((_, ys)) => ys.push(g.y),
            None => by_x.push((g.x, vec![g.y])),
        }
    }
    by_x.sort_by(|a, b| a.0.total_cmp(&b.0));
    let columns: Vec<usize> = by_x.iter().enumerate().filter(|(_, (_, ys))| ys.len() == 3).map(|(i, _)| i).collect();
    assert_eq!(columns.len(), 2, "two `\\vdots` columns of three periods each: {by_x:?}");
    let (l, r) = (columns[0], columns[1]);
    let sorted = |ys: &[f64]| {
        let mut v = ys.to_vec();
        v.sort_by(f64::total_cmp);
        v
    };
    let ddots: Vec<f64> = by_x[l + 1..r].iter().flat_map(|(_, ys)| ys.clone()).collect();
    (sorted(&by_x[l].1), sorted(&by_x[r].1), ddots)
}

/// The construction, not just its total height: three text-font periods in one
/// column, 4 pt apart — an *absolute* 4 pt, the same at every class size,
/// because `\vdots` sets `\baselineskip4\p@` itself.
#[test]
fn vdots_is_three_text_periods_four_points_apart() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    for (size, want) in DOTS {
        let gs = glyphs(&matrices(size));
        let (left, right, _) = dot_columns(&gs);
        for (i, w) in want.iter().enumerate() {
            close(&format!("{size}pt: `\\vdots` period {}", i + 1), left[i], *w);
            // The second `\vdots` of the same row is on the same three baselines.
            assert!((right[i] - left[i]).abs() < 0.01, "{size}pt: the two `\\vdots` disagree: {left:?} / {right:?}");
        }
        // 4 pt = 3.9851 bp, whatever the class size.
        for i in 0..2 {
            let gap = left[i + 1] - left[i];
            assert!((gap - 3.9851).abs() < 0.01, "{size}pt: `\\vdots` period gap {gap:.4} bp, pdflatex 3.9851");
        }
    }
}

/// `\ddots` descends left to right: three periods 3 pt apart vertically
/// (`\raise7\p@`, `\raise4\p@`, `\raise\p@`), the leftmost the highest.
/// Getting the raises the other way round draws `⋰`, which is what the first
/// attempt at this change did.
#[test]
fn ddots_descends_three_points_per_period() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    for (size, vdots) in DOTS {
        let gs = glyphs(&matrices(size));
        let (_, _, ddots) = dot_columns(&gs);
        assert_eq!(ddots.len(), 3, "{size}pt: three periods in `\\ddots`: {ddots:?}");
        for i in 0..2 {
            // 3 pt = 2.9888 bp, downwards.
            let gap = ddots[i + 1] - ddots[i];
            assert!((gap - 2.9888).abs() < 0.01, "{size}pt: `\\ddots` step {gap:.4} bp, pdflatex +2.9888 (descending)");
        }
        // Its middle period shares the `\vdots` row's middle baseline, and its
        // top reaches 1 pt above it: the two constructions are the same height.
        close(&format!("{size}pt: `\\ddots` middle period"), ddots[1], vdots[1]);
        assert!((ddots[0] - (vdots[0] + 0.9963)).abs() < 0.01, "{size}pt: `\\ddots` top at {:.4}, 1 pt below the `\\vdots` top {:.4}", ddots[0], vdots[0]);
    }
}
