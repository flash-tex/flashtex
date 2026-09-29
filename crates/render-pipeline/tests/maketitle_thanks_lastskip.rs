//! `\thanks` in `\maketitle` leaves `\lastskip` at 0 for the next
//! `\addvspace`.
//!
//! article.cls's `\maketitle` runs `\@maketitle`, which ends with
//! `\vskip 1.5em`, and only then `\@thanks`: the `\footnotetext` of each
//! `\thanks`, which puts an insert node on the vertical list. So when the
//! document has a `\thanks`, the list ends in that insert and not in the
//! 1.5em glue. The next `\@startsection` (or a list's `\@item`) runs
//! `\addvspace`, sees `\lastskip = 0`, and adds its whole skip. Without a
//! `\thanks` it adds only the excess over 1.5em, which is nothing at any
//! class size.
//!
//! Before the fix the pipeline always merged with the 1.5em. Every body
//! line after a `\thanks` title was therefore high by the heading's
//! before-skip: 14.94 bp at 10pt, 16.36 at 11pt and 17.56 at 12pt for
//! `\section`; 15.27 at 11pt for `\subsection`; 9.96/11.96/12.95 for
//! `itemize`; 11.96 for `abstract` at 11pt. A paragraph right after the
//! title has no `\addvspace` and never moved.
//!
//! Oracle: pdflatex (TeX Live 2026), each probe below with and without
//! `\thanks{Collaborators: none.}` in `\author`. The distance from the
//! title's baseline to the first body line is read off the shipped PDF
//! with PyMuPDF, in bp. The same distance without `\thanks` is pinned too,
//! because that case must not move.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 0.05;

fn probe(size: &str, thanks: bool, body: &str) -> String {
    format!(
        "\\documentclass[{size}]{{article}}\n\\title{{CS 70 Homework 2}}\n\\author{{Sam Rivera{}}}\n\\date{{}}\n\\begin{{document}}\n\\maketitle\n{body}\\end{{document}}\n",
        if thanks { "\\thanks{Collaborators: none.}" } else { "" }
    )
}

/// The baseline, in bp, of the first glyph run on page 1 that starts with `word`.
fn baseline_of(text: &str, word: &str) -> f64 {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "maketitle-thanks-lastskip", &fonts, &RenderOptions::default());
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.trim_start().starts_with(word) {
            if let Some(g) = run.glyphs.first() {
                return g.baseline_y.to_bp();
            }
        }
    }
    panic!("no glyph run starting `{word}` on page 1");
}

const SECTION: &str = "\\section*{1. Logic}\nText here.\n";
const NUMBERED: &str = "\\section{Logic}\nText here.\n";
const SUBSECTION: &str = "\\subsection{Part}\nText.\n";
const LIST: &str = "\\begin{itemize}\n\\item One.\n\\item Two.\n\\end{itemize}\nAfter.\n";
const ABSTRACT: &str = "\\begin{abstract}\nShort abstract.\n\\end{abstract}\n\\section{Intro}\nText.\n";
const PARAGRAPH: &str = "Text right after the title.\n\nSecond paragraph.\n";

/// (class size, body, first word of the measured line, with `\thanks`, without).
const CASES: &[(&str, &str, &str, f64, f64)] = &[
    ("10pt", SECTION, "Text", 108.565, 93.621),
    ("10pt", NUMBERED, "Text", 108.565, 93.621),
    ("10pt", LIST, "After.", 117.559, 107.596),
    ("11pt", SECTION, "Text", 117.355, 100.991),
    ("11pt", NUMBERED, "Text", 117.355, 100.991),
    ("11pt", SUBSECTION, "Text", 108.438, 93.173),
    ("11pt", LIST, "After.", 132.155, 120.200),
    ("11pt", ABSTRACT, "Short", 100.723, 88.767),
    ("11pt", PARAGRAPH, "Text", 72.180, 72.180),
    ("12pt", SECTION, "Text", 132.221, 114.662),
    ("12pt", NUMBERED, "Text", 132.221, 114.662),
    ("12pt", LIST, "After.", 145.205, 132.254),
];

#[test]
fn a_thanks_insert_leaves_no_lastskip_for_the_next_addvspace() {
    if !common::lm_available() {
        eprintln!("SKIP a_thanks_insert_leaves_no_lastskip_for_the_next_addvspace: Latin Modern not installed");
        return;
    }
    let mut failures = Vec::new();
    for &(size, body, word, with, without) in CASES {
        for (thanks, want) in [(true, with), (false, without)] {
            let tex = probe(size, thanks, body);
            let got = baseline_of(&tex, word) - baseline_of(&tex, "CS");
            if (got - want).abs() > TOL {
                failures.push(format!("{size} thanks={thanks} `{}`: title to `{word}` {got:.3} bp, pdflatex {want:.3}", body.lines().next().unwrap_or("")));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
