//! Regressions found by the pipeline fuzzer (`examples/fuzz_render.rs`,
//! `tests/fuzz_support`). Each input is the minimised case; each test runs
//! the route the finding came from (render, then the exact PDF export that
//! `flashtex build` writes) and must not panic, overflow or hang.

mod common;
mod fuzz_support;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::{pdf, render, FontSet, RenderOptions, Rendered};

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

/// Renders `body` in a figure against a project holding the fuzzer's images
/// and returns the exact PDF route's result.
fn export_figure(tag: &str, graphic: &str) -> Result<usize, String> {
    let root = std::env::temp_dir().join(format!("flashtex-fuzz-regression-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    fuzz_support::prepare_project(&root);
    let root = std::fs::canonicalize(&root).unwrap();
    let text = format!(
        "\\documentclass{{article}}\\usepackage{{graphicx}}\\begin{{document}}\nx\\begin{{figure}}[h]\\includegraphics{graphic}\\end{{figure}}\n\\end{{document}}\n"
    );
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(root.clone()), ..RenderOptions::default() };
    let rendered = render(&[SourceDocument { path: "main.tex", text: &text }], "main.tex", 1, "fuzz", &fonts, &options);
    let out = pdf::write_pdf_exact(&rendered.v2, fonts.dirs(), Some(&root)).map(|o| o.bytes.len());
    let _ = std::fs::remove_dir_all(&root);
    out
}

#[test]
fn an_image_too_large_to_embed_does_not_fail_the_export() {
    // Before: `flashtex build` wrote no PDF at all:
    // `transform[0]: 2147483647 is out of range` (2^31-pixel PNG header),
    // `transform[3]: -1000000000 is out of range` (1e9 pixels tall),
    // `width: 9223372036854776000 is not an integer tick value` (a PDF with
    // a 1e308 MediaBox), `PNG: size 2147483647x2147483647 is outside
    // 1..=65536` (the same PNG scaled down to 16383pt).
    for (tag, graphic) in [
        ("huge", "{huge.png}"),
        ("wide", "{wide.png}"),
        ("tall", "{tall.png}"),
        ("badpdf", "{bad.pdf}"),
        ("hugejpg", "{huge.jpg}"),
        ("scaled-huge", "[width=16383pt]{huge.png}"),
    ] {
        if let Err(e) = export_figure(tag, graphic) {
            panic!("{graphic}: {e}");
        }
    }
}

#[test]
fn an_image_scaled_to_nothing_or_past_the_range_does_not_fail_the_export() {
    // Before: `image box 0x0 ticks is not positive` ([scale=0],
    // [width=0pt], [angle=45,width=0pt]); `width: 150994944000000000 is not
    // an integer tick value` ([scale=1e9]); `transform[0]: 14400000 is out
    // of range` ([scale=100000]).
    for (tag, graphic) in [
        ("scale0", "[scale=0]{images/red-72.png}"),
        ("width0", "[width=0pt]{images/red-72.png}"),
        ("angle-width0", "[angle=45,width=0pt]{images/red-72.png}"),
        ("scale1e9", "[scale=1e9]{images/red-72.png}"),
        ("scale1e5", "[scale=100000]{images/red-72.png}"),
    ] {
        if let Err(e) = export_figure(tag, graphic) {
            panic!("{graphic}: {e}");
        }
    }
}

#[test]
fn an_ordinary_image_is_still_embedded() {
    let with = export_figure("red", "{images/red-72.png}").expect("export");
    let without = export_figure("zero", "[scale=0]{images/red-72.png}").expect("export");
    assert!(with > without, "the painted image adds bytes: {with} vs {without}");
}

/// Wall time of `adapter::adapt` alone for `text`.
fn adapt_time(text: &str) -> std::time::Duration {
    let docs = [SourceDocument { path: "main.tex", text }];
    let parsed = flashtex_compiler::parser::parse_project(&docs, "main.tex");
    let labels = flashtex_render_pipeline::adapter::Labels::from_parsed(&parsed);
    let started = std::time::Instant::now();
    std::hint::black_box(flashtex_render_pipeline::adapter::adapt(&[text], 0, &parsed, &RenderOptions::default(), &labels));
    started.elapsed()
}

#[test]
fn a_long_list_is_adapted_without_rescanning_the_source_per_item() {
    // Before: every `\item` re-read the source up to itself for the list
    // stack (rescanning to the item after each `\begin`), the `\setlist`
    // calls and the theorem environments, so 10 000 items took 3.3 s in
    // `split_at_page_breaks` (release) and a 5 000-paragraph article with a
    // `center` per paragraph 8 s. After: 14 ms and 1.1 s.
    let items = "\\item x\n".repeat(10_000);
    let text = format!("\\documentclass{{article}}\\begin{{document}}\n\\begin{{itemize}}{items}\\end{{itemize}}\n\\end{{document}}\n");
    let took = adapt_time(&text);
    assert!(took < std::time::Duration::from_secs(10), "10 000 list items took {took:?} to adapt");
}

#[test]
fn list_items_inside_many_open_environments_are_adapted_in_linear_time() {
    // Before: for each `\item`, every `\begin` before it restarted a search
    // for the next `\end` that ran to the item, so 300 items inside 1 000
    // open `center`s rendered in 7.4 s (release; 400 nested `itemize` took
    // 1.1 s). After: 25 ms, byte-identical PDF.
    let text = format!(
        "\\documentclass{{article}}\\begin{{document}}\n{}\\begin{{itemize}}{}\\end{{itemize}}{}\\end{{document}}\n",
        "\\begin{center}\n".repeat(1000),
        "\\item x\n".repeat(300),
        "\\end{center}\n".repeat(1000)
    );
    let took = adapt_time(&text);
    assert!(took < std::time::Duration::from_secs(10), "took {took:?} to adapt");
}
