//! A `description` `\item` with no `[...]` optional label still gets
//! `\descriptionlabel`'s `\hspace\labelsep`: article.cls defines
//! `\descriptionlabel` as `\hspace\labelsep \normalfont\bfseries #1`, so an
//! unlabelled item's text starts `\labelsep` in from the list's left margin,
//! exactly as after a zero-width label.
//!
//! Every expected coordinate below was measured with
//! `tools/visual-oracle/pdftext.py` on the PDF pdfTeX (TeX Live 2026)
//! produced from the same source; the oracle is not run here. Coordinates
//! are bp from the page's top-left corner.

mod common;

use common::*;

const SOURCE: &str = concat!(
    "\\documentclass[10pt]{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage{lmodern}\n",
    "\\pagestyle{empty}\n",
    "\\begin{document}\n",
    "\\begin{description}\n",
    "\\item[Short] text a\n",
    "\\item text c\n",
    "\\end{description}\n",
    "\\end{document}\n",
);

fn word_at<'a>(words: &'a [Word], text: &str, baseline: f64) -> &'a Word {
    words
        .iter()
        .find(|w| w.text == text && (w.baseline - baseline).abs() < 0.5)
        .unwrap_or_else(|| panic!("no word {text:?} at baseline {baseline} in {words:?}"))
}

fn close(actual: f64, expected: f64, tol: f64, what: &str) {
    assert!(
        (actual - expected).abs() < tol,
        "{what}: {actual} vs pdflatex {expected}"
    );
}

#[test]
fn unlabelled_description_item_starts_labelsep_in_from_the_margin() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SOURCE);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let words = words_of(&r);
    // The unlabelled item's text starts `\labelsep` (5pt) in from the
    // margin: without `\descriptionlabel`'s `\hspace\labelsep` it sat at
    // 133.768, 4.982bp too far left.
    close(word_at(&words, "text", 154.690).x, 138.750, 0.05, "unlabelled item text");
    // Labelled items are unaffected: the label stays flush at the margin
    // and its body follows by the label width plus `\labelsep`.
    close(word_at(&words, "Short", 134.765).x, 133.768, 0.05, "labelled item label");
    close(word_at(&words, "text", 134.765).x, 166.382, 0.05, "labelled item text");
}
