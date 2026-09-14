//! A formula opening with `\displaystyle` is laid out in display style.
//!
//! `\displaystyle` is a style switch, not a command with an argument: it
//! changes the style for the rest of the enclosing group (TeX §1171). The
//! pinned compiler models none of that — `\displaystyle`, `\textstyle`,
//! `\scriptstyle` and `\scriptscriptstyle` are all one zero-width
//! `Nucleus::Space` atom (`crates/compiler/src/math.rs`, the same arm as
//! `\nonumber`), which the pipeline then skipped. So
//! `$\displaystyle\sum_{n=1}^{\infty}x_n$` was set in *text* style, with the
//! limits beside the operator as scripts.
//!
//! That is a vertical defect as much as a horizontal one. A text-style sum is
//! about 8 pt shorter than a display-style one, and a short box never trips
//! TeX's interline rule (§679: interline glue is `\baselineskip - \prevdepth
//! - height`, or `\lineskip` when that falls below `\lineskiplimit`), so the
//! following baseline stayed a plain `\baselineskip` away — and so did every
//! later line on the page. It is the largest single cause of the vertical
//! drift down `fixtures/real-world/ps-calculus`, whose five `\displaystyle`
//! formulas took page 1's median `dy` to -20.83 bp.
//!
//! Expected numbers are pdflatex's (TeX Live 2025, 11 pt `article`,
//! `margin=1in`), read from its PDF's content stream: with `\displaystyle`
//! the `\infty` sits 13.637 bp above the text baseline and the `n=1` 12.954
//! below it; without it, 5.027 above and 3.225 below.

mod common;

use common::*;

const DOC: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{amsmath}
\pagestyle{empty}
\parindent 0pt
\begin{document}
Tall %%SWITCH%%\sum_{n=1}^{\infty} x_n$ formula.
\end{document}
";

const WRAPPED: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{amsmath}
\pagestyle{empty}
\parindent 0pt
\begin{document}
Tall $\displaystyle\sum_{n=1}^{\infty} x_n$ formula and then enough ordinary
words to force this paragraph onto a second line here now.
\end{document}
";

/// (how far the `\infty` of the upper limit sits above the formula line's
/// text baseline, how far the lowest run sits below it), in bp.
///
/// The `\sum` glyph itself is not measured: cmex10's `summationtext` and
/// `summationdisplay` carry their ink on a raised baseline of their own,
/// which the OpenType face this paints from does not, so the two sides
/// disagree about that glyph's *origin* while drawing the same ink.
fn limit_offsets(source: &str) -> (f64, f64) {
    let r = render_one(source);
    let words = words_of(&r);
    let names = || words.iter().map(|w| w.text.clone()).collect::<Vec<_>>();
    let base = words
        .iter()
        .find(|w| w.text.starts_with("Tall"))
        .unwrap_or_else(|| panic!("no run starting `Tall` in {:?}", names()))
        .baseline;
    let sup = words
        .iter()
        .find(|w| w.text.contains('\u{221e}'))
        .unwrap_or_else(|| panic!("no run holding the upper limit in {:?}", names()))
        .baseline;
    let sub = words.iter().map(|w| w.baseline).fold(f64::NEG_INFINITY, f64::max);
    (base - sup, sub - base)
}

#[test]
fn a_leading_displaystyle_stacks_the_operator_limits() {
    if !lm_available() {
        return;
    }
    let (rise, drop) = limit_offsets(&DOC.replace("%%SWITCH%%", r"$\displaystyle"));
    assert!(
        (rise - 13.637).abs() < 0.05 && (drop - 12.954).abs() < 0.05,
        "`\\displaystyle\\sum`: limits {rise:.3} above / {drop:.3} below the baseline, pdflatex has 13.637 / 12.954"
    );
}

#[test]
fn without_the_switch_the_same_sum_keeps_text_style_scripts() {
    if !lm_available() {
        return;
    }
    let (rise, drop) = limit_offsets(&DOC.replace("%%SWITCH%%", "$"));
    assert!(
        (rise - 5.027).abs() < 0.05 && (drop - 3.225).abs() < 0.05,
        "plain `\\sum`: scripts {rise:.3} above / {drop:.3} below the baseline, pdflatex has 5.027 / 3.225"
    );
}

/// The interline consequence: the display-style box is deep enough that TeX
/// clamps the glue to `\lineskip`, so the next line of the same paragraph is
/// much further down than one `\baselineskip` (13.549 bp at 11 pt). pdflatex
/// puts it 19.738 bp down and this puts it 22.555 — the remaining 2.8 bp is a
/// separate defect (the lower limit's box runs deeper than pdfTeX's), so the
/// assertion is on the clamp firing at all, which is what a discarded
/// `\displaystyle` prevented.
#[test]
fn the_tall_formula_pushes_the_next_line_past_baselineskip() {
    if !lm_available() {
        return;
    }
    let r = render_one(WRAPPED);
    let words = words_of(&r);
    let first = words.iter().find(|w| w.text.starts_with("Tall")).expect("first line").baseline;
    let second = words
        .iter()
        .map(|w| w.baseline)
        .filter(|b| *b > first + 16.0)
        .fold(f64::INFINITY, f64::min);
    assert!(
        second.is_finite() && second - first > 18.0,
        "next baseline is {:.3} bp below the tall formula's line; one \\baselineskip is 13.549 and pdflatex has 19.738",
        second - first
    );
}
