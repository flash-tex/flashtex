//! A `\tag{..}` whose label is not plain upright text (GH-441).
//!
//! The compiler gives such a label as `math::Nucleus::TextRun` — a run of
//! text and nested math pieces — where a plain `\tag{hi}` is still one
//! `Nucleus::Text`. `adapter::strip_tag` used to read only the flat string,
//! so the rich label was dropped and the display fell through to the
//! **automatic equation number**: `\tag{hi $x^2$}` printed a plausible
//! `(3)`, silently, with no diagnostic. That is worse than the error #441 was filed for — a
//! reader cross-referencing "equation (1)" against a source that says
//! `\tag{hi $x^2$}` has nothing to notice.
//!
//! pdflatex (TeX Live 2026, 10 pt article + lmodern + amsmath, PyMuPDF word
//! extraction) on this file's `THREE_TAGS` document:
//!
//! ```text
//! '(hi)'  x0 461.433      % \tag{hi}
//! '(x2)'  x0 459.573      % \tag{$x^2$}
//! '(hi'   x0 447.959      % \tag{hi $x^2$}
//! 'x2)'   x0 463.461
//! ```
//!
//! No number anywhere: an equation with a `\tag` is never also numbered.
//! FlashTeX before this change set `(2)` and `(3)` in their place (the
//! counter keeps stepping), and said nothing.
//!
//! What this file pins is that the silent substitution is gone, and (since
//! PR #585's placement landed) that the rich label is set as amsmath's
//! `\maketag@@@` box: its text in the body face, its `$x^2$` as math with
//! a real superscript, flush to the right margin where pdflatex puts it --
//! within 0.5 bp, like the plain control -- with nothing to report. (Before
//! that it was set as the flattened text `(hi x²)`, 1.3 bp off, under a
//! `math_limitation` note.)
#![cfg(feature = "compiler-node-surface")]

mod common;

use common::*;
use flashtex_render_pipeline::Rendered;

const THREE_TAGS: &str = r"\documentclass[10pt]{article}
\usepackage[T1]{fontenc}
\usepackage{lmodern}
\usepackage{amsmath}
\pagestyle{empty}
\begin{document}
Text.
\begin{equation} a = b \tag{hi} \end{equation}
Text.
\begin{equation} c = d \tag{$x^2$} \end{equation}
Text.
\begin{equation} e = f \tag{hi $x^2$} \end{equation}
\end{document}
";

fn texts(r: &Rendered) -> Vec<String> {
    words_of(r).into_iter().map(|w| w.text).collect()
}

fn tag_limitations(r: &Rendered) -> Vec<&str> {
    r.v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "math_limitation" && d.message.contains("\\tag"))
        .map(|d| d.message.as_str())
        .collect()
}

/// The regression itself: a mixed text/math label must not turn into the
/// automatic number. Before the fix the page read `(2)`/`(3)` here, with
/// no diagnostic of any kind.
#[test]
fn a_rich_tag_is_never_silently_replaced_by_the_automatic_number() {
    if !lm_available() {
        return;
    }
    let r = render_one(THREE_TAGS);
    let words = texts(&r);
    // Not just `(1)`: the counter keeps stepping over the tagged displays,
    // so the substituted numbers here were `(2)` and `(3)` — no `\tag`ged
    // display may carry an automatic number at all.
    let numbers: Vec<&String> = words
        .iter()
        .filter(|w| w.starts_with('(') && w.ends_with(')') && w[1..w.len() - 1].chars().all(|c| c.is_ascii_digit()) && w.len() > 2)
        .collect();
    assert!(
        numbers.is_empty(),
        "automatic equation numbers {numbers:?} were set for \\tag'ged displays: {words:?}"
    );
    // The label's own text reaches the page, and its math is math: `x` in
    // math italic, `2` a raised script (its own run), not the Unicode `²`.
    assert!(words.iter().any(|w| w == "(hi"), "\\tag{{hi $x^2$}} lost its text: {words:?}");
    assert!(words.iter().filter(|w| *w == "x").count() >= 2, "\\tag{{$x^2$}} / \\tag{{hi $x^2$}} lost their math: {words:?}");
    assert!(!words.iter().any(|w| w.contains('\u{b2}')), "a tag's superscript was flattened to a text character: {words:?}");
}

/// Set exactly: every tag word where pdflatex puts it (the header's PyMuPDF
/// origins, within 0.5 bp), with the superscript `2` on its own raised
/// baseline. `(x2)` is `(` + `x` + `2` + `)` here because the math box
/// paints each glyph as its own run.
#[test]
fn a_rich_tag_is_set_where_pdflatex_sets_it() {
    if !lm_available() {
        return;
    }
    let r = render_one(THREE_TAGS);
    let words = words_of(&r);
    let at = |text: &str, x: f64| {
        words
            .iter()
            .find(|w| w.text == text && (w.x - x).abs() <= 0.5)
            .unwrap_or_else(|| panic!("{text:?} not within 0.5 bp of x={x}: {words:?}"))
    };
    // `\tag{$x^2$}`: pdflatex `(` at 459.564, `x` at 463.438, `2` at 469.136.
    let open = at("(", 459.564);
    let x = at("x", 463.438);
    let two = at("2", 469.136);
    assert!((x.baseline - open.baseline).abs() < 0.01, "x off the tag's baseline");
    assert!((open.baseline - two.baseline - 3.616).abs() <= 0.5, "superscript not raised (pdflatex: 3.616 bp)");
    // `\tag{hi $x^2$}`: pdflatex `(hi` at 447.949, `x` at 463.444.
    at("(hi", 447.949);
    at("x", 463.444);
}

/// Whatever is not set exactly is *said* -- and a rich tag now is set
/// exactly, so there is nothing to say: the `math_limitation` note the
/// approximation carried is gone, and no other tag note took its place.
#[test]
fn a_rich_tag_reports_no_limitation() {
    if !lm_available() {
        return;
    }
    let r = render_one(THREE_TAGS);
    let notes = tag_limitations(&r);
    assert!(notes.is_empty(), "rich tags are set as math; nothing to report: {notes:?}");
}

/// Control: a plain `\tag{hi}` is untouched by all of this — set exactly
/// where pdflatex sets it, and with nothing to report.
#[test]
fn a_plain_tag_is_unchanged_and_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    let r = render_one(THREE_TAGS);
    let hi = words_of(&r).into_iter().find(|w| w.text == "(hi)").expect("\\tag{hi} sets (hi)");
    assert!((hi.x - 461.433).abs() <= 0.5, "\\tag{{hi}} at {:.3} bp, pdflatex 461.433", hi.x);
    let notes = tag_limitations(&r);
    assert!(!notes.iter().any(|n| n.contains("(hi)")), "a plain tag reported a limitation: {notes:?}");
}

/// Control: the automatic number still works where there is no `\tag`, so
/// the test above is not passing because numbering broke.
#[test]
fn an_untagged_equation_still_gets_its_number() {
    if !lm_available() {
        return;
    }
    let r = render_one(
        "\\documentclass[10pt]{article}\n\\usepackage{amsmath}\n\\begin{document}\nText.\n\\begin{equation} a = b \\end{equation}\n\\end{document}\n",
    );
    let words = texts(&r);
    assert!(words.iter().any(|w| w == "(1)"), "the automatic number is gone: {words:?}");
    assert!(tag_limitations(&r).is_empty());
}
