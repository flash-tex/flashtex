//! GH-323, PDF half: an exported PDF carries a real `/Link` annotation with a
//! `/URI` action for each `\url`/`\href`, at the rectangle pdflatex puts it.
//!
//! The rectangles themselves are checked against pdflatex in
//! `links_navigation.rs`; this file checks they survive the y flip into PDF
//! user space and reach `/Annots`.

mod common;

use common::{lm_available, render_one};
use flashtex_render_pipeline::pdf::write_pdf_exact;

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{hyperref}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn pdf_of(body: &str) -> Vec<u8> {
    let r = render_one(&doc(body));
    write_pdf_exact(&r.v2, &[], None).expect("exact PDF").bytes
}

/// The `/Link` annotation dictionaries, as raw PDF source. The writer emits
/// one object per annotation, so a substring scan is enough and this test
/// needs no PDF parser.
fn link_objects(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    text.split("endobj")
        .filter(|o| o.contains("/Subtype /Link") || o.contains("/Subtype/Link"))
        .map(|o| o.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

#[test]
fn an_exported_pdf_carries_a_uri_link_annotation() {
    if !lm_available() {
        return;
    }
    let bytes = pdf_of(r"See \url{https://example.com} and \href{https://ex.org/a}{click here} end.");
    let links = link_objects(&bytes);
    assert_eq!(links.len(), 2, "{links:#?}");
    assert!(links[0].contains("/S/URI"), "{}", links[0]);
    assert!(links[0].contains("(https://example.com)"), "{}", links[0]);
    assert!(links[1].contains("(https://ex.org/a)"), "{}", links[1]);
    // A link must add no ink: no border, no colour.
    assert!(links[0].contains("/Border[0 0 0]"), "{}", links[0]);
    assert!(!links[0].contains("/C["), "{}", links[0]);
}

/// pdflatex on the same source, read back with PyMuPDF:
///
/// ```text
/// https://example.com Rect(165.427001953125, 126.8499755859375, 266.7959899902344, 137.9749755859375)
/// ```
///
/// The page is 792 bp tall, so `/Rect` is `[165.427 654.025 266.796 665.150]`
/// y-up -- which is what the oracle PDF's own annotation dictionary says.
#[test]
fn the_rectangle_is_flipped_into_pdf_user_space() {
    if !lm_available() {
        return;
    }
    let bytes = pdf_of(r"See \url{https://example.com} and \href{https://ex.org/a}{click here} end.");
    let links = link_objects(&bytes);
    let rect = links[0]
        .split("/Rect [")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .expect("/Rect")
        .split_whitespace()
        .map(|t| t.parse::<f64>().expect("decimal"))
        .collect::<Vec<_>>();
    assert_eq!(rect.len(), 4, "{rect:?}");
    for (got, want) in rect.iter().zip([165.427, 654.025, 266.796, 665.150]) {
        assert!((got - want).abs() <= 0.5, "{rect:?} against pdflatex's [165.427 654.025 266.796 665.150]");
    }
    // y-up: the lower-left y is below the upper-right y.
    assert!(rect[1] < rect[3], "{rect:?}");
}

#[test]
fn a_document_with_no_links_exports_the_bytes_it_always_did() {
    if !lm_available() {
        return;
    }
    let bytes = pdf_of("Ordinary text with no link at all.");
    assert!(link_objects(&bytes).is_empty());
    assert!(!String::from_utf8_lossy(&bytes).contains("/Annots"));
}
