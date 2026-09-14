//! A run of math characters keeps the italic correction of its last
//! character; an `\hbox` does not.
//!
//! tex.web §752 creates the character node for an Ord noad's nucleus and then
//! decides what to do with `delta`:
//!
//! ```text
//! delta:=char_italic(cur_f)(cur_i); p:=new_character(...);
//! if (math_type(nucleus(q))=math_text_char)and(space(cur_f)<>0) then
//!   delta:=0; {no italic correction in mid-word of text font}
//! if (math_type(subscr(q))=empty)and(delta<>0) then
//!   begin link(p):=new_kern(delta); delta:=0; end
//! ```
//!
//! and §753's `make_ord` is what turns a character into a `math_text_char`:
//! only when the *next* noad is an Ord..Punct whose nucleus is a math char of
//! the **same family**. In `l i m` that is true of `l` and `i` and false of
//! `m`, which therefore stays a plain `math_char` and keeps its correction.
//! So the box is `l i m \kern0.05731`, not three bare advances.
//!
//! An `\hbox` in the math list -- amsmath's `\text`, and its `\tag`, whose
//! label is `\maketag@@@#1 -> \hbox{\m@th\normalfont#1}` (amsmath.sty 1211) --
//! is not a run of math characters at all and has no correction. The compiler
//! spells both constructs `Nucleus::Text`, so the pipeline re-reads the
//! control word at the atom's span (`typeset::math_text_is_hbox`) exactly as
//! it already does for `\left`/`\right` and `\mathbin`.
//!
//! **Oracle.** Every number below is TeX Live 2025 pdflatex on this machine
//! under the harness preamble (`\documentclass[12pt]{article}`, `T1`,
//! `lmodern`, `margin=1in`, `\parindent=0pt`), read with `\showbox` rather
//! than from the PDF: pdfTeX rounds a kern to 1/1000 em when it writes the
//! content stream (0.012 bp at 12 pt), which is a fifth of the effect being
//! measured here, so the page positions are too coarse to state it. TeX's own
//! box algebra is exact. pdflatex is an oracle and is never in the product
//! path.
//!
//! ```text
//! \setbox0=\hbox{$\mathrm{lim}x$}\showbox0
//! .\hbox(8.38399+0.0)x16.3773    .\hbox(8.26648+0.0)x16.31999   <- \text{lim}
//! ..\OT1/lmr/m/n/12 l            ..\hbox(8.26648+0.0)x16.31999
//! ..\OT1/lmr/m/n/12 i            ...\T1/lmr/m/n/12 l
//! ..\OT1/lmr/m/n/12 m            ...\T1/lmr/m/n/12 i
//! ..\kern0.05731                 ...\T1/lmr/m/n/12 m
//! .\OML/lmm/m/it/12 x            .\OML/lmm/m/it/12 x
//! ```
//!
//! OT1 and T1 agree on `l`, `i` and `m` (both runs are 16.31999 pt of
//! advances), so the entire 0.05731 pt difference is the correction.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

/// 1 bp = 1.00375 TeX pt (72.27 pt and 72 bp to the inch), the conversion
/// `tools/visual-oracle/rank.py` uses.
fn bp(pt: f64) -> f64 {
    pt / 1.00375
}

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

/// The distance from the first `l` (or `f`) of the run to the math-italic `x`
/// that follows it: the run's whole advance, correction included.
fn run_to_x(body: &str, first: &str) -> f64 {
    let g = math_glyphs(body);
    let start = g.iter().find(|g| g.0 == first).unwrap_or_else(|| panic!("no {first:?} among {g:?}"));
    let x = g.iter().find(|g| g.0 == "x").unwrap_or_else(|| panic!("no `x` among {g:?}"));
    x.1 - start.1
}

/// The whole effect in one comparison: the same three letters, the same face,
/// the same size, one set as math characters and one as an `\hbox`.
///
/// This is the assertion that fails on main, where `shape_run` discarded the
/// correction for every run without a math-alphabet font key:
///
/// ```text
/// `\mathrm{lim}` keeps its last character's italic correction:
///   16.2590 bp, pdflatex 16.3161 bp (off by -0.0571)
/// ```
#[test]
fn a_math_character_run_keeps_its_last_italic_correction_and_an_hbox_does_not() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let math_chars = run_to_x("$\\mathrm{lim}x$", "l");
    let hbox = run_to_x("$\\text{lim}x$", "l");
    close("`\\mathrm{lim}` keeps its last character's italic correction", math_chars, bp(16.3773));
    close("`\\text{lim}` is an hbox and has none", hbox, bp(16.31999));
    close("and the difference between them is exactly the correction", math_chars - hbox, bp(0.05731));
}

/// `f` has by far the largest correction in the roman text font (0.84708 pt
/// at 12 pt, against `m`'s 0.05731), so it separates the two constructs by
/// more than a tolerance could ever hide:
///
/// ```text
/// \setbox0=\hbox{$\mathrm{f}$}\showbox0   ->  .\OT1/lmr/m/n/12 f
///                                             .\kern0.84708
/// \setbox0=\hbox{$\text{f}$}\showbox0     ->  .\hbox(8.26648+0.0)x3.59024
/// ```
///
/// A single `\mathrm{f}` is a lone math character, which `make_ord` never
/// converts to a `math_text_char` (there is no following noad), so it keeps
/// the correction for the same reason the `m` of `lim` does.
#[test]
fn the_correction_is_the_fonts_own_and_f_shows_it_at_full_size() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let math_chars = run_to_x("$\\mathrm{f}x$", "f");
    let hbox = run_to_x("$\\text{f}x$", "f");
    close("`\\mathrm{f}` carries f's 0.84708 pt correction", math_chars, bp(4.43732));
    close("`\\text{f}` carries none", hbox, bp(3.59024));
    close("difference", math_chars - hbox, bp(0.84708));
}

/// A named operator is the same kind of run, reached through a different
/// command, and it is the case the corpus actually contains. `\lim` adds the
/// Op class's thin space on each side, which `\mathrm{lim}` does not have:
///
/// ```text
/// \setbox0=\hbox{$\lim x$}\showbox0
/// .\hbox(8.38399+0.0)x16.3773
/// ..\kern 0.0 ..l ..i ..m ..\kern0.05731
/// .\glue(\thinmuskip) 1.99997
/// .\OML/lmm/m/it/12 x
/// ```
#[test]
fn a_named_operator_carries_the_correction_inside_its_op_atom() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    close("`\\lim x`: the word, its correction, then the Op thin space", run_to_x("$\\lim x$", "l"), bp(16.3773 + 1.99997));
}

/// `\bmod` and `\pmod` reach `Nucleus::Text` by yet another route, and
/// `(mod` ends in `d`, whose correction is 0 in this font -- so this test
/// pins the *absence* of a change as much as its presence, and guards the
/// `\pmod` spacing that `crates/compiler/src/math.rs` measured separately.
///
/// ```text
/// \setbox0=\hbox{$a\bmod b$}\showbox0
/// .\hbox(8.38399+0.0)x22.52136
/// ..\OT1/lmr/m/n/12 m ..\OT1/lmr/m/n/12 o ..\kern0.32639 ..\OT1/lmr/m/n/12 d
/// ```
///
/// The `\kern0.32639` sits between `o` and `d`, and it is the TFM's own
/// lig/kern program rather than an italic correction: `ec-lmr12.tfm` has
/// `(LABEL C o)(KRN C d R 0.0272)`, and 0.0272 em at 12 pt is 0.32639 pt.
/// An italic correction would come *after* the last character, and there is
/// none -- `\wd` of `$\mathrm{mod}$` and of `$\text{mod}$` are the same
/// 22.52136 pt, where `$\mathrm{m}$` (9.84932) and `$\text{m}$` (9.792)
/// differ by `m`'s 0.05732.
///
/// So `m` to `d` is `\wd` of `$\text{mo}$` (15.66699) plus that kern.
#[test]
fn bmod_is_a_math_character_run_whose_last_character_has_no_correction() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("$a\\bmod b$");
    let m = g.iter().find(|g| g.0 == "m").unwrap_or_else(|| panic!("no `m` among {g:?}"));
    let d = g.iter().find(|g| g.0 == "d").unwrap_or_else(|| panic!("no `d` among {g:?}"));
    close("`mod` sets `m o` with the TFM's own kern before `d`", d.1 - m.1, bp(15.66699 + 0.32639));
}

fn close(what: &str, got: f64, want: f64) {
    assert!((got - want).abs() < 0.01, "{what}: {got:.4} bp, pdflatex {want:.4} bp (off by {:+.4})", got - want);
}
