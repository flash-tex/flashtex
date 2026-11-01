//! Run-in headings: `\paragraph` and `\subparagraph` set their label on the
//! same line as the following body text (article.cls `\@startsection` with a
//! negative after-skip), they do not start a fresh line of their own.
//!
//! The expected `(x, baseline)` numbers below are glyph origins measured by
//! the supervisor by actually running `/Library/TeX/texbin/pdflatex` on the
//! fixture `fixtures/headings-run-in/article-10.tex` and reading the origins
//! with PyMuPDF's `get_texttrace()` — PDF native coordinates, i.e. x from
//! the left and y from the bottom, in bp. The pipeline's display list is
//! top-down (y from the page's top-left corner), so this test converts with
//! the rendered page's own height before comparing; the reference numbers
//! themselves are used verbatim. Tolerance is 0.1 bp, the usual gate.
//!
//! Tighter reference to #768's original 38-case ask: this is a first,
//! minimal slice covering only `\paragraph`/`\subparagraph`, not the full
//! set of sectioning commands.

mod common;

use common::{lm_available, render_one, words_of, Word};

const TOL: f64 = 0.1;

const SOURCE: &str = include_str!("fixtures/headings-run-in/article-10.tex");

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.text == text)
        .unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

/// `\paragraph` and `\subparagraph` run into their body line: the label and
/// the first body word share one baseline, pinned to pdflatex's origins.
#[test]
fn paragraph_and_subparagraph_run_into_their_body_lines() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let rendered = render_one(SOURCE);
    assert!(!rendered.v2.pages.is_empty(), "{:?}", rendered.v2.diagnostics);
    // The reference baselines are bottom-up (PDF native); the display list
    // is top-down, so flip with the page's own height.
    let page_h = rendered.v2.pages[0].height.to_bp();
    let words = words_of(&rendered);
    let bottom_up = |w: &Word| page_h - w.baseline;

    let head = word(&words, "ParaHead");
    assert_eq!(head.page, 1, "the \\paragraph stays on page 1: {words:?}");
    close(head.x, 133.768, "ParaHead x");
    close(bottom_up(head), 657.235, "ParaHead baseline");

    let body = word(&words, "ParaBody");
    assert_eq!(body.page, 1, "the \\paragraph body stays on page 1: {words:?}");
    close(body.x, 193.255, "ParaBody x");
    close(bottom_up(body), 657.235, "ParaBody baseline");

    let sub_head = word(&words, "SubparaHead");
    assert_eq!(sub_head.page, 1, "the \\subparagraph stays on page 1: {words:?}");
    close(sub_head.x, 133.768, "SubparaHead x");
    close(bottom_up(sub_head), 631.340, "SubparaHead baseline");

    let sub_body = word(&words, "SubparaBody");
    assert_eq!(sub_body.page, 1, "the \\subparagraph body stays on page 1: {words:?}");
    close(sub_body.x, 211.207, "SubparaBody x");
    close(bottom_up(sub_body), 631.340, "SubparaBody baseline");

    // That is what "run-in" means: the label does not get its own line, it
    // shares the body text's baseline.
    assert!(
        (bottom_up(head) - bottom_up(body)).abs() < TOL,
        "\\paragraph label and body share a baseline: {} vs {}",
        bottom_up(head),
        bottom_up(body)
    );
    assert!(
        (bottom_up(sub_head) - bottom_up(sub_body)).abs() < TOL,
        "\\subparagraph label and body share a baseline: {} vs {}",
        bottom_up(sub_head),
        bottom_up(sub_body)
    );
}
