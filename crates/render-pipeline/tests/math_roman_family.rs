//! Math family 0 (`operators`) is `cmr*`, not `rm-lmr*`, unless the document
//! loads `lmodern`.
//!
//! `fontmath.ltx` declares `\DeclareSymbolFont{operators}{OT1}{cmr}{m}{n}`.
//! `\usepackage[T1]{fontenc}` re-encodes *text* and never math, so only
//! `lmodern` — which rebinds the symbol font itself — moves family 0 to
//! `lmr`. `rm-lmr` is not a scaled `cmr`: its digits are 0.0147 em shorter
//! and its `i` 0.0381 em shorter, so a `\frac{1}{n}` numerator, a radicand
//! or a `\sum` limit made of family-0 characters sets a box that much
//! shorter, and in display math every baseline below it moves with the box
//! (GH-DISPLAY-BOX-HEIGHT, the top-ranked finding of #750's corpus sweep).
//!
//! Every number below was read out of pdfTeX 1.40.29 (TeX Live 2026),
//! `article`, with
//!
//! ```tex
//! \setbox0=\hbox{$1\immediate\write16{F0 \fontname\textfont0}$}
//! \immediate\write16{DIG \the\ht0}
//! \setbox0=\hbox{$\sin$}\immediate\write16{SIN \the\ht0}
//! ```
//!
//! | packages | `\textfont0` | `\ht$1$` | `\ht$\sin$` |
//! |---|---|---|---|
//! | (none) or `[T1]{fontenc}` 10pt | `cmr10` | 6.44444 pt | 6.67859 pt |
//! | (none) or `[T1]{fontenc}` 11pt | `cmr10 at 10.95pt` | 7.05666 pt | 7.31305 pt |
//! | (none) or `[T1]{fontenc}` 12pt | `cmr12` | 7.73332 pt | 7.96431 pt |
//! | + `lmodern` 10pt | `rm-lmr10` | 6.29724 pt | 6.29724 pt |
//! | + `lmodern` 11pt | `rm-lmr10 at 10.95pt` | 6.89548 pt | 6.89548 pt |
//! | + `lmodern` 12pt | `rm-lmr12` | 7.55675 pt | 7.55675 pt |
//!
//! The `\sin` column is the *text-run* route (`mathtext::shape_run`), which
//! still takes the document's text TFM; it is measured but not asserted
//! here. What this file pins is the family-0 glyph box, which is what the
//! display box height is built from.

mod common;

use std::rc::Rc;

use common::lm_available;
use flashtex_math_layout::{MathFontMetrics, SizeClass};
use flashtex_render_pipeline::fonts::{Family, FontSet, Role};
use flashtex_render_pipeline::mathfont::{MathFonts, MathSizes};
use flashtex_render_pipeline::mathtex::TexMathMetrics;
use flashtex_render_pipeline::style::math_roman_lm;

/// pdfTeX prints dimensions rounded to 5 decimals; 0.0001 pt is 15x finer
/// than that and 5000x finer than the project's 0.5 bp glyph gate.
const TOL: f64 = 0.0001;

fn metrics(base: u32, roman_lm: bool, fonts: &FontSet) -> TexMathMetrics {
    let text = match base {
        10 => 10.0,
        11 => 10.95,
        _ => 12.0,
    };
    let (script, script_script) = match base {
        10 => (7.0, 5.0),
        _ => (8.0, 6.0),
    };
    let r = fonts.resolve(Family::LatinModern, Role::Math, text);
    assert!(r.substituted.is_none(), "Latin Modern Math at {text}pt");
    let sizes = MathSizes { text, script, script_script };
    let otf = Rc::new(MathFonts::new(r.face, sizes).expect("MATH table"));
    let m = TexMathMetrics::new(base, false, roman_lm, otf, fonts);
    assert!(m.roman_available(), "rm-lmr TFMs installed");
    m
}

fn height(base: u32, roman_lm: bool, ch: char, fonts: &FontSet) -> f64 {
    metrics(base, roman_lm, fonts)
        .text_glyph(ch, SizeClass::Text)
        .unwrap_or_else(|| panic!("no family-0 glyph for {ch:?}"))
        .height
}

#[test]
fn digits_are_cmr_tall_without_lmodern() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    // `\ht\hbox{$1$}` under pdflatex, no `lmodern`.
    for (base, want) in [(10, 6.44444), (11, 7.05666), (12, 7.73332)] {
        for ch in ['0', '1', '8'] {
            let got = height(base, false, ch, &fonts);
            assert!(
                (got - want).abs() < TOL,
                "{base}pt {ch:?}: {got} pt, pdflatex cmr {want} pt"
            );
        }
    }
}

#[test]
fn digits_are_rm_lmr_tall_with_lmodern() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    // `\ht\hbox{$1$}` under pdflatex with `\usepackage{lmodern}`.
    for (base, want) in [(10, 6.29724), (11, 6.89548), (12, 7.55675)] {
        for ch in ['0', '1', '8'] {
            let got = height(base, true, ch, &fonts);
            assert!(
                (got - want).abs() < TOL,
                "{base}pt {ch:?}: {got} pt, pdflatex rm-lmr {want} pt"
            );
        }
    }
}

/// The two designs differ by 0.0147 em on a digit — far under the 0.5 bp
/// glyph gate for one glyph, which is exactly why picking the wrong one
/// went unnoticed, and exactly why it matters: it is a *box height*, so it
/// displaces every baseline under the display, not one glyph.
#[test]
fn the_two_roman_designs_are_not_the_same_metrics() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    for (base, want) in [(10, 0.14720), (11, 0.16118), (12, 0.17657)] {
        let d = height(base, false, '1', &fonts) - height(base, true, '1', &fonts);
        assert!(
            (d - want).abs() < TOL,
            "{base}pt: cmr is {d} pt taller than rm-lmr, pdflatex says {want} pt"
        );
    }
}

/// The quantity the page actually moves with: the **height of the display
/// box**, which is what `typeset::display_block` reads off the laid-out
/// `MathBox` to place the display in the vertical list.
///
/// `\ht\hbox{$\displaystyle ...$}` under pdfTeX 1.40.29, `article` +
/// `amsmath`, `[T1]{fontenc}`, with and without `lmodern`:
///
/// | formula | 10pt | 11pt | 12pt |
/// |---|---|---|---|
/// | `\frac{a}{b}` (either) | 11.07062 | 12.12231 | 13.28476 |
/// | `\frac{1}{b}` no `lmodern` | 13.20952 | 14.46440 | 15.85141 |
/// | `\frac{1}{b}` `lmodern` | 13.06232 | 14.30322 | 15.67484 |
///
/// So the answer to "is the fraction rule wrong?" is no: `\frac{a}{b}` was
/// always exact, at every size, on both designs. What was wrong is the
/// height of the *character* the numerator is made of, whenever that
/// character is family 0. The same holds for a radicand and for an
/// operator's limits, which is why one cause covers the three constructs
/// #750's F2 named.
fn frac_box(base: u32, roman_lm: bool, numerator: char, fonts: &FontSet) -> (f64, f64) {
    use flashtex_math_layout::{Atom, AtomClass, MathList, Nucleus, Style};
    let list = MathList::new(vec![Atom::new(
        AtomClass::Ord,
        Nucleus::Fraction {
            numerator: MathList::new(vec![Atom::symbol(numerator)]),
            denominator: MathList::new(vec![Atom::symbol('b')]),
            thickness: None,
            left: None,
            right: None,
        },
    )]);
    let m = metrics(base, roman_lm, fonts);
    let b = flashtex_math_layout::layout(&list, Style::DISPLAY, &m);
    (b.height, b.depth)
}

#[test]
fn a_fraction_of_math_italic_is_exact_on_either_roman_design() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    for (base, h, d) in [(10, 11.07062, 6.85951), (11, 12.12231, 7.51115), (12, 13.28476, 8.23141)] {
        for roman_lm in [false, true] {
            let (gh, gd) = frac_box(base, roman_lm, 'a', &fonts);
            assert!(
                (gh - h).abs() < TOL && (gd - d).abs() < TOL,
                "{base}pt roman_lm={roman_lm}: \\frac{{a}}{{b}} is {gh}/{gd} pt, pdflatex {h}/{d} pt"
            );
        }
    }
}

#[test]
fn a_fraction_with_a_digit_numerator_follows_the_documents_roman_font() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    // (base, height without lmodern, height with lmodern); the depth is the
    // denominator's and is the same either way.
    for (base, cm, lm, depth) in [
        (10, 13.20952, 13.06232, 6.85951),
        (11, 14.46440, 14.30322, 7.51115),
        (12, 15.85141, 15.67484, 8.23141),
    ] {
        for (roman_lm, want) in [(false, cm), (true, lm)] {
            let (gh, gd) = frac_box(base, roman_lm, '1', &fonts);
            assert!(
                (gh - want).abs() < TOL && (gd - depth).abs() < TOL,
                "{base}pt roman_lm={roman_lm}: \\frac{{1}}{{b}} is {gh}/{gd} pt, pdflatex {want}/{depth} pt"
            );
        }
    }
}

/// A radicand is the same story: `\sqrt{a}` is exact either way at 10 pt
/// (where `cmex10` and `lmex10` agree on the radical sign), and `\sqrt{1}`
/// is 0.0736 pt short with the wrong roman design.
#[test]
fn a_digit_radicand_follows_the_documents_roman_font() {
    use flashtex_math_layout::{Atom, AtomClass, MathList, Nucleus, Style};
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let sqrt = |ch: char, roman_lm: bool| {
        let list = MathList::new(vec![Atom::new(
            AtomClass::Ord,
            Nucleus::Radical { radicand: MathList::new(vec![Atom::symbol(ch)]), degree: None },
        )]);
        let m = metrics(10, roman_lm, &fonts);
        flashtex_math_layout::layout(&list, Style::DISPLAY, &m).height
    };
    // `\ht\hbox{$\displaystyle\sqrt{..}$}`, 10pt, pdfTeX 1.40.29.
    for (ch, cm, lm) in [('a', 8.49092, 8.49092), ('1', 9.56036, 9.48677)] {
        for (roman_lm, want) in [(false, cm), (true, lm)] {
            let got = sqrt(ch, roman_lm);
            assert!(
                (got - want).abs() < TOL,
                "\\sqrt{{{ch}}} roman_lm={roman_lm}: {got} pt, pdflatex {want} pt"
            );
        }
    }
}

/// Only `lmodern` rebinds `operators`; `fontenc` re-encodes text alone.
#[test]
fn only_lmodern_selects_the_lmr_operators_font() {
    let p = |names: &[&str]| names.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    assert!(!math_roman_lm(&p(&[])));
    assert!(!math_roman_lm(&p(&["amsmath", "amssymb", "geometry"])));
    assert!(math_roman_lm(&p(&["lmodern"])));
    assert!(math_roman_lm(&p(&["amsmath", "lmodern"])));
}
