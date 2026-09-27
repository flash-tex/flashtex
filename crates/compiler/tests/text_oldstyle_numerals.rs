//! Text-mode `\oldstylenums{digits}`: old-style numerals.
//!
//! Stated pdflatex oracle (TeX Live 2026, `\documentclass{article}` at
//! 10pt, exit code 0, zero `!` errors) for
//! `A \oldstylenums{1234567890} B next.`: pdflatex `\showbox` shows the
//! digits set as `....\TS1/cmr/m/n/10 1` through `....\TS1/cmr/m/n/10 0`
//! (latex.ltx `\oldstylenums` selects the TS1 encoding subset for the
//! argument, i.e. `\textzerooldstyle`..`\textnineoldstyle`), with ordinary
//! interword glue around them. The surrounding `B next.` positions are
//! unchanged by the fix: lining and old-style digits share their widths.
//!
//! The compiler records that TS1 selection as [`TextStyle::oldstyle`] on
//! the argument's runs (inherited through nested groups and commands, like
//! any other style property); the render pipeline reads it when choosing
//! the run's font. What is pinned here: zero diagnostics for the oracle
//! source, the mark on exactly the argument's runs, unchanged advances —
//! and, by glyph name rather than by eye, that the selected slots are the
//! old-style ones (`zerooldstyle`..`nineoldstyle` in TS1/cmr) rather than
//! the lining ones (`zero`..`nine` in OT1/cmr).
use flashtex_compiler::incremental::{CompileOutput, compile_full, LayoutConstraints};
use flashtex_compiler::parser::{Block, Inline, Parsed, parse};
use flashtex_tex_text_encoding::fonts::glyph_name;

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

fn texts(inlines: &[Inline]) -> Vec<&str> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn oldstyle_marks(inlines: &[Inline]) -> Vec<(&str, bool)> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } => Some((text.as_str(), style.oldstyle)),
            _ => None,
        })
        .collect()
}

/// The oracle source compiles with zero diagnostics, through the full
/// layout as well as the parser.
#[test]
fn oracle_source_has_zero_diagnostics() {
    let source = document("A \\oldstylenums{1234567890} B next.");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let laid_out = compile_full(&source, LayoutConstraints::default());
    assert!(laid_out.diagnostics.is_empty(), "{:?}", laid_out.diagnostics);
}

/// Exactly the argument's runs carry the old-style mark: the digits, not
/// the words around them, and the mark does not leak past the group.
#[test]
fn mark_covers_exactly_the_argument() {
    let inlines = para_inlines(parse(&document("A \\oldstylenums{1234567890} B next.")));
    assert_eq!(
        oldstyle_marks(&inlines),
        [
            ("A", false),
            ("1234567890", true),
            ("B", false),
            ("next.", false),
        ],
        "{inlines:?}"
    );
    assert_eq!(texts(&inlines), ["A", "1234567890", "B", "next."], "{inlines:?}");
}

/// Nested content inherits the mark (the kernel switches the whole group),
/// while `\normalfont` inside leaves TS1 and clears it.
#[test]
fn nested_content_inherits_but_normalfont_clears() {
    let inlines = para_inlines(parse(&document("A \\oldstylenums{1\\textbf{2}3} B next.")));
    assert_eq!(
        oldstyle_marks(&inlines),
        [("A", false), ("1", true), ("2", true), ("3", true), ("B", false), ("next.", false),],
        "{inlines:?}"
    );
    let inlines = para_inlines(parse(&document("A \\oldstylenums{1\\normalfont 2} B next.")));
    assert_eq!(
        oldstyle_marks(&inlines),
        [("A", false), ("1", true), ("2", false), ("B", false), ("next.", false),],
        "{inlines:?}"
    );
}

/// By glyph name: the slots the mark selects (TS1/cmr, `tcrm1000`) are the
/// old-style variants, while the unmarked slots (OT1/cmr, `cmr10`) are the
/// lining ones. This is the slot identity the pipeline's font choice must
/// preserve — checked here rather than by eye.
#[test]
fn marked_slots_are_the_oldstyle_glyphs_by_name() {
    let oldstyle = ["zerooldstyle", "oneoldstyle", "twooldstyle", "threeoldstyle", "fouroldstyle",
        "fiveoldstyle", "sixoldstyle", "sevenoldstyle", "eightoldstyle", "nineoldstyle"];
    let lining = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];
    for (digit, (want_old, want_lining)) in oldstyle.iter().zip(lining.iter()).enumerate() {
        let slot = 0x30 + digit as u8;
        assert_eq!(glyph_name("tcrm1000.tfm", slot), Some(*want_old), "slot {slot:#x}");
        assert_eq!(glyph_name("cmr10.tfm", slot), Some(*want_lining), "slot {slot:#x}");
    }
}

/// The mark changes no advance (lining and old-style share widths): the
/// marked digits lay out exactly as the same digits under `\textbf`, which
/// splices its argument the same way. So no position moves.
#[test]
fn marked_digits_keep_their_advances() {
    let marked = compile_full(
        &document("A \\oldstylenums{1234567890} B next."),
        LayoutConstraints::default(),
    );
    let bold = compile_full(
        &document("A \\textbf{1234567890} B next."),
        LayoutConstraints::default(),
    );
    assert!(marked.diagnostics.is_empty(), "{:?}", marked.diagnostics);
    let words = |out: &CompileOutput| {
        out.pages
            .iter()
            .flat_map(|page| page.items.iter())
            .map(|item| (item.text.clone(), item.x_pt))
            .collect::<Vec<_>>()
    };
    let (marked_words, bold_words) = (words(&marked), words(&bold));
    assert_eq!(marked_words.len(), bold_words.len(), "{marked_words:?} vs {bold_words:?}");
    for ((marked_text, marked_x), (bold_text, bold_x)) in marked_words.iter().zip(bold_words.iter()) {
        assert_eq!(marked_text, bold_text);
        assert!((marked_x - bold_x).abs() < 0.005, "{marked_words:?} vs {bold_words:?}");
    }
}
