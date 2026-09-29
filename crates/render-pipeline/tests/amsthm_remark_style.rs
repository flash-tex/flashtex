//! amsthm's `remark` theorem style, against pdflatex.
//!
//! `\th@remark` (amsthm.sty 229-233) differs from `plain` and `definition`
//! in two ways the pipeline missed:
//!
//! * its skips are half of `\topsep`
//!   (`\thm@preskip\topsep \divide\thm@preskip\tw@ \thm@postskip\thm@preskip`).
//!   The pipeline gave it the whole `\topsep`, so each boundary between a
//!   remark and ordinary text sat 4 / 4.5 / 5 pt too low at a 10 / 11 / 12 pt
//!   base, and the error piled up down the page;
//! * the head font is `\itshape` and the number is `\@upn{#2}` (`\textup`),
//!   whose `\check@icl` runs `\sw@slant`: the blank before the number is
//!   taken off, the italic correction of the name's last letter goes in, and
//!   the blank goes back. The number and everything after it on the line sat
//!   one italic correction early (0.748 bp for the cmti10 `e` of `Note`).
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), two runs,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, word origins read from the PDF
//! (`tools/visual-oracle/pdftext.py`) on [`PROBE`] at each class size. The
//! values below are pdflatex's, in bp; before this change the pipeline had
//! `Note 1.` 3.985 bp low with `1.` 0.748 bp left, the second note 7.970 bp
//! low, and `Lambda` 15.940 bp low at 10pt.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// Every boundary a remark-style environment can have: after a paragraph,
/// after another remark, before a `definition`, unnumbered (`\newtheorem*`)
/// between a paragraph and a `plain` theorem, and nested in a `proof`
/// (whose `\topsep` is `6pt plus 6pt`, halved to 3pt).
const PROBE: &str = r"\documentclass[SIZEpt]{article}
\usepackage{amsthm}
\pagestyle{empty}
\newtheorem{theorem}{Theorem}
\theoremstyle{definition}
\newtheorem{defn}{Definition}
\theoremstyle{remark}
\newtheorem{note}{Note}
\newtheorem*{remark*}{Remark}
\begin{document}
Alpha opening paragraph.
\begin{note}Beta body.\end{note}
\begin{note}[Named]Gamma body.\end{note}
\begin{defn}Delta body.\end{defn}
Epsilon between.
\begin{remark*}Zeta body.\end{remark*}
\begin{theorem}Eta body.\end{theorem}
\begin{proof}Theta body.
\begin{note}Iota nested.\end{note}
Kappa continues.
\end{proof}
Lambda closing.
\end{document}
";

/// The word model's own agreement on this probe is within 0.005 bp.
const TOL: f64 = 0.05;

/// The origin `(x, baseline)` in bp of every glyph run on page 1, with its
/// text.
fn runs(size: u32) -> Vec<(String, f64, f64)> {
    let text = PROBE.replace("SIZE", &size.to_string());
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: &text }];
    let r = render(&docs, "main.tex", 1, "amsthm-remark-style", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        if let Some(g) = run.glyphs.first() {
            out.push((run.text.trim_start().to_string(), g.origin_x.to_bp(), g.baseline_y.to_bp()));
        }
    }
    out
}

/// The `nth` (0-based) run whose text starts with `word`.
fn at(runs: &[(String, f64, f64)], word: &str, nth: usize) -> (f64, f64) {
    runs.iter()
        .filter(|(t, _, _)| t.starts_with(word))
        .nth(nth)
        .map(|(_, x, y)| (*x, *y))
        .unwrap_or_else(|| panic!("no run #{nth} starting `{word}`; runs: {:?}", runs.iter().map(|r| &r.0).collect::<Vec<_>>()))
}

fn check(size: u32, label: &str, got: f64, expect: f64) {
    assert!(
        (got - expect).abs() <= TOL,
        "{size}pt {label}: {got:.3} bp, pdflatex {expect:.3} bp (off by {:+.3} bp)",
        got - expect
    );
}

/// (class size, word, which occurrence, pdflatex x, pdflatex baseline).
/// `x` is `None` where only the baseline is of interest.
#[allow(clippy::type_complexity)]
const ORACLE: &[(u32, &[(&str, usize, Option<f64>, f64)])] = &[
    (
        10,
        &[
            ("Alpha", 0, None, 134.765),
            ("Note", 0, None, 150.705),
            ("1", 0, Some(158.471), 150.705),
            ("Beta", 0, Some(171.489), 150.705),
            ("Note", 1, None, 166.645),
            ("2", 0, Some(158.471), 166.645),
            ("Definition", 0, None, 186.570),
            ("Epsilon", 0, None, 206.496),
            ("Remark", 0, None, 222.436),
            ("Zeta", 0, Some(175.170), 222.436),
            ("Theorem", 0, None, 242.361),
            ("Theta", 0, None, 262.286),
            ("Note", 2, None, 277.230),
            ("Iota", 0, Some(171.489), 277.230),
            ("Kappa", 0, None, 292.174),
            ("Lambda", 0, None, 312.100),
        ],
    ),
    (
        11,
        &[
            ("Alpha", 0, None, 140.742),
            ("Note", 0, None, 158.775),
            ("Beta", 0, Some(166.634), 158.775),
            ("Note", 1, None, 176.807),
            ("2", 0, Some(152.848), 176.807),
            ("Epsilon", 0, None, 221.838),
            ("Zeta", 0, Some(170.653), 239.871),
            ("Theta", 0, None, 284.902),
            ("Iota", 0, Some(166.634), 301.440),
            ("Kappa", 0, None, 317.978),
            ("Lambda", 0, None, 340.493),
        ],
    ),
    (
        12,
        &[
            ("Alpha", 0, None, 137.753),
            ("Note", 0, None, 157.181),
            ("Beta", 0, Some(154.281), 157.181),
            ("Note", 1, None, 176.608),
            ("2", 0, Some(139.868), 176.608),
            ("Epsilon", 0, None, 225.425),
            ("Zeta", 0, Some(158.590), 244.852),
            ("Theta", 0, None, 293.669),
            ("Iota", 0, Some(154.281), 311.103),
            ("Kappa", 0, None, 328.538),
            ("Lambda", 0, None, 351.950),
        ],
    ),
];

#[test]
fn remark_style_skips_are_half_topsep_and_its_number_follows_an_italic_correction() {
    if !common::lm_available() {
        eprintln!("SKIP remark_style_skips_are_half_topsep_and_its_number_follows_an_italic_correction: Latin Modern not installed");
        return;
    }
    for &(size, words) in ORACLE {
        let runs = runs(size);
        for &(word, nth, x, y) in words {
            let (gx, gy) = at(&runs, word, nth);
            check(size, &format!("`{word}` #{nth} baseline"), gy, y);
            if let Some(x) = x {
                check(size, &format!("`{word}` #{nth} x"), gx, x);
            }
        }
    }
}

/// A `\newenvironment` wrapper of a remark-style environment is that
/// environment (GH-1126): half `\topsep` on both sides and the corrected
/// number. pdflatex, article 10pt: `Alpha` 134.765, `Remark 1.` 150.705 with
/// `1.` at x 171.765 and `Beta` at 184.783, `Gamma` 166.645.
#[test]
fn a_wrapper_of_a_remark_style_environment_is_remark_style() {
    if !common::lm_available() {
        eprintln!("SKIP a_wrapper_of_a_remark_style_environment_is_remark_style: Latin Modern not installed");
        return;
    }
    let text = r"\documentclass{article}
\usepackage{amsthm}
\pagestyle{empty}
\theoremstyle{remark}
\newtheorem{remark}{Remark}
\newenvironment{myremark}{\begin{remark}}{\end{remark}}
\begin{document}
Alpha opening paragraph.
\begin{myremark}Beta body.\end{myremark}
Gamma closing paragraph.
\end{document}
";
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "amsthm-remark-wrapper", &fonts, &RenderOptions::default());
    let mut runs = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        if let Some(g) = run.glyphs.first() {
            runs.push((run.text.trim_start().to_string(), g.origin_x.to_bp(), g.baseline_y.to_bp()));
        }
    }
    for (word, x, y) in [("Alpha", None, 134.765), ("Remark", None, 150.705), ("1", Some(171.765), 150.705), ("Beta", Some(184.783), 150.705), ("Gamma", None, 166.645)] {
        let (gx, gy) = at(&runs, word, 0);
        check(10, &format!("`{word}` baseline"), gy, y);
        if let Some(x) = x {
            check(10, &format!("`{word}` x"), gx, x);
        }
    }
}
