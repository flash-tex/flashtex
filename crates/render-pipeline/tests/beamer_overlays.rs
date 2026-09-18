//! beamer Tier 2, overlays (issue #944; corpus PR #943) against pdflatex.
//!
//! Oracle: pdflatex 3.141592653-2.6-1.40.29 (TeX Live 2026),
//! `\documentclass{beamer}` with no packages. The corpus deck is
//! `fixtures/real-world/beamer-overlays/main.tex` (its pinned
//! `reference.pdf`: 14 pages = 3 + 3 + 2 + 2 + 3 + 1 slides); the reflow /
//! `[<+->]` deck below was measured in the same session as the fix
//! (2026-09-18). Positions are `tools/visual-oracle/pdftext.py`'s (bp from
//! the paper's top-left corner, baseline of the word's first glyph);
//! *visible* words are the ones whose origin lies inside the MediaBox --
//! beamer paints covered text 2000 bp off the page
//! (`\pgfsys@begininvisible`), which is how the reference says "covered".
//! Every assertion here failed on the T0/T1 branch (6 pages, every
//! `\alert` red, `\only` alternatives both set).
//!
//! Needs the bundled Latin Modern faces and their metrics
//! (`FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` at TeX Live's
//! `lm`/`ec`/`amsfonts/symbols` TFMs), like every oracle test in this crate.

mod common;

use common::{lm_available, render_one, words_of, Word};
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::Rendered;

fn corpus() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-overlays/main.tex")).expect("corpus deck")
}

fn deck(body: &str) -> String {
    format!("\\documentclass{{beamer}}\n\\setbeamertemplate{{navigation symbols}}{{}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn reflow_deck() -> String {
    deck(concat!(
        "\\begin{frame}\n\\frametitle{Reflow}\n",
        "\\only<1>{Short.}\\only<2>{A much longer alternative that is typeset on the second slide and runs on to a second line of the frame.}\n\n",
        "Closing paragraph.\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Plus}\n\\begin{itemize}[<+->]\n\\item First step.\n\\item Second step.\n\\item Third step.\n\\end{itemize}\n",
        "\\alert{Always red} and \\alert<3>{red on three}.\n\\end{frame}\n",
    ))
}

fn word_n<'a>(words: &'a [Word], page: u32, text: &str, nth: usize) -> Option<&'a Word> {
    words.iter().filter(|w| w.page == page && w.text == text).nth(nth)
}

/// `(x, baseline)` of the `nth` run reading `text` on `page`, within `tol`
/// bp of the oracle.
fn at_n(words: &[Word], page: u32, text: &str, nth: usize, x: f64, baseline: f64, tol: f64) {
    let w = word_n(words, page, text, nth)
        .unwrap_or_else(|| panic!("no word {text:?} (#{nth}) on page {page}: {:?}", words.iter().filter(|w| w.page == page).map(|w| &w.text).collect::<Vec<_>>()));
    assert!(
        (w.x - x).abs() <= tol && (w.baseline - baseline).abs() <= tol,
        "page {page} {text:?}: ours ({:.3}, {:.3}), pdflatex ({x:.3}, {baseline:.3}), tolerance {tol}",
        w.x,
        w.baseline
    );
}

fn at(words: &[Word], page: u32, text: &str, x: f64, baseline: f64, tol: f64) {
    at_n(words, page, text, 0, x, baseline, tol)
}

/// The run reading `text` is absent from `page` (covered or omitted).
fn absent(words: &[Word], page: u32, text: &str) {
    assert!(word_n(words, page, text, 0).is_none(), "page {page} shows {text:?}, pdflatex covers it");
}

fn page_texts(words: &[Word], page: u32) -> Vec<String> {
    words.iter().filter(|w| w.page == page).map(|w| w.text.clone()).collect()
}

fn run_paint(r: &Rendered, page: u32, text: &str) -> (f64, f64, f64) {
    for p in &r.v2.pages {
        if p.number != page {
            continue;
        }
        for it in p.resident_items() {
            if let Item::GlyphRun(run) = it {
                if run.text == text {
                    return (run.paint.r, run.paint.g, run.paint.b);
                }
            }
        }
    }
    panic!("no run {text:?} on page {page}");
}

/// 3 + 3 + 2 + 2 + 3 + 1 pages, each slide carrying its frame's title.
#[test]
fn one_page_per_slide_with_the_frametitle_repeated() {
    if !lm_available() {
        return;
    }
    let r = render_one(&corpus());
    assert_eq!(r.v2.pages.len(), 14, "pdflatex ships 14 pages");
    let w = words_of(&r);
    let titles: Vec<&str> = (1..=14).map(|p| word_n(&w, p, "Pause", 0).map(|_| "Pause").or_else(|| word_n(&w, p, "Item", 0).map(|_| "Item")).or_else(|| word_n(&w, p, "Only", 0).map(|_| "Only")).or_else(|| word_n(&w, p, "Uncover", 0).map(|_| "Uncover")).or_else(|| word_n(&w, p, "Onslide", 0).map(|_| "Onslide")).or_else(|| word_n(&w, p, "No", 0).map(|_| "No")).unwrap_or("?")).collect();
    assert_eq!(titles, ["Pause", "Pause", "Pause", "Item", "Item", "Item", "Only", "Only", "Uncover", "Uncover", "Onslide", "Onslide", "Onslide", "No"]);
    for page in 1..=14 {
        at(&w, page, titles[page as usize - 1], 8.504, 21.057, 0.05);
    }
}

/// `\pause`: the material after the k-th pause is covered on slides <= k
/// and keeps its space -- the paragraph above and the first item never
/// move.
#[test]
fn pause_covers_and_keeps_the_space() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&corpus()));
    for page in 1..=3 {
        at(&w, page, "Three", 28.346, 107.169, 0.05);
        at(&w, page, "Parse", 50.165, 123.707, 0.05);
    }
    absent(&w, 1, "Expand");
    absent(&w, 1, "Break");
    at(&w, 2, "Expand", 50.165, 140.245, 0.05);
    absent(&w, 2, "Break");
    at(&w, 3, "Expand", 50.165, 140.245, 0.05);
    at(&w, 3, "Break", 50.165, 156.783, 0.05);
    // The visible word set grows slide by slide.
    assert_eq!(page_texts(&w, 1).len(), 14);
    assert_eq!(page_texts(&w, 2).len(), 20);
    assert_eq!(page_texts(&w, 3).len(), 25);
}

/// `\item<2->`, `<3->`, `<2->`: each item (label included) is covered on
/// the slides before its own; item 4 sits at its final place on slide 2
/// while item 3 is still covered.
#[test]
fn item_overlays_cover_label_and_text() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&corpus()));
    at(&w, 4, "Always", 50.165, 109.811, 0.05);
    assert_eq!(page_texts(&w, 4).len(), 9, "{:?}", page_texts(&w, 4));
    absent(&w, 4, "Appears");
    absent(&w, 4, "Also");
    at(&w, 5, "Appears", 50.165, 126.349, 0.05);
    assert!(word_n(&w, 5, "Appears", 1).is_none(), "item 3 is covered on slide 2");
    at(&w, 5, "Also", 50.165, 159.425, 0.05);
    at_n(&w, 6, "Appears", 1, 50.165, 142.887, 0.05);
    at(&w, 6, "third", 123.136, 142.887, 0.05);
    assert_eq!(page_texts(&w, 6).len(), 30);
    // Labels: one triangle per visible item.
    let labels = |page: u32| w.iter().filter(|x| x.page == page && x.text == "\u{25B6}").count();
    assert_eq!((labels(4), labels(5), labels(6)), (1, 3, 4));
}

/// `\only`: the other alternative takes no space, so the page reflows
/// (corpus: both alternatives happen to fill two lines; the reflow deck
/// below has a one-line and a two-line alternative).
#[test]
fn only_omits_and_reflows() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&corpus()));
    at(&w, 7, "absent", 28.346, 130.920, 0.05);
    absent(&w, 7, "longer");
    at(&w, 8, "longer", 28.346, 130.920, 0.05);
    at(&w, 8, "did.", 295.978, 130.920, 0.05);
    absent(&w, 8, "absent");
    at(&w, 7, "closing", 51.828, 144.469, 0.05);
    at(&w, 8, "closing", 51.828, 144.469, 0.05);

    let r = render_one(&reflow_deck());
    assert_eq!(r.v2.pages.len(), 5);
    let w = words_of(&r);
    at(&w, 1, "Short.", 28.346, 122.790, 0.05);
    at(&w, 1, "Closing", 28.346, 136.340, 0.05);
    absent(&w, 1, "alternative");
    at(&w, 2, "alternative", 99.044, 117.371, 0.05);
    at(&w, 2, "frame.", 171.866, 130.920, 0.05);
    at(&w, 2, "Closing", 28.346, 144.469, 0.05);
    absent(&w, 2, "Short.");
}

/// `\uncover<2->`: covered on slide 1 with its space reserved (the line
/// after it does not move); `\alert<2>`: red on slide 2 only.
#[test]
fn uncover_keeps_space_and_alert_colours_its_slide() {
    if !lm_available() {
        return;
    }
    let r = render_one(&corpus());
    let w = words_of(&r);
    at(&w, 9, "incremental", 92.982, 101.112, 0.05);
    absent(&w, 9, "invisible");
    at_n(&w, 9, "The", 1, 28.346, 168.858, 0.05);
    at(&w, 10, "invisible", 111.135, 128.210, 0.05);
    at(&w, 10, "slides.", 28.346, 155.309, 0.05);
    at_n(&w, 10, "The", 1, 28.346, 168.858, 0.05);
    assert_eq!(run_paint(&r, 9, "incremental"), (0.0, 0.0, 0.0));
    assert_eq!(run_paint(&r, 10, "incremental"), (1.0, 0.0, 0.0));
    // The covered paragraph is 24 words (`incremental` and its colon are
    // two runs on both slides, so run counts are compared, not sizes).
    assert_eq!(page_texts(&w, 10).len() - page_texts(&w, 9).len(), 24);
}

/// `\onslide<2->` mid-paragraph covers the rest of the line without
/// breaking it; `\onslide<3->` after the list covers the closing text.
#[test]
fn onslide_runs_to_the_next_onslide() {
    if !lm_available() {
        return;
    }
    let w = words_of(&render_one(&corpus()));
    for page in 11..=13 {
        at(&w, page, "First", 28.346, 114.980, 0.05);
    }
    assert_eq!(page_texts(&w, 11).len(), 6, "{:?}", page_texts(&w, 11));
    at(&w, 12, "Second", 147.165, 114.980, 0.05);
    at(&w, 12, "bullet", 61.071, 131.518, 0.05);
    absent(&w, 12, "Third");
    at(&w, 13, "Third", 28.346, 148.056, 0.05);
    assert_eq!(page_texts(&w, 13).len(), 27);
    at(&w, 14, "specifications", 142.292, 128.210, 0.05);
}

/// `\begin{itemize}[<+->]`: one slide per item; `\alert{}` without a
/// specification is red on every slide, `\alert<3>` on the third only.
#[test]
fn itemize_plus_default_and_alert_forms() {
    if !lm_available() {
        return;
    }
    let r = render_one(&reflow_deck());
    let w = words_of(&r);
    at(&w, 3, "First", 50.165, 110.158, 0.05);
    absent(&w, 3, "Second");
    at(&w, 4, "Second", 50.165, 126.696, 0.05);
    absent(&w, 4, "Third");
    at(&w, 5, "Third", 50.165, 143.234, 0.05);
    for page in 3..=5 {
        at(&w, page, "Always", 28.346, 159.772, 0.05);
        at(&w, page, "three", 133.741, 159.772, 0.05);
        assert_eq!(run_paint(&r, page, "Always"), (1.0, 0.0, 0.0));
    }
    assert_eq!(run_paint(&r, 3, "on"), (0.0, 0.0, 0.0));
    assert_eq!(run_paint(&r, 4, "on"), (0.0, 0.0, 0.0));
    assert_eq!(run_paint(&r, 5, "on"), (1.0, 0.0, 0.0));
}
