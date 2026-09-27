//! GH-949: a blank in a macro *body* between two parameters is an interword
//! space. `\newcommand{\two}[2]{#1 #2}` then `(\two{a}{b})` must set
//! `(a b)`, not `(ab)`: the gap between the two argument tokens lives in the
//! definition (`#1 #2`), not at the call site (`}{`).
//!
//! The byte-scanning `BodyCursor` path that dropped it was replaced by the
//! compiler's node-based `glue_before` (3272ea6b); these pin the behaviour.
//!
//! ## Oracle
//!
//! Every number is a word origin (first glyph x and baseline, bp) read by
//! `tools/visual-oracle/pdftext.py` from pdfLaTeX's output for the same
//! document: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, MacTeX).
//! pdflatex is an oracle only, never in the product path.

mod common;

use flashtex_render_pipeline::display::Item;

/// The corpus harness's exact-route tolerance for a word origin.
const TOL: f64 = 0.01;

const DOC: &str = "\\documentclass{article}\n\
\\newcommand{\\two}[2]{#1 #2}\\newcommand{\\tg}[2]{#1#2}\n\
\\begin{document}\n\
(\\two{a}{b})\n\n\
\\two{a}{b}\n\n\
First \\two{a}{b} last.\n\n\
\\tg{a}{b}\n\
\\end{document}\n";

/// `(text, x)` of every word on the given baseline, in order. Glyph runs
/// are split at interword glue, so each run is one word.
fn words_on(runs: &[(String, f64, f64)], baseline: f64) -> Vec<(String, f64)> {
    runs.iter()
        .filter(|(_, _, y)| (y - baseline).abs() <= 0.05)
        .map(|(text, x, _)| (text.clone(), *x))
        .collect()
}

fn check(got: &[(String, f64)], want: &[(&str, f64)]) {
    assert_eq!(
        got.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(),
        want.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
        "{got:?}"
    );
    for ((text, x), (_, want_x)) in got.iter().zip(want) {
        assert!((x - want_x).abs() <= TOL, "{text}: x {x} vs pdflatex {want_x} ({got:?})");
    }
}

#[test]
fn a_body_blank_between_two_parameters_is_an_interword_space() {
    if !common::lm_available() {
        return;
    }
    let r = common::render_one(DOC);
    assert_eq!(r.v2.pages.len(), 1);
    let mut runs = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        let Some(g) = run.glyphs.first() else { continue };
        runs.push((run.text.trim().to_string(), g.origin_x.to_bp(), g.baseline_y.to_bp()));
    }
    // pdflatex: `(a` 148.712, `b)` 160.885 on baseline 134.765.
    check(&words_on(&runs, 134.765), &[("(a", 148.712), ("b)", 160.885)]);
    // Paragraph-initial call: `a` 148.712, `b` 157.011.
    check(&words_on(&runs, 146.720), &[("a", 148.712), ("b", 157.011)]);
    // Mid-paragraph call.
    check(
        &words_on(&runs, 158.675),
        &[("First", 148.712), ("a", 173.007), ("b", 181.316), ("last.", 190.168)],
    );
    // No blank in the body: the arguments stay glued.
    check(&words_on(&runs, 170.630), &[("ab", 148.712)]);
}
