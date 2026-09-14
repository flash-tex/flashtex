//! `\big`..`\Bigg` are sized by whichever of the two definitions is in
//! force, and which one that is depends only on whether `amsmath` is loaded.
//!
//! * **amsmath** (`amsmath.sty` 721-738, `\bBigg@`):
//!   `\vcenter to <factor>\big@size{}` where `\big@size` is 1.2 x the height
//!   plus depth of `\Mathstrutbox@` (the text-size roman `(`), so the
//!   delimiter grows with the body size.
//! * **the LaTeX kernel** (`fontmath.ltx` 513-520), in force when amsmath is
//!   *not* loaded: `\vbox to <pt>{}` with `pt` an absolute 8.5 / 11.5 /
//!   14.5 / 17.5, and family 3 is `sfixed*cmex10`, so the delimiter does not
//!   move with the body size at all.
//!
//! `\showbox` under pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025) of
//! `\Big[` in an `article`, the delimiter glyph's own box (height+depth):
//!
//! | body size | no `amsmath` | `amsmath` |
//! |---|---|---|
//! | 10pt | 18.00017 | 18.00017 |
//! | 11pt | 18.00017 | 19.71019 |
//! | 12pt | 18.00017 | 21.60020 |
//!
//! math-layout models both (`BigSizing`, PR #239); selecting between them is
//! this crate's job, because the package list is the pipeline's to read.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// The `(gid, font size in bp)` of the `\Big[` delimiter glyph.
fn big_bracket(class_option: &str, amsmath: bool) -> (u16, String) {
    let fonts = FontSet::with_default_dirs(&[]);
    let preamble = if amsmath { "\\usepackage{amsmath}" } else { "" };
    let text = format!(
        "\\documentclass[{class_option}]{{article}}{preamble}\\begin{{document}}$\\Big[ x$\\end{{document}}"
    );
    let docs = [SourceDocument { path: "main.tex", text: &text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let Item::GlyphRun(run) = it {
                for g in &run.glyphs {
                    let c = &run.clusters[g.cluster as usize];
                    let ch = run.text[c.text_start_byte as usize..c.text_end_byte as usize]
                        .chars()
                        .next()
                        .unwrap_or('?');
                    if ch == '[' {
                        return (g.gid, format!("{:.4}", run.font_size.to_bp()));
                    }
                }
            }
        }
    }
    panic!("no `[` glyph painted for {class_option} amsmath={amsmath}");
}

/// Without amsmath the kernel's lengths are absolute: the *same* cmex
/// variant at the *same* size in a 10, 11 and 12 pt document.
///
/// This is the regression. Before the pipeline selected the rule, a 12 pt
/// document without amsmath was sized by amsmath's rule anyway and reached
/// one variant further up the cmex chain — a visibly taller bracket than
/// pdflatex sets.
#[test]
fn without_amsmath_big_does_not_move_with_the_body_size() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let ten = big_bracket("10pt", false);
    let eleven = big_bracket("11pt", false);
    let twelve = big_bracket("12pt", false);
    assert_eq!(ten, eleven, "11pt without amsmath must match 10pt");
    assert_eq!(ten, twelve, "12pt without amsmath must match 10pt");
    // cmex10, whatever the body size: family 3 is `sfixed*cmex10`.
    assert_eq!(ten.1, "9.9626", "the kernel rule sets \\Big[ from cmex10");
}

/// With amsmath loaded, `\big@size` is 1.2 x `\Mathstrutbox@`, so the
/// delimiter tracks the body size.
#[test]
fn with_amsmath_big_tracks_the_body_size() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let sizes: Vec<String> = ["10pt", "11pt", "12pt"]
        .iter()
        .map(|c| big_bracket(c, true).1)
        .collect();
    assert_eq!(sizes, vec!["9.9626", "10.9091", "11.9552"], "amsmath's \\bBigg@ scales");
}

/// `amssymb`/`amsfonts` are the msam/msbm *symbol fonts*; neither loads
/// `amsmath`, so neither may select amsmath's sizing rule. Guards against
/// "simplifying" this by reusing the existing `ams_symbol_fonts` predicate,
/// which is `amssymb || amsfonts` and would be wrong here.
///
/// Note what this test deliberately does **not** claim. Rendered output for
/// an `amssymb`-only document is currently *identical* to an `amsmath` one,
/// because `amsfonts.sty` redeclares `OMX/cmex/m/n` with several designs
/// instead of `sfixed*cmex10` (what `ExtensionSizing::Designs` models, PR
/// #204), and at these sizes the kernel's absolute 11.5pt target and
/// amsmath's scaled one resolve to the same cmex variant. The design
/// selection and the sizing rule are two separate mechanisms keyed off two
/// different packages; only the second is this test's subject, so it is
/// asserted where it is actually decided rather than through a coincidence
/// in the painted glyph.
#[test]
fn amssymb_alone_does_not_select_the_amsmath_rule() {
    use flashtex_render_pipeline::adapter::package_options;
    let amssymb_only = "\\documentclass[12pt]{article}\\usepackage{amssymb}\\begin{document}$\\Big[ x$\\end{document}";
    assert_eq!(package_options(amssymb_only, "amsmath"), None, "amssymb must not read as amsmath");
    assert_eq!(package_options(amssymb_only, "amssymb"), Some(String::new()), "amssymb is loaded");
    let amsfonts_only = "\\documentclass[12pt]{article}\\usepackage{amsfonts}\\begin{document}$\\Big[ x$\\end{document}";
    assert_eq!(package_options(amsfonts_only, "amsmath"), None, "amsfonts must not read as amsmath");
}
