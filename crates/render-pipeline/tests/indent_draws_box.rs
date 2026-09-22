//! `\indent` draws the first-line indent box for the paragraph that follows
//! it, even immediately after a heading, where `\@afterheading` suppresses
//! it (`\section{X}\indent Text.`). The mirror image of `\noindent`, which
//! only suppresses the box.
//!
//! The test is differential: the same document with and without `\indent`
//! must differ in the first word's left edge by exactly `\parindent`
//! (article at 10pt: 15pt, i.e. `15 * 72 / 72.27` bp), so no oracle
//! constants are baked in.

mod common;

/// Tolerance in bp, matching the oracle-backed indent tests.
const TOL: f64 = 0.5;

/// `\parindent` of 10pt article in bp (`size10.clo`: 15pt; TeX
/// `1in = 72.27pt = 72bp`).
const PARINDENT_BP: f64 = 15.0 * 72.0 / 72.27;

const WITH_INDENT: &str = r"\documentclass{article}
\pagestyle{empty}
\begin{document}
\section{X}\indent Text.
\end{document}
";

const WITHOUT_INDENT: &str = r"\documentclass{article}
\pagestyle{empty}
\begin{document}
\section{X}Text.
\end{document}
";

fn first_text_x(src: &str) -> f64 {
    let r = common::render_one(src);
    common::words_of(&r)
        .into_iter()
        .find(|w| w.text == "Text.")
        .unwrap_or_else(|| panic!("no 'Text.' word rendered for {src:?}"))
        .x
}

#[test]
fn indent_after_heading_draws_parindent_box() {
    if !common::lm_available() {
        eprintln!("SKIP indent_draws_box: Latin Modern not installed");
        return;
    }
    let with = first_text_x(WITH_INDENT);
    let without = first_text_x(WITHOUT_INDENT);
    assert!(
        (with - without - PARINDENT_BP).abs() < TOL,
        "`\\indent` after a heading must shift the first word by \\parindent: with={with:.3}bp without={without:.3}bp want delta={PARINDENT_BP:.3}bp",
    );
}
