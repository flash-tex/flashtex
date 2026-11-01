//! An `\item` with no body text still sets its label on a line of its own.
//!
//! `\@item` puts the label in `\@labels` and `\everypar` sets it when the
//! paragraph opens; the next `\item` (or `\end`) then issues `\par`, and a
//! paragraph whose horizontal list is only the label box is still one line.
//! So `\begin{itemize}\item\end{itemize}` ships a page holding the bullet,
//! and `\item\item Text` shows two bullets on two lines, `\itemsep +
//! \parsep + \baselineskip` = 20pt apart. The adapter used to drop the
//! block outright (no inlines, no parts) and the typesetter would have
//! refused a box-less list before prepending the label; the lone-item
//! document then failed with "display list has no pages", and in a longer
//! list the empty item's line vanished and the item after it was read as
//! the list's opener (its gap was scanned from the document start and
//! found the `\begin`).
//!
//! Every expected coordinate below was measured on the PDF pdflatex
//! (MacTeX 2026 full, pdfTeX 1.40.29) produced from the same source, with
//! `tools/visual-oracle/pdftext.py`; the oracle is not run here.
//! Coordinates are bp from the page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.3;

fn render(src: &str) -> Vec<Word> {
    let r = render_one(src);
    assert_eq!(r.v2.pages.len(), 1, "one page expected: {:?}", r.v2.diagnostics);
    words_of(&r)
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

/// Baselines of every bullet, top to bottom.
fn bullets(words: &[Word]) -> Vec<(f64, f64)> {
    let mut b: Vec<(f64, f64)> = words.iter().filter(|w| w.text == "\u{2022}").map(|w| (w.x, w.baseline)).collect();
    b.sort_by(|a, c| a.1.total_cmp(&c.1));
    b
}

#[test]
fn a_lone_empty_item_ships_one_page_with_its_bullet() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = render("\\documentclass{article}\n\\begin{document}\n\\begin{itemize}\n\\item\n\\end{itemize}\n\\end{document}\n");
    let b = bullets(&words);
    assert_eq!(b.len(), 1, "one bullet: {words:?}");
    close(b[0].0, 148.714, "bullet x");
    close(b[0].1, 134.765, "bullet baseline");
}

#[test]
fn an_empty_item_before_a_text_item_keeps_its_own_line() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = render("\\documentclass{article}\n\\begin{document}\n\\begin{itemize}\n\\item\n\\item Text\n\\end{itemize}\n\\end{document}\n");
    let b = bullets(&words);
    assert_eq!(b.len(), 2, "two bullets: {words:?}");
    close(b[0].1, 134.765, "first (empty) item baseline");
    // \itemsep 4pt + \parsep 4pt + \baselineskip 12pt below the empty line.
    close(b[1].1, 154.690, "second item baseline");
    let text = words.iter().find(|w| w.text == "Text").expect("Text");
    close(text.x, 158.676, "item text x");
    close(text.baseline, 154.690, "item text on the second bullet's line");
}

#[test]
fn runs_of_empty_items_in_itemize_and_enumerate() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = render(
        "\\documentclass{article}\n\\begin{document}\n\\begin{itemize}\n\\item\n\\end{itemize}\n\\begin{enumerate}\n\\item\n\\end{enumerate}\n\\begin{itemize}\n\\item\n\\item\n\\item\n\\end{itemize}\n\\begin{itemize}\n\\item\n\\item Text here\n\\item\n\\end{itemize}\n\\end{document}\n",
    );
    let b = bullets(&words);
    let expected = [134.765, 178.600, 198.526, 218.451, 240.369, 260.294, 280.219];
    assert_eq!(b.len(), expected.len(), "bullets: {b:?}");
    for (i, (got, want)) in b.iter().zip(expected).enumerate() {
        close(got.0, 148.714, &format!("bullet {i} x"));
        close(got.1, want, &format!("bullet {i} baseline"));
    }
    let one = words.iter().find(|w| w.text == "1.").expect("1.");
    close(one.x, 145.945, "enumerate label x");
    close(one.baseline, 156.682, "enumerate label baseline");
    let text = words.iter().find(|w| w.text == "Text").expect("Text");
    close(text.baseline, 260.294, "text item baseline");
}
