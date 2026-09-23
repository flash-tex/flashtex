//! amsthm's automatic QED mark is `\openbox`, four rules — not a character.
//!
//! `\end{proof}` appends `\qedsymbol` after an `\hfill`, and amsthm.sty
//! defines that as
//!
//! ```text
//! \def\openbox{\leavevmode
//!   \hbox to.77778em{\hfil\vrule
//!                    \vbox to.675em{\hrule width.6em\vfil\hrule}%
//!                    \vrule\hfil}}
//! ```
//!
//! an *open* square `.6em` wide and `.675em` tall drawn with `\vrule`/`\hrule`
//! at TeX's default 0.4 pt thickness. The compiler emits U+220E (END OF PROOF,
//! a *filled* square) for it, which Latin Modern has no glyph for at all, so
//! before this the marker warned `missing_glyph` and drew nothing (GH#443).
//!
//! # Oracle
//!
//! pdflatex, TeX Live 2026, `/Library/TeX/texbin/pdflatex`, on
//!
//! ```text
//! \documentclass{article}
//! \usepackage{amsthm}
//! \begin{document}
//! \begin{proof}Body.\end{proof}
//! \end{document}
//! ```
//!
//! The page's content stream holds no glyph for the mark — it is four
//! stroked segments of width 0.398 (the whole point of this test), which as
//! filled rectangles are the [`ORACLE`] rows below:
//!
//! ```text
//! q 1 0 0 1 470.417 657.235 cm []0 d 0 J 0.398 w 0 0 m 0 6.725 l S Q
//! q 1 0 0 1 470.616 663.761 cm []0 d 0 J 0.398 w 0 0 m 5.978 0 l S Q
//! q 1 0 0 1 470.616 657.435 cm []0 d 0 J 0.398 w 0 0 m 5.978 0 l S Q
//! q 1 0 0 1 476.793 657.235 cm []0 d 0 J 0.398 w 0 0 m 0 6.725 l S Q
//! ```
//!
//! with the text baseline at y 657.235 from the page bottom (`Proof.` starts
//! at x 133.768) and the page 792 bp tall, so y-down page coordinates are
//! `792 - y`. `\fontdimen6` of cmr10 is 10.00002pt, which is the `em` the
//! `.6`/`.675`/`.77778` are of — confirmed by the `\large` run in the same
//! oracle, where the box grows with cmr12's quad of 11.74988pt (width 7.024,
//! height 7.902) rather than with the nominal 12 pt.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

/// Diagnostics other than the pinned compiler's stale `\qedhere` reports
/// and the expected lmmi/lmex outline-resource profile notes (see
/// `v2_and_math.rs`: any math render carries those): the vendored compiler
/// predates `\qedhere` recognition (`crates/compiler` implements it), so it
/// still warns `unknown_command` while the pipeline places the box. A
/// re-pin drops those lines; every other diagnostic is unexpected.
fn unexpected_diagnostics(r: &flashtex_render_pipeline::Rendered) -> Vec<String> {
    r.v2
        .diagnostics
        .iter()
        .filter(|d| !d.message.contains("\\qedhere") && d.code != "math_resource_profile")
        .map(|d| format!("[{}] {}", d.code, d.message))
        .collect()
}

/// The lowest edge of the four box rules: amsthm's `\openbox` sits on the
/// baseline of the line it ends.
fn box_bottom(rules: &[(f64, f64, f64, f64)]) -> f64 {
    rules.iter().map(|r| r.1 + r.3).fold(f64::MIN, f64::max)
}

/// The box's right edge: flush against the text width wherever it is set.
fn box_right(rules: &[(f64, f64, f64, f64)]) -> f64 {
    rules.iter().map(|r| r.0 + r.2).fold(f64::MIN, f64::max)
}

/// The oracle's right edge: the text box is flush right, so a `\qedhere`
/// box on any line ends at the same x.
fn oracle_right() -> f64 {
    ORACLE.iter().map(|r| r.0 + r.2).fold(f64::MIN, f64::max)
}

/// The four rules of the mark, in y-down page bp, as
/// `(left, top, width, height)`, read off the oracle's stroked segments: a
/// stroke of width `w` down a line is a rectangle reaching `w/2` either side
/// of it.
const ORACLE: [(f64, f64, f64, f64); 4] = [
    // \vrule, left: x 470.417 ± 0.199, y 657.235 .. 663.960
    (470.218, 128.040, 0.398, 6.725),
    // \hrule, top: y 663.761 ± 0.199, x 470.616 .. 476.594
    (470.616, 128.040, 5.978, 0.398),
    // \hrule, bottom: y 657.435 ± 0.199
    (470.616, 134.366, 5.978, 0.398),
    // \vrule, right: x 476.793 ± 0.199
    (476.594, 128.040, 0.398, 6.725),
];

/// The oracle's text baseline, y-down (792 - 657.235).
const BASELINE: f64 = 134.765;

const SRC: &str = r"\documentclass{article}
\usepackage{amsthm}
\begin{document}
\begin{proof}Body.\end{proof}
\end{document}
";

/// Every rule on page 1, as `(left, top, width, height)` in bp, in reading
/// order (top to bottom, then left to right).
fn rules_of(r: &flashtex_render_pipeline::Rendered) -> Vec<(f64, f64, f64, f64)> {
    let mut out: Vec<(f64, f64, f64, f64)> = r.v2.pages[0]
        .resident_items()
        .into_iter()
        .filter_map(|i| match i {
            Item::Rule(rule) => Some((rule.x.to_bp(), rule.top.to_bp(), rule.width.to_bp(), rule.height.to_bp())),
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a display list"));
    out
}

#[test]
fn proof_ends_with_amsthms_open_box_where_pdflatex_draws_it() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SRC);

    // The mark is drawn at all, and it is not a glyph: nothing in the page's
    // text carries U+220E, and no `missing_glyph` is reported for it.
    let words = words_of(&r);
    assert!(
        words.iter().all(|w| !w.text.contains('\u{220E}')),
        "the QED mark must not be set as U+220E, which Latin Modern has no glyph for: {words:?}"
    );
    let missing: Vec<_> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "missing_glyph")
        .map(|d| d.message.clone())
        .collect();
    assert!(missing.is_empty(), "a proof must not report a missing glyph, got {missing:?}");

    let rules = rules_of(&r);
    assert_eq!(rules.len(), 4, "amsthm's \\openbox is four rules, got {rules:?}");

    // Rules: 0.1 bp. The mark sits on the proof's last text line, so its
    // vertical placement is checked against that baseline too.
    for (got, want) in rules.iter().zip(ORACLE.iter()) {
        let d = (got.0 - want.0, got.1 - want.1, got.2 - want.2, got.3 - want.3);
        assert!(
            d.0.abs() < 0.1 && d.1.abs() < 0.1 && d.2.abs() < 0.1 && d.3.abs() < 0.1,
            "rule at {got:?} bp, pdflatex draws it at {want:?} bp (delta {d:?})"
        );
    }

    // The box sits on the last line's baseline: its bottom rule's bottom edge
    // is the baseline, and the two `\vrule` reach .675em above it.
    let body = words
        .iter()
        .find(|w| w.text.contains("Body"))
        .unwrap_or_else(|| panic!("no body run in {words:?}"));
    assert!(
        (body.baseline - BASELINE).abs() < 0.5,
        "the proof's last line is at baseline {} bp, pdflatex sets it at {BASELINE} bp",
        body.baseline
    );
    let bottom = rules.iter().map(|r| r.1 + r.3).fold(f64::MIN, f64::max);
    assert!(
        (bottom - BASELINE).abs() < 0.1,
        "the \\openbox sits on the baseline ({BASELINE} bp); its lowest edge is at {bottom} bp"
    );
}

/// `em` is the current font's quad (`\fontdimen6`), not the nominal type
/// size, so the mark grows with the class option.
///
/// The same oracle on `\documentclass[12pt]{article}` sets the body in cmr12,
/// whose quad is 11.74988pt, and draws
///
/// ```text
/// q 1 0 0 1 491.134 654.247 cm []0 d 0 J 0.398 w 0 0 m 0 7.902 l S Q
/// q 1 0 0 1 491.333 661.949 cm []0 d 0 J 0.398 w 0 0 m 7.024 0 l S Q
/// q 1 0 0 1 491.333 654.446 cm []0 d 0 J 0.398 w 0 0 m 7.024 0 l S Q
/// q 1 0 0 1 498.556 654.247 cm []0 d 0 J 0.398 w 0 0 m 0 7.902 l S Q
/// ```
///
/// with the baseline at 654.247 from the page bottom. Measuring the `.6em`
/// and `.675em` against the nominal 12 pt instead of the quad would give
/// 7.176 and 8.070 bp, 0.15 bp and 0.17 bp out -- past what a rule is
/// allowed.
#[test]
fn the_open_box_is_measured_in_the_current_fonts_quad() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(
        r"\documentclass[12pt]{article}
\usepackage{amsthm}
\begin{document}
\begin{proof}Body.\end{proof}
\end{document}
",
    );
    let rules = rules_of(&r);
    assert_eq!(rules.len(), 4, "amsthm's \\openbox is four rules, got {rules:?}");

    const ORACLE_12PT: [(f64, f64, f64, f64); 4] = [
        (490.935, 129.851, 0.398, 7.902),
        (491.333, 129.851, 7.024, 0.398),
        (491.333, 137.355, 7.024, 0.398),
        (498.357, 129.851, 0.398, 7.902),
    ];
    for (got, want) in rules.iter().zip(ORACLE_12PT.iter()) {
        let d = (got.0 - want.0, got.1 - want.1, got.2 - want.2, got.3 - want.3);
        assert!(
            d.0.abs() < 0.1 && d.1.abs() < 0.1 && d.2.abs() < 0.1 && d.3.abs() < 0.1,
            "rule at {got:?} bp, pdflatex draws it at {want:?} bp (delta {d:?})"
        );
    }
}

/// Renders a proof source and returns its box rules and word boxes, after
/// asserting the page carries no literal `\qedhere`, no missing glyph, no
/// unexpected diagnostic — and exactly one box, not a placed one plus the
/// automatic one.
fn qedhere_render(src: &str) -> (Vec<(f64, f64, f64, f64)>, Vec<Word>) {
    let r = render_one(src);
    let words = words_of(&r);
    assert!(
        words.iter().all(|w| !w.text.contains("qedhere")),
        "no literal \\qedhere reaches the page: {words:?}"
    );
    let missing: Vec<_> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "missing_glyph")
        .collect();
    assert!(missing.is_empty(), "no missing glyph, got {missing:?}");
    let unexpected = unexpected_diagnostics(&r);
    assert!(unexpected.is_empty(), "unexpected diagnostics: {unexpected:?}");
    let rules = rules_of(&r);
    assert_eq!(rules.len(), 4, "one \\openbox is four rules, got {rules:?}");
    (rules, words)
}

/// `\qedhere` in ordinary text puts the box on that line, flush right,
/// and the automatic end-of-proof box no longer also appears: exactly one
/// box, sitting where the automatic one would.
#[test]
fn qedhere_in_text_places_the_box_on_that_line_and_suppresses_the_automatic_one() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (rules, _words) = qedhere_render(
        r"\documentclass{article}
\usepackage{amsthm}
\begin{document}
\begin{proof}Body text \qedhere\end{proof}
\end{document}
",
    );
    for (got, want) in rules.iter().zip(ORACLE.iter()) {
        let d = (got.0 - want.0, got.1 - want.1, got.2 - want.2, got.3 - want.3);
        assert!(
            d.0.abs() < 0.1 && d.1.abs() < 0.1 && d.2.abs() < 0.1 && d.3.abs() < 0.1,
            "rule at {got:?} bp, the end-of-proof box sits at {want:?} bp (delta {d:?})"
        );
    }
}

/// `\qedhere` as the last thing of a display puts the box on the display's
/// own line, flush right within the display width, instead of leaving the
/// automatic box on a line of its own after the display.
#[test]
fn qedhere_in_a_display_places_the_box_on_the_displays_own_line() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(
        r"\documentclass{article}
\usepackage{amsthm}
\begin{document}
\begin{proof}Consider \[x^2 + y^2 = z^2 \qedhere\]\end{proof}
\end{document}
",
    );
    let words = words_of(&r);
    assert!(
        words.iter().all(|w| !w.text.contains("qedhere")),
        "no literal \\qedhere reaches the page: {words:?}"
    );
    let missing: Vec<_> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "missing_glyph")
        .collect();
    assert!(missing.is_empty(), "no missing glyph, got {missing:?}");
    let unexpected = unexpected_diagnostics(&r);
    assert!(unexpected.is_empty(), "unexpected diagnostics: {unexpected:?}");

    // One box, not a placed one plus the automatic one after the display.
    let rules = rules_of(&r);
    assert_eq!(rules.len(), 4, "one \\openbox is four rules, got {rules:?}");

    // On the display's own line: its lowest edge is the display baseline,
    // below which no second box follows.
    let text_baseline = words
        .iter()
        .find(|w| w.text.contains("Consider"))
        .map(|w| w.baseline)
        .expect("proof text line in {words:?}");
    let display_baseline = words
        .iter()
        .filter(|w| w.page == 1 && w.baseline > text_baseline + 1.0 && w.baseline < text_baseline + 100.0)
        .map(|w| w.baseline)
        .fold(f64::MIN, f64::max);
    assert!(
        (box_bottom(&rules) - display_baseline).abs() < 0.1,
        "the box sits on the display baseline ({} bp); its lowest edge is at {} bp",
        display_baseline,
        box_bottom(&rules)
    );
    // Flush right within the display width, right of the equation itself.
    assert!(
        (box_right(&rules) - oracle_right()).abs() < 0.1,
        "the box ends flush right at {} bp, the text box at {} bp",
        box_right(&rules),
        oracle_right()
    );
    let formula_right = words
        .iter()
        .filter(|w| w.page == 1 && w.baseline > text_baseline + 1.0 && w.baseline < text_baseline + 100.0)
        .map(|w| w.x + w.width)
        .fold(f64::MIN, f64::max);
    let box_left = rules.iter().map(|r| r.0).fold(f64::MAX, f64::min);
    assert!(
        formula_right < box_left,
        "the box starts at {box_left} bp, right of the equation ending at {formula_right} bp"
    );
}

/// `\qedhere` in the last equation of an `align` puts the box on that
/// row's own line, flush right, with no automatic box after the display.
#[test]
fn qedhere_in_the_last_align_row_places_the_box_on_that_row() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(
        r"\documentclass{article}
\usepackage{amsthm}
\usepackage{amsmath}
\begin{document}
\begin{proof}We have
\begin{align*}
a &= b \\
c &= d \qedhere
\end{align*}
\end{proof}
\end{document}
",
    );
    let words = words_of(&r);
    assert!(
        words.iter().all(|w| !w.text.contains("qedhere")),
        "no literal \\qedhere reaches the page: {words:?}"
    );
    let missing: Vec<_> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "missing_glyph")
        .collect();
    assert!(missing.is_empty(), "no missing glyph, got {missing:?}");
    let unexpected = unexpected_diagnostics(&r);
    assert!(unexpected.is_empty(), "unexpected diagnostics: {unexpected:?}");

    let rules = rules_of(&r);
    assert_eq!(rules.len(), 4, "one \\openbox is four rules, got {rules:?}");

    // On the last row's own line: the lowest box edge is a display-row
    // baseline, below the proof's text line.
    let text_baseline = words
        .iter()
        .find(|w| w.text.contains("We"))
        .map(|w| w.baseline)
        .expect("proof text line in {words:?}");
    let last_row_baseline = words
        .iter()
        .filter(|w| w.page == 1 && w.baseline > text_baseline + 1.0 && w.baseline < text_baseline + 100.0)
        .map(|w| w.baseline)
        .fold(f64::MIN, f64::max);
    assert!(
        (box_bottom(&rules) - last_row_baseline).abs() < 0.1,
        "the box sits on the last row's baseline ({} bp); its lowest edge is at {} bp",
        last_row_baseline,
        box_bottom(&rules)
    );
    assert!(
        (box_right(&rules) - oracle_right()).abs() < 0.1,
        "the box ends flush right at {} bp, the text box at {} bp",
        box_right(&rules),
        oracle_right()
    );
}
