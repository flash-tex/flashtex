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
