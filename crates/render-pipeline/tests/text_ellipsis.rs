//! `\dots`, `\ldots`, `\textellipsis` and a literal `…` are all the same
//! thing in text mode: `\textellipsis`, which T1 does not declare, so the
//! encoding-independent kernel default typesets it (`latex.ltx` 10071):
//!
//!     .\kern\fontdimen3\font .\kern\fontdimen3\font .\kern\fontdimen3\font
//!
//! Three periods of the current font, each followed by that font's interword
//! stretch — 15.60 bp at 12 pt Latin Modern, against the 7.70 bp of the single
//! U+2026 glyph. The last of them is a period, so it leaves the period's space
//! factor (3000) behind and the interword glue after it carries `\fontdimen7`.
//!
//! Every number below was measured with TeX Live 2025 pdflatex on this machine
//! under the harness preamble (`\documentclass[12pt]{article}`, `T1`,
//! `lmodern`, `margin=1in`, `\parindent=0pt`), glyph origins read from the
//! content stream by `tools/visual-oracle/pdftext.py`. pdflatex is an oracle:
//! it is never in the product path.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

fn doc(body: &str) -> String {
    format!("\\begin{{document}}{body}\\end{{document}}")
}

/// `(glyph x in bp, the text of the cluster the glyph belongs to)` for every
/// text glyph on page 1, in order. A cluster holds as many glyphs as the
/// character it came from sets, so the three periods of one ellipsis all
/// report `…` — that is the property being asserted, not an artefact.
fn glyphs(body: &str) -> Vec<(f64, String)> {
    let r = render_one(&doc(body));
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Text {
                continue;
            }
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                out.push((g.origin_x.to_bp(), run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string()));
            }
        }
    }
    out
}

fn xs(body: &str) -> Vec<f64> {
    glyphs(body).into_iter().map(|(x, _)| x).collect()
}

fn close(what: &str, got: f64, want: f64) {
    close_to(what, got, want, 0.02)
}

fn close_to(what: &str, got: f64, want: f64, tol: f64) {
    assert!((got - want).abs() < tol, "{what}: {got:.4} bp, pdflatex {want:.4} bp (off by {:+.4})", got - want);
}

/// pdflatex sets `ellipsis\dots;` with the periods at x 105.9480, 111.1485 and
/// 116.3490 (step 5.2005 = the 3.2518 bp period plus `\fontdimen3` 1.9487) and
/// the `;` at 121.5615 — 15.61 bp of ellipsis, not one 7.70 bp glyph.
#[test]
fn dots_sets_three_kerned_periods() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = glyphs("ellipsis\\dots; and more.");
    eprintln!("{g:?}");
    let x = xs("ellipsis\\dots; and more.");
    // `e l l i p s i s` are glyphs 0..8, the three periods 8..11, then `;`.
    assert!(x.len() > 12, "expected the ellipsis to set three glyphs: {g:?}");
    close("first period", x[8], 105.9480);
    close("second period", x[9], 111.1485);
    close("third period", x[10], 116.3490);
    close("the `;` after the ellipsis", x[11], 121.5615);
    assert_eq!(
        (g[8].1.as_str(), g[9].1.as_str(), g[10].1.as_str()),
        ("\u{2026}", "\u{2026}", "\u{2026}"),
        "the three periods are one cluster, whose text is the `…` that was written: {g:?}"
    );
    assert_eq!(g[11].1, ";", "and the cluster after it is the `;`: {g:?}");
}

/// `\ldots`, `\textellipsis` and a literal `…` (declared `\textellipsis` by
/// `utf8.def`) are the same construct and set the same three periods.
#[test]
fn ldots_textellipsis_and_a_literal_ellipsis_agree() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let dots = xs("ellipsis\\dots; and more.");
    for body in ["ellipsis\\ldots; and more.", "ellipsis\\textellipsis; and more.", "ellipsis\u{2026}; and more."] {
        assert_eq!(xs(body), dots, "{body:?} must set the same glyphs as \\dots");
    }
}

/// The ellipsis' last character is a period, so the space after it is
/// `\fontdimen2 + \fontdimen7` (space factor 3000), not `\fontdimen2` alone.
/// pdflatex sets `literal… here.` with `h` at x 122.7307, 5.2127 bp after the
/// last period's 117.5180; `\fontdimen2` alone would put it at 121.4198.
#[test]
fn the_space_after_an_ellipsis_is_a_sentence_space() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let g = glyphs("literal\u{2026} here.");
    eprintln!("{g:?}");
    let h = g.iter().position(|(_, t)| t == "h").unwrap_or_else(|| panic!("no `h` in {g:?}"));
    close("`h` after the ellipsis", g[h].0, 122.7307);
}

/// The surrounding word is shaped by the TFM, not dropped out of it: U+2026
/// has no T1 slot, so `shape_tfm` used to give up on the *whole run* and one
/// ellipsis sent `ellipsis` through the OpenType metrics instead. pdflatex's
/// `ellipsis` here starts at 72.0000 with `e l l i p s i s` at 77.2029,
/// 80.4547, 83.7065, 86.9583, 93.4620, 98.0791, 101.3309.
///
/// This is a sub-0.005 bp effect, so the tolerance has to be tight to see it:
/// the TFM path lands within 0.0005 bp of every one of those, the OpenType
/// path drifts to 0.0049 bp by the last two letters. The numbers on both
/// sides are exact computations, not noisy measurements.
#[test]
fn the_word_around_an_ellipsis_keeps_tfm_metrics() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let x = xs("ellipsis\\dots; and more.");
    for (i, want) in [77.2029, 80.4547, 83.7065, 86.9583, 93.4620, 98.0791, 101.3309].iter().enumerate() {
        close_to(&format!("glyph {} of `ellipsis`", i + 1), x[i + 1], *want, 0.003);
    }
}
