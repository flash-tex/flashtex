//! Named operators (`\sin`, `\lim`) and `\mathrm` runs are laid out from
//! math family 0 (`operators`), not the document's text font.
//!
//! `fontmath.ltx` declares `\DeclareSymbolFont{operators}{OT1}{cmr}{m}{n}`
//! and only `lmodern` rebinds it to `lmr`; `\usepackage[T1]{fontenc}`
//! re-encodes text and never math. But these runs never went through
//! `roman_glyph`: `mathtext::shape_run` shaped the whole word with
//! `fonts.resolve(family, Role::Text { .. })`, so an operator name measured
//! the text TFM (`ec-lmr*`, or `ecrm*` under `[T1]{fontenc}` without
//! `lmodern`). The designs are not the same metrics: `cmr10`'s `i` is
//! 0.667859 em tall against `ec-lmr10`'s 0.629725, so `\sin` was 0.38 bp
//! short at 10 pt on the default gate (and 0.045/0.031/0.002 bp short on
//! the T1 gate at 10/11/12 pt), with matching width errors.
//!
//! Every number below was read out of pdfTeX 1.40.29 (TeX Live 2026),
//! `article`, with
//!
//! ```tex
//! \setbox0=\hbox{$\sin$}\immediate\write16{SIN \the\wd0\space \the\ht0}
//! \setbox0=\hbox{$\lim$}\immediate\write16{LIM \the\wd0\space \the\ht0}
//! \setbox0=\hbox{$\mathrm{lim}$}\immediate\write16{RMLIM \the\wd0\space \the\ht0}
//! ```
//!
//! at 10/11/12 pt with each of no packages, `[T1]{fontenc}`, `lmodern`,
//! and `[T1]{fontenc}` + `lmodern`. `fontenc` alone never moves math:
//! its column is identical to the no-package column, and `lmodern`'s to
//! the combined one. `\mathrm{lim}` is identical to `\lim` everywhere
//! (same letters, same last correction), so pinning `lim` pins both: the
//! pipeline builds both through `TextSink::atom_corrected`.
//!
//! | run | 10pt cmr | 10pt rm-lmr | 11pt cmr | 11pt rm-lmr | 12pt cmr | 12pt rm-lmr |
//! |---|---|---|---|---|---|---|
//! | `\sin` wd | 12.2778 | 12.34996 | 13.44418 | 13.52318 | 14.42624 | 14.48396 |
//! | `\sin` ht | 6.67859 | 6.29724 | 7.31305 | 6.89548 | 7.96431 | 7.55675 |
//! | `\lim` wd | 13.88893 | 13.96294 | 15.20836 | 15.28941 | 16.31927 | 16.3773 |
//! | `\lim` ht | 6.94444 | 6.88875 | 7.60416 | 7.54317 | 8.33331 | 8.38399 |
//!
//! (Depths are 0 pt throughout.) Note the 12 pt `lmodern` `\lim` height:
//! `ec-lmr12`'s `l` is 0.688874 em against `rm-lmr12`'s 0.698666, so the
//! text TFM was 0.12 bp short there even with `lmodern` loaded.
//!
//! What this file pins is the text-run box (`TextRunMetrics` over an
//! `atom_corrected` run): whichever text font the document uses, the box
//! follows the document's roman font. Painting is unchanged — the run
//! keeps the text face's glyph ids — so only widths, heights and depths
//! move.

mod common;

use std::rc::Rc;

use common::lm_available;
use flashtex_math_layout as ml;
use flashtex_render_pipeline::fonts::{Family, FontSet, Role};
use flashtex_render_pipeline::mathfont::{MathFonts, MathSizes};
use flashtex_render_pipeline::mathtex::TexMathMetrics;
use flashtex_render_pipeline::mathtext::{substitute, TextRunMetrics, TextSink};

/// pdfTeX prints dimensions rounded to 5 decimals; 0.0001 pt is 15x finer
/// than that and 5000x finer than the project's 0.5 bp glyph gate.
const TOL: f64 = 0.0001;

/// `(width, height, depth)` of an `atom_corrected` run of `word` (the
/// `\sin`/`\lim`/`\mathrm{lim}` construction) at the text size of `base`,
/// through the real TeX metrics provider.
fn run_box(base: u32, family: Family, roman_lm: bool, word: &str) -> (f64, f64, f64) {
    let fonts = FontSet::with_default_dirs(&[]);
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
    let tex = TexMathMetrics::new(base, false, roman_lm, otf, &fonts);
    assert!(tex.roman_available(), "rm-lmr TFMs installed");
    let mut sink = TextSink::default();
    let atom = sink.atom_corrected(word);
    let list = ml::MathList::new(vec![atom]);
    let metrics =
        TextRunMetrics::new(&tex, &fonts, fonts.shaper(), family, roman_lm, &sink.texts, &sink.keys, &sink.italics);
    let mut laid = ml::layout_with_report(&list, ml::Style::TEXT, &metrics);
    assert!(laid.limitations.is_empty(), "{:?}", laid.limitations);
    let (runs, _notices) = metrics.finish();
    substitute(&mut laid.root, &runs);
    (laid.root.width, laid.root.height, laid.root.depth)
}

/// `\sin` at 10/11/12 pt, with and without `lmodern`, under either text
/// font: the box is the document's roman font either way.
#[test]
fn sin_follows_the_documents_roman_font() {
    if !lm_available() {
        return;
    }
    // (base, width/height without lmodern, width/height with lmodern).
    for (base, cm, lm) in [
        (10, (12.2778, 6.67859), (12.34996, 6.29724)),
        (11, (13.44418, 7.31305), (13.52318, 6.89548)),
        (12, (14.42624, 7.96431), (14.48396, 7.55675)),
    ] {
        for (roman_lm, (w, h)) in [(false, cm), (true, lm)] {
            for family in [Family::LatinModern, Family::ComputerModern] {
                let (gw, gh, gd) = run_box(base, family, roman_lm, "sin");
                assert!(
                    (gw - w).abs() < TOL && (gh - h).abs() < TOL && gd.abs() < TOL,
                    "{base}pt {family:?} roman_lm={roman_lm}: \\sin is {gw}/{gh}/{gd} pt, pdflatex {w}/{h}/0 pt"
                );
            }
        }
    }
}

/// `\lim` (and hence `\mathrm{lim}`, the same construction and the same
/// measured box) at 10/11/12 pt, with and without `lmodern`.
#[test]
fn lim_follows_the_documents_roman_font() {
    if !lm_available() {
        return;
    }
    for (base, cm, lm) in [
        (10, (13.88893, 6.94444), (13.96294, 6.88875)),
        (11, (15.20836, 7.60416), (15.28941, 7.54317)),
        (12, (16.31927, 8.33331), (16.3773, 8.38399)),
    ] {
        for (roman_lm, (w, h)) in [(false, cm), (true, lm)] {
            for family in [Family::LatinModern, Family::ComputerModern] {
                let (gw, gh, gd) = run_box(base, family, roman_lm, "lim");
                assert!(
                    (gw - w).abs() < TOL && (gh - h).abs() < TOL && gd.abs() < TOL,
                    "{base}pt {family:?} roman_lm={roman_lm}: \\lim is {gw}/{gh}/{gd} pt, pdflatex {w}/{h}/0 pt"
                );
            }
        }
    }
}
