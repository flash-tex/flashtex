//! A `tikzpicture` environment in a paragraph with other material -- words
//! before or after it, a second picture, `\qquad` glue -- is one box in the
//! line, as pdflatex sets it (`\pgfpicture` ends with
//! `\leavevmode\box\pgfpic`). It used to be split out as a picture on a
//! line of its own, putting the words before and after it on separate
//! lines. A picture alone in its paragraph keeps the `Block::Picture` path.
//!
//! Expected positions are pdflatex's (MacTeX 2026, oracle only; read back
//! with tools/visual-oracle/pdftext.py from the PDF of `SRC`), in bp from
//! the page's top-left corner. The cargo test never runs TeX.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const SRC: &str = r"\documentclass{article}
\usepackage{tikz}
\begin{document}
Plain paragraph first.

Aa \begin{tikzpicture}\draw (0,0) rectangle (1,1);\end{tikzpicture} Bb.

Text then \begin{tikzpicture}\draw (0,0) circle (5pt);\end{tikzpicture} text after, and enough words to wrap this paragraph onto a second line so that the line spacing after a picture is checked as well.

\begin{tikzpicture}\draw (0,0) rectangle (.6,.4);\end{tikzpicture} starts this paragraph, with its indent.

\noindent\begin{tikzpicture}\draw (0,0) rectangle (.6,.4);\end{tikzpicture} follows a noindent.

\begin{tikzpicture}\draw (0,0) rectangle (1,.5);\end{tikzpicture}
\begin{tikzpicture}\draw (0,0) rectangle (1,.5);\end{tikzpicture}

\begin{center}
\begin{tikzpicture}\draw (0,0) rectangle (1,.5);\end{tikzpicture}\qquad
\begin{tikzpicture}\draw (0,0) circle (.25);\end{tikzpicture}
\end{center}

Middle text.

\begin{itemize}
\item Words \begin{tikzpicture}[baseline=-.5ex]\draw (0,0) rectangle (1,.3);\end{tikzpicture} words.
\end{itemize}

Last line.
\end{document}
";

/// pdflatex's word origins (x, baseline y) in bp.
const EXPECTED: &[(&str, f64, f64)] = &[
    ("Plain", 148.712, 134.765),
    // A 1cm square on the baseline between two words: one tall line.
    ("Aa", 148.712, 166.443),
    ("Bb.", 196.552, 166.443),
    ("Text", 148.712, 178.398),
    ("then", 172.575, 178.398),
    ("text", 210.192, 178.398),
    ("after,", 231.562, 178.398),
    ("second", 133.768, 190.353),
    ("that", 196.975, 190.353),
    // A picture opening a paragraph takes the `\parindent`; after
    // `\noindent` it does not.
    ("starts", 169.440, 205.024),
    ("follows", 154.496, 219.694),
    // Two pictures side by side, then two centred with `\qquad` between:
    // one line each (their heights set "Middle").
    ("Middle", 148.712, 282.710),
    ("Words", 158.676, 304.628),
    ("words.", 221.819, 304.628),
    ("Last", 148.712, 326.546),
];

/// `\item \begin{tikzpicture}`: the label is material, so the picture sits
/// after the bullet on the item's first line (it used to lose its bullet
/// and sit 12 bp low).
const LIST: &str = r"\documentclass{article}
\usepackage{tikz}
\begin{document}
Plain paragraph first.

\begin{itemize}
\item One.
\item \begin{tikzpicture}\draw (0,0) rectangle (1,.3);\end{tikzpicture}
\item Words \begin{tikzpicture}\draw (0,0) rectangle (1,.3);\end{tikzpicture} words.
\item Three.
\end{itemize}

Last line.
\end{document}
";

const LIST_EXPECTED: &[(&str, f64, f64)] = &[
    ("One.", 158.676, 156.682),
    ("•", 148.714, 176.608),
    ("•", 148.714, 196.533),
    ("Words", 158.676, 196.533),
    ("words.", 221.819, 196.533),
    ("•", 148.714, 216.458),
    ("Three.", 158.676, 216.458),
    ("Last", 148.712, 238.376),
];

#[test]
fn a_tikzpicture_among_words_is_a_box_in_the_line() {
    check(SRC, EXPECTED, 9);
}

#[test]
fn a_tikzpicture_after_an_item_label_is_on_the_items_line() {
    check(LIST, LIST_EXPECTED, 2);
}

fn check(src: &str, expected: &[(&str, f64, f64)], pictures: usize) {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let docs = [SourceDocument { path: "main.tex", text: src }];
    let out = render(&docs, "main.tex", 1, "tikz-inline-paragraph", &fonts, &RenderOptions::default());
    for d in &out.v2.diagnostics {
        assert!(d.severity != flashtex_render_pipeline::display::Severity::Error, "error diagnostic: {d:?}");
    }
    let page = &out.v2.pages[0];
    let runs: Vec<(String, f64, f64)> = page
        .resident_items()
        .iter()
        .filter_map(|i| match i {
            Item::GlyphRun(r) if !r.glyphs.is_empty() => Some((r.text.clone(), r.glyphs[0].origin_x.0 as f64 / TICKS_PER_BP, r.glyphs[0].baseline_y.0 as f64 / TICKS_PER_BP)),
            _ => None,
        })
        .collect();
    let texts: Vec<&str> = runs.iter().map(|r| r.0.as_str()).collect();
    assert!(!texts.iter().any(|t| t.contains("draw") || t.contains("rectangle") || t.contains(';')), "picture source leaked as text: {texts:?}");
    let mut from = 0;
    for &(word, x, y) in expected {
        let Some(at) = runs[from..].iter().position(|r| r.0 == word) else {
            panic!("word {word:?} not found after index {from}: {texts:?}");
        };
        let r = &runs[from + at];
        assert!((r.1 - x).abs() <= 0.01, "{word:?}: x {:.3} vs pdflatex {x:.3}", r.1);
        assert!((r.2 - y).abs() <= 0.01, "{word:?}: baseline {:.3} vs pdflatex {y:.3}", r.2);
        from += at + 1;
    }
    // Every picture is painted once, one path each.
    let paths = page.resident_items().iter().filter(|i| matches!(i, Item::Path(_))).count();
    assert_eq!(paths, pictures, "expected each picture's path once");
}
