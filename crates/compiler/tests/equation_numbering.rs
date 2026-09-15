//! Counter numbering through the full compile path (expansion pass +
//! parser): amsmath `\numberwithin` in the preamble, the kernel's
//! `\counterwithin`/`\counterwithout` (engine primitives handed to the parser
//! by `expansion::HOST_PRELUDE`), and amsmath `subequations`. Expected
//! numbers are pdflatex's (display-placement fixtures 17 and 18).

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

fn numbers(output: &CompileOutput) -> Vec<String> {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .filter(|item| item.text.starts_with('(') && item.text.ends_with(')'))
        .map(|item| item.text.clone())
        .collect()
}

fn not_supported(output: &CompileOutput) -> Vec<String> {
    output
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        // amsmath itself is reported as "recognised but not implemented"
        // on main; only counter and environment gaps matter here.
        .filter(|m| !m.starts_with("packages "))
        .filter(|m| {
            m.contains("not supported") || m.contains("not implemented") || m.contains("No counter")
        })
        .collect()
}

#[test]
fn numberwithin_in_the_preamble_numbers_equations_within_sections() {
    let source = r"\documentclass{article}
\usepackage{amsmath}
\numberwithin{equation}{section}
\begin{document}
\section{First}
Text.
\begin{equation} a = b \end{equation}
\section{Second}
Text.
\begin{equation} c = d \label{e:c} \end{equation}
Ref \eqref{e:c}.
\end{document}";
    let output = compile(source);
    assert_eq!(not_supported(&output), Vec::<String>::new());
    assert_eq!(numbers(&output), ["(1.1)", "(2.1)", "(2.1)"]);
}

#[test]
fn subequations_number_with_letters_and_a_leading_label_gets_the_parent() {
    let source = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
Before.
\begin{equation} z = 0 \end{equation}
\begin{subequations}\label{e:sub}
\begin{align} a &= b \label{e:s1}\\ c &= d \end{align}
\begin{equation} e = f \end{equation}
\end{subequations}
Refs \eqref{e:sub}, \eqref{e:s1}.
\begin{equation} g = h \end{equation}
\end{document}";
    let output = compile(source);
    assert_eq!(not_supported(&output), Vec::<String>::new());
    assert_eq!(
        numbers(&output),
        ["(1)", "(2a)", "(2b)", "(2c)", "(2)", "(2a)", "(3)"]
    );
}

#[test]
fn counterwithin_reaches_the_parser_and_counterwithout_undoes_it() {
    let source = r"\documentclass{article}
\counterwithin{equation}{section}
\begin{document}
\section{A}
\begin{equation} x \end{equation}
\counterwithout{equation}{section}
\section{B}
\begin{equation} y \end{equation}
\end{document}";
    let output = compile(source);
    assert_eq!(not_supported(&output), Vec::<String>::new());
    assert_eq!(numbers(&output), ["(1.1)", "(2)"]);
}
