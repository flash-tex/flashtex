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
//! What this file pins is that the silent substitution is gone. The rich
//! label is set as its text (the same flattening the compiler uses for
//! `\eqref` to the tag) and the approximation is reported. It is *not* yet
//! amsmath's `\maketag@@@` box, so its position is ~1.3 bp from pdflatex's,
//! against 0.005 bp for the plain control below; PR #585 sets it exactly.
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
    // The label's own text reaches the page.
    assert!(words.iter().any(|w| w == "(hi"), "\\tag{{hi $x^2$}} lost its text: {words:?}");
    assert!(words.iter().any(|w| w.starts_with('x') && w.ends_with(')')), "\\tag{{hi $x^2$}} lost its math: {words:?}");
    assert!(words.iter().any(|w| w == "(x\u{b2})"), "\\tag{{$x^2$}} lost its label: {words:?}");
}

/// Whatever is not set exactly is *said*. A silent approximation is the
/// failure mode this test exists for, so the diagnostic is as much of the
/// contract as the glyphs are.
#[test]
fn a_rich_tag_reports_that_its_math_is_set_as_text() {
    if !lm_available() {
        return;
    }
    let r = render_one(THREE_TAGS);
    let notes = tag_limitations(&r);
    assert_eq!(notes.len(), 2, "one note per rich tag, none for the plain one: {notes:?}");
    for note in &notes {
        assert!(note.contains("#441"), "{note:?}");
    }
    assert!(notes.iter().any(|n| n.contains("(hi x\u{b2})")), "{notes:?}");
    assert!(notes.iter().any(|n| n.contains("(x\u{b2})")), "{notes:?}");
    // The note is anchored at the `\tag`, not at the whole display.
    let at = r
        .v2
        .diagnostics
        .iter()
        .find(|d| d.code == "math_limitation" && d.message.contains("(hi x\u{b2})"))
        .and_then(|d| d.sources.first())
        .map(|s| THREE_TAGS[s.start_byte..].starts_with("\\tag{hi $x^2$}"));
    assert_eq!(at, Some(true), "the note is not at its own \\tag");
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
