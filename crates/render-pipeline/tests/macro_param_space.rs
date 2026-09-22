//! A blank between two parameter tokens in a macro body is an interword
//! space: `\newcommand{\two}[2]{#1 #2}` then `\two{a}{b}` sets `a b`.
//!
//! `adapter::token_gap` walks a macro's replacement text with a body cursor
//! (`BodyCursor`) so the gap between two argument tokens is read from the
//! definition (`#1 #2` holds a blank) rather than from the call site
//! (`\two{a}{b}` holds `}{` there). The cursor was only seeded when a body
//! *word* was read first; a body of bare parameters never seeded it, so the
//! gap between the two arguments fell back to the call-site bytes and the
//! words glued into one run `ab`.

mod common;

use flashtex_render_pipeline::display::Item;

/// The glyph-run texts on the body page, in order (the folio excluded).
fn runs(text: &str) -> Vec<String> {
    let r = common::render_one(text);
    assert_eq!(r.v2.pages.len(), 1);
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        let Some(g) = run.glyphs.first() else { continue };
        if g.baseline_y.to_bp() > 700.0 {
            continue;
        }
        out.push(run.text.trim().to_string());
    }
    out
}

#[test]
fn space_between_two_parameters_survives_expansion() {
    if !common::lm_available() {
        return;
    }
    let text = "\\documentclass{article}\n\\newcommand{\\two}[2]{#1 #2}\n\\begin{document}\n\\two{a}{b}\n\\end{document}\n";
    assert_eq!(runs(text), ["a", "b"]);
}

/// The same seam mid-paragraph: the first argument's gap comes from the
/// source before the call, the second's from the body's blank.
#[test]
fn space_between_two_parameters_mid_paragraph() {
    if !common::lm_available() {
        return;
    }
    let text = "\\documentclass{article}\n\\newcommand{\\two}[2]{#1 #2}\n\\begin{document}\nFirst \\two{a}{b} last.\n\\end{document}\n";
    assert_eq!(runs(text), ["First", "a", "b", "last."]);
}

#[test]
fn adjacent_parameters_without_a_blank_stay_glued() {
    if !common::lm_available() {
        return;
    }
    let text = "\\documentclass{article}\n\\newcommand{\\two}[2]{#1#2}\n\\begin{document}\n\\two{a}{b}\n\\end{document}\n";
    assert_eq!(runs(text), ["ab"]);
}
