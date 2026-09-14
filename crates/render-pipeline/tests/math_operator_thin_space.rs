//! `\limsup` and `\liminf` have a thin space inside them.
//!
//! They are the only two of the kernel's thirty-odd "Log-like functions"
//! whose definition is not a single word:
//!
//! ```text
//! \DeclareRobustCommand\limsup{\mathop{\operator@font lim\,sup}}   % latex.ltx 15528
//! \DeclareRobustCommand\liminf{\mathop{\operator@font lim\,inf}}   % latex.ltx 15529
//! ```
//!
//! The compiler has no glue inside a named operator, so it emits the whole
//! thing as one `Nucleus::Text("limsup")` and the `\,` is lost. The pipeline
//! re-reads the control word at the atom's span
//! (`typeset::operator_thin_space_split`) exactly as it already does for
//! `\left`/`\right`, `\mathbin` and the limit placement, and builds the
//! operator as `lim` + 3mu + `sup`.
//!
//! **Oracle**, TeX Live 2025 pdflatex under the harness preamble
//! (`\documentclass[12pt]{article}`, `T1`, `lmodern`, `margin=1in`,
//! `\parindent=0pt`), read with `\showbox` because pdfTeX rounds kerns to
//! 1/1000 em in the content stream. pdflatex is an oracle and is never in the
//! product path.
//!
//! ```text
//! \setbox0=\hbox{$\limsup$}\showbox0     \setbox0=\hbox{$\liminf$}\showbox0
//! .\hbox(8.38399+2.33331)x36.06792       .\hbox(8.38399+0.0)x32.60657
//! ..\kern 0.0                            ..\kern 0.0
//! ..\OT1/lmr/m/n/12 l                    ..\OT1/lmr/m/n/12 l
//! ..\OT1/lmr/m/n/12 i                    ..\OT1/lmr/m/n/12 i
//! ..\OT1/lmr/m/n/12 m                    ..\OT1/lmr/m/n/12 m
//! ..\kern0.05731                         ..\kern0.05731
//! ..\glue 1.99997                        ..\glue 1.99997
//! ..\OT1/lmr/m/n/12 s                    ..\OT1/lmr/m/n/12 i
//! ..\OT1/lmr/m/n/12 u                    ..\OT1/lmr/m/n/12 n
//! ..\OT1/lmr/m/n/12 p                    ..\OT1/lmr/m/n/12 f
//!                                        ..\kern0.84708
//! ```
//!
//! Note the `\kern0.05731` *before* the glue. Splitting the word is what
//! earns it: `lim`'s `m` is now followed by glue rather than by a math
//! character of the same family, so §753's `make_ord` never demotes it to a
//! `math_text_char` and §752 leaves it its italic correction. The gap from
//! `m` to `s` is therefore 2.05728 pt, not the 1.99997 pt of the glue alone
//! -- which is why a fix that inserted only a thin space would still be
//! 0.057 pt short.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

/// 1 bp = 1.00375 TeX pt.
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

fn nth(g: &[(String, f64)], text: &str, n: usize) -> f64 {
    g.iter().filter(|g| g.0 == text).nth(n).unwrap_or_else(|| panic!("no {n}th {text:?} among {g:?}")).1
}

fn close(what: &str, got: f64, want: f64) {
    assert!((got - want).abs() < 0.01, "{what}: {got:.4} bp, pdflatex {want:.4} bp (off by {:+.4})", got - want);
}

/// The whole of the fix in one number: how far `s` sits after the `m` of
/// `lim`. This is the assertion that fails on today's main, which sets
/// `limsup` as one word with nothing between `m` and `s`:
///
/// ```text
/// `\limsup`: `s` after the `m` of `lim`: 9.7554 bp, pdflatex 11.8050 bp (off by -2.0496)
/// ```
///
/// (9.7554 bp is `m`'s own advance, i.e. no gap at all, and 2.0496 bp is the
/// 2.05728 pt of correction plus glue.)
#[test]
fn limsup_has_a_thin_space_and_an_italic_correction_between_lim_and_sup() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("$\\limsup$");
    // `m` advance 9.792 pt, then the correction 0.05731 and the glue 1.99997.
    close("`\\limsup`: `s` after the `m` of `lim`", nth(&g, "s", 0) - nth(&g, "m", 0), bp(9.792 + 0.05731 + 1.99997));
}

/// `\liminf` the same, and it also pins the *trailing* correction: `f` ends
/// the second run, so pdfTeX closes the box with `\kern0.84708`. Measuring
/// `i`-to-`n` as well keeps the second run's own internal geometry honest.
#[test]
fn liminf_splits_at_the_thin_space_too() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("$\\liminf$");
    // The second `i` of `liminf` is the one that opens `inf`.
    close("`\\liminf`: `inf`'s `i` after the `m` of `lim`", nth(&g, "i", 1) - nth(&g, "m", 0), bp(9.792 + 0.05731 + 1.99997));
}

/// The split must come from the *control word*, not from the letters: a
/// document that writes the same six letters as an ordinary upright group
/// gets no thin space at all, because `\mathrm{limsup}` is one run of math
/// characters and TeX puts nothing inside it.
///
/// ```text
/// \setbox0=\hbox{$\mathrm{limsup}$}\showthe\wd0  ->  34.01064pt
/// \setbox0=\hbox{$\limsup$}\showthe\wd0          ->  36.06792pt
/// ```
///
/// and the difference, 2.05728 pt, is exactly the 0.05731 correction plus the
/// 1.99997 glue -- the two things the split adds and nothing else. (The
/// `\mathrm` form has no correction of its own either: `p` ends the run and
/// its italic correction is 0, so 34.01064 pt is six bare advances.)
#[test]
fn mathrm_limsup_is_one_word_with_no_thin_space() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("$\\mathrm{limsup}$");
    close("`\\mathrm{limsup}`: `s` immediately after `m`", nth(&g, "s", 0) - nth(&g, "m", 0), bp(9.792));
}
