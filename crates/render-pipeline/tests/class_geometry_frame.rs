//! The page frame comes from `flashtex-class-geometry` (standard class +
//! geometry, exact sp). Expected positions are pdflatex's own `\pdfsavepos`
//! readings (MacTeX 2026, pdfTeX 1.40.29) of `\noindent\pdfsavepos Hello`
//! on the first line, converted from the bottom-left origin to the display
//! list's top-left origin with `\pdfpageheight`.

mod common;

use common::*;
use flashtex_render_pipeline::display::TICKS_PER_BP;

/// One scaled point in PDF big points.
const SP_BP: f64 = 72.0 / 72.27 / 65536.0;

fn bp(sp: i64) -> f64 {
    sp as f64 * SP_BP
}

/// `(page width bp, page height bp, first word x bp, first baseline bp)`.
fn first_line(src: &str) -> (f64, f64, f64, f64) {
    let r = render_one(src);
    let page = &r.v2.pages[0];
    let w = words_of(&r);
    let first = w.first().expect("a word on page 1");
    assert_eq!(first.text, "Hello");
    (
        page.width.0 as f64 / TICKS_PER_BP,
        page.height.0 as f64 / TICKS_PER_BP,
        first.x,
        first.baseline,
    )
}

fn assert_within_2sp(got_bp: f64, want_sp: i64, what: &str) {
    let d = (got_bp - bp(want_sp)).abs() / SP_BP;
    assert!(d <= 2.0, "{what}: got {got_bp} bp, pdflatex {} bp ({d:.2} sp off)", bp(want_sp));
}

/// pdflatex: `\pdfpagewidth=614.295pt`, `\pdfpageheight=794.96999pt`
/// (52099153 sp) for both documents below.
const LETTER_HEIGHT_SP: i64 = 52_099_153;

#[test]
fn a4paper_without_geometry_ships_a_us_letter_page_with_the_a4_text_block() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `\paperwidth=597.50787pt` (A4) but no geometry: pdfTeX keeps the
    // pdftexconfig.tex MediaBox (8.5in x 11in). Text block: A4's
    // `\textwidth=345pt`, `\textheight=598pt`, placed from the top-left.
    let (w, h, x, y) = first_line("\\documentclass[a4paper]{article}\n\\setlength{\\parindent}{0pt}\n\\begin{document}\nHello world.\n\\end{document}\n");
    assert_eq!((w, h), (612.0, 792.0), "Letter MediaBox");
    assert_within_2sp(x, 8_209_694, "text left");
    assert_within_2sp(y, LETTER_HEIGHT_SP - 43_168_563, "first baseline");
}

#[test]
fn default_article_is_letter_with_class_margins() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `\oddsidemargin=62pt`, `\textwidth=345pt`, `\textheight=550pt`.
    let (w, h, x, y) = first_line("\\documentclass{article}\n\\setlength{\\parindent}{0pt}\n\\begin{document}\nHello world.\n\\end{document}\n");
    assert_eq!((w, h), (612.0, 792.0));
    assert_within_2sp(x, 8_799_518, "text left");
    assert_within_2sp(y, LETTER_HEIGHT_SP - 43_234_099, "first baseline");
}

#[test]
fn a4paper_with_geometry_ships_an_a4_page() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // geometry's pdftex driver sets `\pdfpagewidth=\paperwidth`; pdfTeX
    // writes the MediaBox as `595.276 841.89` (\pdfdecimaldigits=3).
    let (w, h, x, y) = first_line("\\documentclass[a4paper]{article}\n\\usepackage[margin=1in]{geometry}\n\\setlength{\\parindent}{0pt}\n\\begin{document}\nHello world.\n\\end{document}\n");
    assert!((w - 595.276).abs() < 1e-6 && (h - 841.89).abs() < 1e-6, "{w} x {h}");
    // 1in margins; first baseline 1in + \topskip (10pt).
    assert_within_2sp(x, 4_736_286, "text left");
    assert_within_2sp(y, 4_736_286 + 10 * 65536, "first baseline");
}
