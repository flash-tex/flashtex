//! hyperref `\nameref` and `\autoref`: text resolution through the same
//! label machinery `\ref` uses (`Inline::Label`, resolved after layout).

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::{Font, LayoutConstraints};

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
fn nameref_prints_the_section_title() {
    let source = concat!(
        r"\section{Introduction}\label{sec:intro}",
        r"See \nameref{sec:intro}.",
    );
    let output = compile(source);
    let rendered = text(&output);
    // Once for the heading itself, once for the `\nameref`.
    assert_eq!(rendered.matches("Introduction").count(), 2, "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn nameref_resolves_forward_references_like_ref() {
    let source = concat!(
        r"See \nameref{sec:intro}.",
        r"\section{Introduction}\label{sec:intro}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert_eq!(rendered.matches("Introduction").count(), 2, "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn autoref_prints_the_type_name_and_number() {
    let source = concat!(
        r"\section{Introduction}\label{sec:intro}",
        r"\subsection{Background}\label{sec:bg}",
        r"See \autoref{sec:intro} and \autoref{sec:bg}.",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("Section 1"), "{rendered}");
    assert!(rendered.contains("Section 1.1"), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn autoref_names_numbered_floats() {
    let source = concat!(
        r"\begin{figure}placeholder\caption{A demo}\label{fig:a}\end{figure}",
        r"\begin{table}\caption{Readings}\label{tab:b}\end{table}",
        r"See \autoref{fig:a} and \autoref{tab:b}.",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("Figure 1"), "{rendered}");
    assert!(rendered.contains("Table 1"), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn autoref_parenthesises_equation_numbers_like_eqref() {
    let source = concat!(
        r"\begin{equation}x\label{eq:x}\end{equation}",
        r"See \autoref{eq:x}.",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("Equation (1)"), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn nameref_prints_float_captions_and_falls_back_for_equations() {
    let source = concat!(
        r"\begin{figure}placeholder\caption{A demo}\label{fig:a}\end{figure}",
        r"\begin{equation}x\label{eq:x}\end{equation}",
        r"See \nameref{fig:a} and \nameref{eq:x}.",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("A demo"), "{rendered}");
    // Equations record no title, so `\nameref` prints the number like `\ref`.
    assert!(rendered.contains(" 1 "), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn starred_forms_compile_like_the_unstarred_ones() {
    let source = concat!(
        r"\section{Introduction}\label{sec:intro}",
        r"See \autoref*{sec:intro} and \nameref*{sec:intro}.",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("Section 1"), "{rendered}");
    assert_eq!(rendered.matches("Introduction").count(), 2, "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn undefined_autoref_and_nameref_render_bold_marks_and_warn_like_ref() {
    let source = r"See \autoref{nope} and \nameref{gone}.";
    let output = compile(source);
    let marks: Vec<_> = output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .filter(|item| item.text == "??")
        .collect();
    assert_eq!(marks.len(), 2);
    assert!(marks.iter().all(|item| item.font == Font::TimesBold));
    let warnings: Vec<_> = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .collect();
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0].message, "Reference `nope' on page 1 undefined");
    assert_eq!(warnings[1].message, "Reference `gone' on page 1 undefined");
}
