//! `\marginpar[<left>]{<right>}` (GH-TABLE2-UNTESTED-7): the compiler parses
//! a real margin note (`Inline::Marginpar`), always set in the right margin,
//! rather than reporting the command as unsupported.
use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn texts(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn paragraphs(blocks: &[Block]) -> Vec<&Vec<Inline>> {
    blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .collect()
}

fn margin_notes(blocks: &[Block]) -> Vec<&Vec<Inline>> {
    blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flat_map(|inlines| {
            inlines.iter().filter_map(|i| match i {
                Inline::Marginpar { text, .. } => Some(text),
                _ => None,
            })
        })
        .collect()
}

#[test]
fn marginpar_note_is_kept_as_a_margin_note_without_diagnostics() {
    let parsed = parse(&doc("Text\\marginpar{note text} more."));
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    let notes = margin_notes(&parsed.blocks);
    assert_eq!(notes.len(), 1, "{:?}", parsed.blocks);
    assert_eq!(texts(notes[0]), "note text");
}

#[test]
fn marginpar_optional_left_argument_is_warned_and_ignored() {
    let parsed = parse(&doc("Text\\marginpar[left note]{right note} more."));
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error),
        "the [left] form must produce no error diagnostics: {:?}",
        parsed.diagnostics
    );
    let warnings: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    assert_eq!(warnings.len(), 1, "{:?}", parsed.diagnostics);
    assert_eq!(
        warnings[0].message,
        "\\marginpar's optional [left] argument is ignored: the note is always set in the right margin"
    );
    // The required `{right}` argument is the note that is kept.
    let notes = margin_notes(&parsed.blocks);
    assert_eq!(notes.len(), 1, "{:?}", parsed.blocks);
    let note_text = texts(notes[0]);
    assert!(
        note_text.contains("right note"),
        "note text {note_text:?}"
    );
    assert!(
        !note_text.contains("left"),
        "the ignored [left] argument must not reach the note: {note_text:?}"
    );
}

#[test]
fn a_blank_line_inside_a_marginpar_is_a_paragraph_break_in_the_note() {
    // Like `\@footnotetext`, the argument is `\long`: a blank line inside it
    // is a paragraph break in the note, not the end of the argument.
    let parsed = parse(&doc(
        "Before\\marginpar{First part.\n\nSecond part.} after the note.\n\nNext paragraph.",
    ));
    assert!(
        !parsed.diagnostics.iter().any(|d| d.message.contains("missing its closing brace") || d.message.contains("unmatched")),
        "{:?}",
        parsed.diagnostics
    );
    let paras = paragraphs(&parsed.blocks);
    assert_eq!(
        paras.len(),
        2,
        "the note's blank line must not end the host paragraph"
    );
    let note = margin_notes(&parsed.blocks);
    assert_eq!(note.len(), 1, "{:?}", parsed.blocks);
    let note_text = texts(note[0]);
    assert!(
        note_text.contains("First") && note_text.contains("Second"),
        "note text {note_text:?}"
    );
    assert!(
        note[0].iter().any(|i| matches!(i, Inline::LineBreak { .. })),
        "the note keeps its paragraph break"
    );
    let host_text = texts(paras[0]);
    assert!(
        host_text.contains("after") && !host_text.contains("Second"),
        "host text {host_text:?}"
    );
    assert!(texts(paras[1]).contains("Next"));
}
