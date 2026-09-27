//! Command arguments that name something (a `\label` key, a hyperref
//! destination, a contents-list entry) are not text.
//!
//! pdflatex, TeX Live 2026, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`,
//! `article`, word origins:
//!
//! * `\section{Some text \label{x} in the middle}` sets `in` at 235.02bp,
//!   exactly where `\section{Some text in the middle}` sets it. FlashTeX set
//!   the key: `x` at 235.02bp and `in` 13.90bp right. A `\ref` to the key
//!   is the section number.
//! * `Some text \hypertarget{t}{} in the middle` sets `in` at 199.35bp, two
//!   interword glues after `text` (the empty `{}` sets nothing). FlashTeX
//!   set `t` at 196.04bp and `in` 3.88bp right.
//! * `Some text \addcontentsline{toc}{section}{x} in the middle` sets `in`
//!   at 199.35bp, two glues too (`\protected@write` has no `\@bsphack`).
//!   FlashTeX set `in` 3.32bp left.
use flashtex_compiler::parser::{parse, Block, Inline};

fn blocks(preamble: &str, body: &str) -> Vec<Block> {
    parse(&format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n")).blocks
}

fn words(inlines: &[Inline]) -> Vec<String> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } if !text.is_empty() => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Interword glues (`glue_before` of every run, the empty ones included).
fn glues(inlines: &[Inline]) -> usize {
    inlines.iter().filter(|inline| matches!(inline, Inline::Text { glue_before: Some(_), .. })).count()
}

fn first_paragraph(blocks: &[Block]) -> &[Inline] {
    blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines.as_slice()),
            _ => None,
        })
        .expect("a paragraph")
}

fn heading(blocks: &[Block]) -> &[Inline] {
    blocks
        .iter()
        .find_map(|block| match block {
            Block::Heading { content, .. } => Some(content.as_slice()),
            _ => None,
        })
        .expect("a heading")
}

fn labels(inlines: &[Inline]) -> Vec<(String, String)> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Label { key, value, .. } => Some((key.clone(), value.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_label_in_a_section_title_registers_and_is_not_text() {
    let parsed = blocks("", "\\section{Intro}\n\\section{Some text \\label{x} in the middle}\nSee \\ref{x}.");
    let title = heading(&parsed[1..]);
    assert_eq!(words(title), ["Some", "text", "in", "the", "middle"]);
    // `\@bsphack`/`\@esphack`: one glue between `text` and `in`.
    assert_eq!(glues(title), 4, "{title:?}");
    assert_eq!(labels(title), [("x".to_string(), "2".to_string())]);
    let parsed = blocks("", "\\section{Methods\\label{sec:m}}");
    assert_eq!(words(heading(&parsed)), ["Methods"]);
    assert_eq!(labels(heading(&parsed)), [("sec:m".to_string(), "1".to_string())]);
    let parsed = blocks("", "\\subsection{Sub \\label{s}}");
    assert_eq!(words(heading(&parsed)), ["Sub"]);
}

#[test]
fn a_label_in_a_chapter_title_registers_and_is_not_text() {
    let parsed =
        parse("\\documentclass{report}\n\\begin{document}\n\\chapter{Start\\label{ch}}\nText.\n\\end{document}\n").blocks;
    let debug = format!("{parsed:?}");
    assert!(debug.contains("text: \"Start\""), "{debug}");
    assert!(!debug.contains("text: \"ch\""), "{debug}");
    assert!(debug.contains("Label { key: \"ch\", value: \"1\""), "{debug}");
}

#[test]
fn hyperref_destination_names_are_not_text() {
    let parsed = blocks("\\usepackage{hyperref}\n", "A \\hypertarget{t}{} B \\hypertarget{u}{Here} C \\hyperlink{t}{Go} D.");
    let para = first_paragraph(&parsed);
    assert_eq!(words(para), ["A", "B", "Here", "C", "Go", "D."]);
    // `{}` sets nothing, so both spaces around it are glue; the two
    // commands with text have one on each side.
    assert_eq!(glues(para), 6, "{para:?}");
}

#[test]
fn contents_entries_are_not_text() {
    let parsed = blocks("", "Some text \\addcontentsline{toc}{section}{x} in \\addtocontents{toc}{y} the middle.");
    let para = first_paragraph(&parsed);
    assert_eq!(words(para), ["Some", "text", "in", "the", "middle."]);
    assert_eq!(glues(para), 6, "{para:?}");
    let parsed = blocks("", "Before.\n\n\\addcontentsline{toc}{section}{Extra}\n\nAfter.");
    let all: Vec<String> = parsed
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(words(inlines)),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(all, ["Before.", "After."]);
}
