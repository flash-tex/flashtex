//! Re-measurable large-document timing (issue #65). Ignored by default because
//! it times a ~300-page compile; run it with
//! `cargo test --release --test large_document_timing -- --ignored --nocapture`.
//! `src/bin/large_doc_bench.rs` is the fuller benchmark (median of N, protocol
//! path, reply digest); this test shares its document generator.

#[allow(dead_code)]
#[path = "../src/bin/large_doc_bench.rs"]
mod bench;

use flashtex_compiler::incremental::{compile_full_project, Session};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::SourceDocument;
use std::time::Instant;

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn large_document_cold_and_one_character_edit() {
    let constraints = LayoutConstraints::default();
    let base = bench::large_document(450);
    let mid = base.len() / 2;
    let offset = mid + base[mid..].find("ordinary words").expect("anchor") + "ordinary".len();
    let mut edited = base.clone();
    edited.insert(offset, 'x');
    let docs = |text| {
        [SourceDocument {
            path: "main.tex",
            text,
        }]
    };

    let started = Instant::now();
    let cold = compile_full_project(&docs(&base), "main.tex", constraints);
    let cold_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert!(cold.pages.len() >= 300, "{} pages", cold.pages.len());
    assert!(cold.diagnostics.is_empty(), "{:?}", cold.diagnostics);

    let mut session = Session::new();
    session.compile_project(&docs(&base), "main.tex", constraints);
    let started = Instant::now();
    let warm = session.compile_project(&docs(&edited), "main.tex", constraints);
    let edit_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(
        warm.output,
        compile_full_project(&docs(&edited), "main.tex", constraints)
    );
    println!(
        "large document: {} pages; cold {cold_ms:.1} ms; one-character edit {edit_ms:.1} ms",
        cold.pages.len()
    );
}
