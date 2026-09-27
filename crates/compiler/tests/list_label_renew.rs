//! A `\renewcommand` of `\labelenumi`..`\labelenumiv` / `\labelitemi`..`\labelitemiv`
//! changes the labels `enumerate` / `itemize` print. Those are ordinary
//! class macros (article.cls 348-359; report, book and letter define the
//! same eight), so under pdflatex the renewal is silent and applies from
//! that point on. Ground truth: TeX Live 2026 `pdflatex
//! -interaction=nonstopmode` over the minimal document in the check-in
//! prints `(a)`, `(b)`, `[i]` and the circ bullet at label xMins 140.964,
//! 140.410, 167.309 and 148.712.

use flashtex_compiler::parser::{self, Block, ItemLabel};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n{body}")
}

fn parsed(source: &str) -> parser::Parsed {
    parser::parse(source)
}

/// Every `\item`'s label text, in document order.
fn label_texts(source: &str) -> Vec<String> {
    parsed(source)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { item, .. } => item.clone(),
            _ => None,
        })
        .map(|item| item.text().to_string())
        .collect()
}

#[test]
fn renewed_labelenumi_changes_enumerate_labels() {
    let source = doc(concat!(
        "\\renewcommand{\\labelenumi}{(\\alph{enumi})}\n",
        "\\begin{document}\n",
        "\\begin{enumerate}\n",
        "\\item a\n",
        "\\item b\n",
        "\\end{enumerate}\n",
        "\\end{document}\n",
    ));
    assert_eq!(label_texts(&source), ["(a)", "(b)"]);
    // Silent under pdflatex+article, so silent here too: the expansion
    // engine's `Command \labelenumi undefined` does not survive.
    assert!(parsed(&source).diagnostics.is_empty(), "{:?}", parsed(&source).diagnostics);
}

#[test]
fn renewed_labelitemi_changes_itemize_labels() {
    let source = doc(concat!(
        "\\renewcommand{\\labelitemi}{$\\circ$}\n",
        "\\begin{document}\n",
        "\\begin{itemize}\n",
        "\\item b\n",
        "\\item[$\\circ$] c\n",
        "\\end{itemize}\n",
        "\\end{document}\n",
    ));
    let labels: Vec<ItemLabel> = parsed(&source)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { item, .. } => item.clone(),
            _ => None,
        })
        .collect();
    assert_eq!(labels.len(), 2);
    // U+2218, the compiler's own `$\circ$` glyph (see `COMMAND_GLYPHS`):
    // pdflatex's PDF extracts the same cmsy glyph as U+25E6 through its
    // ToUnicode map, but inside this compiler the renewal must read exactly
    // like the explicit label on the next item.
    assert_eq!(labels[0].text(), "∘");
    assert_eq!(labels[0].text(), labels[1].text());
    assert!(parsed(&source).diagnostics.is_empty(), "{:?}", parsed(&source).diagnostics);
}

#[test]
fn renewed_labelenumii_changes_nested_enumerate_labels() {
    let source = doc(concat!(
        "\\renewcommand{\\labelenumii}{[\\roman{enumii}]}\n",
        "\\begin{document}\n",
        "\\begin{enumerate}\n",
        "\\item a\n",
        "\\begin{enumerate}\n",
        "\\item b\n",
        "\\end{enumerate}\n",
        "\\end{enumerate}\n",
        "\\end{document}\n",
    ));
    assert_eq!(label_texts(&source), ["1.", "[i]"]);
}

#[test]
fn a_renewal_applies_from_that_point_on_only() {
    let source = doc(concat!(
        "\\begin{document}\n",
        "\\begin{enumerate}\n",
        "\\item a\n",
        "\\end{enumerate}\n",
        "\\renewcommand{\\labelenumi}{(\\alph{enumi})}\n",
        "\\begin{enumerate}\n",
        "\\item b\n",
        "\\end{enumerate}\n",
        "\\end{document}\n",
    ));
    assert_eq!(label_texts(&source), ["1.", "(a)"]);
}

#[test]
fn newcommand_of_a_predefined_label_is_a_noop() {
    // pdflatex errors (`Command \labelenumi already defined`) and keeps the
    // class meaning; the label stays the default either way.
    let source = doc(concat!(
        "\\newcommand{\\labelenumi}{(\\alph{enumi})}\n",
        "\\begin{document}\n",
        "\\begin{enumerate}\n",
        "\\item a\n",
        "\\end{enumerate}\n",
        "\\end{document}\n",
    ));
    assert_eq!(label_texts(&source), ["1."]);
}

#[test]
fn beamer_ignores_the_renewal_but_keeps_the_error() {
    // Beamer's own `enumerate item` template typesets the label, never
    // `\labelenumi` (`texdef -t latex -c beamer labelenumi` is undefined),
    // so pdflatex both keeps the template and reports the renewal as
    // undefined. This compiler does the same.
    let source = "\\documentclass{beamer}\n\\renewcommand{\\labelenumi}{(\\alph{enumi})}\n\\begin{document}\n\\begin{frame}\n\\begin{enumerate}\n\\item a\n\\end{enumerate}\n\\end{frame}\n\\end{document}\n";
    assert_eq!(label_texts(source), ["1."]);
    assert!(
        parsed(source)
            .diagnostics
            .iter()
            .any(|d| d.message == "LaTeX Error: Command \\labelenumi undefined."),
        "{:?}",
        parsed(source).diagnostics
    );
}

#[test]
fn renewal_inside_verbatim_is_documentation_not_code() {
    // `pdflatex -interaction=nonstopmode verb.tex && pdftotext verb.pdf -`
    // (TeX Live 2026) prints the renewal line literally, then `1. a`: the
    // verbatim body never runs.
    let source = doc(concat!(
        "\\begin{document}\n",
        "\\begin{verbatim}\n",
        "\\renewcommand{\\labelenumi}{(\\alph{enumi})}\n",
        "\\end{verbatim}\n",
        "\\begin{enumerate}\n",
        "\\item a\n",
        "\\end{enumerate}\n",
        "\\end{document}\n",
    ));
    assert_eq!(label_texts(&source), ["1."]);
}

#[test]
fn renewal_inside_verb_is_documentation_not_code() {
    // Same mechanism inline: pdflatex prints
    // `\renewcommand{\labelenumi}{(\alph{enumi})}`, then `1. a`.
    let source = doc(concat!(
        "\\begin{document}\n",
        "\\verb|\\renewcommand{\\labelenumi}{(\\alph{enumi})}|\n",
        "\\begin{enumerate}\n",
        "\\item a\n",
        "\\end{enumerate}\n",
        "\\end{document}\n",
    ));
    assert_eq!(label_texts(&source), ["1."]);
}

#[test]
fn renewal_inside_lstlisting_and_comment_is_documentation_not_code() {
    // `pdflatex -interaction=nonstopmode` with listings/comment loaded
    // prints `1. a` in both cases (the lstlisting body echoes literally;
    // the comment body vanishes). Both environments sit in the body: an
    // lstlisting in the preamble errors out with no PDF at all.
    for env in ["lstlisting", "comment"] {
        let source = doc(&format!(
            "\\begin{{document}}\n\\begin{{{env}}}\n\\renewcommand{{\\labelenumi}}{{(\\alph{{enumi}})}}\n\\end{{{env}}}\n\\begin{{enumerate}}\n\\item a\n\\end{{enumerate}}\n\\end{{document}}\n",
        ));
        assert_eq!(label_texts(&source), ["1."], "env {env}");
    }
}

#[test]
fn renewal_inside_lstinline_and_verb_cap_is_documentation_not_code() {
    // Same mechanism inline: pdflatex (listings/fancyvrb loaded) prints
    // `\renewcommand{\labelenumi}{(\alph{enumi})}`, then `1. a`.
    for inline in [
        "\\lstinline|\\renewcommand{\\labelenumi}{(\\alph{enumi})}|\n",
        "\\lstinline[language=C]|\\renewcommand{\\labelenumi}{(\\alph{enumi})}|\n",
        "\\Verb|\\renewcommand{\\labelenumi}{(\\alph{enumi})}|\n",
    ] {
        let source = doc(&format!(
            "\\begin{{document}}\n{inline}\\begin{{enumerate}}\n\\item a\n\\end{{enumerate}}\n\\end{{document}}\n",
        ));
        assert_eq!(label_texts(&source), ["1."], "inline {inline}");
    }
}

#[test]
fn other_list_label_classes_stay_silent_too() {
    // `texdef -t latex -c <class> labelenumi` is defined for each of these
    // (TeX Live 2026), and pdflatex renews silently, printing `(a) a`
    // (verified for amsbook).
    for class in ["amsbook", "amsproc", "scrlttr2", "scrletter"] {
        let source = format!(
            "\\documentclass{{{class}}}\n\\renewcommand{{\\labelenumi}}{{(\\alph{{enumi}})}}\n\\begin{{document}}\n\\begin{{enumerate}}\n\\item a\n\\end{{enumerate}}\n\\end{{document}}\n",
        );
        assert_eq!(label_texts(&source), ["(a)"], "class {class}");
        assert!(
            parsed(&source).diagnostics.is_empty(),
            "class {class}: {:?}",
            parsed(&source).diagnostics
        );
    }
}

#[test]
fn def_spelling_renews_too() {
    let source = doc(concat!(
        "\\def\\labelitemi{\\textendash}\n",
        "\\begin{document}\n",
        "\\begin{itemize}\n",
        "\\item b\n",
        "\\end{itemize}\n",
        "\\end{document}\n",
    ));
    assert_eq!(label_texts(&source), ["–"]);
}
