//! Unclosed environments shut by `\end{document}` report exactly one
//! diagnostic naming the innermost open environment, at every nesting depth.
//!
//! Oracle: TeX Live 2026 pdflatex (`/Library/TeX/texbin/pdflatex`, pdfTeX
//! 1.40.29) in `-interaction=nonstopmode` emits a single
//! `! LaTeX Error: \begin{<innermost>} on input line 1 ended by
//! \end{document}.` for each shape below (`grep -c "^!"` on the log is 1;
//! content still renders, 1 page via `pdfinfo`). The preamble in every case
//! is `\documentclass{article}` plus `\newtheorem{question}{Question}`
//! wherever the `question` environment is used (`\newtheorem` is kernel
//! LaTeX, so no package loading is involved).

use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::LayoutConstraints;

fn messages(text: &str) -> Vec<String> {
    compile_full(text, LayoutConstraints::default())
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

/// `\end{document}` closing over `depth` unclosed environments must yield
/// exactly one diagnostic, and it must name the innermost open environment.
fn assert_single_innermost(text: &str, innermost: &str) {
    let all = messages(text);
    assert_eq!(all.len(), 1, "{text}: {all:?}");
    assert!(
        all[0].contains(innermost),
        "{text}: diagnostic names {innermost}: {:?}",
        all[0]
    );
    assert!(
        all[0].contains(r"\end{document}"),
        "{text}: diagnostic is about \\end{{document}}: {:?}",
        all[0]
    );
}

const PREAMBLE: &str = "\\documentclass{article}\n\\newtheorem{question}{Question}\n";

#[test]
fn two_levels_unclosed_report_only_the_innermost() {
    // pdflatex: 1 error, `\begin{center} ... ended by \end{document}`.
    assert_single_innermost(
        &format!(
            "{PREAMBLE}\\begin{{document}}\n\\begin{{question}}\n\\begin{{center}}\ntext\n\\end{{document}}"
        ),
        "center",
    );
}

#[test]
fn three_levels_all_open_report_only_the_innermost() {
    // pdflatex: 1 error, `\begin{enumerate} ... ended by \end{document}`.
    assert_single_innermost(
        &format!(
            "{PREAMBLE}\\begin{{document}}\n\\begin{{question}}\n\\begin{{center}}\n\\begin{{enumerate}}\n\\item x\n\\end{{document}}"
        ),
        "enumerate",
    );
}

#[test]
fn three_levels_with_innermost_closed_report_only_the_innermost_open() {
    // pdflatex: 1 error, `\begin{center} ... ended by \end{document}`.
    assert_single_innermost(
        &format!(
            "{PREAMBLE}\\begin{{document}}\n\\begin{{question}}\n\\begin{{center}}\n\\begin{{enumerate}}\n\\item x\n\\end{{enumerate}}\n\\end{{document}}"
        ),
        "center",
    );
}

#[test]
fn four_levels_all_open_report_only_the_innermost() {
    // pdflatex: 1 error, `\begin{enumerate} ... ended by \end{document}`.
    assert_single_innermost(
        &format!(
            "{PREAMBLE}\\begin{{document}}\n\\begin{{question}}\n\\begin{{center}}\n\\begin{{quote}}\n\\begin{{enumerate}}\n\\item x\n\\end{{document}}"
        ),
        "enumerate",
    );
}

#[test]
fn four_levels_with_innermost_closed_report_only_the_innermost_open() {
    // pdflatex: 1 error, `\begin{quote} ... ended by \end{document}`.
    assert_single_innermost(
        &format!(
            "{PREAMBLE}\\begin{{document}}\n\\begin{{question}}\n\\begin{{center}}\n\\begin{{quote}}\n\\begin{{enumerate}}\n\\item x\n\\end{{enumerate}}\n\\end{{document}}"
        ),
        "quote",
    );
}

#[test]
fn matched_same_named_nesting_stays_silent() {
    // pdflatex: 0 errors. Must not regress: only the *unclosed* path changed.
    let all = messages(
        "\\documentclass{article}\n\\begin{document}\n\\begin{itemize}\n\\item a\n\\begin{itemize}\n\\item b\n\\end{itemize}\n\\end{itemize}\n\\end{document}",
    );
    assert!(all.is_empty(), "{all:?}");
}

#[test]
fn mid_document_mismatch_stays_a_single_diagnostic() {
    // pdflatex: 1 error, `\begin{center} ... ended by \end{itemize}`
    // (content still renders). The mid-document path is untouched.
    let all = messages(
        "\\documentclass{article}\n\\begin{document}\n\\begin{center}body\\end{itemize}tail\\end{document}",
    );
    assert_eq!(all.len(), 1, "{all:?}");
    assert!(all[0].contains("center"), "{all:?}");
}

#[test]
fn environment_inside_a_macro_argument_stays_silent() {
    // pdflatex: 0 errors.
    let all = messages(
        "\\documentclass{article}\n\\begin{document}\n\\textbf{\\begin{center}x\\end{center}}\n\\end{document}",
    );
    assert!(all.is_empty(), "{all:?}");
}

#[test]
fn end_of_input_without_end_document_is_unchanged() {
    // No `\end{document}` at all: the end-of-input sweep still reports the
    // open environment (pinned by `recovery.rs` too).
    let all = messages("\\begin{document}Visible");
    assert_eq!(all.len(), 1, "{all:?}");
    assert!(
        all[0].contains("unterminated environment 'document'"),
        "{all:?}"
    );
}
