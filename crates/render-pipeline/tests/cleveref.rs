//! GH-CLEVEREF pipeline coverage. The vendored compiler must be re-pinned
//! before this can consume the compiler's named-reference inline node.

mod common;

#[test]
#[ignore = "needs vendor re-pin past GH-CLEVEREF"]
fn cleveref_names_render_as_text() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{cleveref}",
        r"\begin{document}",
        r"\section{First}",
        r"\section{Second}\label{s2}",
        r"\begin{equation}x\label{e1}\end{equation}",
        r"\begin{equation}y\label{e2}\end{equation}",
        r"See \Cref{s2}; \cref{e1,e2}.",
        r"\end{document}",
    );
    let rendered = common::render_one(source);
    let text = common::words_of(&rendered)
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(text.contains("Section 2"), "{text}");
    assert!(text.contains("eqs. (1) and (2)"), "{text}");
}
