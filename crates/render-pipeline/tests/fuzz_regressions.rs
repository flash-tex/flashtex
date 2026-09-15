//! Regressions found by the pipeline fuzzer (`examples/fuzz_render.rs`,
//! `tests/fuzz_support`). Each input is the minimised case; each test runs
//! the route the finding came from (render, then the exact PDF export that
//! `flashtex build` writes) and must not panic, overflow or hang.

mod common;

use flashtex_render_pipeline::{pdf, FontSet, Rendered};

/// Renders `text` as `main.tex` and exports it through the exact PDF route.
fn render_and_export(text: &str) -> Rendered {
    let rendered = common::render_one(text);
    let fonts = FontSet::with_default_dirs(&[]);
    let _ = pdf::write_pdf_exact(&rendered.v2, fonts.dirs(), None);
    rendered
}

#[test]
fn multicol_scan_after_a_trailing_backslash() {
    // Before: panic `multicol.rs:261 start byte index 38 is out of bounds
    // for string of length 37` (the scan stepped two bytes past a `\`).
    render_and_export("\\begin{multicols}{2}x\\end{multicols}\\");
}

#[test]
fn multicol_scan_after_a_backslash_before_a_multibyte_character() {
    // Before: panic `multicol.rs:261 start byte index 12 is not a char
    // boundary; it is inside 'é'`.
    render_and_export("multicols \\é");
    render_and_export("\\begin{multicols}{2}\\é\\end{multicols}");
}
