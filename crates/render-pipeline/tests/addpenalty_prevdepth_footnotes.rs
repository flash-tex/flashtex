//! A page broken at a heading keeps no depth below its last line when a
//! skip came before that heading. This matters for the footnotes under it.
//!
//! latex.ltx's `\addpenalty` runs when `\lastskip` is nonzero (a display's
//! `\belowdisplayskip`, a list's or theorem's closing `\@topsepadd`). It
//! backs up `\prevdepth` together with that skip:
//! `\vskip-(\lastskip+\prevdepth) \penalty#1 \vskip\prevdepth
//! \vskip\lastskip`. A page that breaks at that penalty therefore ends
//! at its last line's baseline, and `\@makecol`'s `\skip\footins` is
//! measured from there. pdflatex's `\tracingoutput` for the list probe
//! below shows this: the item line `\hbox(8.2125+2.73749)`, `\penalty -51`,
//! `\glue 9.0 plus 3.0 minus 5.0`, `\glue -11.73749 plus -3.0 minus -5.0`,
//! and then `\glue 10.0 plus 4.0 minus 2.0` (`\skip\footins`).
//!
//! The pipeline put the penalty straight after the line, so the depth
//! stayed in. Under `\raggedbottom` the footnotes sit right below the
//! text, and they were low by that depth: 2.73 bp after a list or a
//! theorem, and 5.48 bp after a display with a `\Bigl(` in it. On a page
//! that has to shrink, the whole page shrank by the extra amount. A list
//! closing right before a heading also never reached the heading at all
//! (`Block::Heading::list_end`).
//!
//! Oracle: pdflatex (TeX Live 2026) on the probes built by [`probe`]. The
//! numbers are the baselines of the two page-1 footnote lines, read off
//! the shipped PDF with PyMuPDF, in bp.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 0.05;
const FILL: &str = "Some words to fill out the line of text with enough material to wrap around a few times here. ";

/// `k` extra fill sentences in the opening paragraph; every section ends
/// with `tail` (numbered by the section).
fn probe(k: usize, tail: &str) -> String {
    let mut s = String::from("\\documentclass[11pt]{article}\n\\usepackage[margin=1in]{geometry}\n\\usepackage{amsthm}\n\\newtheorem{theorem}{Theorem}\n\\begin{document}\n");
    s.push_str("Opening\\footnote{A note on the first page.} ");
    s.push_str(&FILL.repeat(2 + k));
    s.push_str("\n\n");
    for i in 0..8 {
        let t = tail.replace("#", &i.to_string());
        s.push_str(&format!("\\section{{Part {i}}}\n{}\n{t}", FILL.repeat(3)));
        if i == 3 {
            s.push_str(&format!("Second page note\\footnote{{A note on a later page.}} here. {FILL}\n{t}"));
        }
    }
    s.push_str("\\end{document}\n");
    s
}

const DISPLAY: &str = "\\[ x_{#}^2 + y^2 = \\Bigl(\\frac{a}{b}\\Bigr)^2 \\]\n";
const LIST: &str = "\\begin{itemize}\n\\item Item with a descender (gjpq) #.\n\\end{itemize}\n";
const THEOREM: &str = "\\begin{theorem}\nStatement # with $\\bigl(x\\bigr)$.\n\\end{theorem}\n";

/// The baselines, in bp, of the page-1 glyph runs that start a footnote text line.
fn note_baselines(text: &str) -> Vec<f64> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "addpenalty-prevdepth", &fonts, &RenderOptions::default());
    // One run per word: a note line is the run "A" followed by "note".
    let words: Vec<(&str, f64)> = r.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run) => run.glyphs.first().map(|g| (run.text.trim(), g.baseline_y.to_bp())),
            _ => None,
        })
        .collect();
    words.windows(2).filter(|w| w[0].0 == "A" && w[1].0 == "note" && (w[0].1 - w[1].1).abs() < 1e-6).map(|w| w[0].1).collect()
}

/// (probe, extra fill, pdflatex page-1 footnote baselines).
const CASES: &[(&str, &str, usize, [f64; 2])] = &[
    // Page 1 breaks at `\section` after `\belowdisplayskip`; the display's
    // `\Bigl(` makes its last line 5.48bp deep. Before: 646.224, 657.183.
    ("display", DISPLAY, 0, [640.746, 651.705]),
    // The same break on a page that has to shrink. Before: 709.041, 720.000.
    ("display", DISPLAY, 6, [708.492, 719.451]),
    // After `\end{itemize}` and after `\end{theorem}`. Before: 649.449, 660.408.
    ("list", LIST, 3, [646.723, 657.682]),
    ("theorem", THEOREM, 3, [646.723, 657.682]),
];

#[test]
fn a_page_broken_at_a_heading_after_a_skip_has_no_depth_above_its_footnotes() {
    if !common::lm_available() {
        eprintln!("SKIP a_page_broken_at_a_heading_after_a_skip_has_no_depth_above_its_footnotes: Latin Modern not installed");
        return;
    }
    let mut failures = Vec::new();
    for &(name, tail, k, want) in CASES {
        let got = note_baselines(&probe(k, tail));
        if got.len() != 2 || got.iter().zip(want).any(|(g, w)| (g - w).abs() > TOL) {
            failures.push(format!("{name} k={k}: page-1 note baselines {got:.3?} bp, pdflatex {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
