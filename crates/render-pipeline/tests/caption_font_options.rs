//! caption.sty's `font=small` caption size and `labelfont=bf` label weight,
//! against pdflatex.
//!
//! pdflatex (TeX Live, `article` at 10pt) with
//! `\usepackage[font=small,labelfont=bf]{caption}` sets the whole caption
//! line in `\small` — 9pt — with the `Figure 1:` label in bold (`cmbx9`)
//! and the body in regular 9pt (`cmr9`). Only the caption line is checked,
//! not the page: caption spacing and float placement are separate concerns.

mod common;

use flashtex_render_pipeline::display::{Item, RunRole};

const SOURCE: &str = "\\documentclass{article}\n\\usepackage[font=small,labelfont=bf]{caption}\n\\begin{document}\nText before the float.\n\\begin{figure}[h]\nA placeholder body.\n\\caption{A small caption line}\n\\end{figure}\nAfter text here.\n\\end{document}\n";

/// One text glyph run: its text, font size in bp, whether its face is bold,
/// and its baseline in bp.
#[derive(Debug)]
struct Run {
    text: String,
    size: f64,
    bold: bool,
    baseline: f64,
}

fn runs(source: &str) -> (Vec<Run>, Vec<String>) {
    let r = common::render_one(source);
    let diags = r.v2.diagnostics.iter().map(|d| d.message.clone()).collect();
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for item in page.resident_items() {
            let Item::GlyphRun(run) = item else { continue };
            if !matches!(run.role, RunRole::Text) || run.glyphs.is_empty() {
                continue;
            }
            let face = r
                .v2
                .fonts
                .iter()
                .find(|f| f.font_id == run.font_id)
                .map(|f| f.postscript_name.clone())
                .unwrap_or_default();
            out.push(Run {
                text: run.text.clone(),
                size: run.font_size.to_bp(),
                bold: face.contains("Bold"),
                baseline: run.glyphs[0].baseline_y.to_bp(),
            });
        }
    }
    (out, diags)
}

fn find<'a>(runs: &'a [Run], word: &str) -> &'a Run {
    runs.iter().find(|r| r.text == word).unwrap_or_else(|| panic!("no run {word:?} in {runs:?}"))
}

#[test]
fn caption_line_is_small_with_a_bold_label() {
    assert!(common::lm_available());
    let (runs, diags) = runs(SOURCE);
    assert!(
        !diags.iter().any(|m| m.contains("caption")),
        "caption warning survived: {diags:?}"
    );
    // pdflatex reference: 9pt (`\small` at a 10pt base) across the line.
    let figure = find(&runs, "Figure");
    let number = find(&runs, "1:");
    let body = find(&runs, "caption");
    for run in [figure, number, body] {
        assert!((run.size - 9.0).abs() < 0.05, "{run:?}: pdflatex sets 9pt");
    }
    // ... the label bold, the body regular, all on one line.
    assert!(figure.bold && number.bold, "{figure:?} {number:?}: pdflatex bolds the label");
    assert!(!body.bold, "{body:?}: pdflatex leaves the body regular");
    assert!(
        (figure.baseline - body.baseline).abs() < 0.01,
        "label and body are one caption line: {figure:?} {body:?}"
    );
    // Control: the surrounding text is untouched 10pt regular.
    let before = find(&runs, "before");
    assert!((before.size - 10.0).abs() < 0.05 && !before.bold, "{before:?}");
}

#[test]
fn caption_options_outside_the_subset_keep_warning_and_defaults() {
    assert!(common::lm_available());
    let source = SOURCE.replace("font=small,labelfont=bf", "font=it");
    let (runs, diags) = runs(&source);
    assert!(
        diags.iter().any(|m| m.contains("packages caption are recognised but not implemented")),
        "unmodelled option must still warn: {diags:?}"
    );
    let figure = find(&runs, "Figure");
    let body = find(&runs, "caption");
    for run in [figure, body] {
        assert!((run.size - 10.0).abs() < 0.05 && !run.bold, "{run:?}: defaults without the options");
    }
}
