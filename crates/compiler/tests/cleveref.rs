use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

fn text(output: &CompileOutput) -> String {
    output
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn cleveref_uses_label_names_and_compresses_section_ranges() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\begin{document}",
        r"\section{One}\label{s1}",
        r"\section{Two}\label{s2}",
        r"\section{Three}\label{s3}",
        r"See \cref{s1,s2,s3}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("sections 1 to 3"), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn cleveref_defaults_equations_to_abbreviated_parenthesised_numbers() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\begin{document}",
        r"\begin{equation}x\label{e1}\end{equation}",
        r"\begin{equation}y\label{e2}\end{equation}",
        r"See \cref{e1,e2}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("eqs. (1) and (2)"), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn cleveref_custom_names_and_undefined_labels_match_ref_diagnostics() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage[capitalise,noabbrev,unused-option]{cleveref}",
        r"\crefname{section}{section}{sections}",
        r"\Crefname{section}{Section}{Sections}",
        r"\begin{document}",
        r"\section{One}\label{s}",
        r"See \cref{s} and \Cref{s} and \cref{missing}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("Section 1"), "{rendered}");
    assert!(rendered.contains("??"), "{rendered}");
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Reference `missing'")),
        "{:?}",
        output.diagnostics
    );
}

#[test]
fn cleveref_parses_starred_ranges_pages_labelcref_and_autoref() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\begin{document}",
        r"\section{One}\label{s1}",
        r"\section{Two}\label{s2}",
        r"\begin{equation}x\label{e1}\end{equation}",
        r"\begin{equation}y\label{e2}\end{equation}",
        r"\crefrange{s1}{s2}; \Crefrange*{s1}{s2}; ",
        r"\cpageref{s1}; \labelcref{e1}; \autoref{s1}; \autoref*{e1}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    for expected in [
        "sections 1 to 2",
        "Sections 1 to 2",
        "page 1",
        "(1)",
        "Section 1",
        "Equation (1)",
    ] {
        assert!(rendered.contains(expected), "missing {expected}: {rendered}");
    }
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}
