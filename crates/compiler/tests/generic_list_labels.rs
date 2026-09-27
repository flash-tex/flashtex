//! latex.ltx's generic `\begin{list}{<label>}{<decl>}`, the parts the
//! render pipeline cannot recover on its own:
//!
//! * `<label>` is `\@itemlabel`, which `\@mklab` sets in text mode for every
//!   `\item` without `[...]`: math in it (`{$\star$}`, the common case) is
//!   label content, as in an explicit `\item[$\star$]`, not a flattened
//!   `⋆` the text font has no glyph for.
//! * `\usecounter{<ctr>}` in `<decl>` numbers the items: each `\item` runs
//!   `\refstepcounter{<ctr>}`, so `{\arabic{enumi}.}` reads `1.`, `2.`, ...
//!   and `\label` takes `\theenumi`. pdflatex (TL2026) prints `1.` and `2.`;
//!   this compiler used to report `\usecounter` unknown and print `0.` twice.
//! * `\end{list}` is `\endtrivlist` (`\@endparenv`): text right after it,
//!   with no blank line, continues without its indent (`\@endpe`), and a
//!   following `\begin` is read in vertical mode.

use flashtex_compiler::parser::{self, Block, Inline, ItemLabel};

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn labels(parsed: &parser::Parsed) -> Vec<ItemLabel> {
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { item, .. } => item.clone(),
            _ => None,
        })
        .collect()
}

#[test]
fn a_math_default_label_is_label_content() {
    let source = doc("\\begin{list}{$\\star$}{\\setlength{\\leftmargin}{2em}}\n\\item One\n\\item Two\n\\end{list}");
    let parsed = parser::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let labels = labels(&parsed);
    assert_eq!(labels.len(), 2);
    for label in &labels {
        match label {
            ItemLabel::Explicit { content, text, span } => {
                assert!(matches!(content.as_slice(), [Inline::Math { .. }]), "{content:?}");
                assert_eq!(text, "⋆");
                assert_eq!(&source[span.start..span.end], "{$\\star$}");
            }
            other => panic!("expected the default label as content, got {other:?}"),
        }
    }
}

#[test]
fn usecounter_numbers_the_items() {
    let source = doc("\\begin{list}{\\arabic{enumi}.}{\\usecounter{enumi}}\n\\item Three\\label{three}\n\\item Four\n\\end{list}\n\\begin{list}{(\\roman{enumii})}{\\usecounter{enumii}}\n\\item Five\n\\item Six\n\\item Seven\n\\end{list}");
    let parsed = parser::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let texts: Vec<String> = labels(&parsed).iter().map(|l| l.text().to_string()).collect();
    assert_eq!(texts, ["1.", "2.", "(i)", "(ii)", "(iii)"]);
}

#[test]
fn text_after_end_list_continues_unindented() {
    let source = doc("Before.\n\\begin{list}{--}{}\n\\item One\n\\end{list}\nXafter no blank line.\n\n\\begin{list}{--}{}\n\\item Two\n\\end{list}\n\nYafter a blank line.");
    let parsed = parser::parse(&source);
    assert_eq!(parsed.blocks.len(), parsed.block_par_starts.len());
    let indent_of = |needle: &str| {
        parsed
            .blocks
            .iter()
            .zip(&parsed.block_par_starts)
            .find(|(block, _)| matches!(block, Block::Paragraph(inlines) if inlines.iter().any(|i| matches!(i, Inline::Text { text, .. } if text.contains(needle)))))
            .map(|(_, start)| start.indent)
            .unwrap_or_else(|| panic!("no paragraph with {needle:?}"))
    };
    assert!(!indent_of("Xafter"), "`\\@endpe` after `\\end{{list}}`");
    assert!(indent_of("Yafter"), "a blank line cancels `\\@endpe`");
}
