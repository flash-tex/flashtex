//! latex.ltx's generic `\begin{list}{<label>}{<decl>}` is the `\list` the
//! class lists are built on: at its depth it takes the same `\@list<i>`
//! margins and skips (`\leftmargini` 25pt, `\labelwidth` 20pt, `\topsep`,
//! `\itemsep`, `\parsep`), then `<decl>` may replace `\leftmargin` or
//! `\labelsep` for this list alone (`\labelwidth` stays the level's).
//! `\makelabel` is `\@mklab` (`\hfil #1`): a label narrower than
//! `\labelwidth` ends `\labelsep` before the text, a wider one pushes the
//! text right. The pipeline used to model none of this: every item sat flush
//! at the margin with its label hung into the left margin, 25 bp left of
//! pdfTeX's.
//!
//! The table is pdfTeX 3.141592653 (TeX Live 2026) origins in bp (x,
//! baseline from the page top) of each word's first glyph for this exact
//! source, `article` 10pt on US letter; pdflatex never runs here.

mod common;

use common::*;

#[test]
fn generic_list_margins_labels_and_skips_match_pdftex() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\\begin{document}\nBefore the lists.\n\\begin{list}{--}{}\n\\item Adefault one\n\\item Bdefault two that is long enough to wrap onto a second line of the text block here.\n\\end{list}\n\nBetween the lists.\n\\begin{list}{--}{\\setlength{\\leftmargin}{2em}}\n\\item Cmargin one\n\\end{list}\n\nSecond between.\n\\begin{list}{}{\\setlength{\\leftmargin}{1em}}\n\\item Dempty label\n\\item[Ex] Eexplicit label\n\\end{list}\n\nThird between.\n\\begin{list}{Long label:}{}\n\\item Fwide label\n\\end{list}\n\nFourth between.\n\\begin{itemize}\n\\item Gouter item\n\\begin{list}{--}{\\setlength{\\labelsep}{1em}}\n\\item Hnested generic\n\\end{list}\n\\end{itemize}\n\nAfter the lists.\n\\end{document}\n";
    assert_pdftex_glyphs(
        src,
        &[
            ("B", "CMR10", 148.712, 134.765), // Before
            ("–", "CMR10", 148.712, 154.690), // label, `\leftmargini` 25pt
            ("A", "CMR10", 158.675, 154.690), // Adefault
            ("–", "CMR10", 148.712, 174.615), // label
            ("B", "CMR10", 158.675, 174.615), // Bdefault
            ("b", "CMR10", 158.675, 186.570), // block: the wrapped line hangs too
            ("B", "CMR10", 148.712, 206.496), // Between
            ("–", "CMR10", 143.731, 226.421), // label, `\leftmargin` 2em
            ("C", "CMR10", 153.694, 226.421), // Cmargin
            ("S", "CMR10", 148.712, 246.346), // Second between
            ("D", "CMR10", 143.731, 266.272), // Dempty: an empty label
            ("E", "CMR10", 126.711, 286.197), // Ex: `\item[Ex]` in the level's 20pt `\labelwidth`
            ("E", "CMR10", 143.737, 286.197), // Eexplicit
            ("T", "CMR10", 148.712, 306.122), // Third between
            ("L", "CMR10", 133.768, 326.047), // Long label: wider than `\labelwidth`
            ("F", "CMR10", 187.327, 326.047), // Fwide: pushed right
            ("F", "CMR10", 148.712, 345.973), // Fourth between
            ("G", "CMR10", 158.677, 365.898), // Gouter
            ("–", "CMR10", 165.649, 385.823), // label, `\leftmarginii` 22pt, `\labelsep` 1em
            ("H", "CMR10", 180.593, 385.823), // Hnested
            ("A", "CMR10", 148.712, 405.748), // After
        ],
        0.1,
    );
}
