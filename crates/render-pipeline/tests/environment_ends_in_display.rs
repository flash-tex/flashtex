//! An environment whose last material is a display, followed by text on the
//! next source line.
//!
//! latex.ltx's `\endtrivlist` runs `\par` (so the text after it never
//! continues the display's paragraph), turns a positive `\lastskip` into
//! `\lastskip + \parskip - \@outerparskip`, and then `\@endparenv`'s
//! `\addvspace\@topsepadd` keeps the larger of that and the environment's
//! own skip. amsthm's `proof` and theorem-like environments end with
//! `\@endpefalse`, so the text after them is indented.
//!
//! Before this fix the pipeline got four of these wrong (11 pt article):
//!
//! - `proof` ending in `\[..\qedhere\]`: the next line was merged into the
//!   display's paragraph, 2.49 bp too high (`\belowdisplayshortskip` alone
//!   instead of max(it, `\@topsepadd` 9 pt)) and unindented;
//! - `proof` ending in `align*` with `\qedhere`, and a theorem ending in
//!   `\[..\]`: the next line unindented;
//! - `quote` ending in a display: the display's skip and `\topsep` were
//!   added (6.5 + 9 pt) where pdfTeX takes max(6.5 + `\parsep` 4.5, 9) = 11
//!   pt, 4.48 bp too low;
//! - `center` ending in a display: the centred line before it has an
//!   infinitely stretched `\leftskip`, so `\predisplaysize` is `\maxdimen`
//!   (§1146-§1148) and the display takes the full `\abovedisplayskip`, not
//!   the short one (the display was 8.97 bp too high).
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2026), 11 pt article, letter,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`; glyph origins read from the
//! PDF with PyMuPDF (`rawdict`). The `\showoutput` of the quote case:
//!
//! ```text
//! ...\glue(\belowdisplayshortskip) 6.5 plus 3.5 minus 3.0
//! ...\glue -6.5 plus -3.5 minus -3.0
//! ...\glue 11.0 plus 4.5 minus 4.0          % + \parsep - \@outerparskip
//! ...\glue -13.12917 plus -4.5 minus -4.0   % \addpenalty
//! ...\penalty -51
//! ...\glue 2.12917
//! ...\glue 11.0 plus 4.5 minus 4.0          % \addvspace{9pt}: 11 wins
//! ```
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

const TOL: f64 = 0.15;

const ENDS: &str = r"\documentclass[11pt]{article}
\usepackage{amsmath,amsthm}
\newtheorem{theorem}{Theorem}
\pagestyle{empty}
\begin{document}
Lead paragraph text.
\begin{proof}
Display ending.
\[ x = y. \qedhere \]
\end{proof}
Aone after the proof.
\begin{proof}
Alignment ending.
\begin{align*}
x &= y. \qedhere
\end{align*}
\end{proof}
Atwo after the proof.
\begin{theorem}
Theorem ending.
\[ x = y. \]
\end{theorem}
Athree after the theorem.
\begin{quote}
Quote ending.
\[ x = y. \]
\end{quote}
Afour after the quote.
\begin{center}
Center ending.
\[ x = y. \]
\end{center}
Afive after the center.
\end{document}
";

/// (glyph, pdfTeX font, x bp, baseline bp): every display's `x` and the
/// first letter of the line after each environment.
const END_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("x", "CMMI10", 290.351, 176.807),
    // After `\[..\qedhere\]` + `\end{proof}`: indented, max(6.5, 9) pt.
    ("A", "CMR10", 142.735, 199.323),
    ("x", "CMMI10", 290.351, 246.346),
    // After `align*` with `\qedhere`: indented.
    ("A", "CMR10", 142.735, 270.854),
    ("x", "CMMI10", 290.351, 317.878),
    // After a theorem ending in a display: indented.
    ("A", "CMR10", 142.735, 342.386),
    ("x", "CMMI10", 290.351, 378.451),
    // After `quote`: unindented (`\@endpe`), 11 pt below the display.
    ("A", "CMR10", 125.798, 402.959),
    // In `center`: the full `\abovedisplayskip`.
    ("x", "CMMI10", 290.351, 449.983),
    ("A", "CMR10", 125.798, 474.491),
];

#[test]
fn text_after_an_environment_that_ends_in_a_display_matches_pdftex() {
    if !common::lm_available() {
        eprintln!("SKIP text_after_an_environment_that_ends_in_a_display_matches_pdftex: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(ENDS, END_GLYPHS, TOL);
}
