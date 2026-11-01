//! `\noindent` governs its paragraph whatever sits between it and the first
//! word, unless that ends the paragraph (#958).
//!
//! `\noindent` in vertical mode is `new_graf` without the indent box (TeX
//! §1091), so `\noindent{A}`, `\noindent{\small A}`, `\noindent\small A`,
//! `\noindent\textbf{C}`, `\noindent \emph{C}` and a comment after the
//! command all start an unindented paragraph. A blank line or `\par` after
//! it ends that (empty, discarded) paragraph, and the next one is indented.
//! FlashTeX used to accept only whitespace between the command and the
//! paragraph's first material, so every one of the first six was indented by
//! `\parindent` (17.559 bp at 12 pt) and the blank-line case was not.
//!
//! ## Oracle
//!
//! pdfTeX 1.40.29 (TeX Live 2026, `/Library/TeX/texbin/pdflatex`) on
//! [`PROBE`]; glyph origins from the PDF's content stream
//! (`tools/visual-oracle/pdftext.py`), in bp, baseline from the page top. The
//! pdfTeX font of each glyph is recorded for reference: the size-switched
//! first words are CMR10/CMTI10 at 10.95 pt (`\small` in 12pt article) and
//! CMBX12 at 14.4 pt (`\large`), which [`first_word_sizes`] pins.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

const TOL: f64 = 0.5;

const PROBE: &str = r"\documentclass[12pt]{article}
\pagestyle{empty}
\begin{document}
\noindent{A} B

\noindent{\small A} B

{\small\noindent A} B

\noindent\small A B

\normalsize\noindent\textbf{C} D

\noindent \emph{C} D

\noindent % a comment
C D

\noindent

C D

\noindent{\bfseries\large C} D

\noindent\textit{\small C} D

\noindent\par C D
\end{document}
";

/// pdfTeX's glyphs: (text, pdfTeX font@size, x bp, baseline y bp).
const EXPECT: &[(&str, &str, f64, f64)] = &[
    ("A", "CMR12@11.96", 110.854, 137.753),
    ("B", "CMR12@11.96", 123.527, 137.753),
    ("A", "CMR10@10.91", 110.854, 152.199),
    ("B", "CMR12@11.96", 122.933, 152.199),
    ("A", "CMR10@10.91", 110.854, 166.645),
    ("B", "CMR12@11.96", 122.933, 166.645),
    ("A", "CMR10@10.91", 110.854, 180.194),
    ("B", "CMR10@10.91", 122.669, 180.194),
    ("C", "CMBX12@11.96", 110.854, 194.640),
    ("D", "CMR12@11.96", 124.465, 194.640),
    ("C", "CMTI12@11.96", 110.854, 209.086),
    ("D", "CMR12@11.96", 124.865, 209.086),
    ("C", "CMR12@11.96", 110.854, 223.532),
    ("D", "CMR12@11.96", 123.206, 223.532),
    ("C", "CMR12@11.96", 128.413, 237.978),
    ("D", "CMR12@11.96", 140.765, 237.978),
    ("C", "CMBX12@14.35", 110.854, 252.423),
    ("D", "CMR12@11.96", 126.408, 252.423),
    ("C", "CMTI10@10.91", 110.854, 266.869),
    ("D", "CMR12@11.96", 124.148, 266.869),
    ("C", "CMR12@11.96", 128.413, 281.315),
    ("D", "CMR12@11.96", 140.765, 281.315),
];

#[test]
fn noindent_reaches_the_paragraph_through_groups_and_declarations() {
    if !common::lm_available() {
        eprintln!("SKIP noindent_before_group: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(PROBE, EXPECT, TOL);
}

/// The first word's size on each line: a size command next to `\noindent`
/// applies to it (pdfTeX's font sizes in bp, from the PDF's `Tf`).
#[test]
fn first_word_sizes() {
    if !common::lm_available() {
        eprintln!("SKIP noindent_before_group: Latin Modern not installed");
        return;
    }
    let r = common::render_one(PROBE);
    let mut sizes = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let flashtex_render_pipeline::display::Item::GlyphRun(run) = item else { continue };
        if run.text.starts_with('A') || run.text.starts_with('C') {
            sizes.push(run.font_size.to_bp());
        }
    }
    let want = [11.96, 10.91, 10.91, 10.91, 11.96, 11.96, 11.96, 11.96, 14.35, 10.91, 11.96];
    assert_eq!(sizes.len(), want.len(), "{sizes:?}");
    for (got, want) in sizes.iter().zip(want) {
        assert!((got - want).abs() < 0.02, "sizes {sizes:?}, pdfTeX {want:?}");
    }
}
