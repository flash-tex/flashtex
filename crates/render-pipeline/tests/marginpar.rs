//! `\marginpar` margin notes (behind the `marginpar` cargo feature, which is
//! off until vendor/compiler is re-pinned past the parser change): the note
//! renders in the right margin, roughly vertically aligned with its anchor,
//! without overlapping the main text column.

#![cfg(feature = "marginpar")]

mod common;

#[test]
fn marginpar_renders_in_the_right_margin_without_overlapping_the_column() {
    if !common::lm_available() {
        eprintln!("SKIP marginpar: Latin Modern fonts not installed");
        return;
    }
    let r = common::render_one(
        "\\documentclass{article}\\begin{document}Text\\marginpar{note}\\end{document}",
    );
    let unsupported: Vec<_> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("\\marginpar is not supported"))
        .collect();
    assert!(unsupported.is_empty(), "{unsupported:?}");
    let words = common::words_of(&r);
    let body = words
        .iter()
        .find(|w| w.text == "Text")
        .expect("body word");
    let note = words
        .iter()
        .find(|w| w.text == "note")
        .expect("margin note");
    assert_eq!(note.page, body.page, "{words:?}");
    // Roughly vertically aligned with the anchor: the note's first
    // baseline sits on the anchor line's baseline.
    assert!(
        (note.baseline - body.baseline).abs() <= 2.0,
        "note at {} vs body at {}",
        note.baseline,
        body.baseline
    );
    // In the margin, not overlapping the main text column: the note
    // starts right of where the body word ends.
    assert!(
        note.x >= body.x + body.width,
        "note at {} overlaps body ending at {}",
        note.x,
        body.x + body.width
    );
}
