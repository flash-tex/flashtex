//! `\ldots` and `\cdots` in math are `\mathinner` of three punctuation atoms.
//!
//! ```text
//! \DeclareRobustCommand\mathellipsis{\mathinner{\ldotp\ldotp\ldotp}}
//! \DeclareRobustCommand\cdots{\mathinner{\cdotp\cdotp\cdotp}}
//! ```
//!
//! which is two facts about spacing, neither of which an ordinary atom has:
//! Punct against Punct is a thin space, so the dots sit 3mu apart, and the
//! Inner class takes a thin space against the Ord on each side. The pinned
//! compiler flattens both to three bare characters -- `Nucleus::Text("...")`
//! for the low dots, `Nucleus::Symbol("⋅⋅⋅")` for the centred ones -- so the
//! pipeline rebuilds the atoms from the control word at the span
//! (`typeset::math_ellipsis_of`), the same idiom as `\left`/`\right`.
//!
//! The *glyphs* were already right: `flashtex_math_layout::cm::symbol_slot`
//! maps `.` to math italic `"3A` (`\ldotp`) and `U+22C5` to cmsy `"01`
//! (`\cdotp`) exactly as plain TeX's `\mathcode`s do. What was wrong was that
//! the low dots never reached that table at all -- `Nucleus::Text` goes to the
//! *text*-font run path, so they were set in `ec-lmr12` -- and that neither
//! form had any of the four thin spaces. Those four are essentially the whole
//! error: the two periods differ by under 0.0001 bp in width.
//!
//! **Oracle**, TeX Live 2025 pdflatex under the harness preamble, `\showbox`
//! and `\showthe\wd`. pdflatex is an oracle and is never in the product path.
//!
//! ```text
//! \wd of $\ldotp$   3.26385pt     \wd of $\cdotp$   3.33334pt
//! \wd of $ab$      11.16374pt     \wd of $a.b$     14.4276pt
//! \wd of $a\ldots b$  28.95518pt  \wd of $a\cdots b$  29.16365pt
//! ```
//!
//! and the `$a\ldots b$` box is
//! `a | thin 1.99997 | hbox 13.7915 (: thin : thin :) | thin 1.99997 | b`.
//!
//! ## What this does not model
//!
//! amsmath **redefines** `\cdots`, `\dotsb`, `\dotsm`, `\dotsc` and `\dotso`
//! through `\extrap@` (amsmath.sty 612-628) to append a 3mu `\,` when the
//! next token is `,`, `;`, `.` or a `\DOTSB` operator. Measured:
//! `$a\cdots ,b$` is 36.42744 pt against `$a,b$` 16.42757, i.e. 19.99987 for
//! the dots, where `$a\cdots b$` against `$ab$` gives 17.99991 -- one extra
//! thin space. Neither the pinned compiler nor this change looks at the
//! following token, so that `\,` is still missing. The kernel's `\ldots` is
//! *not* redefined and is unaffected: `$a\ldots ,b$` minus `$a,b$` is
//! 17.79144, the same as `$a\ldots b$` minus `$ab$`.
//!
//! `\vdots` and `\ddots` are also left alone: they are `\vbox` constructions
//! over *text*-font periods, not runs of math dots, and math-layout has no
//! atom that builds a vbox. `vdots_and_ddots_are_left_alone` pins that.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

/// 1 bp = 1.00375 TeX pt.
fn bp(pt: f64) -> f64 {
    pt / 1.00375
}

/// `\thinmuskip` at 12 pt (3mu of an 18mu quad), pt.
const THIN: f64 = 1.99997;
/// `\wd` of `$\ldotp$` at 12 pt: math italic `"3A`.
const LDOTP: f64 = 3.26385;
/// `\wd` of `$\cdotp$` at 12 pt: cmsy `"01`.
const CDOTP: f64 = 3.33334;

fn doc(body: &str) -> String {
    format!("\\begin{{document}}{body}\\end{{document}}")
}

/// `(text, x bp)` of every math glyph on page 1, in order.
fn math_glyphs(body: &str) -> Vec<(String, f64)> {
    let r = render_one(&doc(body));
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Math {
                continue;
            }
            let mut chars = run.clusters.iter().map(|c| run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string());
            for g in &run.glyphs {
                out.push((chars.next().unwrap_or_default(), g.origin_x.to_bp()));
            }
        }
    }
    out
}

fn xs(body: &str, dot: &str) -> (f64, Vec<f64>, f64) {
    let g = math_glyphs(body);
    let a = g.iter().find(|g| g.0 == "a").unwrap_or_else(|| panic!("no `a` among {g:?}")).1;
    let b = g.iter().find(|g| g.0 == "b").unwrap_or_else(|| panic!("no `b` among {g:?}")).1;
    let dots: Vec<f64> = g.iter().filter(|g| g.0 == dot).map(|g| g.1).collect();
    assert_eq!(dots.len(), 3, "three dots expected, got {dots:?} in {g:?}");
    (a, dots, b)
}

fn close(what: &str, got: f64, want: f64) {
    assert!((got - want).abs() < 0.01, "{what}: {got:.4} bp, pdflatex {want:.4} bp (off by {:+.4})", got - want);
}

/// The low dots. This is the assertion that fails on today's main, which sets
/// three upright roman periods with no spacing of any kind:
///
/// ```text
/// `\ldots`: dot to dot: 3.2518 bp, pdflatex 5.2442 bp (off by -1.9924)
/// ```
#[test]
fn ldots_is_three_punct_periods_of_the_math_italic_family_3mu_apart() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let (_, dots, b) = xs("$a\\ldots b$", ".");
    close("`\\ldots`: dot to dot", dots[1] - dots[0], bp(LDOTP + THIN));
    close("`\\ldots`: dot to dot", dots[2] - dots[1], bp(LDOTP + THIN));
    // The last dot's own advance, then the Inner class's thin space before `b`.
    close("`\\ldots`: last dot to `b`", b - dots[2], bp(LDOTP + THIN));
}

/// The centred dots: the same structure, the cmsy glyph.
#[test]
fn cdots_is_three_punct_centred_dots_3mu_apart() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let (_, dots, b) = xs("$a\\cdots b$", "\u{00B7}");
    close("`\\cdots`: dot to dot", dots[1] - dots[0], bp(CDOTP + THIN));
    close("`\\cdots`: dot to dot", dots[2] - dots[1], bp(CDOTP + THIN));
    close("`\\cdots`: last dot to `b`", b - dots[2], bp(CDOTP + THIN));
}

/// The Inner class's *leading* thin space, isolated from `a`'s own advance by
/// measuring against a bare `.`, which is an Ord (`\mathcode`"013A) and gets
/// no space at all: `$a.b$` is 14.4276 pt against `$ab$`'s 11.16374, i.e.
/// exactly one `\ldotp` and nothing else.
#[test]
fn the_inner_class_adds_a_thin_space_before_the_dots_that_a_bare_period_does_not() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let (a_plain, plain, b_plain) = {
        let g = math_glyphs("$a.b$");
        let a = g.iter().find(|g| g.0 == "a").unwrap().1;
        let d = g.iter().find(|g| g.0 == ".").unwrap().1;
        let b = g.iter().find(|g| g.0 == "b").unwrap().1;
        (a, d, b)
    };
    let (a_dots, dots, _) = xs("$a\\ldots b$", ".");
    // `.` to `b` is the period's own advance and nothing else, which is what
    // `$a.b$` (14.4276) minus `$ab$` (11.16374) says it should be.
    close("a bare `.` is an Ord and takes no space", b_plain - plain, bp(LDOTP));
    // `a` to the first dot is `a`'s advance in both formulas, so the
    // difference between them is exactly the Inner class's leading space.
    close("`\\ldots` opens with the Inner thin space", (dots[0] - a_dots) - (plain - a_plain), bp(THIN));
}

/// `\dots` in math follows the kernel's `\mathellipsis`, so it is the low
/// dots, and `\dotsb` is the centred ones -- the grouping the compiler
/// already uses.
#[test]
fn dots_and_dotsb_follow_the_compilers_own_grouping() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let (_, low, _) = xs("$a\\dots b$", ".");
    close("`\\dots` is the low dots", low[1] - low[0], bp(LDOTP + THIN));
    let (_, mid, _) = xs("$a\\dotsb b$", "\u{00B7}");
    close("`\\dotsb` is the centred dots", mid[1] - mid[0], bp(CDOTP + THIN));
}

/// `\vdots` and `\ddots` are a different construct and must not be caught by
/// the ellipsis rule: pdflatex builds each from *text*-font periods in a
/// `\vbox`, which math-layout cannot express, so they stay the single glyph
/// the compiler emits. This pins that the change did not reach them.
#[test]
fn vdots_and_ddots_are_left_alone() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    for (body, ch) in [("$a\\vdots b$", "\u{22EE}"), ("$a\\ddots b$", "\u{22F1}")] {
        let g = math_glyphs(body);
        let n = g.iter().filter(|g| g.0 == ch).count();
        assert_eq!(n, 1, "{body}: still one glyph, not a rebuilt run of three: {g:?}");
    }
}
