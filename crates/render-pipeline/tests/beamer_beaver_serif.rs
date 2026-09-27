//! beamer `\usecolortheme{beaver}` + `\usefonttheme{serif}` against pdflatex.
//!
//! Oracle: pdflatex 3.141592653-2.6-1.40.29 (TeX Live 2026), minimal deck
//! `\documentclass{beamer}` + the theme lines + one frame with
//! `\frametitle{T}` and body `x`, positions read back with
//! `tools/visual-oracle/pdftext.py` (bp from the paper's top-left corner,
//! baseline of the word's first glyph):
//!
//! ```text
//! pdflatex -interaction=batchmode both.tex   % preamble has both lines
//! python3 tools/visual-oracle/pdftext.py both.pdf
//! ```
//!
//! ```text
//! both  (beaver+serif): T (8.504, 20.260) CMR12; x (28.346, 133.802) CMR10
//! serif only:           T (8.504, 21.256) CMR12; x (28.346, 129.298) CMR10
//! beaver only:          T (8.504, 20.061) CMSS12; x (28.346, 133.563) CMSS10
//! neither (default):    T (8.504, 21.057); x (28.346, 129.059)
//! ```
//!
//! Bisection: beaver moves the title up 1pt (`\nointerlineskip` before the
//! colour box) and grows the frametitle box by `sep` (the colour box's
//! `\@tempswatrue` skips the template's `\vskip-.3cm`), which pushes the
//! `[c]` body down 4.5bp; serif switches the text to roman, whose smaller
//! `1ex` lowers the title 0.2bp. The effects add.
//!
//! Needs the bundled Latin Modern faces and their metrics
//! (`FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` at TeX Live's
//! `lm`/`ec`/`amsfonts/symbols` TFMs), like every oracle test in this crate.

mod common;

use common::{lm_available, render_one, words_of, Word};
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::Rendered;

fn deck(preamble: &str) -> String {
    format!("{preamble}\\begin{{document}}\n\\begin{{frame}}\n\\frametitle{{T}}\nx\n\\end{{frame}}\n\\end{{document}}\n")
}

fn word<'a>(words: &'a [Word], page: u32, text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.page == page && w.text == text)
        .unwrap_or_else(|| {
            panic!(
                "no word {text:?} on page {page}: {:?}",
                words.iter().map(|w| &w.text).collect::<Vec<_>>()
            )
        })
}

/// `(x, baseline)` of the run reading `text` on `page`, within `tol` bp of
/// the oracle.
fn at(words: &[Word], page: u32, text: &str, x: f64, baseline: f64, tol: f64) {
    let w = word(words, page, text);
    assert!(
        (w.x - x).abs() <= tol && (w.baseline - baseline).abs() <= tol,
        "page {page} {text:?}: ours ({:.3}, {:.3}), pdflatex ({x:.3}, {baseline:.3}), tolerance {tol}",
        w.x,
        w.baseline
    );
}

fn font_names(r: &Rendered) -> Vec<String> {
    r.v2.fonts
        .iter()
        .map(|f| f.postscript_name.clone())
        .collect()
}

fn run_paint(r: &Rendered, page: u32, text: &str) -> (f64, f64, f64) {
    for p in &r.v2.pages {
        if p.number != page {
            continue;
        }
        for it in p.resident_items() {
            if let Item::GlyphRun(run) = it {
                if run.text.starts_with(text) {
                    return (run.paint.r, run.paint.g, run.paint.b);
                }
            }
        }
    }
    panic!("no run {text:?} on page {page}");
}

const BOTH: &str = "\\documentclass{beamer}\n\\usecolortheme{beaver}\n\\usefonttheme{serif}\n";
const SERIF: &str = "\\documentclass{beamer}\n\\usefonttheme{serif}\n";
const BEAVER: &str = "\\documentclass{beamer}\n\\usecolortheme{beaver}\n";

#[test]
fn beaver_and_serif_match_pdflatex() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&deck(BOTH)));
    at(&w, 1, "T", 8.504, 20.260, 0.1);
    at(&w, 1, "x", 28.346, 133.802, 0.1);
}

#[test]
fn serif_alone_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&deck(SERIF)));
    at(&w, 1, "T", 8.504, 21.256, 0.1);
    at(&w, 1, "x", 28.346, 129.298, 0.1);
}

#[test]
fn beaver_alone_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&deck(BEAVER)));
    at(&w, 1, "T", 8.504, 20.061, 0.1);
    at(&w, 1, "x", 28.346, 133.563, 0.1);
}

#[test]
fn serif_sets_roman_and_beaver_sets_the_title_colour() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck(BOTH));
    let names = font_names(&r);
    assert!(
        names.iter().any(|n| n.contains("LMRoman")),
        "serif body/title -> LMRoman faces: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n.contains("LMSans")),
        "no sans face on a serif deck: {names:?}"
    );
    let (red, green, blue) = run_paint(&r, 1, "T");
    assert!(
        (red - 0.8).abs() < 1e-9 && green.abs() < 1e-9 && blue.abs() < 1e-9,
        "beaver frametitle in darkred (0.8, 0, 0), got ({red}, {green}, {blue})"
    );
    let r = render_one(&deck(BEAVER));
    let names = font_names(&r);
    assert!(
        names.iter().any(|n| n.contains("LMSans")),
        "beaver alone keeps the sans faces: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n.contains("LMRoman")),
        "no roman face on a beaver-only deck: {names:?}"
    );
}

#[test]
fn unknown_colour_theme_and_postamble_commands_change_nothing() {
    if !lm_available() {
        return;
    }
    // A colour theme this engine does not model leaves the default deck
    // exactly alone.
    let w = words_of(&render_one(&deck(
        "\\documentclass{beamer}\n\\usecolortheme{seahorse}\n",
    )));
    at(&w, 1, "T", 8.504, 21.057, 0.1);
    at(&w, 1, "x", 28.346, 129.059, 0.1);
    // Theme commands after `\begin{document}` are not preamble: the deck
    // keeps the default theme (pdflatex reads them at `\use...` time too,
    // but beamer only honours them in the preamble).
    let w = words_of(&render_one("\\documentclass{beamer}\n\\begin{document}\n\\usecolortheme{beaver}\n\\usefonttheme{serif}\n\\begin{frame}\n\\frametitle{T}\nx\n\\end{frame}\n\\end{document}\n"));
    at(&w, 1, "T", 8.504, 21.057, 0.1);
    at(&w, 1, "x", 28.346, 129.059, 0.1);
}

#[test]
fn theme_commands_are_inert_under_article() {
    if !lm_available() {
        return;
    }
    // pdflatex rejects `\usecolortheme`/`\usefonttheme` under article
    // (`! Undefined control sequence`); the pipeline must not treat them
    // as beamer themes there. The themed article renders word-for-word
    // like the plain one, in roman only.
    let plain = "\\documentclass{article}\n\\begin{document}\nHi.\n\\end{document}\n";
    let themed = "\\documentclass{article}\n\\usecolortheme{beaver}\n\\usefonttheme{serif}\n\\begin{document}\nHi.\n\\end{document}\n";
    let (w0, w1) = (words_of(&render_one(plain)), words_of(&render_one(themed)));
    assert_eq!(w0.len(), w1.len(), "same words: {w0:?} vs {w1:?}");
    for (a, b) in w0.iter().zip(w1.iter()) {
        assert!(
            a.text == b.text && (a.x - b.x).abs() < 1e-9 && (a.baseline - b.baseline).abs() < 1e-9,
            "{a:?} vs {b:?}"
        );
    }
    let names = font_names(&render_one(themed));
    assert!(
        !names.iter().any(|n| n.contains("Sans")),
        "no beamer sans on an article: {names:?}"
    );
}
