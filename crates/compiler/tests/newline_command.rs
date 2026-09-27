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

/// Each table entry's content, row by row, of the first `tabular` in the
/// first paragraph.
fn table_cells(text: &str) -> Vec<Vec<Vec<Inline>>> {
    use flashtex_compiler::tabular::Entry;
    let inlines = first_paragraph(text).expect("a paragraph");
    let table = inlines
        .iter()
        .find_map(|inline| match inline {
            Inline::Tabular(table) => Some(table),
            _ => None,
        })
        .expect("a tabular");
    table
        .entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Row(row) => Some(row.cells.iter().map(|cell| cell.content.clone()).collect()),
            _ => None,
        })
        .collect()
}

fn line_breaks(inlines: &[Inline]) -> usize {
    inlines.iter().filter(|inline| matches!(inline, Inline::LineBreak { .. })).count()
}

fn hfils(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .filter(|inline| matches!(inline, Inline::HFill { order: 1, .. }))
        .count()
}

/// An `l`/`c`/`r` entry is an `\halign` cell, TeX's restricted horizontal
/// mode, where `\newline`'s `\break` does nothing and only its `\hfil`
/// stays. pdflatex (TeX Live 2026, article, `\noindent` before each table):
/// - `\begin{tabular}{l}a\newline b\end{tabular}`: `a` at 139.746bp, `b`
///   at 144.727bp -- `ab` on one line, touching.
/// - `a \newline b` in the same table: identical (`\unskip` drops the space).
/// - `{l}` with a second row `wwwwwwwwww` (71.93bp): `b` at 175.442bp, half
///   the 61.4bp slack right of touching (the other half is the template's
///   `\hfil`); `{c}`: `a` at 160.225bp, `b` at 185.689bp (thirds).
#[test]
fn newline_in_an_lcr_entry_is_an_hfil_not_a_break() {
    for (spec, body) in [("l", "a\\newline b"), ("l", "a \\newline b"), ("c", "a\\newline b"), ("r", "a\\newline b")] {
        let cells = table_cells(&document(&format!("\\noindent\\begin{{tabular}}{{{spec}}}{body}\\\\ wwwwwwwwww\\end{{tabular}}")));
        let cell = &cells[0][0];
        assert_eq!(line_breaks(cell), 0, "{spec} {body}: no line break in an LR entry: {cell:?}");
        assert_eq!(hfils(cell), 1, "{spec} {body}: exactly the one \\hfil: {cell:?}");
        assert_eq!(texts(cell), ["a", "b"], "{spec} {body}: {cell:?}");
        // No interword glue survives the `\unskip`.
        assert!(
            cell.iter().all(|inline| !matches!(inline, Inline::Text { glue_before: Some(_), .. })),
            "{spec} {body}: {cell:?}"
        );
    }
}

/// A `p{3cm}` entry is a `\parbox`: `\newline` breaks there. pdflatex:
/// `\begin{tabular}{p{3cm}}f\newline g\end{tabular}` sets `f` and `g` at the
/// same x (139.746bp) one `\baselineskip` apart (210.381 -> 222.336bp).
#[test]
fn newline_in_a_p_entry_still_breaks() {
    let cells = table_cells(&document("\\noindent\\begin{tabular}{p{3cm}}f\\newline g\\end{tabular}"));
    let cell = &cells[0][0];
    assert_eq!(line_breaks(cell), 1, "{cell:?}");
    assert_eq!(hfils(cell), 0, "{cell:?}");
    // And an `l` column beside it keeps its own restricted mode.
    let cells = table_cells(&document("\\noindent\\begin{tabular}{lp{3cm}}a\\newline b & f\\newline g\\end{tabular}"));
    assert_eq!((line_breaks(&cells[0][0]), hfils(&cells[0][0])), (0, 1), "{:?}", cells[0][0]);
    assert_eq!((line_breaks(&cells[0][1]), hfils(&cells[0][1])), (1, 0), "{:?}", cells[0][1]);
    // Running text after the table breaks as usual.
    let inlines = first_paragraph(&document("\\begin{tabular}{l}a\\newline b\\end{tabular} c\\newline d")).expect("a paragraph");
    assert_eq!(line_breaks(&inlines), 1, "{inlines:?}");
}

/// `\noindent` leaves vertical mode, so `\newline` right after it is in
/// horizontal mode: no error. pdflatex (TeX Live 2026): after a paragraph
/// `A` at y=134.765bp, `\noindent\newline x` sets `x` at 158.675bp -- an
/// empty first line, then `x` one `\baselineskip` (11.955bp) lower.
#[test]
fn newline_after_noindent_is_not_an_error() {
    let inlines = first_paragraph(&document("\\noindent\\newline x")).expect("a paragraph");
    assert_eq!(line_breaks(&inlines), 1, "{inlines:?}");
    assert!(matches!(inlines.first(), Some(Inline::LineBreak { .. })), "the break opens the paragraph: {inlines:?}");
    assert_eq!(texts(&inlines), ["x"], "{inlines:?}");
}
