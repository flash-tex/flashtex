//! An itemize opening right after a description `\item[...]` label runs
//! its first bullet in on the label's own line.
//!
//! A description `\item[Outer]` directly followed by (no blank line, right
//! after) an itemize leaves TeX in `\@inlabel`, so `\@trivlist` sets
//! `\@noparlist`: the nested list takes no `\@topsep` and its first `\item`
//! (`\@donoparitem`) sets the bullet on the pending label's line, as an
//! `\hbox to\labelwidth` past the outer label's trailing `\labelsep`, with
//! the body past one more `\labelsep`. Later lines sit one merged line
//! higher.
//!
//! Every expected coordinate below was measured with
//! `tools/visual-oracle/pdftext.py` on the PDF pdfTeX (TeX Live 2026)
//! produced from the same source; the oracle is not run here. Coordinates
//! are bp from the page's top-left corner.

mod common;

use common::*;

const SOURCE: &str = concat!(
    "\\documentclass[10pt]{article}\n",
    "\\pagestyle{empty}\n",
    "\\begin{document}\n",
    "\\begin{description}\n",
    "\\item[Outer]\n",
    "\\begin{itemize}\n",
    "\\item inner one\n",
    "\\item inner two\n",
    "\\end{itemize}\n",
    "\\item[Next] text\n",
    "\\end{description}\n",
    "\\end{document}\n",
);

fn bullet<'a>(words: &'a [Word], baseline: f64) -> &'a Word {
    words
        .iter()
        .find(|w| w.text == "•" && (w.baseline - baseline).abs() < 0.5)
        .unwrap_or_else(|| panic!("no bullet at baseline {baseline} in {words:?}"))
}

fn close(actual: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (actual - expected).abs() < tol,
        "{what}: {actual} vs pdflatex {expected}"
    );
}

#[test]
fn itemize_first_bullet_runs_in_on_the_description_label_line() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SOURCE);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let words = words_of(&r);
    // The first bullet runs in on the `Outer` line: without the run-in the
    // itemize opened on a new line at (170.631, 154.690).
    let first = bullet(&words, 134.765);
    close(first.x, 180.104, 0.1, "first bullet x");
    close(first.baseline, 134.765, 0.1, "first bullet baseline");
    // The second bullet keeps the itemize indent, one merged line higher.
    let second = bullet(&words, 150.705);
    close(second.x, 170.631, 0.1, "second bullet x");
    close(second.baseline, 150.705, 0.1, "second bullet baseline");
}
