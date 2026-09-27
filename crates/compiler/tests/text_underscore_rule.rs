//! OT1 text-mode `\_`: a kern plus a thin rule, not a glyph.
//!
//! Under OT1 (no `fontenc`), `\_` is `\leavevmode \kern.06em
//! \vbox{\hrule width.3em}` (latex.ltx `\DeclareTextCommandDefault
//! {\textunderscore}`): one 0.06em kern, then a 0.3em-wide 0.4pt-tall rule
//! sitting on the baseline — 0.36em altogether, not 0.42em (there is no
//! second kern; oracled with TeX Live 2026 `pdflatex -interaction=nonstopmode`
//! `\showbox`: `a\_b` is `a`, `\kern 0.59998`, a `(0.4+0.0)x3.00003` rule
//! vbox, `b`). Before this fix the compiler drew the Latin Modern
//! underscore glyph even under OT1, putting the task oracle's `next` at
//! 170.026 instead of 166.137. Under T1 `\_` is already the real
//! underscore glyph, which is unchanged.
//!
//! Oracle reproduced locally: `pdflatex -interaction=nonstopmode` on
//! `a\_b next word` (article 10pt) gives `next` x0=166.137 (PyMuPDF), and
//! the same source with the kern and rule spelled out
//! (`a\kern.06em\vbox{\hrule width.3em}b next word`) gives the identical
//! line, while a trailing second kern moves `next` to 166.735.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline};
use flashtex_compiler::text_builtins::{self, DimenContext};

fn ot1_document(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn t1_document(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage[T1]{{fontenc}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
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

fn at_word(o: &CompileOutput, word: &str) -> f64 {
    o.pages
        .iter()
        .flat_map(|p| p.items.iter())
        .find(|i| i.text == word)
        .map(|i| i.x_pt)
        .unwrap_or_else(|| panic!("{word} missing"))
}

#[test]
fn underscore_construction_matches_the_kernel() {
    let (kern, rule) = text_builtins::ot1_underscore();
    assert_eq!(kern, text_builtins::TextDimen::parse(".06em").unwrap());
    let cx = DimenContext { quad: 10 * 65536, ..DimenContext::default() };
    // 0.06em at 10pt through `scan_dimen`: 0.59998pt, exactly pdflatex's kern.
    assert_eq!(kern.resolve(&cx), 39320);
    let b = rule.resolve(&cx);
    // 0.3em at 10pt through `scan_dimen`: 3.00003pt, exactly pdflatex's rule.
    assert_eq!(b.width, 196610);
    assert_eq!((b.height, b.depth), (26214, 0));
    assert_eq!((b.rule_top, b.rule_bottom), (26214, 0));
    assert!(b.painted());
}

#[test]
fn ot1_underscore_is_kern_then_rule_not_a_glyph() {
    let inlines = paragraph(&ot1_document("a\\_b next word"));
    // No underscore text anywhere in the paragraph.
    for inline in &inlines {
        if let Inline::Text { text, .. } = inline {
            assert!(!text.contains('_'), "{inlines:?}");
        }
    }
    let want_rule = text_builtins::ot1_underscore().1;
    // The `a`, kern, rule, kern, `b` sequence in order.
    let mut seen: Vec<&str> = Vec::new();
    for inline in &inlines {
        match inline {
            Inline::Text { text, .. } if !text.is_empty() => seen.push("text"),
            Inline::Kern { .. } => seen.push("kern"),
            Inline::Rule { rule, .. } if *rule == want_rule => seen.push("rule"),
            Inline::Rule { .. } => seen.push("other-rule"),
            _ => {}
        }
    }
    assert_eq!(
        seen,
        ["text", "kern", "rule", "text", "text", "text"],
        "{inlines:?}"
    );
}

#[test]
fn ot1_underscore_adds_three_point_six_points_at_ten_point() {
    // The construction is 0.36em wide however the layout measures the em:
    // compare against `ab` in the compiler's own layout.
    let (with, without) = (compile(&ot1_document("a\\_b next")), compile(&ot1_document("ab next")));
    assert!(with.diagnostics.is_empty(), "{:?}", with.diagnostics);
    let gap = at_word(&with, "next") - at_word(&without, "next");
    let size = with
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .find(|i| i.text == "next")
        .map(|i| i.font_size_pt)
        .expect("next");
    assert!(
        (gap / size - 0.36).abs() < 0.01,
        "gap {gap} at size {size}: want 0.36em"
    );
}

#[test]
fn space_before_underscore_stays_before_the_kern() {
    // `a \_b`: the space is TeX's glue before the kern, carried on an empty
    // run — not shifted past the rule.
    let inlines = paragraph(&ot1_document("a \\_b"));
    let kinds: Vec<&str> = inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text { text, glue_before, .. } if text.is_empty() => {
                assert!(glue_before.is_some(), "{inlines:?}");
                "glue-run"
            }
            Inline::Text { .. } => "text",
            Inline::Kern { .. } => "kern",
            Inline::Rule { .. } => "rule",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["text", "glue-run", "kern", "rule", "text"], "{inlines:?}");
}

#[test]
fn t1_underscore_stays_a_glyph() {
    let inlines = paragraph(&t1_document("a\\_b next"));
    let mut saw_glyph = false;
    let mut saw_rule = false;
    for inline in &inlines {
        match inline {
            Inline::Text { text, .. } if text.contains('_') => saw_glyph = true,
            Inline::Rule { .. } => saw_rule = true,
            _ => {}
        }
    }
    assert!(saw_glyph, "{inlines:?}");
    assert!(!saw_rule, "{inlines:?}");
}
