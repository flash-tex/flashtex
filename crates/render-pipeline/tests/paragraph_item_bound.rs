//! GH-843 (part 2): a single paragraph past the item limit must fail fast.
//!
//! History. A one-line document of N repetitions of `word ` assembles ~2N
//! paragraph items, and the breaker rejects anything past `MAX_ITEMS`
//! (200 000). That rejection used to arrive only after the whole paragraph
//! had been assembled through a per-word source rescan, so the *failure*
//! path was superlinear: ~2 s at N=160 000, ~7 s at N=320 000, ~59 s at
//! N=900 000. Assembly now stops at the bound and reuses the breaker's own
//! rejection, so the same clean `paragraph_layout_error` arrives in
//! milliseconds.
//!
//! The timing assertions below are complexity guards, not benchmarks: the
//! 20 s cap is ~50x above the current cost and the ratio bound only fires
//! if per-word work goes superlinear again, so they stay quiet on a loaded
//! machine in either profile. The end-to-end test pins the outcome (same
//! error, no pages).

use std::time::{Duration, Instant};

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::adapter::{self, Block, Item, Labels, ParaPart};
use flashtex_render_pipeline::RenderOptions;

mod common;
use common::{lm_available, render_one};

/// Seconds of `adapt` for `text`, kept alive so nothing is optimised away.
fn adapt_time(text: &str) -> (Duration, adapter::Doc) {
    let docs = [SourceDocument { path: "main.tex", text }];
    let parsed = flashtex_compiler::parser::parse_project(&docs, "main.tex");
    let labels = Labels::from_parsed(&parsed);
    let started = Instant::now();
    let doc = std::hint::black_box(adapter::adapt(
        &[text],
        &["main.tex"],
        0,
        &parsed,
        &RenderOptions::default(),
        &labels,
    ));
    (started.elapsed(), doc)
}

/// The paragraph's item lists: `(overlong_markers, total_items)`.
fn paragraph_items(doc: &adapter::Doc) -> (usize, usize) {
    let mut markers = 0;
    let mut items = 0;
    for block in &doc.blocks {
        if let Block::Paragraph { parts, .. } = block {
            for part in parts {
                if let ParaPart::Lines(list) = part {
                    items += list.len();
                    markers += list.iter().filter(|i| matches!(i, Item::Overlong { .. })).count();
                }
            }
        }
    }
    (markers, items)
}

/// An over-limit paragraph assembles to a single marker quickly, and twice
/// the words take about twice the time (linear), not four times.
#[test]
fn overlong_paragraph_fails_during_assembly() {
    let small = "word ".repeat(120_000);
    let big = "word ".repeat(240_000);

    let (t_small, doc_small) = adapt_time(&small);
    let (markers_small, _) = paragraph_items(&doc_small);
    assert_eq!(markers_small, 1, "120k words (~240k items) must trip the bound");

    let (t_big, doc_big) = adapt_time(&big);
    let (markers_big, _) = paragraph_items(&doc_big);
    assert_eq!(markers_big, 1, "240k words (~480k items) must trip the bound");

    // Well under a second per the issue; the 20 s cap only fires if the
    // superlinear path comes back (it took ~60 s in debug before the fix).
    assert!(t_small < Duration::from_secs(20), "adapt(120k words) took {t_small:?}");
    assert!(t_big < Duration::from_secs(20), "adapt(240k words) took {t_big:?}");
    // Doubling the input must not quadruple the work.
    assert!(
        t_big.as_secs_f64() < 3.0 * t_small.as_secs_f64().max(0.001),
        "superlinear assembly: {t_small:?} -> {t_big:?}"
    );
}

/// The fast failure surfaces end to end as the same clean error with no
/// pages. Font-independent by construction: the marker trips in `adapt`,
/// before any shaping, so this holds with or without fonts.
#[test]
fn overlong_paragraph_reports_layout_error_without_pages() {
    let r = render_one(&"word ".repeat(120_000));
    assert!(r.v2.pages.is_empty(), "over-long paragraph must set no pages");
    let layout_errors: Vec<&str> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.code == "paragraph_layout_error")
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(layout_errors.len(), 1, "{:?}", r.v2.diagnostics);
    assert!(
        layout_errors[0].contains("exceeds the bounded limit of 200000"),
        "{}",
        layout_errors[0]
    );
}

/// A large paragraph under the limit assembles normally (no marker): the
/// bound only trips past it, never on a paragraph that can still be set.
#[test]
fn under_limit_paragraph_assembles_normally() {
    let text = "word ".repeat(80_000);
    let (_, doc) = adapt_time(&text);
    let (markers, items) = paragraph_items(&doc);
    assert_eq!(markers, 0, "80k words (~160k items) is under the limit");
    assert_eq!(items, 2 * 80_000 - 1, "one box and one glue per word");
}

/// An ordinary short paragraph is unaffected: one page, no diagnostics.
#[test]
fn short_paragraph_is_unaffected() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let r = render_one("Hello world. This is a short ordinary paragraph with a few sentences.\n");
    assert_eq!(r.v2.pages.len(), 1);
    assert!(r.v2.diagnostics.is_empty(), "{:?}", r.v2.diagnostics);
}
