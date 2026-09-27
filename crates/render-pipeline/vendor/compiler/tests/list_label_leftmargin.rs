//! Generic `\begin{list}{label}{decls}`: the default label is drawn and the
//! decl's `\leftmargin` is honoured. See `src/parser/lists.rs` for the
//! latex.ltx provenance.
use flashtex_compiler::incremental::{compile_full_project, CompileOutput};
use flashtex_compiler::layout::{text_width, Font, LayoutConstraints, MARGIN_PT};
use flashtex_compiler::parser::SourceDocument;

fn compile(source: &str) -> CompileOutput {
    compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: source,
        }],
        "main.tex",
        LayoutConstraints::default(),
    )
}

fn source() -> String {
    "\\documentclass[10pt]{article}\n\\begin{document}\n\\begin{list}{$\\star$}{\\setlength{\\leftmargin}{1cm}}\n\\item a\n\\item b\n\\end{list}\n\\end{document}\n"
        .to_string()
}

fn probe(output: &CompileOutput, text: &str) -> (f64, f64) {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.text == text)
        .map(|item| (item.x_pt, item.baseline_y_pt))
        .unwrap_or_else(|| panic!("missing {text:?}: {:?}", output.pages))
}

#[test]
fn generic_list_draws_default_label_and_honours_leftmargin() {
    // Oracle: RUN, not believed. `pdflatex -interaction=nonstopmode list.tex`
    // (TeX Live 2026) over `\documentclass[10pt]{article}` with the body
    // above; PyMuPDF glyph origins (`get_text("dict")` span bboxes):
    //   '⋆' (CMMI10) x0=152.15, 'a' (CMR10) x0=162.11, same baselines;
    //   'b' repeats one line below. So pdflatex puts the item text exactly
    //   1cm (28.35bp) past the text block (base 133.77bp -> 162.11) and the
    //   star label `labelsep` (0.5em) plus its width left of that.
    const BODY_PT: f64 = 10.0;
    const LABELSEP_PT: f64 = 0.5 * BODY_PT;
    // Lengths resolve to true TeX points throughout layout (the sibling
    // `leftmargin_explicit_dimension_sets_the_margin_directly` test pins
    // `leftmargin=1in` to `MARGIN_PT + 72.27`): 1cm is 72.27/2.54 here,
    // 0.11bp past pdflatex's 28.35bp. That residue is the codebase-wide
    // pt-vs-bp convention, not this list's routing.
    const LEFTMARGIN_PT: f64 = 72.27 / 2.54;
    let output = compile(&source());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    // The default label is drawn: pdflatex's glyph is U+22C6 STAR.
    let (label_x, label_y) = probe(&output, "⋆");
    let (a_x, a_y) = probe(&output, "a");
    let (b_x, b_y) = probe(&output, "b");
    assert_eq!(label_y, a_y, "label sits on the item's baseline");
    assert!(label_x < a_x, "label sits left of the item text");
    // `\\makelabel` placement: the label's right edge ends `labelsep`
    // before the item margin.
    let margin_pt = MARGIN_PT + LEFTMARGIN_PT;
    let label_right = label_x + text_width("⋆", BODY_PT, Font::TimesRoman);
    assert!(
        (label_right - (margin_pt - LABELSEP_PT)).abs() < 0.02,
        "label right edge {label_right:.3} vs margin-labelsep {:.3}",
        margin_pt - LABELSEP_PT
    );
    // The decl's `\leftmargin` is honoured by every item.
    assert!(
        (a_x - margin_pt).abs() < 0.02,
        "item text x {a_x:.3} vs margin {margin_pt:.3}"
    );
    assert!(
        (b_x - margin_pt).abs() < 0.02,
        "second item text x {b_x:.3} vs margin {margin_pt:.3}"
    );
    assert!(b_y > a_y, "items stack vertically");
}
