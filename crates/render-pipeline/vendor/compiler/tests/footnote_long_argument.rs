//! `\footnote{...}` and `\footnotetext{...}` take a `\long` argument
//! (latex.ltx: `\long\def\@footnotetext#1`): a blank line inside the note is
//! a paragraph break in the note, not the end of the argument.
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

#[test]
fn a_blank_line_inside_a_footnote_is_a_paragraph_break_in_the_note() {
    for command in ["footnote", "footnotetext"] {
        let src = doc(&format!("Before\\{command}{{First part.\n\nSecond part.}} after the note.\n\nNext paragraph."));
        let parsed = parse(&src);
        assert!(
            !parsed.diagnostics.iter().any(|d| d.message.contains("missing its closing brace") || d.message.contains("unmatched")),
            "\\{command}: {:?}",
            parsed.diagnostics
        );
        let paras = paragraphs(&parsed.blocks);
        assert_eq!(paras.len(), 2, "\\{command}: the note's blank line must not end the host paragraph");
        let host = paras[0];
        let note = host
            .iter()
            .find_map(|i| match i {
                Inline::Footnote { text: Some(t), .. } => Some(t),
                _ => None,
            })
            .expect("a footnote with text");
        let note_text = texts(note);
        assert!(note_text.contains("First") && note_text.contains("Second"), "\\{command}: note text {note_text:?}");
        assert!(note.iter().any(|i| matches!(i, Inline::LineBreak { .. })), "\\{command}: the note keeps its paragraph break");
        let host_text = texts(host);
        assert!(host_text.contains("after") && !host_text.contains("Second"), "\\{command}: host text {host_text:?}");
        assert!(texts(paras[1]).contains("Next"));
    }
}

#[test]
fn an_unclosed_footnote_is_still_closed_at_the_end_of_its_paragraph() {
    let parsed = parse(&doc("Before\\footnote{never closed\n\nNext paragraph."));
    assert!(
        parsed.diagnostics.iter().any(|d| d.message.contains("missing its closing brace")),
        "{:?}",
        parsed.diagnostics
    );
    assert!(paragraphs(&parsed.blocks).iter().any(|p| texts(p).contains("Next")));
}
