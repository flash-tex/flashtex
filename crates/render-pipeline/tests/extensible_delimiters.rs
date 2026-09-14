//! Vertical extensible delimiters: the cmex pieces tex.web §713 stacks for a
//! fence taller than every fixed size, painted from Latin Modern Math's
//! `MathVariants` vertical glyph assemblies.
//!
//! The reference numbers are pdfTeX 3.141592653 (TeX Live 2025) `\showbox`
//! output for a 10 pt `article`, `$\displaystyle\left<d> \begin{array}{c} a
//! \\ b \\ c \\ d \\ e \\ f \end{array} \right<d>$`: every one of those
//! fences is stacked to 72.00072 pt (the pieces are listed per fence below),
//! and `\left( \frac{\frac{\frac{a}{b}}{c}}{\frac{d}{\frac{e}{f}}} \right)`
//! to 36.00036 pt from a top and a bottom piece with no repeat.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const BP: f64 = 72.0 / 72.27;

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\\begin{{document}}\\[{body}\\]\\end{{document}}")
}

const ROWS: &str = "\\begin{array}{c} a \\\\ b \\\\ c \\\\ d \\\\ e \\\\ f \\end{array}";

/// The painted glyphs of the first cluster whose text is `ch`, as
/// `(glyph id, baseline in TeX pt)`, plus the top and height of its
/// **laid-out TeX box** (`Cluster::box_rect`, the wire's `hit_rects`).
///
/// Not `ink_rect`: that is the extent of the outline painted inside the
/// box, which for these fences is deliberately a different number.
fn fence(body: &str, ch: char) -> (Vec<(u16, f64)>, f64, f64) {
    let r = render_one(&doc(body));
    for page in &r.v2.pages {
        for it in &page.items {
            let Item::GlyphRun(run) = it else { continue };
            for (ci, c) in run.clusters.iter().enumerate() {
                if run.text.as_bytes()[c.text_start_byte..c.text_end_byte] != *ch.to_string().as_bytes() {
                    continue;
                }
                let glyphs: Vec<(u16, f64)> = run
                    .glyphs
                    .iter()
                    .filter(|g| g.cluster as usize == ci)
                    .map(|g| (g.gid, g.baseline_y.to_bp() / BP))
                    .collect();
                return (glyphs, c.box_rect.top.to_bp() / BP, c.box_rect.height.to_bp() / BP);
            }
        }
    }
    panic!("no cluster for {ch:?}");
}

/// Every fence that needs an assembly paints one, and its box is the height
/// pdfTeX stacked the cmex pieces to.
#[test]
fn tall_fences_are_assembled_to_the_height_pdftex_stacks_them_to() {
    if !lm_available() {
        return;
    }
    // (source, cluster character, pdfTeX's stacked height, part count)
    //
    // The part counts are the font's, not cmex's: Latin Modern Math's
    // extender for `(` is 0.498 em where cmex's repeat is 0.6 em, so the
    // same 72.00072 pt takes 9 repeats (11 parts) instead of cmex's 6. The
    // bottom and top parts of `\lceil`/`\lfloor` are absent in both.
    let cases: [(&str, char, f64, usize); 8] = [
        ("\\left( \\frac{\\frac{\\frac{a}{b}}{c}}{\\frac{d}{\\frac{e}{f}}} \\right)", '(', 36.00036, 4),
        ("\\left( ROWS \\right)", '(', 72.00072, 11),
        ("\\left[ ROWS \\right]", '[', 72.00072, 7),
        ("\\left\\{ ROWS \\right\\}", '{', 72.00072, 9),
        ("\\left| ROWS \\right|", '|', 72.00072, 7),
        ("\\left\\lceil ROWS \\right\\rceil", '\u{2308}', 72.00072, 7),
        ("\\left\\lfloor ROWS \\right\\rfloor", '\u{230A}', 72.00072, 7),
        ("\\sqrt{ROWS}", '\u{221A}', 78.00077, 11),
    ];
    for (body, ch, height, parts) in cases {
        let (glyphs, _, h) = fence(&body.replace("ROWS", ROWS), ch);
        assert_eq!(glyphs.len(), parts, "{ch:?} parts");
        // Tick quantisation is 2^-20 bp, so the box lands within 0.0001 pt.
        assert!((h - height).abs() < 1e-3, "{ch:?} box {h} pt, pdfTeX {height} pt");
    }
}

/// The assembly's ink fills the box exactly: every Latin Modern Math
/// vertical part draws from its origin up to its `fullAdvance`, so the
/// bottom part's baseline is the box's bottom edge, the top part's ink
/// reaches the box's top edge, and no joint is wider than the font's
/// connectors allow.
#[test]
fn an_assembly_spans_its_box_without_a_seam() {
    if !lm_available() {
        return;
    }
    let (glyphs, top, height) = fence(&format!("\\left[ {ROWS} \\right]"), '[');
    // `[`'s assembly: bracketleft.bot (1.5 em), .ext (1 em), .top (1.5 em),
    // with minConnectorOverlap 20/1000 em; parts are painted bottom to top.
    let full = |gid: u16| if gid == 2528 { 10.0 } else { 15.0 };
    let bottom = top + height;
    let first = glyphs.first().expect("parts");
    assert!((first.1 - bottom).abs() < 1e-3, "bottom part sits on the box's bottom edge: {} vs {bottom}", first.1);
    let last = glyphs.last().expect("parts");
    assert!(((last.1 - full(last.0)) - top).abs() < 1e-3, "top part's ink reaches the box's top edge");
    for pair in glyphs.windows(2) {
        // Each part is above the one before it, overlapping it by at least
        // `minConnectorOverlap` (0.2 pt at 10 pt) and no more than the
        // shorter part's own length.
        let step = pair[0].1 - pair[1].1;
        let overlap = full(pair[0].0) - step;
        assert!(step > 0.0, "parts go bottom to top");
        assert!(overlap >= 0.2 - 1e-9, "overlap {overlap} pt below minConnectorOverlap");
        assert!(overlap <= full(pair[0].0).min(full(pair[1].0)), "overlap {overlap} pt swallows a part");
    }
}

/// No piece of a stacked delimiter is left unpainted: the whole point of
/// reading the vertical assemblies (before, cmex's top/middle/bottom/repeat
/// slots had no Latin Modern Math mapping and drew nothing).
#[test]
fn assembled_fences_report_no_unmapped_glyph() {
    if !lm_available() {
        return;
    }
    let body = format!(
        "\\left( {ROWS} \\right) \\left[ {ROWS} \\right] \\left\\{{ {ROWS} \\right\\}} \
         \\left| {ROWS} \\right| \\left\\lceil {ROWS} \\right\\rceil \
         \\left\\lfloor {ROWS} \\right\\rfloor \\sqrt{{{ROWS}}}"
    );
    let r = render_one(&doc(&body));
    let bad: Vec<&str> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "math_glyph_unmapped")
        .map(|d| d.message.as_str())
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
}
