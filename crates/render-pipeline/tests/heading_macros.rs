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
        for it in &page.items {
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

/// HW1's `\subsection*{Bonus Problem \hfill \normalfont[1 bonus point]}`:
/// the spaces after `\normalfont` are `ecrm1200`'s, not the head's
/// `ecbx1200` ones, so `[1` sits 1.15bp further right than it did. pdfTeX
/// (TeX Live 2026, 11pt, T1): `\hbox{\large\normalfont[1 bonus point]}` is
/// 77.07414pt, `[1` is 9.13664pt, `bonus` 30.08568pt, and the interword
/// space `\fontdimen2` is 3.91571pt (4.4989pt in `\bfseries`).
#[test]
fn direct_heading_spaces_after_normalfont_use_the_medium_font() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[11pt]{article}\\usepackage[T1]{fontenc}\\begin{document}\n\\subsection*{Bonus Problem \\hfill \\normalfont[1 bonus point]}\nBody.\n\\end{document}";
    let words = layout(src);
    let (text_x, text_w) = measure();
    let (open, bonus, point) = (word(&words, "[1"), word(&words, "bonus"), word(&words, "point]"));
    let right = text_x + text_w;
    assert!((right - open.x - bp(77.07414)).abs() < 0.02, "[1 from the right margin: {} vs {}", right - open.x, bp(77.07414));
    assert!((bonus.x - (open.x + open.width) - bp(3.91571)).abs() < 0.02, "space after [1: {}", bonus.x - (open.x + open.width));
    assert!((point.x - (bonus.x + bonus.width) - bp(3.91571)).abs() < 0.02, "space after bonus: {}", point.x - (bonus.x + bonus.width));
    // A space next to a bold word keeps the head font: `A {\normalfont B} C`.
    let words = layout("\\documentclass[11pt]{article}\\usepackage[T1]{fontenc}\\begin{document}\n\\subsection*{A {\\normalfont B} C \\normalfont D E}\nBody.\n\\end{document}");
    let gap = |l: &str, r: &str| word(&words, r).x - (word(&words, l).x + word(&words, l).width);
    for (l, r) in [("A", "B"), ("B", "C"), ("C", "D")] {
        assert!((gap(l, r) - bp(4.4989)).abs() < 0.02, "{l}-{r} is a bold space: {}", gap(l, r));
    }
    assert!((gap("D", "E") - bp(3.91571)).abs() < 0.02, "D-E is a medium space: {}", gap("D", "E"));
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

/// `\c@secnumdepth` is the class's own counter, not a flat default: article
/// sets it to 3 (`article.cls` line 255), so `\subsubsection` is numbered
/// `1.1.1`; report/book set 2, so the same heading carries no number. The
/// pipeline used to assume 2 for every class, which dropped the number from
/// every `article` `\subsubsection` — and left the contents list, which
/// already derived the class default, writing a `\numberline` for a heading
/// whose printed form had none.
#[test]
fn article_numbers_subsubsection_and_report_does_not() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let body = "\\section{One}\nBody.\n\\subsection{Two}\nBody.\n\\subsubsection{Three}\nTail.\n";
    let art = layout(&format!("\\documentclass{{article}}\\begin{{document}}\n{body}\\end{{document}}"));
    assert!(art.iter().any(|w| w.text == "1.1.1"), "article: no 1.1.1 in {art:?}");
    // `\@seccntformat`: the number, a `\quad`, then the title, all flush left.
    let (n, t) = (word(&art, "1.1.1"), word(&art, "Three"));
    assert!((n.x - word(&art, "1").x).abs() < 0.05, "number flush with \\section's: {} vs {}", n.x, word(&art, "1").x);
    assert!(t.x > n.x + n.width, "title after the number: {} vs {}", t.x, n.x + n.width);

    // report/book stop at 2. `\thesubsection` there is
    // `\thechapter.\arabic{section}.\arabic{subsection}`, so the *subsection*
    // is `0.1.1` before any `\chapter`; the subsubsection would be `0.1.1.1`
    // and must not appear at all.
    let rep = layout(&format!("\\documentclass{{report}}\\begin{{document}}\n{body}\\end{{document}}"));
    assert!(rep.iter().any(|w| w.text == "0.1.1"), "report still numbers \\subsection: {rep:?}");
    assert!(!rep.iter().any(|w| w.text == "0.1.1.1"), "report secnumdepth is 2: {rep:?}");
    assert!(rep.iter().any(|w| w.text == "Three"), "report still sets the title: {rep:?}");

    // An explicit `\setcounter` still wins over the class default.
    let off = layout(&format!("\\documentclass{{article}}\\setcounter{{secnumdepth}}{{2}}\\begin{{document}}\n{body}\\end{{document}}"));
    assert!(!off.iter().any(|w| w.text == "1.1.1"), "\\setcounter{{secnumdepth}}{{2}}: {off:?}");
}
