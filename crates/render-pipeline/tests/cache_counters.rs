//! `RenderCache::counters` (measurement only, design #575 step 0): the
//! `adapted`/`blocks`/`assembled` hit and miss counts and the layout passes
//! `render_cached` runs.

mod common;

use common::lm_available;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::{render_cached, FontSet, RenderCache, RenderOptions};

fn render(text: &str, fonts: &FontSet, cache: &RenderCache) -> u32 {
    let docs = [SourceDocument { path: "main.tex", text }];
    render_cached(&docs, "main.tex", 1, "counters", fonts, &RenderOptions::default(), Some(cache)).passes
}

#[test]
fn counters_track_lookups_and_label_passes() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let cache = RenderCache::new();
    let text = "\\begin{document}\n\\section{One}\\label{a}\nSee Section~\\ref{a} on page~\\pageref{a}.\n\nAnother paragraph.\n\\end{document}\n";

    let passes = render(text, &fonts, &cache);
    let cold = cache.counters();
    assert!(passes >= 2, "\\pageref needs a page-number pass: {passes}");
    assert_eq!(cold.label_passes, u64::from(passes));
    assert!(cold.adapted_misses > 0 && cold.block_misses > 0, "{cold:?}");
    assert_eq!((cold.block_hits, cold.block_misses), cache.stats());

    // The same text again: every lookup hits, and the pass count is added.
    let passes = render(text, &fonts, &cache);
    let warm = cache.counters().since(cold);
    assert_eq!(warm.label_passes, u64::from(passes));
    assert_eq!((warm.adapted_misses, warm.block_misses, warm.assembled_misses), (0, 0, 0), "{warm:?}");
    assert!(warm.adapted_hits > 0 && warm.block_hits > 0 && warm.assembled_hits > 0, "{warm:?}");
}
