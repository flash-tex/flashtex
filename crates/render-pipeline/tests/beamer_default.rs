//! beamer Tier 0 (page model) and Tier 1 (frame geometry) against pdflatex
//! (issue #944; corpus PR #943).
//!
//! Oracle: pdflatex 3.141592653-2.6-1.40.29 (TeX Live 2026),
//! `\documentclass{beamer}` with no packages, positions read back with
//! `tools/visual-oracle/pdftext.py` (bp from the paper's top-left corner,
//! baseline of the word's first glyph). The ladder document below is the
//! one in `crates/render-pipeline/src/typeset/beamer.rs`'s doc comment; its
//! reference numbers were taken in the same session as the fix (2026-09-18)
//! and the title page's from `fixtures/real-world/beamer-default`'s pinned
//! `reference.pdf`. Every assertion here failed on `origin/main` + #855 +
//! #887 (6 pages, roman body, page number, top-aligned frames) and passes
//! with the Tier 0/1 work.
//!
//! Needs the bundled Latin Modern faces and their metrics
//! (`FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` at TeX Live's
//! `lm`/`ec`/`amsfonts/symbols` TFMs), like every oracle test in this crate.

mod common;

use common::{lm_available, render_one, words_of, Word};
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::Rendered;

const PREAMBLE: &str = "\\documentclass{beamer}\n";

fn deck(body: &str) -> String {
    format!("{PREAMBLE}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// The ladder: 1/2/3/7-line bodies under a title, `[t]`, no title, a
/// subtitle, itemize + enumerate, `[b]`, an empty frame, a last frame.
fn ladder() -> String {
    deck(concat!(
        "\\begin{frame}\n\\frametitle{Title}\nOne line.\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Title}\nOne line.\\\\Two lines.\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Title}\nOne line.\\\\Two lines.\\\\Three lines.\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Title}\nOne line.\\\\Two lines.\\\\Three lines.\\\\Four lines.\\\\Five lines.\\\\Six lines.\\\\Seven lines.\n\\end{frame}\n",
        "\\begin{frame}[t]\n\\frametitle{Title}\nOne line.\\\\Two lines.\n\\end{frame}\n",
        "\\begin{frame}\nOne line.\\\\Two lines.\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Title}\n\\framesubtitle{Sub}\nOne line.\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Title}\n\\begin{itemize}\n\\item First item\n\\item Second item\n\\end{itemize}\n\\begin{enumerate}\n\\item First item\n\\item Second item\n\\end{enumerate}\n\\end{frame}\n",
        "\\begin{frame}[b]\n\\frametitle{Title}\nBottom line.\n\\end{frame}\n",
        "\\begin{frame}\n\\end{frame}\n",
        "\\begin{frame}\nLast frame.\n\\end{frame}\n",
    ))
}

/// The `nth` (0-based) glyph run reading exactly `text` on `page`.
fn word_n<'a>(words: &'a [Word], page: u32, text: &str, nth: usize) -> &'a Word {
    words
        .iter()
        .filter(|w| w.page == page && w.text == text)
        .nth(nth)
        .unwrap_or_else(|| panic!("no word {text:?} (#{nth}) on page {page}: {:?}", words.iter().filter(|w| w.page == page).map(|w| &w.text).collect::<Vec<_>>()))
}

fn word<'a>(words: &'a [Word], page: u32, text: &str) -> &'a Word {
    word_n(words, page, text, 0)
}

/// `(x, baseline)` of the first run reading `text` on `page`, within `tol`
/// bp of the oracle.
fn at(words: &[Word], page: u32, text: &str, x: f64, baseline: f64, tol: f64) {
    at_n(words, page, text, 0, x, baseline, tol)
}

fn at_n(words: &[Word], page: u32, text: &str, nth: usize, x: f64, baseline: f64, tol: f64) {
    let w = word_n(words, page, text, nth);
    assert!(
        (w.x - x).abs() <= tol && (w.baseline - baseline).abs() <= tol,
        "page {page} {text:?}: ours ({:.3}, {:.3}), pdflatex ({x:.3}, {baseline:.3}), tolerance {tol}",
        w.x,
        w.baseline
    );
}

fn font_names(r: &Rendered) -> Vec<String> {
    r.v2.fonts.iter().map(|f| f.postscript_name.clone()).collect()
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

#[test]
fn every_frame_is_a_page_including_the_empty_one() {
    if !lm_available() {
        return;
    }
    let r = render_one(&ladder());
    assert_eq!(r.v2.pages.len(), 11, "pdflatex ships 11 pages for the 11 frames (one of them empty)");
    let w = words_of(&r);
    assert!(w.iter().all(|w| w.page != 10), "the empty frame's page carries no text");
    at(&w, 11, "Last", 28.346, 115.386, 0.5);
}

#[test]
fn frame_body_sits_at_forty_percent_of_the_free_height() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&ladder()));
    // `[c]`: `0pt plus 1fill` above, `0pt plus 1.5fill` below.
    at(&w, 1, "One", 28.346, 129.059, 0.5);
    at(&w, 2, "One", 28.346, 123.639, 0.5);
    at(&w, 2, "Two", 28.346, 137.188, 0.5);
    at(&w, 3, "One", 28.346, 118.219, 0.5);
    at(&w, 4, "One", 28.346, 96.541, 0.5);
    at(&w, 4, "Seven", 28.346, 177.836, 0.5);
    // `[t]`: `.2cm plus .5\paperheight` above.
    at(&w, 5, "One", 28.346, 42.006, 0.5);
    // No frametitle: the body box is the whole `\textheight`.
    at(&w, 6, "One", 28.346, 109.966, 0.5);
    // `[b]`: everything above.
    at(&w, 9, "Bottom", 28.346, 268.141, 0.5);
}

#[test]
fn frametitle_box_and_subtitle() {
    if !lm_available() {
        return;
    }
    let r = render_one(&ladder());
    let w = words_of(&r);
    for page in [1u32, 2, 3, 4, 5, 7, 8, 9] {
        at(&w, page, "Title", 8.504, 21.057, 0.05);
    }
    for page in [6u32, 10, 11] {
        assert!(!w.iter().any(|w| w.page == page && w.text == "Title"), "page {page} has no frametitle");
    }
    let title = word(&w, 1, "Title");
    assert!((title.width - 27.6).abs() < 1.5, "`\\Large` cmss12 at 14.4pt: {title:?}");
    at(&w, 7, "Sub", 8.504, 35.104, 0.05);
    at(&w, 7, "One", 28.346, 136.232, 0.5);
    let (red, green, blue) = run_paint(&r, 1, "Title");
    assert!((red - 0.2).abs() < 1e-9 && (green - 0.2).abs() < 1e-9 && (blue - 0.7).abs() < 1e-9, "structure colour rgb(0.2,0.2,0.7), got ({red}, {green}, {blue})");
}

#[test]
fn body_is_sans_and_ragged_right_with_no_page_number() {
    if !lm_available() {
        return;
    }
    let r = render_one(&ladder());
    let names = font_names(&r);
    assert!(names.iter().any(|n| n.contains("LMSans10")), "cmss10 body -> LMSans10: {names:?}");
    assert!(names.iter().any(|n| n.contains("LMSans12")), "cmss12 frametitle -> LMSans12: {names:?}");
    assert!(!names.iter().any(|n| n.contains("LMRoman")), "no roman face on a default-theme deck: {names:?}");
    let w = words_of(&r);
    // No folio: nothing near the paper's bottom edge but the `[b]` frame's
    // own line, and no lone digit anywhere.
    for word in &w {
        assert!(!(word.text.chars().all(|c| c.is_ascii_digit()) && word.baseline > 250.0), "page number printed: {word:?}");
    }
    // `\raggedright`: the 7-line frame's lines start at the margin and no
    // line is stretched to the measure (every line is a short `\\` line).
    for first in ["One", "Two", "Three", "Four", "Five", "Six", "Seven"] {
        assert!((word(&w, 4, first).x - 28.346).abs() < 0.05, "{first} on page 4");
    }
    let line = word(&w, 4, "line.");
    assert!(line.x + line.width < 334.0, "a ragged line is never stretched to the measure: {line:?}");
}

#[test]
fn lists_use_beamers_margins_labels_and_skips() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&ladder()));
    // `\leftmargini` 2em, `\labelsep` .5em: text at 28.35 + 21.82.
    at_n(&w, 8, "First", 0, 50.165, 108.615, 0.5);
    // `\itemsep` 3pt: 16.54bp pitch.
    at_n(&w, 8, "Second", 0, 50.165, 125.153, 0.5);
    // The `\blacktriangleright` label box: msam10's advance, raised 1.25pt,
    // its origin at the label box's left edge.
    at(&w, 8, "\u{25B6}", 36.225, 107.370, 0.5);
    // Adjacent lists: closing `\topsep` + opening `\topsep` (a colour
    // whatsit between them keeps `\addvspace` from merging the two).
    at(&w, 8, "1.", 36.225, 144.680, 0.5);
    at_n(&w, 8, "First", 1, 50.165, 144.680, 0.5);
    at(&w, 8, "2.", 36.225, 161.218, 0.5);
}

#[test]
fn title_page_matches_the_corpus_reference() {
    if !lm_available() {
        return;
    }
    // `fixtures/real-world/beamer-default/main.tex`'s preamble and first frame.
    let src = "\\documentclass{beamer}\n\\title{Incremental Typesetting}\n\\subtitle{Why a from-scratch engine can be fast}\n\\author{J. Whitfield}\n\\institute{FlashTeX corpus}\n\\date{March 2026}\n\\begin{document}\n\\begin{frame}\n  \\titlepage\n\\end{frame}\n\\end{document}\n";
    let r = render_one(src);
    assert_eq!(r.v2.pages.len(), 1);
    let w = words_of(&r);
    at(&w, 1, "Incremental", 111.113, 85.412, 0.5);
    at(&w, 1, "Why", 92.660, 102.473, 0.5);
    at(&w, 1, "J.", 154.796, 140.016, 0.5);
    at(&w, 1, "FlashTeX", 152.164, 162.487, 0.5);
    at(&w, 1, "March", 154.342, 188.549, 0.5);
    let (red, green, blue) = run_paint(&r, 1, "Incremental");
    assert!((red - 0.2).abs() < 1e-9 && (green - 0.2).abs() < 1e-9 && (blue - 0.7).abs() < 1e-9);
}

#[test]
fn sections_between_frames_print_nothing() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck("\\begin{frame}\nA.\n\\end{frame}\n\\section{Motivation}\n\\begin{frame}\nB.\n\\end{frame}"));
    assert_eq!(r.v2.pages.len(), 2);
    let w = words_of(&r);
    assert!(!w.iter().any(|w| w.text.contains("Motivation") || w.text == "1"), "{w:?}");
}

#[test]
fn a_footnote_sits_at_the_frames_foot_and_the_body_keeps_its_place() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck("\\begin{frame}\n\\frametitle{Title}\nA claim with a note.\\footnote{The note text.} More words follow here.\n\\end{frame}"));
    let w = words_of(&r);
    at(&w, 1, "A", 28.346, 125.966, 0.5);
    at(&w, 1, "More", 131.503, 125.966, 0.5);
    at(&w, 1, "The", 44.934, 268.141, 0.5);
}
