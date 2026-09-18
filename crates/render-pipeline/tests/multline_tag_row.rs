//! amsmath `multline` prints its single tag on the last row, wherever the
//! `\tag` was typed.
//!
//! ## Oracle
//!
//! TeX Live 2026 `/Library/TeX/texbin/pdflatex` (preamble
//! `\documentclass{article}\usepackage{amsmath}`, two passes) for
//! ```latex
//! \begin{multline}
//! a + a \\
//! b + b \tag{B}\label{m:mid} \\
//! c + c
//! \end{multline}
//! ```
//! sets `c+c   (B)` on one baseline (`pdftotext -bbox`: the `(B)` word at
//! y 182.64pt shares the `c + c` band, not the middle row's 167.70pt),
//! records `\newlabel{m:mid}{{{B}}...}` in `.aux`, and numbers the next
//! equation (1).

mod common;

use common::*;

const MIDDLE_TAG_MULTLINE: &str = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{multline}
a + a \\
b + b \tag{B} \\
c + c
\end{multline}
\end{document}
";

/// The middle-row `\tag{B}` is set on the last row's baseline: exactly one
/// `(B)` word, on the `c` baseline, not the `b` baseline.
#[test]
fn multline_middle_row_tag_is_set_on_the_last_row() {
    let r = render_one(MIDDLE_TAG_MULTLINE);
    let words = words_of(&r);
    let baseline = |text: &str| -> Vec<f64> {
        words.iter().filter(|w| w.text == text).map(|w| w.baseline).collect()
    };
    let tags = baseline("(B)");
    assert_eq!(tags.len(), 1, "expected exactly one tag word: {words:?}");
    let (a, b, c) = (baseline("a"), baseline("b"), baseline("c"));
    assert_eq!(a.len(), 2, "first row holds two a glyphs: {words:?}");
    assert_eq!(b.len(), 2, "middle row holds two b glyphs: {words:?}");
    assert_eq!(c.len(), 2, "last row holds two c glyphs: {words:?}");
    assert!(
        a[0] < b[0] && b[0] < c[0],
        "rows stack top to bottom: {a:?} {b:?} {c:?}"
    );
    assert!(
        (tags[0] - c[0]).abs() < 0.01,
        "tag at {:.3} must sit on the last row at {:.3}",
        tags[0],
        c[0]
    );
    assert!(
        (tags[0] - b[0]).abs() > 5.0,
        "tag at {:.3} must not sit on the middle row at {:.3}",
        tags[0],
        b[0]
    );
}
