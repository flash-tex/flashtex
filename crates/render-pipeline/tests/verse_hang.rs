//! `verse` continuation lines hang 1.5em past the verse's left margin.
//!
//! article.cls builds `verse` as a `\list` with `\itemindent -1.5em` and
//! `\leftmargin +1.5em`: the first line starts at the list's left margin
//! (the same x as a `quote` line) while a wrapped continuation starts 1.5em
//! further right. Oracle values are pdflatex TL2026, article 10pt, x from
//! the page's left edge.

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

#[test]
fn verse_wrapped_continuation_hangs_one_and_half_em() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass{article}\\begin{document}\n\\begin{verse}\nLine one of verse\\\\\nLine two that is quite long and should wrap around onto another line in the verse env\\\\\nLine three.\n\\end{verse}\n\\end{document}";
    let (_v1, words) = layout(src);
    // First-line starts of the verse's own lines (`Line one`, `Line two`,
    // `Line three`).
    let line_starts: Vec<f64> = words
        .iter()
        .filter(|w| w.text == "Line")
        .map(|w| w.x)
        .collect();
    for x in &line_starts {
        // pdflatex: every verse first line starts at x 158.675.
        assert!((x - 158.675).abs() < 0.05, "verse line start: {x} ({words:?})");
    }
    assert!(line_starts.len() >= 2, "expected verse line starts ({words:?})");
    // The wrapped continuation word: pdflatex puts it at x 173.619, a hang
    // of 14.944bp = 1.5em at 10pt past the line starts.
    let cont = words.iter().find(|w| w.text == "line").unwrap_or_else(|| panic!("no continuation `line` in {words:?}"));
    assert!((cont.x - 173.619).abs() < 0.05, "continuation x: {} ({words:?})", cont.x);
}
