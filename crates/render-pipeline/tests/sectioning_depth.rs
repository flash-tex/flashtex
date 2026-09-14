//! `\c@secnumdepth` comes from the class, not from one hard-coded default.
//!
//! article.cls line 255 `\setcounter{secnumdepth}{3}`, report.cls and
//! book.cls `\setcounter{secnumdepth}{2}`. `\@sect` prints the counter when
//! `\ifnum #2>\c@secnumdepth` is false, so an `article` numbers
//! `\subsubsection` and a `report` does not. The same counter decides the
//! `\numberline` written to the contents list, so the two must agree.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

fn words(text: &str) -> Vec<(String, f64)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for it in &page.items {
            if let Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                out.push((run.text.clone(), first.origin_x.to_bp()));
            }
        }
    }
    out
}

fn body(class: &str) -> String {
    format!(
        "\\documentclass{{{class}}}\n\\usepackage[T1]{{fontenc}}\n\\begin{{document}}\n\
         \\section{{One}}\nBody.\n\n\\subsection{{Two}}\nBody.\n\n\
         \\subsubsection{{Three}}\nBody.\n\\end{{document}}\n"
    )
}

/// The `\subsubsection` heading's own words: the number, then `\quad`, then
/// the title. The title's `x` says whether the number was set.
fn heading_words(words: &[(String, f64)]) -> Vec<(String, f64)> {
    words.iter().filter(|(t, _)| t == "1.1.1" || t == "Three").cloned().collect()
}

#[test]
fn article_numbers_subsubsection_at_its_class_secnumdepth_of_three() {
    if !lm_available() {
        return;
    }
    let w = words(&body("article"));
    let h = heading_words(&w);
    assert_eq!(h.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(), ["1.1.1", "Three"], "{h:?}");
    let (number_x, title_x) = (h[0].1, h[1].1);
    let left = h.iter().map(|(_, x)| *x).fold(f64::INFINITY, f64::min);
    assert!((number_x - left).abs() < 1e-6, "the number starts the heading: {h:?}");
    // `\@seccntformat` puts `\quad` (1 em of the heading's `\normalsize`
    // face) between the number and the title, so the title is pushed right.
    assert!(title_x - number_x > 20.0, "title not pushed past the number: {h:?}");
}

#[test]
fn report_leaves_subsubsection_unnumbered_at_its_class_secnumdepth_of_two() {
    if !lm_available() {
        return;
    }
    let w = words(&body("report"));
    let h = heading_words(&w);
    assert_eq!(h.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(), ["Three"], "{h:?}");
}

#[test]
fn an_explicit_setcounter_still_wins_over_the_class_default() {
    if !lm_available() {
        return;
    }
    let src = body("article").replace("\\begin{document}", "\\setcounter{secnumdepth}{2}\n\\begin{document}");
    let h = heading_words(&words(&src));
    assert_eq!(h.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(), ["Three"], "{h:?}");
}
