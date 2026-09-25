//! `\newline` (LaTeX kernel): a forced line break with no `\\`-style
//! `*`/`[<dimen>]` form.
//!
//! latex.ltx defines `\newline` as `\@normalcr`: it ends the current line
//! (`\hfil\break`) and stays in the same paragraph — no `\parskip` glue, no
//! indentation of the next line. Unlike `\\` (`\@xnewline`) it takes neither
//! a `*` nor a `[<dimen>]` argument: pdflatex (TeX Live 2026) typesets both
//! literally, and in vertical mode it errors with `There's no line here to
//! end`, exactly like `\\`/`\linebreak`.
//!
//! Measured pdflatex oracles (commands run 2026-09-25, TeX Live 2026):
//! - `Line one\newline Line two` under article typesets two lines; with
//!   `\parskip=20pt` no extra gap appears between them, and `\showoutput`
//!   emits zero `\parskip` glue nodes, so both lines are one paragraph.
//! - `a\newline[5pt]b` typesets `a`, `[5pt]`, `b` (break, then the literal
//!   text `[5pt]b`); `c\newline*d` typesets `c`, `*`, `d`.
//! - `\newline hello` at the top of the body errors with
//!   `! LaTeX Error: There's no line here to end.`

use flashtex_compiler::parser::{parse, Block, Inline};

const PREAMBLE: &str = r"\documentclass{article}
\setlength{\parindent}{0pt}
";

fn document(body: &str) -> String {
    format!("{PREAMBLE}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// The document's first paragraph, or `None` when the body parsed to
/// anything else (a second paragraph would fail this helper's caller).
fn first_paragraph(text: &str) -> Option<Vec<Inline>> {
    let parsed = parse(text);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    match parsed.blocks.into_iter().next() {
        Some(Block::Paragraph(inlines)) => Some(inlines),
        other => panic!("expected one paragraph, got {other:?}"),
    }
}

fn texts(inlines: &[Inline]) -> Vec<&str> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn newline_breaks_the_line_within_a_single_paragraph() {
    let inlines = first_paragraph(&document(r"Line one\newline Line two"))
        .expect("one paragraph");
    // Exactly one forced break, carrying no `\\[<dimen>]`-style skip.
    let breaks: Vec<_> = inlines
        .iter()
        .filter(|inline| matches!(inline, Inline::LineBreak { .. }))
        .collect();
    assert_eq!(breaks.len(), 1, "inlines: {inlines:?}");
    assert!(
        matches!(breaks[0], Inline::LineBreak { skip_pt: None, .. }),
        "break: {:?}",
        breaks[0]
    );
    // The words either side stay in this same paragraph, in order.
    assert_eq!(texts(&inlines), ["Line", "one", "Line", "two"]);
}

#[test]
fn newline_takes_no_star_or_optional_argument() {
    // pdflatex typesets the `[5pt]` and `*` literally (see module docs).
    let inlines = first_paragraph(&document(r"a\newline[5pt]b"))
        .expect("one paragraph");
    assert!(
        inlines
            .iter()
            .any(|inline| matches!(inline, Inline::LineBreak { skip_pt: None, .. })),
        "no plain break in {inlines:?}"
    );
    assert!(
        texts(&inlines).iter().any(|word| word.contains("[5pt]")),
        "the `[5pt]` must reach the page as text: {inlines:?}"
    );

    let inlines =
        first_paragraph(&document(r"c\newline*d")).expect("one paragraph");
    assert!(
        inlines
            .iter()
            .any(|inline| matches!(inline, Inline::LineBreak { skip_pt: None, .. })),
        "no plain break in {inlines:?}"
    );
    assert!(
        texts(&inlines).iter().any(|word| word.contains('*')),
        "the `*` must reach the page as text: {inlines:?}"
    );
}

#[test]
fn newline_outside_a_paragraph_errors_like_linebreak() {
    // pdflatex: `! LaTeX Error: There's no line here to end.`
    let parsed = parse(&document(r"\newline hello"));
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("There's no line here to end")),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    let breaks = parsed
        .blocks
        .iter()
        .flat_map(|block| match block {
            Block::Paragraph(inlines) => inlines.as_slice(),
            _ => &[],
        })
        .filter(|inline| matches!(inline, Inline::LineBreak { .. }))
        .count();
    assert_eq!(breaks, 0, "the failed break must not emit a node");
}
