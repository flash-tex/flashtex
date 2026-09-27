//! Text-mode `\@`: the end-of-sentence mark (control space).
//!
//! Stated pdflatex oracle (TeX Live 2026, `\documentclass{article}` at
//! 10pt, exit code 0, zero `!` errors) for
//! `Written by NASA\@. Then more text` and `Also I\@. Then more.`:
//! `\@` is `\spacefactor\@m{}` (latex.ltx 9422), so the period after the
//! capital letter ends a sentence (pdflatex `\showbox` shows the wider
//! sentence glue `....\glue 4.44444 plus 4.99997 minus 0.37036` after the
//! `.`), and no literal `@` glyph is set. The oracle pins the first `Then`
//! at x 234.829 and the second at 306.908 (glyph origins in bp from the
//! page's top-left).
//!
//! The sentence glue itself is laid by the render pipeline, which tracks
//! the space factor per word: capitals set 999 (so `NASA.` alone would
//! stay an abbreviation at 1000), while a factor of 1000 or more before a
//! period yields 3000 and the wider glue. The compiler's contract, pinned
//! here: zero diagnostics, no `@` text inline anywhere, and an empty hbox
//! between the capital and the period — a zero-width box, which resets the
//! pipeline's factor to 1000 exactly as `\@` does (and breaks the
//! ligature/kern program across it, as the `{}` in its definition does).
//! The `Then` positions need Computer Modern metrics, which only the
//! pipeline sets; the compiler's own layout uses Core 14 Times and a flat
//! word space, so it cannot reproduce sentence glue at all.
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::parser::{Block, Inline, Parsed, parse};

/// The oracle sources as full documents (default OT1).
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

/// Both oracle sources compile with zero diagnostics, set no literal `@`,
/// and put an empty hbox between the capital and the period.
#[test]
fn at_mark_sets_sentence_period_with_zero_diagnostics() {
    for body in ["Written by NASA\\@. Then more text.", "Also I\\@. Then more."] {
        let source = document(body);
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{body:?}: {:?}", parsed.diagnostics);
        let laid_out = compile_full(&source, LayoutConstraints::default());
        assert!(laid_out.diagnostics.is_empty(), "{body:?}: {:?}", laid_out.diagnostics);
        let inlines = para_inlines(parsed);
        assert!(
            !texts(&inlines).iter().any(|word| word.contains('@')),
            "{body:?}: {inlines:?}"
        );
        let mark = inlines.iter().position(|inline| {
            matches!(inline, Inline::HBox(boxed) if boxed.content.is_empty())
        });
        let period = inlines.iter().position(|inline| {
            matches!(inline, Inline::Text { text, .. } if text == ".")
        });
        let (mark, period) = (mark.expect("hbox mark"), period.expect("period"));
        assert_eq!(mark + 1, period, "{body:?}: {inlines:?}");
    }
}

/// The mark sits directly after the capital: `NASA`, hbox, `.`, then the
/// glued `Then` — the shape the pipeline reads as factor 1000, 3000,
/// sentence glue.
#[test]
fn mark_sits_between_capital_and_period() {
    let parsed = parse(&document("Written by NASA\\@. Then more text."));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let inlines = para_inlines(parsed);
    assert!(matches!(&inlines[2], Inline::Text { text, .. } if text == "NASA"), "{inlines:?}");
    assert!(matches!(&inlines[3], Inline::HBox(boxed) if boxed.content.is_empty()), "{inlines:?}");
    assert!(
        matches!(&inlines[4], Inline::Text { text, space_before: false, .. } if text == "."),
        "{inlines:?}"
    );
    assert!(
        matches!(&inlines[5], Inline::Text { text, .. } if text == "Then"),
        "{inlines:?}"
    );
}

/// A following space still glues (`a\@ b` keeps its space: after a control
/// symbol TeX does not skip blanks), and a mark before any word sets
/// nothing at all (it never starts a paragraph on its own).
#[test]
fn surrounding_spaces_and_paragraph_start() {
    let parsed = parse(&document("a\\@ b next."));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let inlines = para_inlines(parsed);
    assert!(matches!(&inlines[1], Inline::HBox(boxed) if boxed.content.is_empty()), "{inlines:?}");
    assert!(
        matches!(&inlines[2], Inline::Text { text, glue_before: Some(_), .. } if text == "b"),
        "{inlines:?}"
    );
    let parsed = parse(&document("\\@. B next."));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let inlines = para_inlines(parsed);
    assert!(
        !inlines.iter().any(|inline| matches!(inline, Inline::HBox(_))),
        "{inlines:?}"
    );
    assert!(!texts(&inlines).iter().any(|word| word.contains('@')), "{inlines:?}");
}
