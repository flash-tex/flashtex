//! Text-mode `\char<number>` and `\symbol{<number>}`.
//!
//! Stated pdflatex oracle (TeX Live 2026, `\documentclass{article}` at
//! 10pt, exit code 0, zero `!` errors) for
//! `a \symbol{65} b \char`B c \char"43{} d \char 68 e next.`
//! (the backquote form is `` \char`B ``): pdflatex sets A, B, C, D at the
//! four sites and `next.` at x 229.519 (glyph origin in bp from the page's
//! top-left).
//!
//! `\char` takes TeX's general `<number>`: decimal (`\char68`), hex with a
//! leading double-quote (`\char"44`), octal with a leading single-quote
//! (`\char'104`), or a backquote character constant (`` \char`D ``).
//! `\symbol{<number>}` is the same number braced. Each sets the glyph at
//! that slot of the current font (OT1 here: 65-68 are A-D).
//!
//! The `next.` position needs Computer Modern metrics, which only the
//! render pipeline sets; the compiler's own layout uses Core 14 Times and
//! cannot reproduce it to 0.1bp. Pinned here: zero diagnostics for the
//! oracle source, and A/B/C/D set from the encoding's slot tables (never a
//! cast: slot 12 is the `ff` ligature, slot 0 `Gamma`).
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::parser::{Block, Inline, Parsed, parse};

/// The oracle source as a full document (default OT1).
fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// The body paragraph's inlines.
fn para_inlines(parsed: Parsed) -> Vec<Inline> {
    parsed
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("one paragraph")
}

/// The body paragraph's inlines; the source must compile cleanly.
fn paragraph(source: &str) -> Vec<Inline> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    para_inlines(parsed)
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

/// The oracle source compiles with zero diagnostics, through the full
/// layout as well as the parser, and sets A, B, C, D at the four sites.
#[test]
fn oracle_source_sets_abcd_with_zero_diagnostics() {
    let source = document("a \\symbol{65} b \\char`B c \\char\"43{} d \\char 68 e next.");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let laid_out = compile_full(&source, LayoutConstraints::default());
    assert!(laid_out.diagnostics.is_empty(), "{:?}", laid_out.diagnostics);
    let inlines = para_inlines(parsed);
    let words = texts(&inlines);
    assert_eq!(
        words,
        ["a", "A", "b", "B", "c", "C", "d", "D", "e", "next."],
        "{words:?}"
    );
}

/// The four number forms agree: decimal, hex, octal and backquote.
#[test]
fn four_number_forms_agree() {
    let inlines = paragraph(&document("\\char68 \\char\"44 \\char'104 \\char`D next."));
    assert_eq!(texts(&inlines), ["D", "D", "D", "D", "next."], "{inlines:?}");
}

/// A number stops at the first non-digit; the rest typesets (`\char68e`
/// is D followed by `e`, as in TeX).
#[test]
fn number_stops_at_first_non_digit() {
    let inlines = paragraph(&document("x\\char68e next."));
    assert_eq!(texts(&inlines), ["x", "D", "e", "next."], "{inlines:?}");
}

/// The blanks terminating a constant vanish, as in TeX: `\char68 e` sets
/// `De` with no glue (confirmed by `\showbox`), while the space after the
/// `{}` in `\char"43{} d` stays ordinary glue.
#[test]
fn terminating_blanks_vanish_but_brace_space_stays() {
    let inlines = paragraph(&document("a \\symbol{65} b \\char`B c \\char\"43{} d \\char 68 e next."));
    let runs: Vec<(&str, bool, bool)> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, space_before, glue_before, .. } => {
                Some((text.as_str(), *space_before, glue_before.is_some()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        runs,
        [
            ("a", true, false),
            ("A", true, true),
            ("b", true, true),
            ("B", true, true),
            ("c", false, false),
            ("C", true, true),
            ("d", true, true),
            ("D", true, true),
            ("e", false, false),
            ("next.", true, true),
        ],
        "{inlines:?}"
    );
}

/// Slots come from the encoding's tables, never a cast: OT1 slots 11 and
/// 12 are the `ff`/`fi` ligature characters and slot 0 is `Gamma`.
#[test]
fn slots_come_from_the_encoding_table() {
    let inlines = paragraph(&document("\\char11\\char12\\char0 next."));
    assert_eq!(texts(&inlines), ["ﬀ", "ﬁ", "Γ", "next."], "{inlines:?}");
}

/// An out-of-range slot is TeX's "Bad character code": a diagnostic, the
/// number consumed, nothing set.
#[test]
fn out_of_range_slot_diagnoses_and_sets_nothing() {
    let parsed = parse(&document("x \\char300 y next."));
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        parsed.diagnostics[0].message.contains("out of range"),
        "{:?}",
        parsed.diagnostics
    );
    let inlines = para_inlines(parsed);
    let words = texts(&inlines);
    assert_eq!(words, ["x", "y", "next."], "{words:?}");
}

/// A missing number is TeX's "Missing number, treated as zero": a
/// diagnostic, slot 0 set, the offending word left for the main loop.
#[test]
fn missing_number_is_treated_as_zero() {
    let parsed = parse(&document("x \\char e next."));
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        parsed.diagnostics[0].message.contains("needs a character number"),
        "{:?}",
        parsed.diagnostics
    );
    let inlines = para_inlines(parsed);
    let words = texts(&inlines);
    assert_eq!(words, ["x", "\u{0393}", "e", "next."], "{words:?}");
}

/// `\symbol` without braces, or with a non-number inside, diagnoses (and
/// never reports "not supported": the command itself is implemented).
#[test]
fn symbol_without_a_number_diagnoses() {
    for body in ["\\symbol next.", "\\symbol{ab} next."] {
        let parsed = parse(&document(body));
        assert_eq!(parsed.diagnostics.len(), 1, "{body:?}: {:?}", parsed.diagnostics);
        assert!(
            !parsed.diagnostics[0].message.contains("is not supported"),
            "{body:?}: {:?}",
            parsed.diagnostics
        );
    }
}
