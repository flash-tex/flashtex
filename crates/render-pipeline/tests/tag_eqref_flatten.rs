//! A rich `\tag` and an `\eqref` to it must set the same content.
//!
//! Before #441's pipeline half, the tag was an *approximation*: the label
//! was set as the compiler's flattened text, and a composite math atom with
//! no single rendered glyph (`\frac`) fell back to its source text --
//! `\tag{$\frac{1}{2}$}` printed `(\frac{1}{2})`, and the `\eqref` to it
//! had to print the same so a reader following the cross-reference saw
//! what the tag showed. Now both are set as amsmath sets them: the tag as
//! `\maketag@@@`'s `\hbox` and the reference as `\textup{\tagform@{..}}`,
//! so the fraction is a fraction on both paths, its numerals stacked
//! `\textstyle`-style at the script size and no `\frac` on the page.
//!
//! Measured against pdflatex (TeX Live 2026, 10pt article): the tag's
//! `1`/`2` at x 468.438 bp, 3.923 bp above and 3.435 bp below the line's
//! baseline; the reference's at 156.550 bp with the same offsets.
#![cfg(feature = "compiler-node-surface")]

mod common;

use common::*;

const FRAC_TAG_EQREF: &str = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{equation} a = b \tag{$\frac{1}{2}$} \label{e:half} \end{equation}
See \eqref{e:half}.
\end{document}
";

/// The tag's display and the `\eqref` to it: parentheses on the baseline
/// around the fraction's numerals (one v2 run, `12`, whose first glyph sits
/// on the numerator's baseline) on both, at the same offsets from the
/// parenthesis, and no source text anywhere.
#[test]
fn frac_tag_and_eqref_set_the_same_fraction() {
    if !lm_available() {
        eprintln!("SKIP: Latin Modern fonts not installed");
        return;
    }
    let r = render_one(FRAC_TAG_EQREF);
    let words = words_of(&r);
    assert!(words.iter().all(|w| !w.text.contains("frac")), "\\frac source text on the page: {words:?}");
    let opens: Vec<&Word> = words.iter().filter(|w| w.text == "(").collect();
    assert_eq!(opens.len(), 2, "expected the tag's and the reference's `(`: {words:?}");
    // The numerals' run relative to each `(`: (dx, dy).
    let stack = |open: &Word| -> (f64, f64) {
        let run = words
            .iter()
            .find(|w| w.text == "12" && w.x > open.x && w.x < open.x + 12.0)
            .unwrap_or_else(|| panic!("no stacked fraction after `(` at {}: {words:?}", open.x));
        (run.x - open.x, run.baseline - open.baseline)
    };
    let (tag, eqref) = (stack(opens[0]), stack(opens[1]));
    // pdflatex: the numerator 5.066 bp in and 3.923 bp above the baseline.
    assert!((tag.0 - 5.066).abs() < 0.5 && (tag.1 + 3.923).abs() < 0.5, "tag numerator off pdflatex: {tag:?}");
    assert!((tag.0 - eqref.0).abs() < 0.01 && (tag.1 - eqref.1).abs() < 0.01, "tag {tag:?} and \\eqref {eqref:?} differ");
}
