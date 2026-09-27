//! `\thesection` (and the other `\the<ctr>` macros) inside a user's own
//! `\renewcommand{\the<ctr>}` body, and inside `\ref` text.
//!
//! A document `\newcounter{foo}[section]` owns `\thefoo` in the expansion
//! engine, which expands `\renewcommand{\thefoo}{\thesection.\alph{foo}}`
//! natively except for the host-numbered `\thesection` piece: the parser
//! formats that piece from its own counter table, both where `\thefoo`
//! is used directly and in the frozen `\@currentlabel` a `\label` records
//! for `\ref`. Expected values are pdflatex's (TeX Live 2026 oracle:
//! `A 1.b B 1.b` direct and through the reference; after a post-label
//! `\addtocounter{foo}{3}` the direct use prints `1.e` while the reference
//! keeps the value frozen at label time, `1.b`).

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

fn texts(output: &CompileOutput) -> Vec<String> {
    output.pages.iter().flat_map(|page| &page.items).map(|item| item.text.clone()).collect()
}

fn errors(output: &CompileOutput) -> Vec<String> {
    output
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

const PREAMBLE: &str = r"\documentclass{article}\newcounter{foo}[section]\renewcommand{\thefoo}{\thesection.\alph{foo}}\begin{document}";

#[test]
fn redefined_thefoo_prints_thesection_part_directly_and_through_ref() {
    let source = format!(
        r"{PREAMBLE}\section{{S}}\refstepcounter{{foo}}\refstepcounter{{foo}}\label{{f}}A \thefoo{{}} B \ref{{f}} C.\end{{document}}"
    );
    let output = compile(&source);
    assert_eq!(errors(&output), Vec::<String>::new(), "{:?}", output.diagnostics);
    let texts = texts(&output);
    // The direct `\thefoo` arrives as adjacent `1` + `.b` items, the `\ref`
    // as one `1.b` item; joined, both read `1.b`.
    assert_eq!(texts.join("").matches("1.b").count(), 2, "{texts:?}");
    assert!(!texts.iter().any(|t| t.contains("thesection")), "{texts:?}");
}

#[test]
fn post_label_addtocounter_updates_thefoo_but_freezes_the_reference() {
    // pdflatex prints `D 1.e E 1.b`: `\thefoo` reads the live counter while
    // `\ref{f}` keeps the `\@currentlabel` frozen when `\label{f}` ran.
    let source = format!(
        r"{PREAMBLE}\section{{S}}\refstepcounter{{foo}}\refstepcounter{{foo}}\label{{f}}A \thefoo{{}} B \ref{{f}} C.\addtocounter{{foo}}{{3}}D \thefoo{{}} E \ref{{f}} F.\end{{document}}"
    );
    let output = compile(&source);
    assert_eq!(errors(&output), Vec::<String>::new(), "{:?}", output.diagnostics);
    let texts = texts(&output);
    // `A` direct and `B` ref read `1.b`; after the post-label addtocounter
    // the direct `D` reads `1.e` while the frozen `E` ref keeps `1.b`.
    assert_eq!(texts.join("").matches("1.b").count(), 3, "{texts:?}");
    assert_eq!(texts.join("").matches("1.e").count(), 1, "{texts:?}");
}

#[test]
fn label_after_addtocounter_resolves_both_to_the_new_value() {
    // `\@currentlabel` is set by `\refstepcounter`, not by `\label`
    // (pdflatex oracle: a label with no new step after `\addtocounter`
    // still prints the old value), so the new step below brings `foo` to
    // 5 and both uses print `1.e`.
    let source = format!(
        r"{PREAMBLE}\section{{S}}\refstepcounter{{foo}}\refstepcounter{{foo}}\addtocounter{{foo}}{{2}}\refstepcounter{{foo}}\label{{h}}D \thefoo{{}} E \ref{{h}} F.\end{{document}}"
    );
    let output = compile(&source);
    assert_eq!(errors(&output), Vec::<String>::new(), "{:?}", output.diagnostics);
    let texts = texts(&output);
    assert_eq!(texts.join("").matches("1.e").count(), 2, "{texts:?}");
}

#[test]
fn bare_thesection_and_thesubsection_expand_in_body_text() {
    let source = r"\documentclass{article}\begin{document}\section{S}\subsection{T}sec \thesection{} sub \thesubsection{}.\end{document}";
    let output = compile(source);
    assert_eq!(errors(&output), Vec::<String>::new(), "{:?}", output.diagnostics);
    let texts = texts(&output);
    assert!(texts.iter().any(|t| t == "1"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "1.1"), "{texts:?}");
}
