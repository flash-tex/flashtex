//! TeX's tie accent (`\t`): a tie over TWO letters (`\t{oo}`, `\t oo`).
//!
//! The tie itself is drawn like `\accent` (which this compiler does not
//! draw), so the two letters are set as the tie's span: one run each, with
//! a documented boundary between them that stops the lig/kern program
//! exactly as the accent does in TeX. Before this fix the compiler reported
//! `\t is not supported`, dropped the `oo`, and put the task oracle's `b`
//! at 157.015 instead of 170.296.
//!
//! Oracle reproduced locally: `pdflatex -interaction=nonstopmode` on
//! `a \t{oo} b \t oo c next.` (article 10pt) gives `b` x0=170.296
//! (PyMuPDF), and `\showbox` shows `\t{oo}` as two unkerned `o`s with a
//! zero-width accent construction between them — 10.00004pt at 10pt, not
//! the 10.27782pt of typed `oo` — with `\t oo` setting the same letters.
//! The tie bar itself is not drawn (no render-pipeline primitive for it);
//! positions and extraction (`oo`) match pdflatex exactly.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline};

fn document(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn paragraph(source: &str) -> Vec<Inline> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            out.extend(inlines.clone());
        }
    }
    out
}

/// Every laid-out run with its x: the tie's runs carry the `\t` command's
/// span, so whitespace joining would group them differently from the
/// reference's — positions per run are the precise comparison.
fn runs(o: &CompileOutput) -> Vec<(String, i64)> {
    o.pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| (i.text.clone(), (i.x_pt * 100.0).round() as i64))
        .collect()
}

fn messages(o: &CompileOutput) -> Vec<String> {
    o.diagnostics.iter().map(|d| d.message.clone()).collect()
}

fn same_layout(a: &str, b: &str) {
    let (x, y) = (compile(&document(a)), compile(&document(b)));
    assert_eq!(messages(&x), messages(&y), "{a:?} vs {b:?}");
    assert_eq!(runs(&x), runs(&y), "{a:?} vs {b:?}");
}

#[test]
fn tie_sets_two_letters_with_a_boundary_between() {
    let inlines = paragraph(&document("a \\t{oo} b"));
    let runs: Vec<(&str, bool)> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, boundary_before, .. } => Some((text.as_str(), *boundary_before)),
            _ => None,
        })
        .collect();
    assert_eq!(
        runs,
        [("a", false), ("o", false), ("o", true), ("b", false)],
        "{inlines:?}"
    );
}

#[test]
fn braced_and_bare_ties_set_the_same_letters() {
    let inlines = paragraph(&document("a \\t oo b"));
    let runs: Vec<(&str, bool)> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, boundary_before, .. } => Some((text.as_str(), *boundary_before)),
            _ => None,
        })
        .collect();
    assert_eq!(
        runs,
        [("a", false), ("o", false), ("o", true), ("b", false)],
        "{inlines:?}"
    );
}

#[test]
fn tie_lays_out_like_a_kern_broken_oo() {
    // `o{}o` is two `o` runs with a documented boundary between them — the
    // same shape the tie builds — so both lay out identically, with no
    // diagnostics either way.
    same_layout("a \\t{oo} b", "a o{}o b");
    same_layout("a \\t oo b", "a o{}o b");
    same_layout("a \\t{oo} b \\t oo c next.", "a o{}o b o{}o c next.");
}

#[test]
fn oracle_source_is_silent() {
    let out = compile(&document("a \\t{oo} b \\t oo c next."));
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
}

#[test]
fn tie_needs_letters() {
    for body in ["\\t{}", "\\t{ooo}", "x\\t"] {
        let parsed = parse(&document(body));
        assert!(
            !parsed.diagnostics.is_empty(),
            "{body}: expected a warning, got silence"
        );
    }
}
