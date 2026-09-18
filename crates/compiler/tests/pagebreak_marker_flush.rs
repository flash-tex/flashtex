//! `\pagebreak` must flush a pending marker-only paragraph before its
//! vertical-mode penalty (GH-876).
//!
//! `\newpage`/`\clearpage` call `flush_paragraph` first; `\pagebreak` treated
//! a paragraph holding only `\label` whatsits as vertical mode and pushed
//! `Block::Penalty` straight onto `blocks`, leaving the marker-only `para`
//! unflushed so it joined the text after the break. The fix flushes that one
//! case; every test below marked "unchanged" passes identically before and
//! after it.

use flashtex_compiler::parser::{parse, Block, Inline};

fn blocks_of(source: &str) -> Vec<Block> {
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{source:?}: {:?}",
        parsed.diagnostics
    );
    parsed.blocks
}

fn label_keys(inlines: &[Inline]) -> Vec<&str> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Label { key, .. } => Some(key.as_str()),
            _ => None,
        })
        .collect()
}

fn has_text(inlines: &[Inline], needle: &str) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Text { text, .. } => text.contains(needle),
        _ => false,
    })
}

/// The fix: `\label{mk}\pagebreak` (no blank line) ships the marker-only
/// paragraph *before* the penalty, exactly as `\newpage`/`\clearpage` do.
#[test]
fn pagebreak_flushes_label_only_paragraph_before_vertical_penalty() {
    let blocks = blocks_of(r"\label{mk}\pagebreak After.");
    assert_eq!(blocks.len(), 3, "{blocks:?}");
    match &blocks[0] {
        Block::Paragraph(inlines) => assert_eq!(label_keys(inlines), ["mk"], "{blocks:?}"),
        other => panic!("marker paragraph must come first: {other:?}"),
    }
    match &blocks[1] {
        Block::Penalty { value, fil, .. } => {
            assert_eq!((*value, *fil), (-10_000, false), "{blocks:?}");
        }
        other => panic!("penalty must follow the marker paragraph: {other:?}"),
    }
    match &blocks[2] {
        Block::Paragraph(inlines) => {
            assert!(label_keys(inlines).is_empty(), "{blocks:?}");
            assert!(has_text(inlines, "After"), "{blocks:?}");
        }
        other => panic!("text after the break is its own paragraph: {other:?}"),
    }
}

/// Same arm, same bug: `\nopagebreak` after a marker-only paragraph.
#[test]
fn nopagebreak_flushes_label_only_paragraph_before_vertical_penalty() {
    let blocks = blocks_of(r"\label{mk}\nopagebreak After.");
    assert_eq!(blocks.len(), 3, "{blocks:?}");
    match &blocks[0] {
        Block::Paragraph(inlines) => assert_eq!(label_keys(inlines), ["mk"], "{blocks:?}"),
        other => panic!("marker paragraph must come first: {other:?}"),
    }
    match &blocks[1] {
        Block::Penalty { value, fil, .. } => {
            assert_eq!((*value, *fil), (10_000, false), "{blocks:?}");
        }
        other => panic!("penalty must follow the marker paragraph: {other:?}"),
    }
    match &blocks[2] {
        Block::Paragraph(inlines) => assert!(has_text(inlines, "After"), "{blocks:?}"),
        other => panic!("text after the break is its own paragraph: {other:?}"),
    }
}

/// Unchanged: real content before `\pagebreak` stays horizontal — one merged
/// paragraph carrying an inline `PagePenalty`, no vertical `Block::Penalty`.
#[test]
fn pagebreak_after_real_content_stays_horizontal() {
    let blocks = blocks_of(r"Before \pagebreak After.");
    assert_eq!(blocks.len(), 1, "{blocks:?}");
    match &blocks[0] {
        Block::Paragraph(inlines) => {
            assert!(has_text(inlines, "Before"), "{blocks:?}");
            assert!(has_text(inlines, "After"), "{blocks:?}");
            let penalties: Vec<i32> = inlines
                .iter()
                .filter_map(|inline| match inline {
                    Inline::PagePenalty { value, .. } => Some(*value),
                    _ => None,
                })
                .collect();
            assert_eq!(penalties, [-10_000], "{blocks:?}");
        }
        other => panic!("expected one merged paragraph: {other:?}"),
    }
    assert!(
        !blocks
            .iter()
            .any(|block| matches!(block, Block::Penalty { .. })),
        "{blocks:?}"
    );
}

/// Unchanged: a vertical break with nothing pending is just the penalty.
#[test]
fn pagebreak_with_empty_paragraph_is_just_a_penalty() {
    let blocks = blocks_of(r"\pagebreak After.");
    assert_eq!(blocks.len(), 2, "{blocks:?}");
    assert!(
        matches!(blocks[0], Block::Penalty { value: -10_000, .. }),
        "{blocks:?}"
    );
    match &blocks[1] {
        Block::Paragraph(inlines) => assert!(has_text(inlines, "After"), "{blocks:?}"),
        other => panic!("{other:?}"),
    }
}

/// Unchanged: a blank line already flushes the marker paragraph, so the
/// break sees an empty `para` and behaves exactly as before the fix.
#[test]
fn blank_line_between_label_and_pagebreak_is_unchanged() {
    let blocks = blocks_of("\\label{mk}\n\n\\pagebreak After.");
    assert_eq!(blocks.len(), 3, "{blocks:?}");
    match &blocks[0] {
        Block::Paragraph(inlines) => assert_eq!(label_keys(inlines), ["mk"], "{blocks:?}"),
        other => panic!("{other:?}"),
    }
    assert!(
        matches!(blocks[1], Block::Penalty { value: -10_000, .. }),
        "{blocks:?}"
    );
}

/// Unchanged: `\newpage`/`\clearpage` always flushed first; the fix only
/// teaches `\pagebreak`/`\nopagebreak` the same pattern.
#[test]
fn newpage_and_clearpage_with_label_flush_first() {
    for command in ["newpage", "clearpage"] {
        let blocks = blocks_of(&format!("\\label{{mk}}\\{command} After."));
        assert_eq!(blocks.len(), 3, "{command}: {blocks:?}");
        match &blocks[0] {
            Block::Paragraph(inlines) => {
                assert_eq!(label_keys(inlines), ["mk"], "{command}: {blocks:?}");
            }
            other => panic!("{command}: {other:?}"),
        }
        assert!(
            matches!(&blocks[1], Block::PageBreak),
            "{command}: {blocks:?}"
        );
        match &blocks[2] {
            Block::Paragraph(inlines) => {
                assert!(has_text(inlines, "After"), "{command}: {blocks:?}");
            }
            other => panic!("{command}: {other:?}"),
        }
    }
}

/// Unchanged: inside a list the pending `\item` marker still attaches to the
/// item's own paragraph — the flush is skipped there so the item is not split.
#[test]
fn label_inside_item_keeps_item_across_pagebreak() {
    let blocks = blocks_of(r"\begin{itemize}\item\label{mk}\pagebreak After.\end{itemize}");
    assert_eq!(blocks.len(), 2, "{blocks:?}");
    assert!(
        matches!(blocks[0], Block::Penalty { value: -10_000, .. }),
        "{blocks:?}"
    );
    match &blocks[1] {
        Block::ListItem {
            label,
            content,
            ..
        } => {
            assert!(label.is_some(), "{blocks:?}");
            assert_eq!(label_keys(content), ["mk"], "{blocks:?}");
            assert!(has_text(content, "After"), "{blocks:?}");
        }
        other => panic!("item paragraph must stay one item: {other:?}"),
    }
}

/// Unchanged (issue row 2/3 shape at IR level): a hand-written
/// `\thispagestyle`/`\pagestyle` marker between words leaves no inline, so a
/// same-paragraph `\pagebreak` stays on the horizontal path.
#[test]
fn thispagestyle_between_words_stays_on_the_horizontal_path() {
    for marker in ["\\thispagestyle{plain}", "\\pagestyle{empty}"] {
        let source = format!("Before {marker} \\pagebreak After.");
        let blocks = blocks_of(&source);
        assert_eq!(blocks.len(), 1, "{marker}: {blocks:?}");
        match &blocks[0] {
            Block::Paragraph(inlines) => {
                assert!(has_text(inlines, "Before"), "{marker}: {blocks:?}");
                assert!(has_text(inlines, "After"), "{marker}: {blocks:?}");
            }
            other => panic!("{marker}: {other:?}"),
        }
        assert!(
            !blocks
                .iter()
                .any(|block| matches!(block, Block::Penalty { .. })),
            "{marker}: {blocks:?}"
        );
    }
}

/// Unchanged (issue row 1 shape at IR level): `\maketitle` flushes first, so
/// the following `\pagebreak` sees an empty `para` and is just a penalty.
#[test]
fn maketitle_then_pagebreak_is_just_a_penalty_after_the_title() {
    let parsed = parse(r"\title{T}\author{A}\maketitle\pagebreak After.");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    let blocks = parsed.blocks;
    assert_eq!(blocks.len(), 3, "{blocks:?}");
    assert!(
        matches!(blocks[0], Block::TitleBlock { .. }),
        "{blocks:?}"
    );
    assert!(
        matches!(blocks[1], Block::Penalty { value: -10_000, .. }),
        "{blocks:?}"
    );
    match &blocks[2] {
        Block::Paragraph(inlines) => assert!(has_text(inlines, "After"), "{blocks:?}"),
        other => panic!("{other:?}"),
    }
}
