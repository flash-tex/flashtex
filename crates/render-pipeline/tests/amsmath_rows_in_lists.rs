//! amsmath alignments (`align`, `gather`, `multline`, `\intertext`) inside a
//! list item are laid out in the item's measure, not the page's.
//!
//! TeX §1149 sets `\displaywidth` and `\displayindent` from the paragraph's
//! `\parshape`; inside a list that is `\linewidth` and `\@totalleftmargin`.
//! amsmath measures every row against `\displaywidth` and the finished
//! `\halign` is shifted `\displayindent` right (§1206), so an `align*` in an
//! `enumerate` item centres in the item's measure: half the list's
//! `\leftmargin` right of where it would sit at top level. `multline`'s
//! first row starts `\multlinegap` from the item's margin, and `\intertext`
//! is set with `\parshape\@ne\@totalleftmargin\linewidth`.
//!
//! The pipeline centred one-line displays (`\[..\]`, `equation`) this way
//! already; the row path (`rows_block`) used the full `\textwidth` and put
//! every row 13.64 bp (level 1: `\leftmargini` 27.5 pt / 2) left of pdfTeX,
//! 25.64 bp at level 2, `multline`'s first row and `\intertext` 27.27 bp.
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2026), 11 pt article, letter,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`; glyph origins read from the
//! PDF with PyMuPDF (`rawdict`, span origins). pdflatex is an oracle only
//! and never runs in the product path.

mod common;

const TOL: f64 = 0.15;

const LISTS: &str = r"\documentclass[11pt]{article}
\usepackage{amsmath}
\pagestyle{empty}
\begin{document}
\begin{enumerate}
\item One.
\begin{align*}
a &= b + c
\end{align*}
\item Two.
\begin{align}
p &= q
\end{align}
\item Three.
\begin{gather*}
g = h
\end{gather*}
\item Four.
\begin{multline*}
m + m + m + m + m + m + m + m + m + m \\
= n + n + n + n + n + n
\end{multline*}
\item Five.
\begin{enumerate}
\item Inner.
\begin{align*}
u &= v
\end{align*}
\end{enumerate}
\item Six.
\begin{align*}
r &= s \\
\intertext{so that}
t &= w
\end{align*}
\end{enumerate}
\end{document}
";

/// (glyph, pdfTeX font, x bp, baseline bp).
const LIST_GLYPHS: &[(&str, &str, f64, f64)] = &[
    // align* at level 1: centred in \linewidth, \@totalleftmargin in.
    ("a", "CMMI10", 297.238, 165.250),
    // align with its number: the tag sits flush right at the item's edge,
    // which here is the text's right margin (\rightmargin is 0).
    ("p", "CMMI10", 306.114, 218.750),
    ("q", "CMMI10", 326.154, 218.750),
    ("(", "CMR10", 470.514, 218.750),
    ("1", "CMR10", 474.758, 218.750),
    ("g", "CMMI10", 305.549, 272.249),
    ("h", "CMMI10", 325.687, 272.249),
    // multline: first row \multlinegap from the item's margin, last row
    // \multlinegap from the right edge.
    ("m", "CMMI10", 163.034, 328.737),
    ("n", "CMMI10", 467.945, 345.275),
    // Level 2: \@totalleftmargin is \leftmargini + \leftmarginii.
    ("u", "CMMI10", 317.527, 421.290),
    ("v", "CMMI10", 338.331, 421.290),
    // \intertext at the item's margin, the rows around it centred.
    ("r", "CMMI10", 304.825, 472.299),
    ("s", "CMMI10", 324.592, 472.299),
    ("s", "CMR10", 153.071, 499.796),
    ("o", "CMR10", 157.369, 499.796),
    ("t", "CMMI10", 306.111, 527.293),
    ("w", "CMMI10", 324.602, 527.293),
];

#[test]
fn amsmath_rows_in_a_list_item_use_the_items_measure() {
    if !common::lm_available() {
        eprintln!("SKIP amsmath_rows_in_a_list_item_use_the_items_measure: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(LISTS, LIST_GLYPHS, TOL);
}
