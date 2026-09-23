//! A document's own `\newcommand` of a name a package or a class provides
//! (parity 2026-09-23 cause 4, "project macro"): without `\usepackage{siunitx}`
//! `\si`/`\unit` are free, without `\documentclass{letter}` `\cc`/`\ps` are,
//! exactly as in LaTeX -- the compiler declares those host commands to the
//! expansion engine only once the providing file is loaded
//! (`expansion::package_of_built_in`). With the file loaded, the collision is
//! LaTeX's "Command \si already defined." as before.

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

fn errors(output: &CompileOutput) -> Vec<String> {
    output
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

fn text(output: &CompileOutput) -> String {
    output.pages.iter().flat_map(|p| &p.items).map(|i| i.text.as_str()).collect::<Vec<_>>().join(" ")
}

#[test]
fn siunitx_and_letter_names_are_free_without_their_package_or_class() {
    // pdflatex (oracle, MacTeX 2026): no error; the text reads
    // "σ+C and u and arrow." (probe `p2.tex`, cause-4 documents 2501.07538v2,
    // 2501.07662v1, 2501.07099v1, 2501.07528v2).
    let source = r"\documentclass{article}
\usepackage{amssymb}
\newcommand{\si}{\sigma}
\newcommand{\cc}{\mathbb{C}}
\newcommand{\unit}{u}
\newcommand{\ps}{p}
\begin{document}
$\si+\cc$ and \unit\ and \ps.
\end{document}";
    let output = compile(source);
    assert_eq!(errors(&output), Vec::<String>::new());
    let text = text(&output);
    let words: Vec<&str> = text.split_whitespace().collect();
    assert_eq!(words, ["σ", "+", "ℂ", "and", "u", "and", "p", "."], "{text}");
}

#[test]
fn siunitx_names_still_collide_once_siunitx_is_loaded() {
    let source = r"\documentclass{article}
\usepackage{siunitx}
\newcommand{\si}{\sigma}
\begin{document}
\si{\metre}
\end{document}";
    let output = compile(source);
    assert_eq!(errors(&output), ["LaTeX Error: Command \\si already defined."]);
}

#[test]
fn letter_names_still_collide_under_the_letter_class() {
    let source = r"\documentclass{letter}
\newcommand{\cc}{C}
\begin{document}
\begin{letter}{X}
\opening{Dear X,}
Body.
\closing{Yours,}
\cc{Y}
\end{letter}
\end{document}";
    let output = compile(source);
    assert_eq!(errors(&output), ["LaTeX Error: Command \\cc already defined."]);
}
