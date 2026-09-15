//! `\maketitle` sets the author `tabular` even when there is no author.
//!
//! `\@maketitle` is
//!
//! ```text
//! \newpage \null \vskip 2em
//! \begin{center}%
//!   {\LARGE \@title \par}%
//!   \vskip 1.5em%
//!   {\large \lineskip .5em%
//!     \begin{tabular}[t]{c}\@author\end{tabular}\par}%
//!   \vskip 1em%
//!   {\large \@date}%
//! \end{center}\par \vskip 1.5em
//! ```
//!
//! The `tabular` is unconditional. `\author{}` — and no `\author` at all,
//! which only adds a warning — gives a `tabular` with no rows: no ink, no
//! height, no depth, but still a *line* of the centred paragraph, and so
//! still its own interline glue. The pipeline dropped an author with no
//! `tabular` cells, and with it that glue.
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025), `\showoutput`,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on [`PROBE`] with
//! `\author{}` and `\date{}` at an 11 pt base:
//!
//! ```text
//! ...\glue(\baselineskip) 10.10013
//! ...\hbox(11.89987+3.36028)x469.75502     % {\LARGE \@title}
//! ...\glue 16.33182                        % \vskip 1.5em
//! ...\glue(\parskip) 0.0 plus 1.0
//! ...\glue(\parskip) 0.0
//! ...\glue(\baselineskip) 10.63972         % 14.0 - 3.36028 - 0.0
//! ...\hbox(0.0+0.0)x469.75502, glue set 234.87752fil   % the empty tabular
//! ...\glue 10.88788                        % \vskip 1em
//! ...\glue -10.88788
//! ...\glue 10.88788
//! ...\glue -10.88788
//! ...\penalty -51
//! ...\glue 10.88788
//! ...\glue -10.88788
//! ...\glue 12.0 plus 4.0 minus 6.0         % center's \@topsepadd
//! ...\glue 16.33182                        % \@maketitle's closing \vskip 1.5em
//! ...\glue -16.33182
//! ...\penalty -300
//! ...\glue 16.33182
//! ...\glue -16.33182
//! ...\glue 16.49693 plus 4.71341 minus 0.94266   % \section*'s before-skip
//! ...\glue(\parskip) 0.0 plus 1.0
//! ...\glue(\parskip) 0.0
//! ...\glue(\baselineskip) 8.06242          % 18.0 - 0.0 - 9.93758 (\Large)
//! ...\hbox(9.93758+0.0)x469.75502          % "Series and sums"
//! ```
//!
//! The `\vskip 1em` before the (empty) date and the closing `\vskip 1.5em`
//! are each taken straight off again by an `\addvspace`, so the title's
//! baseline down to the heading's is
//!
//! ```text
//! 3.36028 + 16.33182 + 10.63972 + 0.0 + 12.0 + 16.49693 + 8.06242 + 9.93758
//!   = 76.82875 pt = 76.5411 bp
//! ```
//!
//! Without the empty `tabular` line that is 30.328 pt short.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// 1 bp = 1.00375 TeX pt.
const BP: f64 = 1.00375;
const TOL: f64 = 0.5;

/// `AUTHOR` and `DATE` are substituted; both empty is
/// `fixtures/real-world/math-sheet`'s own preamble.
const PROBE: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\title{Formula Sheet: Calculus and Linear Algebra}
\author{AUTHOR}
\date{DATE}
\begin{document}
\maketitle
\thispagestyle{empty}

\section*{Series and sums}
Body text after the heading.
\end{document}
";

fn probe(author: &str, date: &str) -> String {
    PROBE.replace("AUTHOR", author).replace("DATE", date)
}

/// The baseline of the first glyph of the run that starts with `word`.
fn baseline_of(text: &str, word: &str) -> f64 {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "maketitle-empty-author", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.trim_start().starts_with(word) {
            if let Some(g) = run.glyphs.first() {
                return g.baseline_y.to_bp();
            }
        }
    }
    panic!("no glyph run starting `{word}` on page 1");
}

/// Title baseline to `\section*` heading baseline, in bp, for each
/// combination — every number read off the shipped pages of the four probes.
const CASES: &[(&str, &str, f64)] = &[
    // The line the empty `tabular` replaces is 14.0 pt of `\large`
    // `\baselineskip` less the title's depth, and the author's own height
    // and depth where there is one.
    ("A. Reyes", "August 2025", 100.938),
    ("", "August 2025", 100.938),
    ("A. Reyes", "", 76.541),
    ("", "", 76.541),
];

#[test]
fn an_empty_author_still_sets_its_line() {
    if !common::lm_available() {
        eprintln!("SKIP an_empty_author_still_sets_its_line: Latin Modern not installed");
        return;
    }
    for &(author, date, expected) in CASES {
        let tex = probe(author, date);
        let got = baseline_of(&tex, "Series") - baseline_of(&tex, "Formula");
        assert!(
            (got - expected).abs() <= TOL,
            "\\author{{{author}}} \\date{{{date}}}: title to heading {got:.4} bp, pdflatex \
             {expected:.4} bp (off by {:+.4} bp). The empty author `tabular` is a line of no \
             height whose interline glue is {:.3} bp here, and dropping it takes that glue with it.",
            got - expected,
            10.63972 / BP
        );
    }
}
