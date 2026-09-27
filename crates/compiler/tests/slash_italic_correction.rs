//! Text-mode `\/` (italic correction).
//!
//! `\/` is a kern equal to the TFM italic correction of the previous
//! character — zero after a space or after an upright letter — and never
//! prints a glyph. (Before this fix the compiler typeset a literal `/`.)
//!
//! The compiler cannot resolve the kern (it is the previous glyph's
//! `CHARIC` in the font actually set), so each `\/` becomes an empty run
//! carrying `\check@icr`'s decision, which the render pipeline resolves
//! from the last glyph exactly as TeX's tail check does. These tests pin
//! that structure: the oracle positions (`B` at x 170.142, `D` at
//! x 204.682 in pdflatex TL2026, article 10pt, bp from the page top-left)
//! need the pipeline's CM italic corrections, which the compiler's own
//! Core 14 layout does not carry.
//!
//! Oracle reproduced locally: `pdflatex -interaction=nonstopmode` on the
//! source below gives `B` x0=170.142 and `D` x0=204.682 (PyMuPDF), and
//! `{\itshape f\/}C` exceeds `{\itshape f}C` by exactly cmti10's `f`
//! correction (2.11945pt), with a single kern — the group's own
//! `\check@icr` finds the explicit kern as its tail and adds nothing.

use flashtex_compiler::parser::{parse, Block, Inline};

fn document(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
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

fn texts(inlines: &[Inline]) -> Vec<&str> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// Empty runs carrying the correction decision (`after` set, `before`
/// clear), in order.
fn corrections(inlines: &[Inline]) -> Vec<bool> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, style, glue_before, .. }
                if text.is_empty()
                    && style.italic_correction.after
                    && !style.italic_correction.before =>
            {
                Some(glue_before.is_some())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn slash_is_a_kern_never_a_glyph() {
    let inlines = paragraph(&document(
        "A \\textit{it\\/} B {\\itshape f\\/}C {\\itshape f}\\/ D \\textit{f} E next.",
    ));
    // No literal slash anywhere in the paragraph.
    assert!(
        texts(&inlines).iter().all(|text| !text.contains('/')),
        "{:?}",
        texts(&inlines)
    );
    // One correction run per `\/`, each flagged `after` (never `before`).
    assert_eq!(corrections(&inlines).len(), 3, "{inlines:?}");
}

#[test]
fn correction_after_a_space_rides_on_the_glue() {
    // `a \/b`: TeX's tail is the glue, so no kern — but the space itself
    // stays, carried by the correction run exactly like a word carries it.
    let inlines = paragraph(&document("a \\/b"));
    assert_eq!(corrections(&inlines), [true], "{inlines:?}");
    // `a\/b`: bare run, the kern lands on `a`.
    let inlines = paragraph(&document("a\\/b"));
    assert_eq!(corrections(&inlines), [false], "{inlines:?}");
}

#[test]
fn slash_at_the_start_corrects_nothing_but_prints_nothing() {
    let inlines = paragraph(&document("\\/ab"));
    assert_eq!(corrections(&inlines), [false], "{inlines:?}");
    assert!(
        texts(&inlines).iter().all(|text| !text.contains('/')),
        "{:?}",
        texts(&inlines)
    );
}

#[test]
fn typed_slash_is_untouched() {
    // A typed `/` is not a control symbol: it stays literal text.
    let inlines = paragraph(&document("a/b"));
    assert!(corrections(&inlines).is_empty(), "{inlines:?}");
    assert_eq!(
        texts(&inlines).concat(),
        "a/b",
        "{:?}",
        texts(&inlines)
    );
}
