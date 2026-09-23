//! Pinned preamble page-geometry and paragraph-length checks.
//!
//! This file does not invoke pdflatex. Glyph `(x, y)` integers are
//! `\pdfsavepos` measurements of the first `H` (`\parindent=0pt`), taken
//! once with TeX Live 2026 and recorded at commit `245fc959`. y is converted
//! from page-lower-left to the display list's top-left origin with the
//! pinned US Letter `\pdfpageheight`. Line-width expects are TeX/class
//! arithmetic (`Sp::parse`, article `size10.clo` `\textwidth 345\p@`).
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

/// Pinned at `245fc959`: TeX Live 2026 `\pdfpageheight=794.96999pt` (52099153 sp)
/// on US Letter.
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
        "{what}: got {got_bp} bp, pinned {} bp ({d} bp off)",
        bp(want_sp)
    );
}

fn wrap(preamble: &str) -> String {
    format!(
        "\\documentclass{{article}}\n{preamble}\\pagestyle{{empty}}\\begin{{document}}\nHello world.\n\\end{{document}}\n"
    )
}

/// Pinned at `245fc959`: first glyph x=6431375sp, y from bottom=43234099sp.
/// `\textwidth` expect is `Sp::parse("6in")` (6in = 433.62pt).
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

/// Pinned at `245fc959`: first glyph x=4736286sp (`1in` left).
/// `\textwidth` expect is `Sp::parse("6.5in")`.
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

/// Pinned at `245fc959`: class text left x=8799518sp with `\parindent=0pt`.
/// `\textwidth` expect is article `size10.clo` `345pt`.
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

/// Pinned at `245fc959`: first glyph y from bottom=46650818sp after
/// `\textheight=9in` and `\topmargin=-.5in`. x is class text left (8799518sp).
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

/// GH-TABLE2-UNTESTED-7 (also #721): six `PREAMBLE_LENGTHS` registers have a
/// real compiler dispatch arm and are accepted without a diagnostic
/// (`crates/compiler/tests/preamble_lengths.rs`), but nothing checked the
/// value actually reaches rendered page geometry rather than being parsed
/// and dropped. `adapter::adapt`'s resolved `PageParams` (the same struct
/// `param_length`/`assign_param` above read and write) is the render
/// pipeline's own effective value for each register, so this asserts against
/// it directly rather than against a new pdflatex measurement.
#[test]
fn untested_preamble_lengths_reach_the_resolved_page_params() {
    if !lm_available() {
        return;
    }
    // Values chosen to differ from the class's own defaults for a plain
    // `article` on US Letter (11in/12pt/30pt/65pt/10pt): 5 of the original
    // 6 values here happened to equal those defaults, so the assertion
    // could not have failed even if `\setlength` were silently dropped for
    // those five registers. `evensidemargin`'s 0.5in already differed and
    // is kept.
    for (name, dimen, get) in [
        (
            "paperheight",
            "10in",
            (|p: &flashtex_class_geometry::PageParams| p.paperheight) as fn(&flashtex_class_geometry::PageParams) -> Sp,
        ),
        ("evensidemargin", "0.5in", |p| p.evensidemargin),
        ("headheight", "20pt", |p| p.headheight),
        ("footskip", "40pt", |p| p.footskip),
        ("marginparwidth", "50pt", |p| p.marginparwidth),
        ("columnsep", "25pt", |p| p.columnsep),
    ] {
        let default_src = wrap("");
        let default_parsed = parse(&default_src);
        let default_doc = adapter::adapt(
            &[&default_src],
            0,
            &default_parsed,
            &RenderOptions::default(),
            &Labels::default(),
        );
        let default_params = &default_doc
            .style
            .class_geometry
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: class geometry did not resolve for the default document"))
            .params;
        assert_ne!(
            get(default_params),
            Sp::parse(dimen).unwrap(),
            "{name}: test value {dimen} must differ from the class default, or this assertion can't fail"
        );

        let src = wrap(&format!("\\setlength{{\\{name}}}{{{dimen}}}\n"));
        let parsed = parse(&src);
        let doc = adapter::adapt(&[&src], 0, &parsed, &RenderOptions::default(), &Labels::default());
        let params = &doc
            .style
            .class_geometry
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: class geometry did not resolve"))
            .params;
        assert_eq!(
            get(params),
            Sp::parse(dimen).unwrap(),
            "{name}: \\setlength did not reach the resolved page params"
        );
    }
}

/// geometry `[margin=1in]` on letter is 8.5in − 2in = 6.5in (`Sp::parse`);
/// a later `\setlength{\textwidth}{6in}` overwrites that (source order).
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
