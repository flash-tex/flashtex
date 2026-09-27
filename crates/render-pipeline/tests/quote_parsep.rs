//! `\parsep` between paragraphs inside `quote`/`quotation`/`verse`.
//!
//! A `\list` sets `\parskip` to `\parsep` (`latex.ltx`), so a blank line
//! inside `quote` adds `\parsep` (4pt plus 2pt minus 1pt at 10pt) on top of
//! the normal `\baselineskip` -- not just the baselineskip alone. The same
//! holds across a blank-line stanza break inside `verse`. Oracle values are
//! pdflatex TL2026, article 10pt, baselines from the page's top-left.

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
fn quote_blank_line_adds_parsep() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass{article}\\begin{document}\n\\begin{quote}\nQuote one.\n\nQuote two.\n\\end{quote}\n\\end{document}";
    let (_v1, words) = layout(src);
    let one = word(&words, "one.");
    let two = word(&words, "two.");
    // pdflatex (TL2026, PyMuPDF origins): `Quote two` baseline at 150.705,
    // 15.940bp after `Quote one` (baselineskip 11.955bp + parsep 3.985bp).
    assert!((two.baseline - 150.705).abs() < 0.05, "`Quote two` baseline: {} ({words:?})", two.baseline);
    assert!((two.baseline - one.baseline - 15.940).abs() < 0.05, "parsep gap: {} ({words:?})", two.baseline - one.baseline);
}

#[test]
fn verse_stanza_break_adds_parsep() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass{article}\\begin{document}\n\\begin{verse}\nFirst stanza line.\n\nSecond stanza line.\n\\end{verse}\n\\end{document}";
    let (_v1, words) = layout(src);
    let first = word(&words, "First");
    let second = word(&words, "Second");
    // pdflatex (TL2026, PyMuPDF origins): second stanza at 150.705, the
    // same 15.940bp baselineskip-plus-parsep gap.
    assert!((second.baseline - 150.705).abs() < 0.05, "second stanza baseline: {} ({words:?})", second.baseline);
    assert!((second.baseline - first.baseline - 15.940).abs() < 0.05, "parsep gap: {} ({words:?})", second.baseline - first.baseline);
}
