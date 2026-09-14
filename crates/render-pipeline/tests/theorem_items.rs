//! amsthm theorem-like environments are a `\trivlist` holding one `\item`.
//!
//! Two consequences the pipeline used to get wrong, both measured against
//! pdflatex (TeX Live 2025, `\documentclass[11pt]{article}`, T1/ecrm1095 +
//! ecbx1095, the same TFMs this crate reads):
//!
//! * `\trivlist` leaves `\itemindent` at `\z@` and `\@item`'s `\everypar`
//!   takes the `\parindent` box back off the first line, so the head sits
//!   flush on the left margin. Only the *following* paragraphs of the same
//!   environment take the ambient `\parindent` (17pt at 11pt).
//!   `\showthe\itemindent` inside `proof`/`theorem` prints `0.0pt`.
//! * The head font (`\bfseries` for the `plain`/`definition` styles) and the
//!   `plain` style's italic body are declared by amsthm, not written in the
//!   source at the head's span — which is the `\begin` command itself — so
//!   they have to come from the compiler's own scoping.
//!   `\setbox0\hbox{\bfseries Definition}\showthe\wd0` prints `54.2265pt`;
//!   the same text set from the regular face is `47.03323pt`, which is what
//!   this pipeline produced before.
//!
//! The blank inside the synthesised head text ("Definition 1") stands for a
//! space *token*, so it is interword glue and the head is two runs, not one
//! run holding a T1 slot-32 character.

mod common;

use common::*;

/// TeX points to PDF points, the unit of every v2 coordinate.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

/// 11pt article, US Letter, no `geometry`: the text area's left edge.
fn text_left() -> f64 {
    let s = flashtex_render_pipeline::Stylesheet::article(11, flashtex_render_pipeline::fonts::Family::LatinModern, None);
    bp(s.text_x_pt)
}

const SRC: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage{amsthm}
\newtheorem{definition}{Definition}
\begin{document}
\begin{definition}
For every integer there is a larger one, and this sentence is long enough to
wrap onto a second line of the definition so the indent can be measured.

A second paragraph inside the same definition environment, which LaTeX does
indent by the ambient parindent because only the item line loses its box.
\end{definition}
\end{document}
";

#[test]
fn theorem_head_is_flush_left_and_bold_and_later_paragraphs_keep_the_indent() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SRC);
    let words = words_of(&r);
    let left = text_left();

    // The head is its own run: the blank in "Definition 1" became interword
    // glue, so "Definition" and "1" are separate runs.
    let head = words.iter().find(|w| w.text == "Definition").expect("a `Definition` run");
    let number = words.iter().find(|w| w.text == "1").expect("a `1` run");

    // `\trivlist`: `\itemindent\z@` and no `\parindent` box on the item line.
    assert!(
        (head.x - left).abs() < 0.01,
        "theorem head must start at the left margin ({left} bp), not indented: {head:?}"
    );
    // pdflatex: `\setbox0\hbox{\bfseries Definition}\showthe\wd0` = 54.2265pt.
    assert!(
        (head.width - bp(54.2265)).abs() < 0.01,
        "theorem head must be set from the bold face (54.2265pt = {} bp), got {} bp",
        bp(54.2265),
        head.width
    );
    assert!(number.x > head.x + head.width, "the number follows the head: {head:?} {number:?}");

    // The environment's *second* paragraph keeps the ambient `\parindent`
    // (17pt at 11pt), exactly as pdflatex sets it.
    let second = words
        .iter()
        .find(|w| w.text == "A" && w.baseline > head.baseline)
        .expect("the second paragraph's first word");
    assert!(
        (second.x - (left + bp(17.0))).abs() < 0.01,
        "a later paragraph of the same environment is indented by \\parindent (17pt): {second:?}"
    );
}
