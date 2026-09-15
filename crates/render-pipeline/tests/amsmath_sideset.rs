//! amsmath `\sideset` through the whole pipeline (`amsmath-sideset`).
//!
//! **Oracle**: pdflatex (TeX Live, `/Library/TeX/texbin/pdflatex`) `\showbox`
//! of `\hbox{$..$}` and `\hbox{$\displaystyle ..$}` in a 10pt `article`
//! loading `amsmath`; pdflatex is never in the product path. amsmath.sty
//! 921-929 gives, for `x\sideset{_a^b}{_c^d}\sum x`:
//!
//! ```text
//! \hbox(12.88896+6.00005)x38.70879
//! .\OML/cmm/m/it/10 x
//! .\hbox(0.0+0.0)x0.17477                  \hbox to\dimen@{} (Ord)
//! .\glue(\thinmuskip) 1.66663
//! .\hbox(12.88896+6.00005)x23.77022        \mathop
//! ..\kern -0.17477
//! ..\hbox(12.88896+6.00005)x4.83765        box 4
//! ...\vbox(10.50006+5.50006)x0.0
//! ...\vbox(18.889+0.0)x4.83765, shifted 6.00005   b, \kern11.01402, a
//! ..\hbox(12.88896+6.00005)x19.10735       box 6
//! ...\hbox(1.0+15.00012)x14.44447, shifted -9.50006   cmex10 "58
//! ...\vbox(18.889+0.0)x4.66287, shifted 6.00005       d, \kern11.01402, c
//! .\glue(\thinmuskip) 1.66663
//! .\OML/cmm/m/it/10 x
//! ```
//!
//! The `\displaystyle` box is identical but for a `\vbox` around the
//! `\mathop`. Positions below are pt relative to the first `x` (x rightward,
//! y downward from its baseline), converted to bp for the display list.

#![cfg(feature = "amsmath-sideset")]

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

/// 1 bp = 1.00375 TeX pt.
fn bp(pt: f64) -> f64 {
    pt / 1.00375
}

/// The acceptance bound is 0.5bp; the pipeline agrees with pdflatex to
/// better than 0.001bp, so the tests hold it to the usual 0.02bp.
const TOLERANCE_BP: f64 = 0.02;

/// The display `\sum`'s baseline offset in the rendered face. pdfTeX's
/// cmex10 "58 hangs below its baseline (`\hbox(1.0+15.00012)`) and is
/// `shifted -9.50006`; Latin Modern Math's display variant is drawn with
/// the same ink standing on its baseline (10.50006 + 5.50006), so its
/// origin stays on the formula's baseline, as for a plain `\[x\sum x\]`.
const DISPLAY_SUM_Y: f64 = 0.0;

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\\usepackage{{amsmath}}\\begin{{document}}{body}\\end{{document}}")
}

/// `(text, x bp, baseline y bp)` of every math glyph on page 1, in order,
/// plus every diagnostic message.
fn math_glyphs(body: &str) -> (Vec<(String, f64, f64)>, Vec<String>) {
    let r = render_one(&doc(body));
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Math {
                continue;
            }
            let mut chars = run.clusters.iter().map(|c| run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string());
            for g in &run.glyphs {
                out.push((chars.next().unwrap_or_default(), g.origin_x.to_bp(), g.baseline_y.to_bp()));
            }
        }
    }
    let diagnostics = r.v2.diagnostics.iter().map(|d| format!("{}: {}", d.code, d.message)).collect();
    (out, diagnostics)
}

fn nth<'a>(g: &'a [(String, f64, f64)], text: &str, n: usize) -> &'a (String, f64, f64) {
    g.iter().filter(|g| g.0 == text).nth(n).unwrap_or_else(|| panic!("no {n}th {text:?} among {g:?}"))
}

/// Checks glyph `text` #`n` at (`x`, `y`) pt from the first `x`.
fn at(g: &[(String, f64, f64)], what: &str, text: &str, n: usize, x: f64, y: f64) {
    let origin = nth(g, "x", 0);
    let glyph = nth(g, text, n);
    let (dx, dy) = (glyph.1 - origin.1, glyph.2 - origin.2);
    assert!(
        (dx - bp(x)).abs() <= TOLERANCE_BP && (dy - bp(y)).abs() <= TOLERANCE_BP,
        "{what}: ({dx:.4}, {dy:.4}) bp, pdflatex ({:.4}, {:.4}) bp",
        bp(x),
        bp(y)
    );
}

fn skip() -> bool {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return true;
    }
    false
}

#[test]
fn sideset_ab_cd_sum_matches_pdflatex_inline_and_display() {
    if skip() {
        return;
    }
    for body in ["$x\\sideset{_a^b}{_c^d}\\sum x$", "\\[x\\sideset{_a^b}{_c^d}\\sum x\\]"] {
        let (g, diagnostics) = math_glyphs(body);
        assert!(diagnostics.iter().all(|d| !d.contains("sideset") && !d.starts_with("math_limitation")), "{body}: {diagnostics:?}");
        // Left pair on box 4's left edge after the Ord box and \thinmuskip.
        at(&g, &format!("{body}: left sup b"), "b", 0, 5.71527 + 1.66663, -8.02785);
        at(&g, &format!("{body}: left sub a"), "a", 0, 5.71527 + 1.66663, 6.00005);
        // The display \sum after box 4; its width places box 6's scripts.
        at(&g, &format!("{body}: operator"), "∑", 0, 5.71527 + 1.66663 + 4.83765, DISPLAY_SUM_Y);
        // Box 6's scripts after the 14.44447pt glyph.
        let right = 5.71527 + 1.66663 + 4.83765 + 14.44447;
        at(&g, &format!("{body}: right sup d"), "d", 0, right, -8.02785);
        at(&g, &format!("{body}: right sub c"), "c", 0, right, 6.00005);
        // Total width: the trailing x follows the \mathop's \thinmuskip.
        at(&g, &format!("{body}: trailing x"), "x", 1, 38.70879 - 5.71527, 0.0);
    }
}

#[test]
fn unequal_left_scripts_share_their_left_edge() {
    if skip() {
        return;
    }
    // `x\sideset{_{ab}^b}{}\sum x`: \hbox x37.56258; box 4 is 8.35431 wide.
    for body in ["$x\\sideset{_{ab}^b}{}\\sum x$", "\\[x\\sideset{_{ab}^b}{}\\sum x\\]"] {
        let (g, _) = math_glyphs(body);
        at(&g, &format!("{body}: left sup b"), "b", 0, 7.38190, -8.02785);
        at(&g, &format!("{body}: left sub a"), "a", 0, 7.38190, 6.00005);
        at(&g, &format!("{body}: operator"), "∑", 0, 7.38190 + 8.35431, DISPLAY_SUM_Y);
        at(&g, &format!("{body}: trailing x"), "x", 1, 37.56258 - 5.71527, 0.0);
    }
}

#[test]
fn prime_only_sideset_is_an_ord_box_then_the_operator() {
    if skip() {
        return;
    }
    // `x\sideset{}{'}\sum x`: \hbox x32.01382. x-to-box is Ord-Ord (no
    // space), box 4 is empty, and the prime is `shifted -8.02786`.
    let (g, _) = math_glyphs("$x\\sideset{}{'}\\sum x$");
    at(&g, "operator", "∑", 0, 5.71527 + 1.66663, DISPLAY_SUM_Y);
    at(&g, "prime", "\u{2032}", 0, 5.71527 + 1.66663 + 14.44447, -8.02786);
    at(&g, "trailing x", "x", 1, 32.01382 - 5.71527, 0.0);
}
