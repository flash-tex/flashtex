//! `adapter::adapt` stays linear in the number of list items and open
//! environments.
//!
//! These are complexity guards, not benchmarks: the thresholds are orders of
//! magnitude above the current cost, so they stay quiet on a loaded machine
//! and only fire if a per-item source rescan comes back.
//!
//! History. `split_at_page_breaks` asked `list_stack_at`,
//! `in_theorem_environment` and `setlist_calls` about every block, and each of
//! those scanned the source from byte 0, so `adapt` was O(begins x bytes).
//! #623 made them linear with a per-document `SourceIndex` (checked against
//! the old prefix scans by `source_index_matches_prefix_scans`), which is on
//! main — but that is an *identity* test, so nothing on main notices if the
//! cost goes quadratic again while the output stays correct. These two tests
//! close that gap.
//!
//! The inputs come from the render-pipeline mutation fuzzer (PR #627), which
//! found the blowup as "hang in render". Release timings through
//! `fuzz_render --diagnostics`, before #623 -> after #623:
//!
//! | case                                        | before  | after   |
//! |---------------------------------------------|---------|---------|
//! | mutated enumitem worksheet                  | 37.5 s  | 0.29 s  |
//! | page-builder enlarge-00                     | 21.5 s  | 0.19 s  |
//! | 715-page list of 20 000 items               | 23.3 s  | 0.13 s  |
//! | 1 000 nested `itemize`                      | 23.8 s  | 0.15 s  |
//! | 5 000-paragraph article, a `center` each    |  8.0 s  | 1.1 s   |
//! | 300 items inside 1 000 open `center`s       |  7.4 s  | 25 ms   |
//!
//! The last one was checked to produce identical PDF bytes before and after.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::RenderOptions;

/// Wall time of `adapter::adapt` alone for `text`.
fn adapt_time(text: &str) -> std::time::Duration {
    let docs = [SourceDocument { path: "main.tex", text }];
    let parsed = flashtex_compiler::parser::parse_project(&docs, "main.tex");
    let labels = flashtex_render_pipeline::adapter::Labels::from_parsed(&parsed);
    let started = std::time::Instant::now();
    std::hint::black_box(flashtex_render_pipeline::adapter::adapt(
        &[text],
        0,
        &parsed,
        &RenderOptions::default(),
        &labels,
    ));
    started.elapsed()
}

#[test]
fn a_long_list_is_adapted_without_rescanning_the_source_per_item() {
    // Before #623 every `\item` re-read the source up to itself for the list
    // stack, the `\setlist` calls and the theorem environments, so 10 000
    // items took 17.3 s to adapt in this test (3.3 s in `split_at_page_breaks`
    // release).
    let items = "\\item x\n".repeat(10_000);
    let text = format!(
        "\\documentclass{{article}}\\begin{{document}}\n\\begin{{itemize}}{items}\\end{{itemize}}\n\\end{{document}}\n"
    );
    let took = adapt_time(&text);
    assert!(took < std::time::Duration::from_secs(10), "10 000 list items took {took:?} to adapt");
}

#[test]
fn list_items_inside_many_open_environments_are_adapted_in_linear_time() {
    // Before #623, for each `\item` every `\begin` before it restarted a
    // search for the next `\end` that ran to the item, so 300 items inside
    // 1 000 open `center`s took 18.7 s to adapt here (7.4 s to render in
    // release, for a byte-identical PDF).
    let text = format!(
        "\\documentclass{{article}}\\begin{{document}}\n{}\\begin{{itemize}}{}\\end{{itemize}}{}\\end{{document}}\n",
        "\\begin{center}\n".repeat(1000),
        "\\item x\n".repeat(300),
        "\\end{center}\n".repeat(1000)
    );
    let took = adapt_time(&text);
    assert!(took < std::time::Duration::from_secs(10), "took {took:?} to adapt");
}
