//! Grouped and `tabular`-cell graphics inside floats, against the float's own
//! top-level `\includegraphics` path as the oracle.
//!
//! A figure holding `{\includegraphics[width=2cm]{...}}`, or a `tabular`
//! cell holding `\includegraphics`, used to warn that the graphic "is not
//! set yet and takes no space" -- yet the same build's display list already
//! contained the image at the size the top-level path sets. The warning was
//! stale: those cases must be silent and must emit the same image box.
//!
//! `\resizebox{3cm}{!}{\includegraphics{...}}` is the honest remainder: the
//! adapter sets a transform's content untransformed, so the image goes out
//! at its unscaled size. That keeps a (narrower) warning saying exactly
//! that, instead of claiming it takes no space.
//!
//! The image is `fixtures/float-graphics/images/wide-72.png`; no TeX runs
//! here. pdflatex reference: 2cm = 56.6929pt wide; `\resizebox{3cm}` would
//! be 85.3582pt wide, while the unscaled natural box is 200x100bp.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/float-graphics");
const IMAGE: &str = "images/wide-72.png";
const TOL: f64 = 1.0;

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage[margin=1in]{{geometry}}\n\\usepackage{{graphicx}}\n\\setlength{{\\parindent}}{{0pt}}\n\\pagestyle{{empty}}\n\\begin{{document}}\nText before.\n\n{body}\n\nText after.\n\\end{{document}}\n"
    )
}

fn figure(inner: &str) -> String {
    doc(&format!("\\begin{{figure}}[h]\n\\centering\n{inner}\n\\caption{{A caption.}}\n\\end{{figure}}"))
}

struct Outcome {
    images: Vec<(f64, f64, f64, f64)>,
    diags: Vec<String>,
}

fn render_body(body: &str) -> Outcome {
    let tex = figure(body);
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(DIR.into()), ..RenderOptions::default() };
    let sources = [SourceDocument { path: "main.tex", text: &tex }];
    let r = render(&sources, "main.tex", 1, "float-grouped-graphics", &fonts, &options);
    let mut images: Vec<(f64, f64, f64, f64)> = r
        .v2
        .pages
        .iter()
        .flat_map(|p| {
            p.resident_items().iter().filter_map(|it| match it {
                Item::Image(i) => Some((i.x.to_bp(), i.top.to_bp(), i.width.to_bp(), i.height.to_bp())),
                _ => None,
            })
        })
        .collect();
    images.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let diags: Vec<String> = r.v2.diagnostics.iter().map(|d| format!("{}: {}", d.code, d.message)).collect();
    Outcome { images, diags }
}

fn float_or_graphics_diags(out: &Outcome) -> Vec<&String> {
    out.diags.iter().filter(|d| d.starts_with("float") || d.starts_with("graphics") || d.starts_with("image")).collect()
}

fn close(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    (a.0 - b.0).abs() <= TOL && (a.1 - b.1).abs() <= TOL && (a.2 - b.2).abs() <= TOL && (a.3 - b.3).abs() <= TOL
}

#[test]
fn grouped_and_tabular_graphics_match_the_top_level_box() {
    if !common::lm_available() {
        return;
    }
    // The oracle: the same graphic at the float's own level, which goes
    // through `Piece::Graphic` (gated against pdflatex by
    // `float_graphics_oracle`).
    let top = render_body(&format!("\\includegraphics[width=2cm]{{{IMAGE}}}"));
    assert_eq!(top.images.len(), 1, "top-level images: {:?}\ndiags: {:?}", top.images, top.diags);
    assert!(float_or_graphics_diags(&top).is_empty(), "top-level diags: {:?}", top.diags);

    let grouped = render_body(&format!("{{\\includegraphics[width=2cm]{{{IMAGE}}}}}"));
    assert!(float_or_graphics_diags(&grouped).is_empty(), "grouped diags: {:?}", grouped.diags);
    assert_eq!(grouped.images.len(), 1, "grouped images: {:?}\ndiags: {:?}", grouped.images, grouped.diags);
    assert!(close(grouped.images[0], top.images[0]), "grouped {:?} vs top-level {:?}", grouped.images[0], top.images[0]);

    let cell = render_body(&format!("\\begin{{tabular}}{{c}}\n\\includegraphics[width=2cm]{{{IMAGE}}} \\\\\n\\end{{tabular}}"));
    assert!(float_or_graphics_diags(&cell).is_empty(), "tabular-cell diags: {:?}", cell.diags);
    assert_eq!(cell.images.len(), 1, "tabular-cell images: {:?}\ndiags: {:?}", cell.images, cell.diags);
    assert!(close(cell.images[0], top.images[0]), "tabular-cell {:?} vs top-level {:?}", cell.images[0], top.images[0]);
}

#[test]
fn a_graphic_the_float_box_drops_still_warns() {
    if !common::lm_available() {
        return;
    }
    // Footnote text is omitted inside a float box (`float_footnote_unplaced`),
    // so a graphic there really takes no space and the original warning stays
    // due -- including when the run mentions no transform at all.
    let out = render_body(&format!("Text\\footnote{{A note with \\includegraphics[width=2cm]{{{IMAGE}}}.}}"));
    assert!(out.images.is_empty(), "dropped graphic images: {:?}", out.images);
    assert!(
        out.diags.iter().any(|d| d.starts_with("float_content_unsupported") && d.contains("takes no space")),
        "the original warning: {:?}",
        out.diags
    );
}

#[test]
fn resizebox_keeps_a_narrower_warning_and_sets_the_graphic_unscaled() {
    if !common::lm_available() {
        return;
    }
    // The unscaled oracle: the same file with no keys, grouped so it takes
    // the content-run path like the `\resizebox` case does.
    let natural = render_body(&format!("{{\\includegraphics{{{IMAGE}}}}}"));
    assert_eq!(natural.images.len(), 1, "natural images: {:?}\ndiags: {:?}", natural.images, natural.diags);

    let out = render_body(&format!("\\resizebox{{3cm}}{{!}}{{\\includegraphics{{{IMAGE}}}}}"));
    assert_eq!(out.images.len(), 1, "resizebox images: {:?}\ndiags: {:?}", out.images, out.diags);
    // pdflatex would set 3cm = 85.36bp wide; the adapter sets a transform's
    // content untransformed, so the box keeps its natural size instead.
    assert!(
        close(out.images[0], natural.images[0]),
        "resizebox {:?} should keep the natural box {:?}",
        out.images[0],
        natural.images[0]
    );
    assert!(
        (out.images[0].2 - 85.36).abs() > 5.0,
        "resizebox {:?} must not claim the scaled 3cm width it does not set",
        out.images[0]
    );
    let stale: Vec<&String> = out.diags.iter().filter(|d| d.contains("takes no space")).collect();
    assert!(stale.is_empty(), "the stale warning must be gone: {:?}", out.diags);
    assert_eq!(
        out.diags.iter().filter(|d| d.starts_with("float_content_unsupported")).count(),
        1,
        "exactly the narrower resizebox warning: {:?}",
        out.diags
    );
    assert!(out.diags.iter().any(|d| d.contains("resizebox") && d.contains("unscaled")), "narrower warning: {:?}", out.diags);
}
