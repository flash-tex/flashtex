//! `\setcounter`/`\addtocounter` on `enumi`..`enumiv` inside a nested
//! `enumerate` retarget the open list's counter, so the next `\item` at that
//! nesting level steps from the assigned value (latex.ltx: `\enumerate`
//! opens with `\usecounter{enum<i>}`, and `\item` without `[<label>]`
//! `\refstepcounter`s it; assigning `\c@enum<i>` therefore moves the next
//! label). See `src/parser/lists.rs` for the list provenance.

use flashtex_compiler::parser::{self, Block, ItemLabel};

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn label_texts(source: &str) -> Vec<String> {
    parser::parse(source)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem {
                item: Some(item), ..
            } => Some(item.text().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn setcounter_enumi_moves_the_next_outer_label() {
    let source = doc(concat!(
        "\\begin{enumerate}\n",
        "\\item one\n",
        "\\begin{enumerate}\n",
        "\\item inner a\n",
        "\\end{enumerate}\n",
        "\\setcounter{enumi}{6}\n",
        "\\item seven\n",
        "\\item eight\n",
        "\\end{enumerate}\n",
    ));
    assert_eq!(label_texts(&source), ["1.", "(a)", "7.", "8."]);
}

#[test]
fn addtocounter_enumi_adds_relatively() {
    let source = doc(concat!(
        "\\begin{enumerate}\n",
        "\\item one\n",
        "\\item two\n",
        "\\addtocounter{enumi}{2}\n",
        "\\item five\n",
        "\\end{enumerate}\n",
    ));
    assert_eq!(label_texts(&source), ["1.", "2.", "5."]);
}

#[test]
fn setcounter_on_one_level_leaves_nested_levels_alone() {
    // Assigning the inner level moves only inner labels; the outer level
    // keeps its own count.
    let source = doc(concat!(
        "\\begin{enumerate}\n",
        "\\item one\n",
        "\\begin{enumerate}\n",
        "\\item inner a\n",
        "\\setcounter{enumii}{3}\n",
        "\\item inner d\n",
        "\\end{enumerate}\n",
        "\\item two\n",
        "\\end{enumerate}\n",
    ));
    assert_eq!(label_texts(&source), ["1.", "(a)", "(d)", "2."]);
    // Assigning the outer level while the inner list is open does not move
    // the inner labels; the outer label steps from the assigned value once
    // the inner list has closed.
    let source = doc(concat!(
        "\\begin{enumerate}\n",
        "\\item one\n",
        "\\begin{enumerate}\n",
        "\\item inner a\n",
        "\\setcounter{enumi}{6}\n",
        "\\item inner b\n",
        "\\end{enumerate}\n",
        "\\item seven\n",
        "\\end{enumerate}\n",
    ));
    assert_eq!(label_texts(&source), ["1.", "(a)", "(b)", "7."]);
    // A level-2 regression check on the label shape itself.
    let parsed = parser::parse(&source);
    let inners: Vec<String> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem {
                item: Some(ItemLabel::Counter { value, .. }),
                ..
            } if *value == 2 => Some(item_text(block)),
            _ => None,
        })
        .collect();
    assert_eq!(inners, ["(b)"]);
}

#[test]
fn setcounter_enumiii_moves_only_the_third_level() {
    let source = doc(concat!(
        "\\begin{enumerate}\n",
        "\\item one\n",
        "\\begin{enumerate}\n",
        "\\item inner a\n",
        "\\begin{enumerate}\n",
        "\\item deep i\n",
        "\\setcounter{enumiii}{4}\n",
        "\\item deep v\n",
        "\\end{enumerate}\n",
        "\\item inner b\n",
        "\\end{enumerate}\n",
        "\\item two\n",
        "\\end{enumerate}\n",
    ));
    assert_eq!(label_texts(&source), ["1.", "(a)", "i.", "v.", "(b)", "2."]);
}

fn item_text(block: &Block) -> String {
    match block {
        Block::ListItem {
            item: Some(item), ..
        } => item.text().to_string(),
        _ => String::new(),
    }
}
