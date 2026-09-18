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
