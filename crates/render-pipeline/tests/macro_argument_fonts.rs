//! A font declaration in a user macro's body applies to the macro's
//! arguments.
//!
//! The compiler gives an argument's tokens their own source span, and the
//! pipeline reads bold/italic/family from the source brace groups at that
//! span (`adapter::Styles::at`). The argument's bytes sit at the call site,
//! outside every group the *definition* opened, so
//! `\newcommand{\note}[1]{{\small\bfseries #1}}` set `\note{words}` in
//! `LMRoman9-Regular`: the size came through (the compiler scopes sizes) and
//! the series did not. The narrower medium face then reflowed the paragraph —
//! `fixtures/divergence-probes/min2-preamble-switches` put `followed by` on
//! the wrong line, 419.03 bp from pdfLaTeX. `fixtures/real-world/cv`'s
//! `\cvsection` (`{\large\bfseries #1}`) has the same shape.
//!
//! ## Oracle
//!
//! Every number below is a word origin (first glyph x, baseline y from the
//! page top, bp) read by `tools/visual-oracle/pdftext.py` from pdfLaTeX's
//! output for the document in the same test: pdfTeX 3.141592653-2.6-1.40.29
//! (TeX Live 2026, MacTeX), `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two
//! passes. The committed TeX Live 2025 `reference.pdf` of the probe gives the
//! identical word list. pdflatex is an oracle only, never in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// The corpus harness's own word tolerance.
const TOL: f64 = 0.5;

/// A word pdfLaTeX set: text, x, baseline, and whether its face is bold
/// (`SFBX*`) or italic (`SFTI*`).
type Word = (&'static str, f64, f64, Face);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Face {
    Regular,
    Bold,
    Italic,
}

/// The words of page 1 in reading order (the page number excluded), with the
/// face each glyph run was painted in.
fn words(text: &str) -> Vec<(String, f64, f64, Face)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "macro-argument-fonts", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        let Some(g) = run.glyphs.first() else { continue };
        let face = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).map(|f| f.postscript_name.clone()).unwrap_or_default();
        let face = if face.contains("Bold") {
            Face::Bold
        } else if face.contains("Italic") {
            Face::Italic
        } else {
            Face::Regular
        };
        let baseline = g.baseline_y.to_bp();
        // The folio sits in the page foot. A comma right after a group
        // is its own run here (it is outside the bold group) and part of
        // the preceding word in pdftext's reading of the reference.
        if baseline > 700.0 || run.text.trim() == "," {
            continue;
        }
        out.push((run.text.trim().to_string(), g.origin_x.to_bp(), baseline, face));
    }
    out
}

fn check(name: &str, tex: &str, expected: &[Word]) {
    let got = words(tex);
    let texts: Vec<&str> = got.iter().map(|w| w.0.as_str()).collect();
    assert_eq!(texts, expected.iter().map(|w| w.0).collect::<Vec<_>>(), "{name}: the words differ");
    for (g, &(text, x, y, face)) in got.iter().zip(expected) {
        assert_eq!(g.3, face, "{name}: `{text}` painted {:?}, pdflatex sets it {face:?}", g.3);
        assert!(
            (g.1 - x).abs() <= TOL && (g.2 - y).abs() <= TOL,
            "{name}: `{text}` at ({:.3}, {:.3}) bp, pdflatex ({x:.3}, {y:.3}) (dx {:+.3}, dy {:+.3})",
            g.1,
            g.2,
            g.1 - x,
            g.2 - y
        );
    }
}

const PREAMBLE_SWITCHES: &str = r"\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\newcommand{\note}[1]{{\small\bfseries #1}}
\renewcommand{\baselinestretch}{1.0}
\begin{document}

Body text and \note{a note set through a macro that uses the size and series
switches in its definition}, followed by more body text.
\end{document}
";

use Face::{Bold as B, Italic as I, Regular as R};

/// `fixtures/divergence-probes/min2-preamble-switches` verbatim.
#[test]
fn a_size_and_series_switch_in_a_macro_body_sets_the_argument_bold() {
    if !common::lm_available() {
        return;
    }
    check(
        "min2-preamble-switches",
        PREAMBLE_SWITCHES,
        &[
            ("Body", 86.944, 81.963, R),
            ("text", 113.755, 81.963, R),
            ("and", 134.892, 81.963, R),
            ("a", 154.642, 81.963, B),
            ("note", 163.744, 81.963, B),
            ("set", 187.878, 81.963, B),
            ("through", 205.008, 81.963, B),
            ("a", 245.740, 81.963, B),
            ("macro", 254.841, 81.963, B),
            ("that", 287.177, 81.963, B),
            ("uses", 310.422, 81.963, B),
            ("the", 333.504, 81.963, B),
            ("size", 352.334, 81.963, B),
            ("and", 372.991, 81.963, B),
            ("series", 393.878, 81.963, B),
            ("switches", 423.256, 81.963, B),
            ("in", 465.479, 81.963, B),
            ("its", 478.272, 81.963, B),
            ("definition", 493.473, 81.963, B),
            ("followed", 72.000, 93.918, R),
            ("by", 110.460, 93.918, R),
            ("more", 124.289, 93.918, R),
            ("body", 149.224, 93.918, R),
            ("text.", 174.394, 93.918, R),
        ],
    );
}

/// An ungrouped declaration in the body stays in force after the call, as
/// it would written at the call site.
#[test]
fn an_ungrouped_declaration_in_a_macro_body_outlives_the_call() {
    if !common::lm_available() {
        return;
    }
    check(
        "ungrouped",
        r"\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\newcommand{\note}[1]{\small\bfseries #1}
\begin{document}

Body text and \note{a note set through a macro} more body text.
\end{document}
",
        &[
            ("Body", 86.944, 81.963, R),
            ("text", 113.367, 81.963, R),
            ("and", 134.115, 81.963, R),
            ("a", 153.479, 81.963, B),
            ("note", 162.178, 81.963, B),
            ("set", 185.899, 81.963, B),
            ("through", 202.617, 81.963, B),
            ("a", 242.936, 81.963, B),
            ("macro", 251.625, 81.963, B),
            ("more", 283.558, 81.963, B),
            ("body", 310.476, 81.963, B),
            ("text.", 337.289, 81.963, B),
        ],
    );
}

/// Two arguments in two faces, with replacement text between them: the
/// body space before `#2` is in the call site's font, not the italic of the
/// first argument the source bytes in between belong to.
#[test]
fn each_argument_takes_the_declarations_around_its_own_parameter() {
    if !common::lm_available() {
        return;
    }
    check(
        "two arguments",
        r"\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\newcommand{\pair}[2]{\textit{#1} and \textbf{#2}}
\begin{document}

Body text and \pair{first words}{second words} more body text.
\end{document}
",
        &[
            ("Body", 86.944, 81.963, R),
            ("text", 113.367, 81.963, R),
            ("and", 134.115, 81.963, R),
            ("first", 153.479, 81.963, I),
            ("words", 174.227, 81.963, I),
            ("and", 202.927, 81.963, R),
            ("second", 222.301, 81.963, B),
            ("words", 259.429, 81.963, B),
            ("more", 292.027, 81.963, R),
            ("body", 316.953, 81.963, R),
            ("text.", 342.132, 81.963, R),
        ],
    );
}
