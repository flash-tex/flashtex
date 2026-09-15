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
        r"\usepackage[noabbrev,unused-option]{cleveref}",
        r"\Crefname{section}{Heading}{Headings}",
        r"\crefname{section}{sec}{secs}",
        r"\begin{document}",
        r"\section{One}\label{s}",
        r"See \cref{s} and \Cref{s} and \cref{missing}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("sec 1"), "{rendered}");
    assert!(rendered.contains("Heading 1"), "{rendered}");
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
fn cleveref_crefname_does_not_derive_over_builtin_capital_name() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\crefname{section}{sec}{secs}",
        r"\begin{document}",
        r"\section{One}\label{s}",
        r"See \Cref{s}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("Section 1"), "{rendered}");
    assert!(!rendered.contains("Sec 1"), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn cleveref_subsection_names_override_the_section_alias() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\crefname{subsection}{subsec}{subsecs}",
        r"\crefname{subsubsection}{subsubsec}{subsubsecs}",
        r"\begin{document}",
        r"\section{One}\label{sec}",
        r"\subsection{Nested}\label{sub}",
        r"\subsubsection{Deep}\label{subsub}",
        r"See \cref{sec}; \cref{sub}; \cref{subsub}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    for expected in ["section 1", "subsec 1.1", "subsubsec 1.1.1"] {
        assert!(rendered.contains(expected), "missing {expected}: {rendered}");
    }
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn cleveref_parses_starred_ranges_pages_and_labelcref() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\begin{document}",
        r"\section{One}\label{s1}",
        r"\section{Two}\label{s2}",
        r"\begin{equation}x\label{e1}\end{equation}",
        r"\begin{equation}y\label{e2}\end{equation}",
        r"\crefrange{s1}{s2}; \Crefrange*{s1}{s2}; ",
        r"\cpageref{s1}; \labelcref{e1};",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    for expected in [
        "sections 1 to 2",
        "Sections 1 to 2",
        "page 1",
        "(1)",
    ] {
        assert!(rendered.contains(expected), "missing {expected}: {rendered}");
    }
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn cleveref_uses_english_conjunctions_groups_and_section_aliases() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\begin{document}",
        r"\section{One}\label{s1}\label{sec}",
        r"\section{Two}\label{s2}",
        r"\section{Three}",
        r"\section{Four}\label{s4}",
        r"\subsection{Nested}\label{sub}",
        r"\begin{equation}x\label{eq}\end{equation}",
        r"\begin{figure}\caption{A figure}\label{fig}\end{figure}",
        r"See \cref{s1,s2,s4}; \cref{sec,eq,fig}; \cref{sub}; \crefrange{sec}{sub}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    for expected in [
        "sections 1, 2 and 4",
        "section 1, eq. (1), and fig. 1",
        "section 4.1",
        "sections 1 to 4.1",
    ] {
        assert!(rendered.contains(expected), "missing {expected}: {rendered}");
    }
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn cleveref_capitalise_keeps_abbreviations() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage[capitalise]{cleveref}",
        r"\begin{document}",
        r"\begin{equation}x\label{e}\end{equation}",
        r"See \cref{e} and \Cref{e}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert_eq!(rendered.matches("Eq. (1)").count(), 1, "{rendered}");
    assert_eq!(rendered.matches("Equation (1)").count(), 1, "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn cleveref_noabbrev_uses_full_capitalised_names() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage[capitalise,noabbrev]{cleveref}",
        r"\begin{document}",
        r"\begin{equation}x\label{e}\end{equation}",
        r"See \cref{e} and \Cref{e}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert_eq!(rendered.matches("Equation (1)").count(), 2, "{rendered}");
    assert!(!rendered.contains("Eq. (1)"), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn cleveref_compresses_page_and_label_ranges() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\begin{document}",
        r"\section{One}\label{p1}\newpage",
        r"\section{Two}\label{p2}\newpage",
        r"\section{Three}\label{p3}",
        r"\begin{equation}x\label{e1}\end{equation}",
        r"\begin{equation}y\label{e2}\end{equation}",
        r"\begin{equation}z\label{e3}\end{equation}",
        r"See \cpageref{p1,p2,p3}; \labelcref{e1,e2,e3}.",
        r"\end{document}",
    );
    let output = compile(source);
    let rendered = text(&output);
    assert!(rendered.contains("pages 1 to 3"), "{rendered}");
    assert!(rendered.contains("(1) to (3)"), "{rendered}");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}
