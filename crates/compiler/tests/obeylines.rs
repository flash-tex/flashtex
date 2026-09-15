//! `\obeylines` (LaTeX2e kernel): every source newline ends the line, like
//! `\\`, for the rest of the enclosing group.
//!
//! Real-TeX semantics pinned here (verified against the kernel behaviour,
//! not guessed):
//! - A lone newline breaks the line. The compiler's lexer folds a lone
//!   newline into `TokenKind::Space`, so the parser recovers it from the
//!   space token's own source bytes and lowers it to the same
//!   `Inline::LineBreak` a literal `\\` produces — no new render-pipeline
//!   primitive.
//! - A blank line is still `\par`: the lexer already emits one `ParBreak`
//!   for the whole run, exactly as two newlines in a row mean `\par` in
//!   ordinary TeX regardless of `\obeylines`. (Mechanically TeX sees
//!   `\par\par` there and the second is a vertical-mode no-op; the
//!   observable result — one paragraph break, no empty line — is the same.)
//! - Spaces around the newline vanish with the break. Preserving them is
//!   `\obeyspaces`' separate job, out of scope here.
//! - The effect is group-scoped (`{...}` or `\begin...\end`), reusing the
//!   `declared_alignment` save/restore template.

use flashtex_compiler::layout;
use flashtex_compiler::parser::{parse, Block, Inline};

/// The document's text paragraphs/verse blocks as plain inline sequences.
fn paragraphs(text: &str) -> Vec<Vec<Inline>> {
    let parsed = parse(text);
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("not supported")),
        "unexpected unsupported diagnostic: {:?}",
        parsed.diagnostics
    );
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines)
            | Block::Styled {
                content: inlines, ..
            }
            | Block::ListItem {
                content: inlines, ..
            } => Some(inlines.clone()),
            _ => None,
        })
        .collect()
}

/// Just the word text of one inline sequence, in order.
fn words(inlines: &[Inline]) -> Vec<&str> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn line_breaks(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .filter(|inline| matches!(inline, Inline::LineBreak { .. }))
        .count()
}

/// Baselines carrying each word, in document order.
fn word_lines(text: &str) -> Vec<(String, f64)> {
    let parsed = parse(text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    layout::layout(&parsed.blocks)
        .into_iter()
        .flat_map(|page| page.items)
        .map(|item| (item.text, item.baseline_y_pt))
        .collect()
}

const PREAMBLE: &str = "\\documentclass{article}\n";

#[test]
fn each_source_line_breaks_like_a_forced_break() {
    let text = format!(
        "{PREAMBLE}{}",
        "\\begin{document}\n{\\obeylines\nalpha\nbeta\ngamma\\par}\n\\end{document}\n"
    );
    let blocks = paragraphs(&text);
    assert_eq!(blocks.len(), 1);
    assert_eq!(words(&blocks[0]), vec!["alpha", "beta", "gamma"]);
    // One `\\`-equivalent break per newline, with no skip of its own.
    assert_eq!(line_breaks(&blocks[0]), 2);
    assert!(blocks[0].iter().all(|inline| match inline {
        Inline::LineBreak { skip_pt, .. } => skip_pt.is_none(),
        _ => true,
    }));
}

#[test]
fn broken_lines_land_on_their_own_output_lines() {
    let obeyed = format!(
        "{PREAMBLE}{}",
        "\\begin{document}\n{\\obeylines\nalpha\nbeta\ngamma\\par}\n\\end{document}\n"
    );
    let filled = format!(
        "{PREAMBLE}{}",
        "\\begin{document}\nalpha beta gamma\\par\n\\end{document}\n"
    );
    let baselines = |text: &str| {
        word_lines(text)
            .into_iter()
            .filter(|(text, _)| matches!(text.as_str(), "alpha" | "beta" | "gamma"))
            .map(|(_, y)| y)
            .collect::<Vec<_>>()
    };
    let broken = baselines(&obeyed);
    assert_eq!(broken.len(), 3);
    assert!(
        broken[0] < broken[1] && broken[1] < broken[2],
        "each line on its own output line: {broken:?}"
    );
    let joined = baselines(&filled);
    assert_eq!(joined.len(), 3);
    assert!(
        joined[0] == joined[1] && joined[1] == joined[2],
        "without \\obeylines the words fill onto one line: {joined:?}"
    );
}

#[test]
fn a_blank_line_still_starts_a_new_paragraph() {
    // A blank line is two newlines in a row, i.e. `\par` in ordinary TeX,
    // regardless of `\obeylines` — which only redefines what a *single*
    // newline means. So no break node appears and the text either side
    // becomes two paragraphs, staying in force across the blank line.
    let text = format!(
        "{PREAMBLE}{}",
        "\\begin{document}\n{\\obeylines\nalpha\n\nbeta\\par}\n\\end{document}\n"
    );
    let blocks = paragraphs(&text);
    assert_eq!(blocks.len(), 2);
    assert_eq!(words(&blocks[0]), vec!["alpha"]);
    assert_eq!(words(&blocks[1]), vec!["beta"]);
    assert_eq!(line_breaks(&blocks[0]), 0);
    assert_eq!(line_breaks(&blocks[1]), 0);
}

#[test]
fn the_effect_ends_where_the_group_ends() {
    let text = format!(
        "{PREAMBLE}{}",
        "\\begin{document}\n{\\obeylines\nalpha\nbeta\\par}\ngamma\ndelta\\par\n\\end{document}\n"
    );
    let blocks = paragraphs(&text);
    assert_eq!(blocks.len(), 2);
    assert_eq!(line_breaks(&blocks[0]), 1);
    // After the `}` the newline is an ordinary space again: one paragraph.
    assert_eq!(words(&blocks[1]), vec!["gamma", "delta"]);
    assert_eq!(line_breaks(&blocks[1]), 0);
}

#[test]
fn the_effect_ends_at_the_environments_end() {
    let text = format!(
        "{PREAMBLE}{}",
        "\\begin{document}\n\\begin{quote}\\obeylines\nalpha\nbeta\\par\\end{quote}\ngamma\ndelta\\par\n\\end{document}\n"
    );
    let blocks = paragraphs(&text);
    assert_eq!(blocks.len(), 2);
    assert_eq!(line_breaks(&blocks[0]), 1);
    assert_eq!(words(&blocks[1]), vec!["gamma", "delta"]);
    assert_eq!(line_breaks(&blocks[1]), 0);
}

#[test]
fn surrounding_spaces_are_not_preserved() {
    // Leading/trailing spaces on each line vanish with the break; keeping
    // them would be `\obeyspaces`, which stays unimplemented.
    let text = format!(
        "{PREAMBLE}{}",
        "\\begin{document}\n{\\obeylines\n   alpha   \n   beta\\par}\n\\end{document}\n"
    );
    let blocks = paragraphs(&text);
    assert_eq!(blocks.len(), 1);
    assert_eq!(words(&blocks[0]), vec!["alpha", "beta"]);
    assert_eq!(line_breaks(&blocks[0]), 1);
}

#[test]
fn spaces_from_a_macro_body_do_not_break() {
    // Only newly scanned source newlines obey: whitespace arriving via macro
    // replacement text was tokenised before `\\obeylines` could apply, so it
    // stays a space — even when the definition itself spans source lines.
    let text = format!(
        "{PREAMBLE}{}",
        "\\newcommand{\\twowords}{alpha\nbeta}\n\\begin{document}\n{\\obeylines \\twowords\\par}\n\\end{document}\n"
    );
    let blocks = paragraphs(&text);
    assert_eq!(blocks.len(), 1);
    assert_eq!(words(&blocks[0]), vec!["alpha", "beta"]);
    assert_eq!(line_breaks(&blocks[0]), 0);
}
