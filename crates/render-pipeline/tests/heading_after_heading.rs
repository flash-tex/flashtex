//! A heading right after a heading, and a heading right after an empty
//! contents list.
//!
//! latex.ltx `\@startsection`:
//!
//! ```text
//! \if@nobreak
//!   \everypar{}%
//! \else
//!   \addpenalty\@secpenalty\addvspace\@tempskipa
//! \fi
//! ```
//!
//! `\@xsect` ends a display heading with `\vskip #5 \@afterheading`, and
//! `\@afterheading` sets `\@nobreaktrue`. So a heading straight after a
//! heading gets neither the penalty nor its before-skip: only the first
//! head's after-skip stands between them, and there is no breakpoint there.
//!
//! `\tableofcontents`, `\listoffigures` and `\listoftables` are a `\section*`
//! (`\chapter*` in report/book) followed by `\@starttoc`, which ends with
//! `\@nobreakfalse`. When the list is empty — no figures, or the first
//! run — a heading next *does* take the `\addvspace` branch, whose
//! `\@xaddvskip` keeps the larger of its before-skip and the list head's
//! after-skip. The pipeline treated the list head like any other head, so
//! `\listoffigures\listoftables` set the second head 1.2ex (5.144bp at
//! 10pt) too high: the AUX-lists lane's "`\section*{A}\section*{B}`" report.
//! In report the list head is a `\chapter*`, whose trailing `\vskip 40\p@`
//! already exceeds every before-skip, so nothing moves there.
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), `\usepackage[T1]{fontenc}`,
//! `\usepackage[margin=1in]{geometry}`, `\pagestyle{empty}`. `\zsavepos`
//! placed right *after* each anchor word (a marker before the first glyph of
//! a line can attach to the wrong line), `posy` sp → bp as
//! `/65536/1.00375`, compared baseline to baseline. 262 such pairs across
//! article 10/11/12pt and report 10/11pt — section/subsection/subsubsection
//! pairs starred and unstarred, `\chapter` then `\section`, and the lists
//! followed by every level — match to 0.003bp after the change; the ones
//! pinned below are the representative rows.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 0.02;

/// `class` is the size option, `report`-prefixed for report.
fn doc(class: &str, body: &str) -> String {
    let (cls, size) = class.strip_prefix("report").map_or(("article", class), |s| ("report", s));
    format!("\\documentclass[{size}]{{{cls}}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage[margin=1in]{{geometry}}\n\\pagestyle{{empty}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// `(page, baseline in bp)` of the first glyph run containing each word.
fn positions(text: &str, words: &[&str]) -> Vec<(usize, f64)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "heading-after-heading", &fonts, &RenderOptions::default());
    words
        .iter()
        .map(|w| {
            r.v2.pages
                .iter()
                .enumerate()
                .find_map(|(pi, page)| {
                    page.items.iter().find_map(|item| match item {
                        Item::GlyphRun(run) if run.text.contains(w) => run.glyphs.first().map(|g| (pi + 1, g.baseline_y.to_bp())),
                        _ => None,
                    })
                })
                .unwrap_or_else(|| panic!("no glyph run containing `{w}`"))
        })
        .collect()
}

fn gap(class: &str, body: &str, a: &str, b: &str) -> f64 {
    let p = positions(&doc(class, body), &[a, b]);
    assert_eq!(p[0].0, p[1].0, "`{a}` and `{b}` on different pages");
    p[1].1 - p[0].1
}

fn check(class: &str, body: &str, a: &str, b: &str, pdflatex: f64) {
    let got = gap(class, body, a, b);
    assert!(
        (got - pdflatex).abs() <= TOL,
        "{class} `{}`: `{a}` to `{b}` is {got:.4} bp, pdflatex {pdflatex:.4} bp (off by {:+.4})",
        body.replace('\n', " "),
        got - pdflatex
    );
}

#[test]
fn a_heading_after_an_empty_list_takes_its_addvspace() {
    if !common::lm_available() {
        return;
    }
    // Before the change: 27.7984 / 28.7356 / 33.7568 — the section
    // after-skip plus the `\Large` `\baselineskip`, as for two real heads.
    check("10pt", "\\listoffigures\n\\listoftables\nCharlie follows.", "Figures", "Tables", 32.9422);
    check("11pt", "\\listoffigures\n\n\\listoftables\nCharlie follows.", "Figures", "Tables", 34.3681);
    check("12pt", "\\tableofcontents\n\\section*{Bravo}\nCharlie follows.", "Contents", "Bravo", 39.9292);
    check("10pt", "\\listoffigures\n\\section{Bravo}\nCharlie follows.", "Figures", "Bravo", 32.9422);
    // A smaller head: `\addvspace{3.25ex}` still beats the list head's 2.3ex.
    check("10pt", "\\listoffigures\n\\subsection{Bravo}\nCharlie follows.", "Figures", "Bravo", 27.8851);
    check("11pt", "\\listoffigures\n\\subsubsection*{Bravo}\nCharlie follows.", "Figures", "Bravo", 28.8105);
    // The body after the second list head is unchanged.
    check("10pt", "\\listoffigures\n\\listoftables\nCharlie follows.", "Tables", "Charlie", 21.8185);
}

#[test]
fn report_list_heads_already_leave_the_larger_skip() {
    if !common::lm_available() {
        return;
    }
    check("report10pt", "\\listoffigures\n\\section{Bravo}\nCharlie follows.", "Figures", "Bravo", 57.7833);
    check("report11pt", "\\listoffigures\n\\subsection{Bravo}\nCharlie follows.", "Figures", "Bravo", 53.7983);
}

/// Two real heads: only the first one's after-skip, whatever the second.
#[test]
fn a_heading_after_a_heading_has_no_before_skip() {
    if !common::lm_available() {
        return;
    }
    let body = |a: &str, b: &str| format!("Lead paragraph of ordinary text.\n\n\\{a}{{Alpha}}\n\\{b}{{Bravo}}\nCharlie follows.");
    check("10pt", &body("section*", "section*"), "Alpha", "Bravo", 27.7961);
    check("10pt", &body("section", "section"), "Alpha", "Bravo", 27.7961);
    check("10pt", &body("section", "subsubsection"), "Alpha", "Bravo", 21.8185);
    check("11pt", &body("subsection*", "subsection*"), "Alpha", "Bravo", 20.9914);
    check("11pt", &body("subsection", "subsubsection"), "Alpha", "Bravo", 20.5929);
    check("12pt", &body("subsection*", "section"), "Alpha", "Bravo", 29.6370);
    check("report10pt", &body("subsection", "subsection"), "Alpha", "Bravo", 20.3803);
    let chapter = |a: &str, b: &str| format!("\\{a}{{Alpha}}\n\\{b}{{Bravo}}\nCharlie follows.");
    check("report10pt", &chapter("chapter", "section"), "Alpha", "Bravo", 57.7833);
    check("report11pt", &chapter("chapter*", "section*"), "Alpha", "Bravo", 57.7833);
    check("report10pt", &chapter("chapter*", "subsection"), "Alpha", "Bravo", 53.7983);
}

/// `\addpenalty\@secpenalty` is in the same `\else` branch: two heads at the
/// foot of a page move together. With 47 one-line paragraphs above them
/// pdflatex keeps both heads and the body on page 1; with 48 all three go
/// to page 2. The pipeline used to break between the heads at the
/// `\@secpenalty` it still placed there (47-50 lines: `Alpha` on page 1,
/// `Bravo` on page 2).
#[test]
fn no_page_break_between_two_heads() {
    if !common::lm_available() {
        return;
    }
    for (lines, page) in [(46, 1), (47, 1), (48, 2), (49, 2), (50, 2)] {
        let fill: String = (1..=lines).map(|i| format!("Line {i}.\\par ")).collect();
        let text = doc("10pt", &format!("{fill}\n\\section*{{Alpha}}\n\\section*{{Bravo}}\nCharlie one.\\par Delta two.\\par"));
        let p = positions(&text, &["Alpha", "Bravo", "Charlie"]);
        let pages: Vec<usize> = p.iter().map(|x| x.0).collect();
        assert_eq!(pages, [page; 3], "{lines} lines of filler: pages of Alpha/Bravo/Charlie (pdflatex: all on {page})");
    }
}
