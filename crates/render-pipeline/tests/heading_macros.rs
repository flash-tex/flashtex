//! Headings built by user macros (`\newcommand{\problem}[2]{\subsection*{Problem
//! #1 \hfill \normalfont[#2 points]}}`, the shape in fixtures/real-world/hw1),
//! and `\\[<dimen>]`. The compiler gives every replacement-text token the
//! invocation's span and drops `\hfill` from titles, so the pipeline reads
//! the definition: the spaces around `#1`/`#2`, the `\hfill` (second-order
//! glue, so `[4 points]` sits at the right margin against `\parfillskip`)
//! and `\normalfont` (the compiler's own heading style) come from it.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[derive(Debug, Clone)]
struct Word {
    text: String,
    x: f64,
    baseline: f64,
    width: f64,
    face: String,
}

fn layout(text: &str) -> Vec<Word> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let mut words = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                let last = run.glyphs.last().expect("non-empty");
                words.push(Word {
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    width: (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64 / flashtex_render_pipeline::display::TICKS_PER_BP,
                    face: fonts.by_font_id(&run.font_id).map(|f| f.name.clone()).unwrap_or_default(),
                });
            }
        }
    }
    words
}

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

/// 11pt article, US Letter: the text area's left edge and `\textwidth` in bp.
fn measure() -> (f64, f64) {
    let s = flashtex_render_pipeline::Stylesheet::article(11, flashtex_render_pipeline::fonts::Family::LatinModern, None);
    (bp(s.text_x_pt), bp(s.text_width_pt))
}

#[test]
fn macro_heading_spaces_hfill_and_normalfont_come_from_the_definition() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[11pt]{article}\n\\newcommand{\\problem}[2]{\\subsection*{Problem #1 \\hfill \\normalfont[#2 points]}}\n\\begin{document}\n\\problem{1}{4}\nBody text.\n\\end{document}";
    let words = layout(src);
    let (text_x, text_w) = measure();
    let problem = word(&words, "Problem");
    let one = word(&words, "1");
    let open = word(&words, "[4");
    let points = word(&words, "points]");
    // `Problem #1`: an interword space (not glued as `Problem1`).
    let gap = one.x - (problem.x + problem.width);
    assert!(gap > bp(2.5) && gap < bp(5.0), "space between Problem and 1: {gap}");
    // `[#2 points]`: `[` glued to `4`, a space before `points]`.
    assert_eq!(open.text, "[4");
    let gap = points.x - (open.x + open.width);
    assert!(gap > bp(2.0) && gap < bp(5.0), "space between [4 and points]: {gap}");
    // `\hfill` (fill order) pushes `[4 points]` to the right margin.
    let right = points.x + points.width;
    assert!((right - (text_x + text_w)).abs() < 0.05, "[4 points] ends at the right margin: {right} vs {}", text_x + text_w);
    assert!(problem.x - text_x < 0.05, "Problem starts at the left margin: {}", problem.x);
    assert!((one.baseline - problem.baseline).abs() < 1e-6 && (points.baseline - problem.baseline).abs() < 1e-6, "one line");
    // `\bfseries` from `\@startsection` for the title, `\normalfont` after.
    assert!(problem.face.to_ascii_lowercase().contains("bold"), "Problem is bold: {}", problem.face);
    assert!(one.face.to_ascii_lowercase().contains("bold"), "1 is bold: {}", one.face);
    assert!(!open.face.to_ascii_lowercase().contains("bold"), "[4 is medium: {}", open.face);
    assert!(!points.face.to_ascii_lowercase().contains("bold"), "points] is medium: {}", points.face);
}

#[test]
fn direct_heading_hfill_reaches_the_right_margin() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[11pt]{article}\\begin{document}\n\\subsection*{Bonus Problem \\hfill \\normalfont[1 bonus point]}\nBody.\n\\end{document}";
    let words = layout(src);
    let (text_x, text_w) = measure();
    let bonus = word(&words, "Bonus");
    let point = word(&words, "point]");
    assert!((point.x + point.width - (text_x + text_w)).abs() < 0.05, "right margin: {}", point.x + point.width);
    assert!(bonus.face.to_ascii_lowercase().contains("bold") && !point.face.to_ascii_lowercase().contains("bold"));
    // `Problem \hfill \normalfont[1`: one interword space before the glue,
    // none after `\normalfont`.
    let problem = word(&words, "Problem");
    assert!(problem.x - (bonus.x + bonus.width) > bp(2.5));
}

#[test]
fn line_break_dimen_adds_vertical_space_after_the_line() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // A plain paragraph: `\\[3pt]` is `\vadjust{\vskip 3pt}` after its line.
    let src = "\\documentclass[11pt]{article}\\begin{document}\nFirst line\\\\[3pt]\nSecond line\\\\\nThird line\n\\end{document}";
    let words = layout(src);
    let (first, second, third) = (word(&words, "First"), word(&words, "Second"), word(&words, "Third"));
    assert!((second.baseline - first.baseline - bp(13.6 + 3.0)).abs() < 0.05, "3pt below the first line: {}", second.baseline - first.baseline);
    assert!((third.baseline - second.baseline - bp(13.6)).abs() < 0.05, "plain \\\\: {}", third.baseline - second.baseline);
    // `center`: `\\[7pt]` is `\par \addvspace{-\parskip} \vskip 7pt`; after
    // a final one the next paragraph's `\parskip` (0pt plus 1pt) is
    // cancelled and only the 7pt separates the lines. `\\[3pt]` inside the
    // block adds 3pt to the pitch.
    let src = "\\documentclass[11pt]{article}\\begin{document}\n\\begin{center}\n{\\Large\\bfseries Title}\\\\[3pt]\n{\\LARGE\\bfseries Sheet}\\\\[7pt]\n\nSolutions here\n\\end{center}\n\\end{document}";
    let words = layout(src);
    let (title, sheet, solutions) = (word(&words, "Title"), word(&words, "Sheet"), word(&words, "Solutions"));
    // `\Large` 14.4pt/18pt and `\LARGE` 17.28pt/22pt at 11pt: the lines are
    // set at body `\baselineskip` (13.6pt) unless the glyph boxes force
    // `\lineskip`; the 3pt/7pt come on top of whatever the pitch is.
    let pitch_a = sheet.baseline - title.baseline;
    let pitch_b = solutions.baseline - sheet.baseline;
    let plain = layout("\\documentclass[11pt]{article}\\begin{document}\n\\begin{center}\n{\\Large\\bfseries Title}\\\\\n{\\LARGE\\bfseries Sheet}\\\\\n\nSolutions here\n\\end{center}\n\\end{document}");
    let (pt, ps, pso) = (word(&plain, "Title"), word(&plain, "Sheet"), word(&plain, "Solutions"));
    assert!((pitch_a - (ps.baseline - pt.baseline) - bp(3.0)).abs() < 0.05, "3pt: {} vs {}", pitch_a, ps.baseline - pt.baseline);
    assert!((pitch_b - (pso.baseline - ps.baseline) - bp(7.0)).abs() < 0.05, "7pt: {} vs {}", pitch_b, pso.baseline - ps.baseline);
}
