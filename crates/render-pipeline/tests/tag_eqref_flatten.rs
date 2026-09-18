//! A rich `\tag` and an `\eqref` to it must flatten identically.
//!
//! The compiler stores the label value with
//! `text_run_reference_text_with_source`, so a composite math atom with no
//! single rendered glyph (e.g. `\frac`) falls back to its source text in an
//! `\eqref`. `adapter::strip_tag` used the without-source variant for the
//! tag's own display, which falls back to `(1/2)` instead — so
//! `\tag{$\frac{1}{2}$}` printed `((1/2))` on the display while `\eqref`
//! to it printed `(\frac{1}{2})`. A reader following the cross-reference
//! saw different text than the tag itself shows.
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

/// The tag's own display and the `\eqref` reference read alike: the only
/// parenthesized words on the page are the tag and the reference (up to the
/// sentence period after the reference), and they are equal.
#[test]
fn frac_tag_and_eqref_produce_identical_flattened_text() {
    let r = render_one(FRAC_TAG_EQREF);
    let words: Vec<String> = words_of(&r).into_iter().map(|w| w.text).collect();
    let paren: Vec<&String> = words.iter().filter(|w| w.starts_with('(')).collect();
    assert_eq!(paren.len(), 2, "expected exactly the tag and one \\eqref: {words:?}");
    let tag = paren.iter().find(|w| !w.ends_with('.')).expect("tag word: {words:?}");
    let eqref = paren.iter().find(|w| w.ends_with('.')).expect("\\eqref word: {words:?}");
    // The composite atom falls back to its source text on both paths, not
    // to the sourceless `(1/2)` approximation.
    assert!(tag.contains("\\frac"), "tag lost its source fallback: {words:?}");
    assert_eq!(
        eqref.strip_suffix('.').unwrap_or(eqref),
        tag.as_str(),
        "tag display {tag:?} and \\eqref {eqref:?} differ: {words:?}"
    );
}
