//! Preamble page-geometry and paragraph lengths against pdflatex
//! (TeX Live 2026, `/Library/TeX/texbin/pdflatex`).
//!
//! Positions are `\pdfsavepos` of the first `H` with `\parindent=0pt`,
//! from the page lower-left, converted to the display list's top-left
//! origin with `\pdfpageheight`. Line width is `\the\textwidth`.
//! Tolerance is 0.1 bp (the acceptance gate).

mod common;

use common::*;
use flashtex_class_geometry::Sp;
use flashtex_compiler::parser::parse;
use flashtex_render_pipeline::adapter::{self, Labels};
use flashtex_render_pipeline::RenderOptions;

const SP_BP: f64 = 72.0 / 72.27 / 65536.0;

fn bp(sp: i64) -> f64 {
    sp as f64 * SP_BP
}

/// pdflatex: `\pdfpageheight=794.96999pt` (52099153 sp) on US Letter.
const LETTER_HEIGHT_SP: i64 = 52_099_153;

fn first_xy_and_measure(src: &str) -> (f64, f64, f64, flashtex_render_pipeline::adapter::Doc) {
    let r = render_one(src);
    let w = words_of(&r);
    let first = w.first().expect("a word on page 1");
    assert_eq!(first.text, "Hello");
    let parsed = parse(src);
    let doc = adapter::adapt(&[src], 0, &parsed, &RenderOptions::default(), &Labels::default());
    let tw_bp = doc.style.text_width_pt * 72.0 / 72.27;
    (first.x, first.baseline, tw_bp, doc)
}

fn assert_within_0_1bp(got_bp: f64, want_sp: i64, what: &str) {
    let d = (got_bp - bp(want_sp)).abs();
    assert!(
        d < 0.1,
        "{what}: got {got_bp} bp, pdflatex {} bp ({d} bp off)",
        bp(want_sp)
    );
}

fn wrap(preamble: &str) -> String {
    format!(
        "\\documentclass{{article}}\n{preamble}\\pagestyle{{empty}}\\begin{{document}}\nHello world.\n\\end{{document}}\n"
    )
}

/// pdflatex: `\textwidth=433.62pt`, `\oddsidemargin=25.865pt`,
/// first glyph x=6431375sp, y from bottom=43234099sp.
#[test]
fn setlength_textwidth_and_addtolength_oddsidemargin() {
    if !lm_available() {
        return;
    }
    let src = wrap(
        "\\setlength{\\textwidth}{6in}\\addtolength{\\oddsidemargin}{-.5in}\\setlength{\\parindent}{0pt}\n",
    );
    let (x, y, tw, _) = first_xy_and_measure(&src);
    assert_within_0_1bp(x, 6_431_375, "first glyph x");
    assert_within_0_1bp(y, LETTER_HEIGHT_SP - 43_234_099, "first baseline");
    assert_within_0_1bp(tw, Sp::parse("6in").unwrap().0, "line width / textwidth");
}

/// pdflatex: `\textwidth=469.75499pt`, `\oddsidemargin=0pt`,
/// first glyph x=4736286sp (`1in`).
#[test]
fn tex_assignments_textwidth_paperwidth_oddsidemargin() {
    if !lm_available() {
        return;
    }
    let src = wrap(
        "\\textwidth=6.5in \\paperwidth=8.5in \\oddsidemargin=0in\\setlength{\\parindent}{0pt}\n",
    );
    let (x, y, tw, _) = first_xy_and_measure(&src);
    assert_within_0_1bp(x, 4_736_286, "first glyph x");
    assert_within_0_1bp(y, LETTER_HEIGHT_SP - 43_234_099, "first baseline");
    assert_within_0_1bp(tw, Sp::parse("6.5in").unwrap().0, "line width / textwidth");
}

/// pdflatex: `\parindent=0pt`, `\parskip=6.0pt`; first glyph stays at
/// the class text left (8799518sp) because indent is zero.
#[test]
fn setlength_parindent_zero_and_parskip_six() {
    if !lm_available() {
        return;
    }
    let src = wrap("\\setlength{\\parindent}{0pt}\\setlength{\\parskip}{6pt}\n");
    let (x, y, tw, doc) = first_xy_and_measure(&src);
    assert_within_0_1bp(x, 8_799_518, "first glyph x");
    assert_within_0_1bp(y, LETTER_HEIGHT_SP - 43_234_099, "first baseline");
    assert_within_0_1bp(tw, Sp::parse("345pt").unwrap().0, "line width / textwidth");
    assert!((doc.style.parindent_pt).abs() < 1e-9);
    assert!(
        (doc.style.parskip.natural - 6.0).abs() < 1e-6,
        "{}",
        doc.style.parskip.natural
    );
}

/// pdflatex: `\textheight=650.43pt` (9in), `\topmargin=-36.135pt`,
/// first glyph y from bottom=46650818sp.
#[test]
fn setlength_textheight_and_topmargin() {
    if !lm_available() {
        return;
    }
    let src = wrap(
        "\\setlength{\\textheight}{9in}\\setlength{\\topmargin}{-.5in}\\setlength{\\parindent}{0pt}\n",
    );
    let (x, y, _, _) = first_xy_and_measure(&src);
    assert_within_0_1bp(x, 8_799_518, "first glyph x");
    assert_within_0_1bp(y, LETTER_HEIGHT_SP - 46_650_818, "first baseline");
}

/// geometry after a manual `\setlength{\textwidth}` overwrites it;
/// a later `\setlength` overwrites geometry.
#[test]
fn geometry_and_setlength_last_in_source_order_wins() {
    if !lm_available() {
        return;
    }
    let before = wrap(
        "\\setlength{\\textwidth}{6in}\\usepackage[margin=1in]{geometry}\\setlength{\\parindent}{0pt}\n",
    );
    let after = wrap(
        "\\usepackage[margin=1in]{geometry}\\setlength{\\textwidth}{6in}\\setlength{\\parindent}{0pt}\n",
    );
    let (_, _, tw_before, _) = first_xy_and_measure(&before);
    let (_, _, tw_after, _) = first_xy_and_measure(&after);
    assert_within_0_1bp(
        tw_before,
        Sp::parse("6.5in").unwrap().0,
        "geometry wins over earlier setlength",
    );
    assert_within_0_1bp(
        tw_after,
        Sp::parse("6in").unwrap().0,
        "later setlength wins over geometry",
    );
}
