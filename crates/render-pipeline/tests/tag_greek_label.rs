//! A single Greek-letter `\tag` must not print as an empty label (GH-805).
//!
//! `strip_tag` used to take the math nucleus's rendered string as-is -- for
//! `\tag{$\alpha$}` that is U+03B1 ('α') -- and box it as plain body text,
//! where Latin Modern Roman has no Greek-letter glyphs: the tag printed as
//! an empty `()`. GH-805 made the text face fall back to the math face for
//! such a character, under a `missing_glyph` note; #441's pipeline half
//! (PR #585) now sets the tag as amsmath's `\maketag@@@` box, so the α is
//! a math-italic glyph in the first place and there is nothing to fall
//! back from or to report.
//!
//! pdflatex (TeX Live 2026, 10pt article + lmodern + amsmath, PyMuPDF word
//! extraction) on this file's `ALPHA_TAG` document:
//!
//! ```text
//! '(α)'   x0 463.322  % the \tag itself
//! '(α).'  x0 151.478  % See \eqref{e:alpha}.
//! ```
//!
//! with the α set in LMMathItalic10 (parens in LMRoman10). What this file
//! pins is that the tag and the `\eqref` to it both draw `(α)` -- the α a
//! glyph with a real advance -- where pdflatex puts them, and that no
//! `missing_glyph` note is raised for it.
#![cfg(feature = "compiler-node-surface")]

mod common;

use common::*;

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

/// The tag's own display and the `\eqref` reference both draw `(α)`: the
/// math box paints `(`, `α`, `)` as three runs, each glyph with a real
/// advance, at pdflatex's origins (`(` 463.322, `α` 467.195, `)` 473.605
/// for the tag; 151.478, 155.351, 161.761 for the reference; 0.5 bp).
/// Before GH-805 only the parens had glyphs (the α at zero advance);
/// before #585 the α came from a fallback face at a non-pdflatex advance.
#[test]
fn greek_tag_and_eqref_draw_the_alpha() {
    if !lm_available() {
        return;
    }
    let r = render_one(ALPHA_TAG);
    let words = words_of(&r);
    let at = |text: &str, x: f64| {
        words
            .iter()
            .find(|w| w.text == text && (w.x - x).abs() <= 0.5)
            .unwrap_or_else(|| panic!("{text:?} not within 0.5 bp of x={x}: {words:?}"))
    };
    for (open, alpha, close) in [(463.322, 467.195, 473.605), (151.478, 155.351, 161.761)] {
        let a = at("α", alpha);
        assert!(a.width > 5.0, "the α has no advance: {words:?}");
        let (o, c) = (at("(", open), at(")", close));
        assert!((o.baseline - a.baseline).abs() < 0.01 && (c.baseline - a.baseline).abs() < 0.01, "α off the parens' baseline");
    }
}

/// The α is math set as math, not a text character rescued from a face
/// that lacks it: no `missing_glyph` note names U+03B1.
#[test]
fn greek_tag_raises_no_missing_glyph() {
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
    assert!(notes.is_empty(), "the tag's α is math; nothing to fall back from: {notes:?}");
}
