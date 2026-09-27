//! A `quote` nested inside another `quote` uses the level-2 list margins.
//!
//! The inner quote's `\list` runs `\@listii`, so its indent is
//! `\leftmarginii` (22pt at 10pt) past the outer quote's margin, and the
//! nested list's entering/leaving adds its `topsep`/`parsep` spacing around
//! it. Oracle values are pdflatex TL2026, article 10pt, x from the page's
//! left edge and baselines from the top.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::{Capabilities, V1Payload};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[derive(Debug, Clone)]
struct Word {
    text: String,
    x: f64,
    baseline: f64,
}

fn layout(text: &str) -> (V1Payload, Vec<Word>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    let mut words = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                words.push(Word {
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                });
            }
        }
    }
    (v1, words)
}

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

#[test]
fn nested_quote_takes_leftmarginii_and_nested_spacing() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass{article}\\begin{document}\n\\begin{quote}\nOuter quote text.\n\\begin{quote}\nInner quote text.\n\\end{quote}\nOuter again.\n\\end{quote}\n\\end{document}";
    let (_v1, words) = layout(src);
    // pdflatex: `Inner` at x 180.593 (outer margin 158.675 + 21.918).
    let inner = word(&words, "Inner");
    assert!((inner.x - 180.593).abs() < 0.05, "`Inner` x: {} ({words:?})", inner.x);
    // pdflatex: `Outer again` baseline at 174.615.
    let again = word(&words, "again.");
    assert!((again.baseline - 174.615).abs() < 0.05, "`Outer again` baseline: {} ({words:?})", again.baseline);
}
