//! GH-760: a control symbol's identity (`\=` vs `=`, `\,` vs `,`) comes from
//! `Token::control_symbol_char`, never from span width or definition bytes.
//!
//! These pin copies whose spans belong to other text (`\edef`, a macro
//! calling a macro) plus the plain-character direction the width test got
//! wrong. Every expected shape below was measured with pdflatex (see PR).

use flashtex_compiler::parser::{self, Block, Inline};

fn paragraph_inlines(src: &str) -> Vec<Inline> {
    parser::parse(src)
        .blocks
        .into_iter()
        .flat_map(|block| match block {
            Block::Paragraph(content) => content,
            _ => Vec::new(),
        })
        .collect()
}

fn texts(inlines: &[Inline]) -> Vec<String> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn kerns(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .filter(|inline| matches!(inline, Inline::Kern { .. }))
        .count()
}

/// The first tabbing line's tab-stop count and its texts.
fn first_tabbing_line(src: &str) -> (usize, Vec<String>) {
    let parsed = parser::parse(src);
    let lines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Tabbing { lines, .. } => Some(lines.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no Block::Tabbing in {:#?}", parsed.blocks));
    let line = &lines[0];
    let stops = line
        .content
        .iter()
        .filter(|inline| matches!(inline, Inline::TabStop { .. }))
        .count();
    (stops, texts(&line.content))
}

fn tabbing(prefix: &str, first_row: &str) -> String {
    format!("{prefix}\\begin{{tabbing}}\n{first_row}\\\\\nzz\\>ww\n\\end{{tabbing}}\n")
}

// pdflatex: every one of these sets a stop and prints no `=` (`xxyy`, then
// `zz ww` with `ww` under `yy`).

#[test]
fn edef_copied_tab_stop_sets_a_stop() {
    let (stops, texts) =
        first_tabbing_line(&tabbing("\\edef\\tsE{\\noexpand\\=}", "xx\\tsE yy"));
    assert_eq!((stops, texts), (1, vec!["xx".into(), "yy".into()]));
}

#[test]
fn nested_macro_tab_stop_sets_a_stop() {
    let (stops, texts) =
        first_tabbing_line(&tabbing("\\def\\tsA{\\=}\\def\\tsB{\\tsA}", "xx\\tsB yy"));
    assert_eq!((stops, texts), (1, vec!["xx".into(), "yy".into()]));
}

#[test]
fn plain_equals_from_a_two_byte_macro_is_typeset_in_tabbing() {
    // pdflatex prints `xx=yy` for `\newcommand{\q}{=}` inside tabbing.
    let (stops, texts) = first_tabbing_line(&tabbing("\\newcommand{\\q}{=}", "xx\\q yy"));
    assert_eq!(stops, 0);
    assert_eq!(texts.concat(), "xx=yy");
}

// pdflatex: `a\tcE b` (\tcE = edef'd `\,`) sets `a b` with a thin space
// and no comma.

#[test]
fn edef_copied_thin_space_is_a_kern() {
    let inlines = paragraph_inlines("\\edef\\tcE{\\noexpand\\,}a\\tcE b");
    assert_eq!(kerns(&inlines), 1, "{inlines:#?}");
    assert!(!texts(&inlines).concat().contains(','), "{inlines:#?}");
}

#[test]
fn plain_comma_from_a_two_byte_macro_is_typeset() {
    // pdflatex prints `a,b` for `\newcommand{\z}{,}` then `a\z b`.
    let inlines = paragraph_inlines("\\newcommand{\\z}{,}a\\z b");
    assert_eq!(kerns(&inlines), 0, "{inlines:#?}");
    assert!(texts(&inlines).concat().contains(','), "{inlines:#?}");
}
