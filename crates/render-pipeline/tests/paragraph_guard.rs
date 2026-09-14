//! A paragraph ending in `\\`. TeX breaks at the forced penalty and then
//! at the paragraph's final `\penalty-10000` with nothing but discardables
//! between them: an empty last line, one `\baselineskip` tall (the familiar
//! "Underfull \hbox (badness 10000)"). The pinned paragraph-layout used to
//! panic on that list (slice index on the empty line) and the pipeline
//! dropped the trailing break with a `paragraph_final_linebreak` warning;
//! the unified crate (main `14c2d00c`) sets the empty line, so the pipeline
//! now hands the break through unchanged and the following block sits one
//! line pitch lower, as in LaTeX. Under `\centering`/`\raggedleft` a final
//! `\\` is `\@centercr`'s `\par` and still adds nothing.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, Tick};
use flashtex_render_pipeline::Rendered;

fn body(text: &str) -> String {
    format!("\\documentclass[12pt]{{article}}\n\\begin{{document}}\n{text}\n\\end{{document}}\n")
}

/// `(text, baseline)` of every glyph run on every page, in page order.
fn runs(r: &Rendered) -> Vec<(String, Tick)> {
    r.v2
        .pages
        .iter()
        .flat_map(|p| p.to_items())
        .filter_map(|i| match i {
            Item::GlyphRun(g) => g.glyphs.first().map(|first| (g.text.clone(), first.baseline_y)),
            _ => None,
        })
        .collect()
}

fn baseline_of(r: &Rendered, prefix: &str) -> Tick {
    runs(r).into_iter().find(|(t, _)| t.starts_with(prefix)).map(|(_, y)| y).unwrap_or_else(|| panic!("no run starting with {prefix:?}: {:?}", runs(r)))
}

#[test]
fn trailing_linebreak_sets_an_empty_line_without_a_warning() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    for text in ["Alpha beta\\\\\n\nNext paragraph.", "Alpha beta\\\\\n\n", "Alpha beta\\\\ \n\nNext.", "Alpha $x$\\\\\n", "Alpha\\\\\\\\\n\nNext."] {
        let r = render_one(&body(text));
        assert!(!r.v2.diagnostics.iter().any(|d| d.code == "paragraph_final_linebreak" || d.code == "paragraph_layout_error"), "{text:?}: {:?}", r.v2.diagnostics);
        let texts: Vec<String> = runs(&r).into_iter().map(|(t, _)| t).collect();
        assert!(texts.iter().any(|t| t == "Alpha"), "{text:?}: {texts:?}");
        if text.contains("beta") {
            assert!(texts.iter().any(|t| t == "beta"), "{text:?}: {texts:?}");
        }
        if text.contains("Next") {
            assert!(texts.iter().any(|t| t.starts_with("Next")), "{text:?}: {texts:?}");
        }
    }
}

#[test]
fn the_empty_last_line_is_one_baselineskip_tall() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // 12pt article: \baselineskip 14.5pt. The empty line has no height or
    // depth, so both the interline glue before it and the one after it are
    // a full \baselineskip: the next paragraph lands exactly one pitch lower.
    let pitch = Tick::from_tex_pt(14.5).0;
    let plain = baseline_of(&render_one(&body("Alpha beta\n\nNext.")), "Next");
    let once = baseline_of(&render_one(&body("Alpha beta\\\\\n\nNext.")), "Next");
    let twice = baseline_of(&render_one(&body("Alpha beta\\\\\\\\\n\nNext.")), "Next");
    assert!((once.0 - plain.0 - pitch).abs() <= 2, "one trailing \\\\: {} vs {} (+{pitch})", once.0, plain.0);
    assert!((twice.0 - plain.0 - 2 * pitch).abs() <= 2, "two trailing \\\\: {} vs {} (+2*{pitch})", twice.0, plain.0);
    // A `\\[<dimen>]` at the end keeps its skip as well as the empty line.
    let skipped = baseline_of(&render_one(&body("Alpha beta\\\\[10pt]\n\nNext.")), "Next");
    assert!((skipped.0 - once.0 - Tick::from_tex_pt(10.0).0).abs() <= 2, "\\\\[10pt]: {} vs {}", skipped.0, once.0);
}

#[test]
fn a_centred_trailing_linebreak_is_just_par() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `\@centercr`: the final `\\` ends the paragraph; no empty line.
    let plain = baseline_of(&render_one(&body("\\begin{center}Alpha beta\\end{center}\n\nNext.")), "Next");
    let broken = baseline_of(&render_one(&body("\\begin{center}Alpha beta\\\\\\end{center}\n\nNext.")), "Next");
    assert_eq!(plain, broken);
}

#[test]
fn a_linebreak_inside_a_paragraph_is_still_typeset() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&body("Alpha\\\\beta\n\nNext."));
    assert!(!r.v2.diagnostics.iter().any(|d| d.code == "paragraph_final_linebreak"), "{:?}", r.v2.diagnostics);
    let texts: Vec<String> = runs(&r).into_iter().map(|(t, _)| t).collect();
    assert!(texts.iter().any(|t| t == "Alpha") && texts.iter().any(|t| t == "beta"), "{texts:?}");
}
