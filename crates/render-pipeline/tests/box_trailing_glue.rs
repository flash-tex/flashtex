//! An `\hbox`'s trailing glue is part of the box.
//!
//! TeX drops the glue at the end of a horizontal list only when `\par`
//! ends a paragraph (§1096); the content of an `\hbox` keeps it. So
//! `\underline{\hspace{4cm}}` -- the fill-in blank on every worksheet and
//! exam cover ("Name: \underline{\hspace{4cm}}") -- is 4 cm wide, and
//! `\mbox{\hspace{1cm}}`, `\colorbox{..}{\hspace{1cm}}`, `\underline{x }`
//! and `\textsuperscript{a\hspace{5mm}}` keep their last glue too.
//!
//! The pipeline built every box's content with the paragraph-end rule, so
//! each of these lost its trailing glue: the blank was 0 pt wide and the
//! text after it 113.39 bp (4 cm) too far left.
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2026), 10 pt article, letter,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`; glyph origins read from the
//! PDF with PyMuPDF (`rawdict`). pdflatex is an oracle only and never runs
//! in the product path.

mod common;

const TOL: f64 = 0.15;

const BOXES: &str = r"\documentclass{article}
\usepackage{xcolor}
\pagestyle{empty}
\begin{document}
Name: \underline{\hspace{4cm}} Date: \underline{\hspace{2cm}}

Ba \mbox{\hspace{1cm}} Bb

Ca \underline{x\hspace{1cm}} Cb

Ea \colorbox{yellow}{\hspace{1cm}} Eb

Fa \underline{x } Fb

Ga \underline{\quad} Gb

Ha \textsuperscript{a\hspace{5mm}} Hb
\end{document}
";

/// (glyph, pdfTeX font, x bp, baseline bp): the first letter of each
/// line, and the word after each box.
const BOX_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("N", "CMR10", 148.712, 134.765),
    ("D", "CMR10", 297.798, 134.765),
    ("B", "CMR10", 148.712, 146.720),
    ("B", "CMR10", 195.735, 146.720),
    ("C", "CMR10", 148.712, 158.675),
    ("C", "CMR10", 201.135, 158.675),
    ("E", "CMR10", 148.712, 170.630),
    ("E", "CMR10", 201.440, 170.630),
    ("F", "CMR10", 148.712, 182.585),
    ("F", "CMR10", 174.588, 182.585),
    ("G", "CMR10", 148.712, 194.540),
    ("G", "CMR10", 178.116, 194.540),
    ("H", "CMR10", 148.712, 206.496),
    ("H", "CMR10", 186.450, 206.496),
];

#[test]
fn a_boxes_trailing_glue_is_part_of_its_width() {
    if !common::lm_available() {
        eprintln!("SKIP a_boxes_trailing_glue_is_part_of_its_width: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(BOXES, BOX_GLYPHS, TOL);
}
