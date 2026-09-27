//! GH-828: a box or note argument parsed into a list of its own must not
//! spend the enclosing paragraph's start state. `\noindent\so{AB} x`,
//! `\noindent\mbox{AB} x`, `\noindent\colorbox{yellow}{AB} x`,
//! `\noindent\uline{AB} x` and `\noindent AB\footnote{x} x` all set `A` at
//! the margin in pdfTeX (x 110.854 bp, 12pt article, MacTeX 2026), but the
//! nested parse's own paragraph flush took the pending `\noindent`, so the
//! outer paragraph was indented by `\parindent` (17.559 bp). A run-in
//! `\paragraph{Run} \mbox{AB}` lost its run-in the same way. Evidence:
//! `tools/visual-oracle/pdftext.py` glyph origins; no TeX runs here.
use flashtex_compiler::parser::parse;

fn doc(body: &str) -> String {
    format!(
        "\\documentclass[12pt]{{article}}\n\\usepackage{{color,soul,ulem}}\n\\begin{{document}}\n{body}\n\\end{{document}}"
    )
}

/// The `indent` of each paragraph block, in order.
fn indents(body: &str) -> Vec<bool> {
    let parsed = parse(&doc(body));
    assert_eq!(parsed.block_par_starts.len(), parsed.blocks.len());
    parsed.block_par_starts.iter().map(|start| start.indent).collect()
}

#[test]
fn noindent_survives_a_leading_box_argument() {
    for body in [
        "\\noindent\\so{AB} x",
        "\\noindent \\so{AB} x",
        "\\noindent\\hl{AB} x",
        "\\noindent AB \\so{AB}",
        "\\noindent\\mbox{AB} x",
        "\\noindent\\colorbox{yellow}{AB} x",
        "\\noindent\\uline{AB} x",
        "\\noindent AB\\footnote{x} x",
    ] {
        assert_eq!(indents(body), vec![false], "{body}");
    }
}

#[test]
fn a_box_argument_does_not_suppress_the_indent() {
    for body in ["\\so{AB} x", "x \\so{AB} x", "\\mbox{AB} x", "AB\\footnote{x} x"] {
        assert_eq!(indents(body), vec![true], "{body}");
    }
}

#[test]
fn run_in_heading_reaches_a_paragraph_starting_with_a_box() {
    let parsed = parse(&doc("\\paragraph{Run} \\mbox{AB} x"));
    let run_in: Vec<_> = parsed.block_par_starts.iter().map(|start| start.run_in).collect();
    assert!(run_in.iter().any(Option::is_some), "run-in lost: {run_in:?}");
}
