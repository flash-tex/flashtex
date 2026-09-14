//! `\lim`, `\max`, `\det`, `\sin`, ... are TeX's `\mathop` of upright roman
//! text (`latex.ltx` 15523-15556, the "Log-like functions"), not ordinary atoms:
//! the Op class puts a thin space on each side, and the ten declared without
//! `\nolimits` (`\lim`, `\liminf`, `\limsup`, `\max`, `\min`, `\sup`, `\inf`,
//! `\det`, `\gcd`, `\Pr`) set their scripts as Rule 13a limits *under and over*
//! the word in display style. A following `\limits`/`\nolimits` overrides that.
//!
//! Every number below was measured with TeX Live 2025 pdflatex on this machine
//! under the harness preamble (`\documentclass[12pt]{article}`, `T1`,
//! `lmodern`, `margin=1in`, `\parindent=0pt`), glyph origins read from the
//! content stream by `tools/visual-oracle/pdftext.py`. They are an oracle:
//! pdflatex is never in the product path.
//!
//! Tolerances are 0.07 bp rather than 0.01 because of one *separate* residual
//! this fix does not touch: TeX appends the italic correction of the last
//! character of a multi-character math text run to its box (TeX §755 leaves
//! `delta` on the final `math_char` of `l i m`), so pdflatex's `\lim` box is
//! 16.3773 pt wide against our 16.32 pt of bare advances. That shifts the word
//! by half of it when it is centred over its limits, and by all of it when
//! something follows; it is a bug about text-run boxes, not about limits.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

fn doc(body: &str) -> String {
    format!("\\begin{{document}}{body}\\end{{document}}")
}

/// `(text, x bp, baseline y bp)` of every math glyph on page 1.
fn math_glyphs(body: &str) -> Vec<(String, f64, f64)> {
    let r = render_one(&doc(body));
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Math {
                continue;
            }
            let mut chars = run.clusters.iter().map(|c| run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string());
            for g in &run.glyphs {
                let t = chars.next().unwrap_or_default();
                out.push((t, g.origin_x.to_bp(), g.baseline_y.to_bp()));
            }
        }
    }
    out
}

fn nth<'a>(gs: &'a [(String, f64, f64)], text: &str, n: usize) -> &'a (String, f64, f64) {
    gs.iter().filter(|g| g.0 == text).nth(n).unwrap_or_else(|| panic!("no {n}th {text:?} among {gs:?}"))
}

fn close(what: &str, got: f64, want: f64) {
    assert!((got - want).abs() < 0.07, "{what}: {got:.4} bp, pdflatex {want:.4} bp (off by {:+.4})", got - want);
}

/// `\[\lim_{n\to\infty}a_n\]`: the limit is centred *under* the word, not set
/// as a subscript to its right.
///
/// pdflatex: `l` of `lim` at x 290.9550 baseline 110.3560; the limit `n` at
/// x 288.0760 baseline 116.3340 (5.9780 bp below the operator, and starting
/// 2.8790 bp to its *left*, because the 22.0748 bp limit is wider than the
/// 16.3773 pt word and the two are centred on each other); `a` at 312.1430.
#[test]
fn display_lim_stacks_its_limit_under_the_word() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("Before.\n\n\\[\\lim_{n\\to\\infty}a_n\\]");
    eprintln!("{g:?}");
    let lim = nth(&g, "l", 0);
    let limit = nth(&g, "n", 0);
    let a = nth(&g, "a", 0);
    close("limit below the operator's baseline", limit.2 - lim.2, 5.9780);
    assert!(
        limit.1 < lim.1,
        "the limit is centred under the operator, so it starts left of it: limit x {:.4}, `lim` x {:.4}",
        limit.1,
        lim.1
    );
    close("limit centred under the operator", limit.1 - lim.1, -2.8790);
    close("`a` after the operator", a.1 - lim.1, 21.1880);
}

/// The same formula inline is text style, where `\displaylimits` means
/// ordinary scripts: pdflatex sets the limit at x 121.1550 baseline 85.7480,
/// against the operator's x 104.8338 baseline 83.9550 -- to its right and
/// only 1.7930 bp down.
#[test]
fn inline_lim_sets_its_limit_as_a_subscript() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("Inline $\\lim_{n\\to\\infty}a_n$ here.");
    eprintln!("{g:?}");
    let lim = nth(&g, "l", 0);
    let limit = nth(&g, "n", 0);
    close("subscript below the baseline", limit.2 - lim.2, 1.7930);
    close("subscript to the right of the word", limit.1 - lim.1, 16.3212);
}

/// `\sin` is declared `\mathop{...}\nolimits`, so its scripts stay beside it
/// even in display style. pdflatex: `\[\sin^2 x\]` sets `s` at baseline
/// 110.3560 and the `2` at baseline 105.4200, 4.9360 bp *above* it and
/// 14.4300 bp to its right -- a corner script, not a limit.
#[test]
fn display_sin_keeps_its_script_beside_it() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("Before.\n\n\\[\\sin^2 x\\]");
    eprintln!("{g:?}");
    let sin = nth(&g, "s", 0);
    let two = nth(&g, "2", 0);
    assert!(two.2 < sin.2, "the script is above the baseline: {:.4} vs {:.4}", two.2, sin.2);
    assert!(two.1 > sin.1 + 10.0, "the script is to the right of the word, not centred on it: {:.4} vs {:.4}", two.1, sin.1);
}

/// `\limits`/`\nolimits` override the declaration. The compiler's parser drops
/// both switches without producing an atom (so that the script still attaches
/// to the operator), so the pipeline re-reads them from the source.
///
/// pdflatex, both in display style: `\lim\nolimits_{n}a` puts `n` at x
/// 307.2710 baseline 112.1490, 1.7930 bp below the operator's 110.3560 and to
/// its right; `\sin\limits_{n}a` puts `n` at x 299.3620 baseline 169.1360,
/// 5.9780 bp below the operator's 163.1580 and centred under it.
#[test]
fn limits_and_nolimits_override_the_declaration() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("A.\n\n\\[\\lim\\nolimits_{n}a\\]");
    eprintln!("nolimits: {g:?}");
    let lim = nth(&g, "l", 0);
    let sub = nth(&g, "n", 0);
    close("\\lim\\nolimits subscript below the baseline", sub.2 - lim.2, 1.7930);
    assert!(sub.1 > lim.1 + 10.0, "\\lim\\nolimits keeps the script to the right: {:.4} vs {:.4}", sub.1, lim.1);

    let g = math_glyphs("B.\n\n\\[\\sin\\limits_{n}a\\]");
    eprintln!("limits: {g:?}");
    let sin = nth(&g, "s", 0);
    let limit = nth(&g, "n", 1); // [0] is the `n` of `sin`
    close("\\sin\\limits limit below the baseline", limit.2 - sin.2, 5.9780);
    close("\\sin\\limits limit centred under the word", limit.1 - sin.1, 4.6460);
}

/// The class comes from the *control word* at the atom's span, not from the
/// letters: `\mathrm{lim}` reaches the pipeline as the same `Nucleus::Text`
/// run as `\lim`, but it is an ordinary atom in TeX and gets no Op spacing.
///
/// pdflatex sets `\[\mathrm{lim}_{n}a\]` with `a` 5.6370 bp after the
/// subscript's origin (the `n`'s 5.1383 bp advance plus `\scriptspace`) and
/// `\[\lim\nolimits_{n}a\]` with `a` 7.6290 bp after it -- 1.9920 bp more,
/// which is the 3mu thin space the Op class contributes and the Ord does not.
#[test]
fn mathrm_lim_is_an_ordinary_atom() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let ord = math_glyphs("C.\n\n\\[\\mathrm{lim}_{n}a\\]");
    eprintln!("mathrm: {ord:?}");
    let ord_gap = nth(&ord, "a", 0).1 - nth(&ord, "n", 0).1;
    close("no Op thin space after \\mathrm{lim}", ord_gap, 5.6370);

    let op = math_glyphs("A.\n\n\\[\\lim\\nolimits_{n}a\\]");
    let op_gap = nth(&op, "a", 0).1 - nth(&op, "n", 0).1;
    close("thin space after \\lim", op_gap, 7.6290);
    close("the Op class is worth 3mu at 12pt", op_gap - ord_gap, 1.9920);
}

/// The Op class also spaces an operator with no scripts at all: pdflatex sets
/// `\[\det A\]` with `t` at x 304.1935 (advance 4.5525) and `A` at 310.7426,
/// 1.9966 bp after the word ends.
#[test]
fn det_gets_the_op_thin_space() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = math_glyphs("D.\n\n\\[\\det A\\]");
    eprintln!("{g:?}");
    close("thin space between \\det and its operand", nth(&g, "A", 0).1 - nth(&g, "d", 0).1, 18.2556);
}
