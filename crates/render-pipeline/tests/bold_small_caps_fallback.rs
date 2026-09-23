//! Latin Modern has no bold small-caps design (GH-208): `bx`/`sc` must fall
//! back exactly the way pdfLaTeX falls back, not to a medium-weight face.
//!
//! Real pdflatex (TeX Live 2026, MacTeX; oracle only) logs for
//! `{\bfseries\scshape Hello}` under `lmodern`, in T1 and in OT1:
//!
//! ```text
//! LaTeX Font Warning: Font shape `T1/lmr/bx/sc' undefined
//! (Font)              using `T1/lmr/bx/n' instead
//! ```
//!
//! and embeds `LMRoman10-Bold` for the words: the substitute is the bold
//! upright face, keeping the requested weight and dropping only the shape.
//! This test pins that end to end (NFSS selection in `nfss.rs` is already
//! unit-pinned; what regressed before was the pipeline glue choosing the
//! medium `lmromancaps` design for the raw `bx`/`sc` key instead).

mod common;

use flashtex_render_pipeline::display::{Item, RunRole};

/// `(text, PostScript design)` of every text glyph run on page 1.
fn runs(preamble: &str) -> Vec<(String, String)> {
    let source = format!(
        "\\documentclass{{article}}\n{preamble}\\begin{{document}}\n{{\\bfseries\\scshape Hello}} {{\\scshape\\bfseries World}}\n\\end{{document}}\n"
    );
    let r = common::render_docs(&[("main.tex", &source)], "main.tex");
    let faces: Vec<(String, String)> = r
        .v2
        .pages
        .iter()
        .flat_map(|p| p.resident_items())
        .filter_map(|item| match item {
            Item::GlyphRun(run) if matches!(run.role, RunRole::Text | RunRole::Math) && !run.glyphs.is_empty() => {
                let face = r
                    .v2
                    .fonts
                    .iter()
                    .find(|f| f.font_id == run.font_id)
                    .map(|f| f.postscript_name.clone())
                    .unwrap_or_default();
                Some((run.text.clone(), face))
            }
            _ => None,
        })
        .collect();
    faces
        .into_iter()
        .map(|(text, face)| (text, face.chars().filter(|c| !c.is_ascii_digit()).collect()))
        .collect()
}

fn check(preamble: &str, encoding: &str) {
    let found = runs(preamble);
    for word in ["Hello", "World"] {
        let (_, design) = found
            .iter()
            .find(|(t, _)| t == word)
            .unwrap_or_else(|| panic!("no run {word:?} in {found:?}"));
        // Bold upright, as pdfLaTeX's `using .../bx/n instead`: never the
        // medium small-caps design (`LMRomanCaps-Regular`).
        assert_eq!(design, "LMRoman-Bold", "{word} under {encoding}/lmr/bx/sc");
    }
    let source = format!(
        "\\documentclass{{article}}\n{preamble}\\begin{{document}}\n{{\\bfseries\\scshape Hello}} {{\\scshape\\bfseries World}}\n\\end{{document}}\n"
    );
    let r = common::render_docs(&[("main.tex", &source)], "main.tex");
    let expected = format!("Font shape `{encoding}/lmr/bx/sc' undefined, using `{encoding}/lmr/bx/n' instead");
    assert!(
        r.v2.diagnostics.iter().any(|d| d.code == "font_shape_substituted" && d.message == expected),
        "no {expected:?} in {:?}",
        r.v2.diagnostics.iter().map(|d| (d.code.clone(), d.message.clone())).collect::<Vec<_>>()
    );
}

#[test]
fn t1_lmodern_bold_small_caps_falls_back_to_bold_upright() {
    assert!(common::lm_available());
    check("\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n", "T1");
}

#[test]
fn ot1_lmodern_bold_small_caps_falls_back_to_bold_upright() {
    assert!(common::lm_available());
    check("\\usepackage{lmodern}\n", "OT1");
}
