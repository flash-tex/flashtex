use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

fn texts(output: &CompileOutput) -> Vec<&str> {
    output
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.as_str()))
        .collect()
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
    let items = texts(&output);
    assert!(items.contains(&"sections"), "{items:?}");
    assert!(items.contains(&"1 to 3"), "{items:?}");
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
    let items = texts(&output);
    assert!(items.contains(&"eqs."), "{items:?}");
    assert!(items.contains(&"(1) and (2)"), "{items:?}");
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
    let items = texts(&output);
    assert!(items.contains(&"Section"), "{items:?}");
    assert!(items.contains(&"1"), "{items:?}");
    assert!(items.contains(&"??"), "{items:?}");
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Reference `missing'")),
        "{:?}",
        output.diagnostics
    );
}
