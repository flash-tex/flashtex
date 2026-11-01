//! amsthm `\qedhere`: the end-of-proof box on the current line, flush
//! right, suppressing the automatic box `\end{proof}` would append.
//! See `src/parser.rs` (`P::qedhere`, `P::raw_qedhere`) and `src/math.rs`:
//! in text the marker is the same `HFill` + U+220E pair the automatic box
//! uses (sharing the `\qedhere` span); in a display the token stays a
//! literal marker atom for the render pipeline to strip and place.

use flashtex_compiler::math::Nucleus;
use flashtex_compiler::parser::{self, Block, Inline};

/// `(HFill span, U+220E span)` for every marker pair in every paragraph.
fn marker_pairs(source: &str) -> Vec<(flashtex_compiler::Span, flashtex_compiler::Span)> {
    let inlines: Vec<Inline> = parser::parse(source)
        .blocks
        .into_iter()
        .flat_map(|block| match block {
            Block::Paragraph(inlines) => inlines,
            _ => Vec::new(),
        })
        .collect();
    inlines
        .windows(2)
        .filter_map(|pair| match (&pair[0], &pair[1]) {
            (Inline::HFill { span: a, .. }, Inline::Text { text, span: b, .. }) if text == "\u{220E}" => {
                Some((*a, *b))
            }
            _ => None,
        })
        .collect()
}

fn messages(source: &str) -> Vec<String> {
    parser::parse(source).diagnostics.into_iter().map(|d| d.message).collect()
}

fn qedhere_mentioned(source: &str) -> bool {
    messages(source).iter().any(|m| m.contains("qedhere"))
}

#[test]
fn qedhere_in_text_places_the_marker_and_suppresses_the_automatic_box() {
    let source = "\\begin{proof}Body \\qedhere\\end{proof}";
    let pairs = marker_pairs(source);
    assert_eq!(pairs.len(), 1, "one box total, not a placed one plus the automatic one: {pairs:?}");
    let (fill, glyph) = &pairs[0];
    assert_eq!(fill, glyph, "the pair shares one span, the existing marker contract");
    assert_eq!(
        &source[fill.start..fill.end],
        "\\qedhere",
        "the shared span is the command itself, not \\end{{proof}}"
    );
    assert!(!qedhere_mentioned(source), "{:?}", messages(source));
}

#[test]
fn qedhere_in_a_display_leaves_a_marker_atom_and_suppresses_the_automatic_box() {
    let source = "\\begin{proof}See \\[x \\qedhere\\]\\end{proof}";
    let parsed = parser::parse(source);
    let maths: Vec<_> = parsed
        .blocks
        .iter()
        .flat_map(|block| match block {
            Block::Paragraph(inlines) => inlines.clone(),
            _ => Vec::new(),
        })
        .filter_map(|inline| match inline {
            Inline::Math { list, display, .. } if display => Some(list),
            _ => None,
        })
        .collect();
    assert_eq!(maths.len(), 1);
    assert!(
        maths[0].atoms.iter().any(|atom| matches!(&atom.nucleus, Nucleus::Symbol(s) if s == "\\qedhere")),
        "the display keeps the literal marker for the pipeline to place: {:?}",
        maths[0].atoms
    );
    assert!(!qedhere_mentioned(source), "{:?}", messages(source));
    assert!(
        marker_pairs(source).is_empty(),
        "no automatic box follows the display"
    );
}

#[test]
fn qedhere_in_the_last_align_row_claims_the_box() {
    let source = "\\begin{proof}\\begin{align*}a &= b \\\\ c &= d \\qedhere\\end{align*}\\end{proof}";
    assert!(!qedhere_mentioned(source), "{:?}", messages(source));
    assert!(
        marker_pairs(source).is_empty(),
        "no automatic box follows the alignment"
    );
}

#[test]
fn qedhere_in_inline_math_is_dropped_but_still_claims_the_box() {
    let source = "\\begin{proof}See $x$\\qedhere done.\\end{proof}";
    assert!(!qedhere_mentioned(source), "{:?}", messages(source));
    let pairs = marker_pairs(source);
    assert_eq!(pairs.len(), 1, "{pairs:?}");
    assert_eq!(&source[pairs[0].0.start..pairs[0].0.end], "\\qedhere");
}

#[test]
fn proof_without_qedhere_keeps_the_automatic_box() {
    let source = "\\begin{proof}Body.\\end{proof}";
    let pairs = marker_pairs(source);
    assert_eq!(pairs.len(), 1, "{pairs:?}");
    assert_eq!(&source[pairs[0].0.start..pairs[0].0.end], "\\end");
}
