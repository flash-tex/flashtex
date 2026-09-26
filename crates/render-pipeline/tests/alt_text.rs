//! `\includegraphics[alt=...]` reaches the display-list-v2 JSON wire
//! format: end-to-end from a real parsed document through the adapter,
//! typeset and display stages, not just the `GKey::Alt` parser primitive
//! (`crates/render-pipeline/src/graphics.rs`'s own unit test) in isolation.
//!
//! This stops at the JSON boundary, not the exported PDF bytes:
//! `flashtex-render-pipeline` builds against a pinned `vendor/pdf` snapshot
//! (see `vendor/VENDORING.md`), so it cannot take the canonical
//! `crates/pdf` as a test dependency without a re-pin -- the same
//! constraint `crates/render-pipeline/tests/tikzcd_pipeline.rs` and
//! `axis_plot.rs` already document for their own e2e gaps. The other half
//! of the chain (this exact JSON shape -> PDF `/Alt` bytes) is covered by
//! `crates/pdf/tests/alt_text.rs`.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[test]
fn includegraphics_alt_reaches_the_image_item_in_real_display_list_json() {
    if !common::lm_available() {
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/floats");
    let tex = "\\documentclass{article}\n\\usepackage{graphicx}\n\\begin{document}\n\
        \\includegraphics[alt={A red square}]{images/red-72.png}\n\
        \\end{document}\n";
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    let r = render(&[SourceDocument { path: "main.tex", text: tex }], "main.tex", 1, "floats", &fonts, &options);
    assert!(r.v2.diagnostics.is_empty(), "{:?}", r.v2.diagnostics);
    assert!(r.v2.has_images());
    let json = flashtex_compiler::json::write(&r.v2.to_json_with("x", true));
    assert!(
        json.contains("\"alt\":\"A red square\""),
        "alt text missing from the image item's JSON: {}",
        &json[..600.min(json.len())]
    );
}

#[test]
fn includegraphics_without_alt_never_emits_the_key() {
    if !common::lm_available() {
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/floats");
    let tex = "\\documentclass{article}\n\\usepackage{graphicx}\n\\begin{document}\n\
        \\includegraphics{images/red-72.png}\n\
        \\end{document}\n";
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(dir.into()), ..RenderOptions::default() };
    let r = render(&[SourceDocument { path: "main.tex", text: tex }], "main.tex", 1, "floats", &fonts, &options);
    assert!(r.v2.diagnostics.is_empty(), "{:?}", r.v2.diagnostics);
    let json = flashtex_compiler::json::write(&r.v2.to_json_with("x", true));
    assert!(!json.contains("\"alt\""), "{}", &json[..600.min(json.len())]);
}
