//! Issue #892: `\vec` against pdfTeX, by the ink it paints.
//!
//! pdfTeX sets `\vec` as cmmi10 slot `"7E` (`vector`), a *spacing* accent
//! 5pt wide whose arrow sits over the letter. Latin Modern Math has no
//! spacing arrow: the glyph FlashTeX paints is `uni20D7` (U+20D7 COMBINING
//! RIGHT ARROW ABOVE), the same design with zero advance and its ink left
//! of the origin, so painted at the TeX origin it landed a glyph width left
//! of the letter. #910 (`002b78e68`) shifts the paint so the two inks'
//! centres coincide; this pins the result end to end against the oracle:
//! the painted arrow's ink box (origin plus the Latin Modern Math outline's
//! bounding box) against pdfTeX's (its origin plus cmmi10's `vector`
//! bounding box, `cmmi10.afm` `B 182 516 625 714`), within 0.5 bp on every
//! side, for `\vec{v}`, `\vec{A}` and `\vec{F}`. The glyph identity differs
//! by construction (a different font program), so identity here is "the
//! Latin Modern Math arrow"; its origin is not pdfTeX's and is not
//! compared. Every other glyph of the issue's repro -- the letters and the
//! nine other accents, which paint their own spacing glyphs at pdfTeX's
//! origin -- is pinned by identity and origin within 0.5 bp (the two cmex
//! wide accents in x only).
//!
//! Expected numbers are pdfTeX 1.40.29 (TeX Live 2026), `pdflatex
//! -interaction=batchmode`, two passes, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, on exactly [`ISSUE_REPRO`], read with
//! `tools/visual-oracle/pdftext.py`'s `page_glyphs` (bp, y from the page
//! top). The oracle never runs here. A 2400 dpi raster of both PDFs agreed:
//! pdfTeX's `\vec{v}` ink [160.92, 165.36] x [127.77, 129.75], FlashTeX's
//! [161.04, 165.18] x [127.80, 129.69].

mod common;

use common::*;
use flashtex_render_pipeline::cff::Cff;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::FontSet;

const ISSUE_REPRO: &str = "\\documentclass{article}
\\begin{document}
X $\\vec{v}$ X $\\hat{v}$ X $\\bar{v}$ X $\\tilde{v}$ X $\\dot{v}$ X $\\ddot{v}$ X
$\\acute{v}$ X $\\grave{v}$ X $\\widehat{abc}$ X $\\widetilde{abc}$ X $\\overline{abc}$ X

X $\\vec{A}$ X $\\hat{A}$ X $\\vec{F}$ X
\\end{document}
";

/// pdfTeX's glyphs other than the three `\vec` arrows: (painted text,
/// pdfTeX font, x bp, baseline y bp); `y = 0.0` means x only (cmex).
const EXPECTED: &[(&str, &str, f64, f64)] = &[
    ("\u{0302}", "CMEX10", 315.853, 0.0),
    ("\u{0303}", "CMEX10", 343.82, 0.0),
    ("X", "CMR10", 148.712, 134.903),
    ("v", "CMMI10", 159.501, 134.903),
    ("X", "CMR10", 168.017, 134.903),
    ("v", "CMMI10", 178.806, 134.903),
    ("ˆ", "CMR10", 179.185, 134.903),
    ("X", "CMR10", 187.311, 134.903),
    ("v", "CMMI10", 198.111, 134.903),
    ("¯", "CMR10", 198.489, 134.903),
    ("X", "CMR10", 206.616, 134.903),
    ("v", "CMMI10", 217.405, 134.903),
    ("˜", "CMR10", 217.784, 134.903),
    ("X", "CMR10", 225.91, 134.903),
    ("v", "CMMI10", 236.708, 134.903),
    ("˙", "CMR10", 238.194, 134.903),
    ("X", "CMR10", 245.213, 134.903),
    ("v", "CMMI10", 256.002, 134.903),
    ("¨", "CMR10", 256.391, 134.903),
    ("X", "CMR10", 264.517, 134.903),
    ("v", "CMMI10", 275.307, 134.903),
    ("´", "CMR10", 275.685, 134.903),
    ("X", "CMR10", 283.812, 134.903),
    ("v", "CMMI10", 294.611, 134.903),
    ("`", "CMR10", 294.99, 134.903),
    ("X", "CMR10", 303.116, 134.903),
    ("a", "CMMI10", 313.908, 134.903),
    ("b", "CMMI10", 319.174, 134.903),
    ("c", "CMMI10", 323.45, 134.903),
    ("X", "CMR10", 331.08, 134.903),
    ("a", "CMMI10", 341.875, 134.903),
    ("b", "CMMI10", 347.141, 134.903),
    ("c", "CMMI10", 351.417, 134.903),
    ("X", "CMR10", 359.047, 134.903),
    ("a", "CMMI10", 369.841, 134.903),
    ("b", "CMMI10", 375.107, 134.903),
    ("c", "CMMI10", 379.383, 134.903),
    ("X", "CMR10", 387.013, 134.903),
    ("ˆ", "CMR10", 183.72, 144.34),
    ("X", "CMR10", 148.712, 146.858),
    ("A", "CMMI10", 159.505, 146.858),
    ("X", "CMR10", 170.294, 146.858),
    ("A", "CMMI10", 181.091, 146.858),
    ("X", "CMR10", 191.88, 146.858),
    ("F", "CMMI10", 202.677, 146.858),
    ("X", "CMR10", 213.786, 146.858),
    ("1", "CMR10", 303.133, 702.635),
];

/// pdfTeX's `\vec` arrows, cmmi10 `"7E` at 10pt: origin (x, baseline y) in
/// bp, for `\vec{v}`, `\vec{A}`, `\vec{F}`.
const VEC_ORIGINS: &[(f64, f64)] = &[(159.123, 134.903), (161.368, 144.34), (204.145, 144.34)];

/// `cmmi10.afm`: `C 126 ; WX 500 ; N vector ; B 182 516 625 714`.
const CMMI10_VECTOR_BBOX: [f64; 4] = [182.0, 516.0, 625.0, 714.0];

const TOL_BP: f64 = 0.5;

/// 10pt in bp.
const SIZE_BP: f64 = 10.0 * 72.0 / 72.27;

#[test]
fn vec_arrow_ink_matches_pdftex() {
    if !lm_available() {
        return;
    }
    let math = FontSet::with_default_dirs(&[])
        .dirs()
        .iter()
        .map(|d| d.join("latinmodern-math.otf"))
        .find(|p| p.is_file())
        .expect("latinmodern-math.otf beside the Latin Modern faces");
    let face = flashtex_font_engine::load_from_path_index(&math, 0).expect("Latin Modern Math loads");
    let table = face.cff_table().expect("Latin Modern Math is CFF");
    let cff = Cff::parse(table).expect("CFF parses");
    let r = render_one(ISSUE_REPRO);
    let mut arrows = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        let face = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).map(|f| f.postscript_name.clone()).unwrap_or_default();
        for g in &run.glyphs {
            let c = &run.clusters[g.cluster as usize];
            if run.text[c.text_start_byte..c.text_end_byte] != *"\u{20D7}" {
                continue;
            }
            assert_eq!(face, "LatinModernMath-Regular", "the arrow is Latin Modern Math's");
            let (bbox, _) = cff.glyph_bbox(table, g.gid).expect("outline");
            let [x0, y0, x1, y1] = bbox.expect("the arrow has ink");
            let em = run.font_size.to_bp() / 1000.0;
            let (x, y) = (g.origin_x.to_bp(), g.baseline_y.to_bp());
            arrows.push([x + x0 * em, y - y1 * em, x + x1 * em, y - y0 * em]);
        }
    }
    assert_eq!(arrows.len(), VEC_ORIGINS.len(), "{arrows:?}");
    arrows.sort_by(|a, b| (a[3], a[0]).partial_cmp(&(b[3], b[0])).unwrap());
    let mut want: Vec<[f64; 4]> = VEC_ORIGINS
        .iter()
        .map(|&(x, y)| {
            let [x0, y0, x1, y1] = CMMI10_VECTOR_BBOX.map(|v| v * SIZE_BP / 1000.0);
            [x + x0, y - y1, x + x1, y - y0]
        })
        .collect();
    want.sort_by(|a, b| (a[3], a[0]).partial_cmp(&(b[3], b[0])).unwrap());
    for (got, want) in arrows.iter().zip(&want) {
        let off = got.iter().zip(want).map(|(g, w)| (g - w).abs()).fold(0.0, f64::max);
        eprintln!("arrow ink {got:.3?} vs pdfTeX {want:.3?}: {off:.3} bp");
        assert!(off <= TOL_BP, "arrow ink {got:.3?} vs pdfTeX's {want:.3?}: {off:.3} bp off");
    }
}

#[test]
fn every_other_glyph_matches_pdftex() {
    if !lm_available() {
        return;
    }
    let r = render_one(ISSUE_REPRO);
    let mut actual: Vec<(String, f64, f64, bool)> = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        for g in &run.glyphs {
            let c = &run.clusters[g.cluster as usize];
            let text = run.text[c.text_start_byte..c.text_end_byte].to_string();
            actual.push((text, g.origin_x.to_bp(), g.baseline_y.to_bp(), false));
        }
    }
    let mut misses = Vec::new();
    for &(text, font, x, y) in EXPECTED {
        let hit = actual
            .iter()
            .enumerate()
            .filter(|(_, (t, ax, ay, used))| !used && t == text && (ax - x).abs() <= TOL_BP && (y == 0.0 || (ay - y).abs() <= TOL_BP))
            .min_by(|a, b| {
                let d = |g: &(String, f64, f64, bool)| (g.1 - x).abs() + if y == 0.0 { 0.0 } else { (g.2 - y).abs() };
                d(a.1).total_cmp(&d(b.1))
            })
            .map(|(i, _)| i);
        match hit {
            Some(i) => actual[i].3 = true,
            None => misses.push(format!("{text:?} ({font} at {x}, {y})")),
        }
    }
    assert!(misses.is_empty(), "{} of {} pdfTeX glyphs unmatched within {TOL_BP} bp:\n{}", misses.len(), EXPECTED.len(), misses.join("\n"));
}
