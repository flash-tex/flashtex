//! A single Greek-letter `\tag` must not print as an empty label (GH-805).
//!
//! `strip_tag` takes the math nucleus's rendered string as-is — for
//! `\tag{$\alpha$}` that is U+03B1 ('α'). The tag is then boxed as plain
//! body text (`display_block` → `number_box` → `word_box` → `text_box`,
//! or `rows_block`'s tag loop → `text_box`), and Latin Modern Roman has no
//! Greek-letter glyphs: `text_box_shaped` reported `missing_glyph` and drew
//! nothing, so the tag printed as an empty `()`.
//!
//! pdflatex (TeX Live 2026, 10pt article + lmodern + amsmath, PyMuPDF word
//! extraction) on this file's `ALPHA_TAG` document:
//!
//! ```text
//! '(α)'   x0 463.326  width 14.167  % the \tag itself
//! '(α).'  x0 151.491  width 16.937  % See \eqref{e:alpha}.
//! ```
//!
//! with the α set in LMMathItalic10 (parens in LMRoman10). That is real
//! math-in-tag (#441, open PR #585) and out of scope here: FlashTeX keeps
//! its flatten-to-text approximation, but the fallback face must draw the
//! character instead of dropping it silently. What this file pins is that
//! the tag and the `\eqref` to it both draw `(α)` — the α has a real glyph
//! with nonzero advance — rather than `()`.
//!
//! Position is deliberately not pinned: the fallback face's advances are
//! not pdflatex's math-italic advances, so an x-oracle would be brittle.
#![cfg(feature = "compiler-node-surface")]

mod common;

use common::*;
use flashtex_render_pipeline::display;

const ALPHA_TAG: &str = r"\documentclass[10pt]{article}
\usepackage{lmodern}
\usepackage{amsmath}
\pagestyle{empty}
\begin{document}
Text.
\begin{equation} a = b \tag{$\alpha$} \label{e:alpha} \end{equation}
See \eqref{e:alpha}.
\end{document}
";

/// The drawn glyph runs of the page: `(text, glyph count, width in bp)`.
fn runs_of(r: &flashtex_render_pipeline::Rendered) -> Vec<(String, usize, f64)> {
    let mut runs = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                let last = run.glyphs.last().expect("non-empty");
                runs.push((
                    run.text.clone(),
                    run.glyphs.len(),
                    (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64
                        / f64::from(display::TICKS_PER_BP),
                ));
            }
        }
    }
    runs
}

/// The tag's own display and the `\eqref` reference both draw `(α)`.
/// Before the fallback the run texts still read `(α)`/`(α).` (the text
/// layer keeps the flattened string) but only the parens had glyphs — 2
/// and 3 drawn glyphs with the α at zero advance — plus a `missing_glyph`
/// warning for U+03B1.
#[test]
fn greek_tag_and_eqref_draw_the_alpha() {
    if !lm_available() {
        return;
    }
    let r = render_one(ALPHA_TAG);
    let runs = runs_of(&r);
    let texts: Vec<String> = runs.iter().map(|(t, _, _)| t.clone()).collect();
    let tag = runs.iter().find(|(t, _, _)| t == "(α)").expect("tag run: {texts:?}");
    let eqref = runs.iter().find(|(t, _, _)| t == "(α).").expect("\\eqref run: {texts:?}");
    assert!(tag.1 >= 3, "tag drew {} glyphs, the α is missing: {texts:?}", tag.1);
    assert!(eqref.1 >= 4, "\\eqref drew {} glyphs, the α is missing: {texts:?}", eqref.1);
    // The fallback glyph has a real advance: before the fix the tag
    // measured ~10.5bp of parens alone against pdflatex's 14.2bp.
    assert!(tag.2 > 12.0, "tag is only {:.2}bp wide, the α has no advance: {texts:?}", tag.2);
    assert!(eqref.2 > 14.0, "\\eqref is only {:.2}bp wide: {texts:?}", eqref.2);
    // ... and the two read alike, as pdflatex sets them.
    assert_eq!(
        eqref.0.strip_suffix('.').unwrap_or(&eqref.0),
        tag.0.as_str(),
        "tag display {:?} and \\eqref {:?} differ: {texts:?}",
        tag.0,
        eqref.0
    );
}

/// The dropped glyph is still reported: falling back to another face is an
/// approximation of the tag the source asked for, not the requested face,
/// so a silent substitution would be the same failure mode as the drop.
#[test]
fn greek_tag_reports_where_its_alpha_came_from() {
    if !lm_available() {
        return;
    }
    let r = render_one(ALPHA_TAG);
    let notes: Vec<&str> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "missing_glyph" && d.message.contains("03B1"))
        .map(|d| d.message.as_str())
        .collect();
    assert!(!notes.is_empty(), "no missing_glyph note for the tag's U+03B1: {:?}", r.v2.diagnostics);
    assert!(
        notes.iter().all(|n| n.contains("instead")),
        "the note should say where the glyph was set: {notes:?}"
    );
}
