//! A cluster's rectangle is its laid-out TeX box; the painted outline's
//! extent is a separate, separately named measurement.
//!
//! Three lanes read a delimiter's height out of a math cluster and got the
//! ink of the Latin Modern Math outline drawn inside its box, then opened
//! engine defects for boxes that were already exact (#265, #266 and their #2
//! comment). `[` and `\{` hid it: their flat outlines happen to fill their
//! boxes, while `(`'s round ends stop 0.33% short and a cmex bar is drawn
//! past its box so stacked copies overlap.
//!
//! The reference numbers are pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025)
//! `\showbox` of `\hbox{$\displaystyle <cmd>[$}` in a 10 pt `article`, the
//! same transcription `crates/math-layout/tests/big_delimiters.rs` pins at
//! the layout layer. This file pins that the display list reports them.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, Rect};

const BP: f64 = 72.0 / 72.27;

fn pt(t: flashtex_render_pipeline::display::Tick) -> f64 {
    t.to_bp() / BP
}

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\\begin{{document}}\\[{body}\\]\\end{{document}}")
}

/// The box and the ink of the first math cluster whose text is `ch`.
fn cluster(body: &str, ch: char) -> (Rect, Option<Rect>) {
    let r = render_one(&doc(body));
    for page in &r.v2.pages {
        for it in &page.items {
            let Item::GlyphRun(run) = it else { continue };
            for c in &run.clusters {
                if run.text[c.text_start_byte..c.text_end_byte] == *ch.to_string() {
                    return (c.box_rect, c.ink_rect);
                }
            }
        }
    }
    panic!("no cluster for {ch:?} in {body}");
}

/// pdfTeX's `\showbox` height + depth of `\big[`, `\Big[`, `\bigg[`,
/// `\Bigg[` in a 10 pt `article` without `amsmath` (the kernel's absolute
/// `\vcenter to` targets, so the same at every body size).
const KERNEL_BIG: [(&str, f64); 4] = [
    ("\\big", 8.50005 + 3.50006),
    ("\\Big", 11.50008 + 6.50009),
    ("\\bigg", 14.50010 + 9.50012),
    ("\\Bigg", 17.50014 + 12.50015),
];

/// pdfTeX rounds to 5 decimals; ticks quantise at 2^-20 bp. 1e-3 pt is well
/// inside both and well outside the ~0.10 pt the ink was wrong by.
const TOLERANCE_PT: f64 = 1e-3;

/// Every fixed-size delimiter step reports the box pdfTeX measured — the
/// number a lane comparing against `\showbox` must get. This is the
/// regression: `\Bigg(` reported 29.90 pt here, its outline's ink, inside an
/// exact 30.00029 pt box.
#[test]
fn fixed_size_delimiter_clusters_report_the_box_pdftex_measured() {
    if !lm_available() {
        return;
    }
    for (cmd, expected) in KERNEL_BIG {
        for (open, close, ch) in [("(", ")", '('), ("[", "]", '['), ("\\{", "\\}", '{')] {
            let body = format!("{cmd}{open} x {cmd}{close}");
            let (b, _) = cluster(&body, ch);
            assert!(
                (pt(b.height) - expected).abs() < TOLERANCE_PT,
                "{cmd}{open}: box {} pt, pdfTeX {expected} pt",
                pt(b.height)
            );
        }
    }
}

/// The paren chain is the one that exposed the bug, so pin it specifically:
/// the box is exact and the painted ink is visibly shorter, and the two are
/// reported as different fields rather than one being passed off as the
/// other.
#[test]
fn a_grown_paren_paints_less_ink_than_its_box_and_both_are_reported() {
    if !lm_available() {
        return;
    }
    let (b, ink) = cluster("\\Bigg( x \\Bigg)", '(');
    let expected = 17.50014 + 12.50015;
    assert!((pt(b.height) - expected).abs() < TOLERANCE_PT, "box {} pt, pdfTeX {expected} pt", pt(b.height));
    let ink = ink.expect("a painted paren has ink bounds");
    assert!(
        pt(ink.height) < pt(b.height) - 0.05,
        "Latin Modern Math's paren ends stop short of the box: ink {} pt, box {} pt",
        pt(ink.height),
        pt(b.height)
    );
    // Horizontally the two disagree in the other direction, which is the
    // clearest argument for keeping them apart: the painted outline starts
    // 2.77 pt inside the box's left edge and ends 0.31 pt past its right
    // one. (Whether a `\Bigg(` should be painted that far right is a
    // question for the painting lane; it is a fact about ink either way, and
    // it is measurable now only because ink has its own field.)
    assert!(pt(ink.x) > pt(b.x) + 1.0, "ink starts inside the box's left edge");
    assert!(
        pt(ink.x) + pt(ink.width) > pt(b.x) + pt(b.width),
        "and ends past its right one: ink {} pt, box {} pt",
        pt(ink.x) + pt(ink.width),
        pt(b.x) + pt(b.width)
    );
}

/// A bracket's flat outline does fill its box, which is why `[` measured
/// "exact" in the same runs and the paren looked like a chain-specific
/// defect. Both now report the same box; only their ink differs.
#[test]
fn a_bracket_fills_its_box_where_a_paren_does_not() {
    if !lm_available() {
        return;
    }
    let (paren, paren_ink) = cluster("\\Bigg( x \\Bigg)", '(');
    let (bracket, bracket_ink) = cluster("\\Bigg[ x \\Bigg]", '[');
    assert!(
        (pt(paren.height) - pt(bracket.height)).abs() < TOLERANCE_PT,
        "the same \\Bigg target gives the same box: paren {} pt, bracket {} pt",
        pt(paren.height),
        pt(bracket.height)
    );
    let (pi, bi) = (paren_ink.expect("paren ink"), bracket_ink.expect("bracket ink"));
    assert!(
        pt(bi.height) > pt(pi.height),
        "the bracket's flat ink reaches further than the paren's round ends: {} pt vs {} pt",
        pt(bi.height),
        pt(pi.height)
    );
}

/// A stacked extensible delimiter already reported its box; pin that it
/// still does, and that its ink is reported separately rather than standing
/// in for it. One cmex extension piece paints past its own box so the
/// stacked copies overlap, which is the other half of the false report.
#[test]
fn a_stacked_delimiter_reports_the_stacked_box_and_its_own_ink() {
    if !lm_available() {
        return;
    }
    let (b, ink) = cluster("\\left| \\frac{\\frac{a}{b}}{c} \\right|", '|');
    // pdfTeX stacks four cmex pieces here: 4 x 6.00006 pt.
    assert!((pt(b.height) - 24.00024).abs() < TOLERANCE_PT, "box {} pt, pdfTeX 24.00024 pt", pt(b.height));
    let ink = ink.expect("a painted assembly has ink bounds");
    // The assembly is built to span the box, so its ink lands on it -- but
    // it is still reported as ink, not as the box.
    assert!(
        (pt(ink.height) - pt(b.height)).abs() < 0.5,
        "the assembly spans its box: ink {} pt, box {} pt",
        pt(ink.height),
        pt(b.height)
    );
}

/// Text clusters were always the box and stay it; they report no ink, which
/// is honest, rather than a box relabelled as one.
#[test]
fn text_clusters_report_a_box_and_no_ink() {
    if !lm_available() {
        return;
    }
    let r = render_one("\\documentclass[10pt]{article}\\begin{document}Wafer office\\end{document}");
    let mut seen = 0;
    for page in &r.v2.pages {
        for it in &page.items {
            let Item::GlyphRun(run) = it else { continue };
            for c in &run.clusters {
                assert_eq!(c.ink_rect, None, "text clusters carry no ink bounds");
                assert!(c.box_rect.height.0 > 0, "a text cluster's box has height");
                seen += 1;
            }
        }
    }
    assert!(seen > 5, "saw {seen} clusters");
}
