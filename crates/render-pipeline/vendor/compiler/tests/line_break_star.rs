//! `\\*` and `\\*[<dimen>]`: the star is a no-page-break newline, not text.
//!
//! latex.ltx defines `\\` as `\@ifstar{\@xnewline\nobreak}{\@xnewline}`: an
//! optional `*` (with surrounding spaces skipped by `\@ifnextchar`) selects
//! the no-page-break variant, and an optional `[<dimen>]` adds that much
//! extra space. The ordinary-paragraph `\\` handler consumed the `[<dimen>]`
//! but never the `*`, so `b\\*[5mm] c` typeset the literal text `*[5mm]`
//! before `c` (only `verse`'s `\@centercr` path called
//! `take_optional_star`). The star carries no page model in this layout —
//! like `\hspace`'s star, both forms parse identically — but it must be
//! consumed so it never reaches the page, and the `[<dimen>]` after it must
//! still be reported on the node.

use flashtex_compiler::parser::{parse, Block, Inline};

/// Every `Inline::LineBreak` in the document's paragraphs, in order, as its
/// reported skip.
fn line_break_skips(text: &str) -> Vec<Option<f64>> {
    let parsed = parse(text);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let inlines = match block {
            Block::Paragraph(inlines)
            | Block::Styled {
                content: inlines, ..
            }
            | Block::ListItem {
                content: inlines, ..
            } => inlines,
            _ => continue,
        };
        for inline in inlines {
            if let Inline::LineBreak { skip_pt, .. } = inline {
                out.push(*skip_pt);
            }
        }
    }
    out
}

/// Every text fragment in the document's paragraphs, in order.
fn paragraph_texts(text: &str) -> Vec<String> {
    let parsed = parse(text);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { text, .. } = inline {
                    out.push(text.clone());
                }
            }
        }
    }
    out
}

const PREAMBLE: &str = r"\documentclass{article}
\setlength{\parindent}{0pt}
";

fn document(body: &str) -> String {
    format!("{PREAMBLE}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// 5 mm in TeX points, measured from live pdflatex (TeX Live 2026):
/// `\showthe\dimexpr5mm\relax` prints `14.22636pt`. `\\*[5mm]` must report
/// exactly the same skip as `\\[5mm]`.
const FIVE_MM_PT: f64 = 14.22636;

#[test]
fn a_starred_line_break_parses_and_leaves_no_text() {
    let text = document("a\\\\* b");
    assert_eq!(line_break_skips(&text), vec![None]);
    assert_eq!(paragraph_texts(&text), vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn a_starred_line_break_with_a_length_reports_the_skip() {
    let text = document("a\\\\[10pt] b\\\\*[5mm] c");
    let skips = line_break_skips(&text);
    assert_eq!(skips.len(), 2);
    assert_eq!(skips[0], Some(10.0));
    let skip = skips[1].expect("`\\\\*[5mm]` must report its length");
    assert!(
        (skip - FIVE_MM_PT).abs() <= 0.05,
        "`\\\\*[5mm]` skip {skip} pt, pdflatex 5mm = {FIVE_MM_PT} pt"
    );
    let words = paragraph_texts(&text);
    assert_eq!(words, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn a_space_before_the_star_still_parses() {
    let text = document("a\\\\ * b");
    assert_eq!(line_break_skips(&text), vec![None]);
    assert_eq!(paragraph_texts(&text), vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn the_star_and_length_never_reach_the_page_as_text() {
    let text = document("a\\\\[10pt] b\\\\*[5mm] c");
    let words = paragraph_texts(&text);
    assert!(
        !words
            .iter()
            .any(|w| w.contains('*') || w.contains('[') || w.contains("5mm")),
        "the star/length reached the page as text: {words:?}"
    );
}
