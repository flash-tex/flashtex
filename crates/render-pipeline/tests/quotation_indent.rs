//! `quotation` (article.cls: `\list{}{\listparindent 1.5em \itemindent
//! \listparindent ...}\item\relax`) starts every paragraph `\listparindent`
//! in: `\list` copies it into `\parindent`, and the first paragraph gets
//! the same width from `\@item` as `\itemindent`. The compiler carries the
//! length on the quotation's list frame (d868ad8bb); the pipeline used to
//! set every quotation paragraph flush at the margin.
//!
//! Only `\noindent` removes it, and for the first paragraph only a
//! `\noindent` written there: `\@item`'s `\everypar` replaces an `\@endpe`
//! one left by an `\end{itemize}` right before the `\begin`. `quote` sets
//! no `\listparindent` and stays flush.
//!
//! The tables are pdfTeX 3.141592653 (TeX Live 2026) origins in bp (x,
//! baseline from the page top) of each word's first glyph for these exact
//! sources on US letter; pdflatex never runs here.

mod common;

use common::*;

const TOL_BP: f64 = 0.1;

#[test]
fn quotation_paragraphs_start_listparindent_in() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\\begin{document}\nBefore text.\n\\begin{quotation}\nFirst paragraph of the quotation that is long enough to wrap onto a second line of text here.\n\nSecond paragraph of it.\n\n\\noindent Third paragraph, not indented.\n\\end{quotation}\n\\begin{quote}\nQuote one.\n\\end{quote}\nAfter.\n\\end{document}\n";
    assert_pdftex_glyphs(
        src,
        &[
            ("B", "CMR10", 148.712, 134.765),  // Before
            ("F", "CMR10", 173.619, 154.690),  // First: 1.5em = 15pt in
            ("w", "CMR10", 431.023, 154.690),  // wrap
            ("o", "CMR10", 158.675, 166.645),  // onto: the wrapped line is flush
            ("h", "CMR10", 272.478, 166.645),  // here.
            ("S", "CMR10", 173.619, 178.600),  // Second
            ("T", "CMR10", 158.675, 190.555),  // Third (\noindent)
            ("Q", "CMR10", 158.675, 212.473),  // Quote: `quote` stays flush
            ("A", "CMR10", 133.768, 234.391),  // After.
        ],
        TOL_BP,
    );
}

#[test]
fn quotation_indent_at_11pt_after_a_list_and_around_a_nested_quote() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass[11pt]{article}\n\\begin{document}\n\\begin{itemize}\n\\item Item text.\n\\end{itemize}\n{\\small\n\\begin{quotation}\nSmall quotation paragraph.\n\nSmall second.\n\\end{quotation}}\n\\begin{quotation}\nOpen words.\n\\begin{quote}\nNested quote.\n\\end{quote}\nBack in the quotation.\n\nLast one.\n\\end{quotation}\n\\end{document}\n";
    assert_pdftex_glyphs(
        src,
        &[
            ("I", "CMR10", 153.069, 140.742), // Item
            // `\small` inside the group: 1.5em of the 10pt font the
            // `\begin` was read in, 14.94 bp.
            ("S", "CMR10", 168.015, 164.653), // Small (right after `\end{itemize}`)
            ("S", "CMR10", 168.015, 176.608), // Small second
            // 11pt: 1.5em = 16.43pt, not `\parindent` (17pt).
            ("O", "CMR10", 169.435, 202.112), // Open
            // `\@endpe` after the nested quote's `\end`: no indent.
            ("B", "CMR10", 153.071, 238.177), // Back
            ("L", "CMR10", 169.435, 251.726), // Last
        ],
        TOL_BP,
    );
}
