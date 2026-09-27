//! A `proof` head is an `\item[<label>]` label box, set at natural width.
//!
//! amsthm's `proof` is `\item[\hskip\labelsep\itshape #1\@addpunct{.}]`, and
//! latex.ltx `\@item` puts the label in `\@tempboxa` and starts the paragraph
//! with `\box\@labels` then `\penalty\z@`: the head is one box. Its interword
//! glue neither stretches nor shrinks, and the head is never broken or
//! hyphenated. A `\newtheorem` head is different: amsthm's `\dth@everypar`
//! `\unhbox`es it into the paragraph (see `theorem_head_separator.rs`).
//!
//! Before this, a multi-word `\begin{proof}[...]` head was ordinary text, so
//! its glue took part in the first line's stretch and shrink and moved
//! pdflatex's line breaks. The first paragraph below is the one in
//! `fixtures/proof-corpus/proof-qedhere`: pdflatex (TeX Live 2026, oracle
//! only, `\tracingparagraphs=1`) finds no feasible break on its first pass —
//! without the head's 4 × 1.11933pt of shrink the one-line setting is
//! overfull — and on its second pass breaks at `sat-` (`@\discretionary via
//! @@0 b=4 p=50`), while the pipeline squeezed the paragraph onto one line
//! (`ratio=-0.9361 b=82`). The second is from a 10pt prose stress document,
//! where the head's stretch pulled `tar-get;` together.
//!
//! Word origins are pdflatex's, read from its PDF (bp from the page top).

mod common;

use common::*;

const SRC: &str = r"\documentclass[11pt]{article}
\usepackage{amsthm}
\begin{document}
\begin{proof}[Proof with a displayed equation]
Equivalently, no positive integers $p, q$ satisfy
\begin{equation}
  p^2 = 2q^2 \qedhere
\end{equation}
\end{proof}

\begin{proof}[The measured compiler work]
is below the 200 ms ordinary warm-edit target; the one-word edit p95 is 29.225 ms, leaving 170.775 ms of that budget. The measured work is the compiler path only.
\end{proof}
\end{document}
";

fn find<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.text == text)
        .unwrap_or_else(|| panic!("no run {text:?} in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
}

#[test]
fn a_proof_head_is_one_natural_width_box() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let words = words_of(&render_one(SRC));

    // pdflatex: `Proof with a displayed equation. Equivalently, … sat-` /
    // `isfy`, the head at its natural spacing.
    let first = find(&words, "Equivalently,").baseline;
    let isfy = find(&words, "isfy");
    assert!(
        isfy.baseline > first + 1.0,
        "pdflatex breaks `sat-isfy` onto a second line; got `isfy` on baseline {} with the head's line at {first}",
        isfy.baseline
    );
    for (text, x) in [("Proof", 125.798), ("with", 155.088), ("displayed", 188.257), ("equation.", 234.819), ("Equivalently,", 283.209)] {
        let w = find(&words, text);
        assert!((w.x - x).abs() < 0.05, "`{text}`: pdflatex {x} bp, got {} bp", w.x);
        assert!((w.baseline - first).abs() < 0.01, "`{text}` is on the head's line");
    }

    // pdflatex: `The measured compiler work. is … warm-edit tar-` / `get; …`.
    let head_line = find(&words, "measured").baseline;
    let get = find(&words, "get;");
    assert!(get.baseline > head_line + 1.0, "pdflatex breaks `tar-get;`; got `get;` on the head's line");
    for (text, x) in [("The", 125.798), ("measured", 148.104), ("compiler", 195.354), ("work.", 239.547), ("is", 270.79)] {
        let w = words
            .iter()
            .find(|w| w.text == text && (w.baseline - head_line).abs() < 0.01)
            .unwrap_or_else(|| panic!("no run {text:?} on the head's line"));
        assert!((w.x - x).abs() < 0.05, "`{text}`: pdflatex {x} bp, got {} bp", w.x);
    }
}
