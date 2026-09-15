//! Text written inside a user macro's body is set in the body's own fonts,
//! and a font *declaration* group never takes an italic correction.
//!
//! 1. The compiler spans every token of a replacement text at the
//!    invocation (`\x`), and the pipeline read the font from the source brace
//!    groups at that span, so `\newcommand{\x}{\textbf{Note:}}` set `Note:`
//!    in the call site's medium face, 4.07 bp narrower than pdfLaTeX's
//!    `SFBX1000` for the rest of the line. The body's own groups around the
//!    token now apply on top of the call site's font
//!    (`adapter::Styles::in_body`), and a blank of the body is set in the
//!    body's font there.
//! 2. `{\itshape leaf} then` got a `\/` after `leaf`. LaTeX adds italic
//!    correction only through `\text@command`'s `\maybe@ic` (`\textit`,
//!    `\emph`, ...), and not before a `\nocorrlist` token (`.` and `,`);
//!    a bare declaration group adds none.
//!
//! ## Oracle
//!
//! Every number below is the origin of the first glyph of a same-font run
//! (x, baseline from the page top, bp) read with
//! `tools/visual-oracle/pdftext.py`'s content-stream replay from pdfLaTeX's
//! output for the document in the same test: pdfTeX 3.141592653-2.6-1.40.29
//! (TeX Live 2026, MacTeX), `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two
//! passes. pdflatex is an oracle only, never in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// The corpus harness's own word tolerance. Every run below is within
/// 0.005 bp of pdfLaTeX; an italic correction is 0.5-2.1 bp.
const TOL: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Face {
    Regular,
    Bold,
    Italic,
    Slanted,
}

use Face::{Bold as B, Italic as I, Regular as R, Slanted as S};

type Run = (&'static str, f64, f64, Face);

/// The glyph runs of page 1 in reading order (the folio excluded), split
/// at blanks, as (text, x, baseline, face).
fn runs(text: &str) -> Vec<(String, f64, f64, Face)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "macro-body-fonts", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut out = Vec::new();
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        let Some(g) = run.glyphs.first() else { continue };
        let name = r.v2.fonts.iter().find(|f| f.font_id == run.font_id).map(|f| f.postscript_name.clone()).unwrap_or_default();
        let face = if name.contains("Bold") {
            B
        } else if name.contains("Italic") {
            I
        } else if name.contains("Slant") {
            S
        } else {
            R
        };
        let baseline = g.baseline_y.to_bp();
        if baseline > 700.0 {
            continue;
        }
        out.push((run.text.trim().to_string(), g.origin_x.to_bp(), baseline, face));
    }
    out
}

fn check(name: &str, tex: &str, expected: &[Run]) {
    let got = runs(tex);
    let texts: Vec<&str> = got.iter().map(|w| w.0.as_str()).collect();
    assert_eq!(texts, expected.iter().map(|w| w.0).collect::<Vec<_>>(), "{name}: the runs differ");
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

/// Literal words of a body inside `\textbf`, `{\itshape ...}`,
/// `{\bfseries ...}` and `\textit`, with an argument between them. Before:
/// `followed` 4.06 bp left of pdfLaTeX, `with` 6.76 bp.
#[test]
fn literal_text_in_a_macro_body_takes_the_body_fonts() {
    if !common::lm_available() {
        return;
    }
    check(
        "body text",
        r"\documentclass{article}
\usepackage[T1]{fontenc}
\newcommand{\x}{\textbf{Note:}}
\newcommand{\y}{{\itshape Remark} and {\bfseries Bold words}}
\newcommand{\z}[1]{\textit{Hint:} #1 done}
\begin{document}
Body text \x{} followed by more body text.

Also \y{} with trailing upright words here.

And \z{an argument} with more upright words after it.
\end{document}
",
        &[
            ("Body", 148.712, 134.765, R),
            ("text", 175.135, 134.765, R),
            ("Note:", 195.883, 134.765, B),
            ("followed", 227.885, 134.765, R),
            ("by", 266.344, 134.765, R),
            ("more", 280.183, 134.765, R),
            ("body", 305.109, 134.765, R),
            ("text.", 330.288, 134.765, R),
            ("Also", 148.712, 146.720, R),
            ("Remark", 171.175, 146.720, I),
            ("and", 207.850, 146.720, R),
            ("Bold", 227.224, 146.720, B),
            ("words", 254.461, 146.720, B),
            ("with", 287.059, 146.720, R),
            ("trailing", 309.742, 146.720, R),
            ("upright", 344.638, 146.720, R),
            ("words", 379.798, 146.720, R),
            ("here.", 408.383, 146.720, R),
            ("And", 148.712, 158.675, R),
            ("Hint:", 170.566, 158.675, I),
            ("an", 197.999, 158.675, R),
            ("argument", 211.830, 158.675, R),
            ("done", 256.407, 158.675, R),
            ("with", 280.197, 158.675, R),
            ("more", 302.881, 158.675, R),
            ("upright", 327.817, 158.675, R),
            ("words", 362.977, 158.675, R),
            ("after", 391.561, 158.675, R),
            ("it.", 415.104, 158.675, R),
        ],
    );
}

/// `{\itshape ...}`, `{\em ...}` and `{\bfseries ...}` add no italic
/// correction. Before: `then` 2.11 bp right of pdfLaTeX, `after.` 4.22 bp.
#[test]
fn a_declaration_group_adds_no_italic_correction() {
    if !common::lm_available() {
        return;
    }
    check(
        "declaration groups",
        r"\documentclass{article}
\usepackage[T1]{fontenc}
\begin{document}
Upright {\itshape leaf} then upright text.

Upright {\itshape f}b more words and {\em ff} after.

Bold {\bfseries f} then more words to see a shift.
\end{document}
",
        &[
            ("Upright", 148.712, 134.765, R),
            ("leaf", 185.819, 134.765, I),
            ("then", 203.901, 134.765, R),
            ("upright", 226.585, 134.765, R),
            ("text.", 261.745, 134.765, R),
            ("Upright", 148.712, 146.720, R),
            ("f", 185.819, 146.720, I),
            ("b", 188.873, 146.720, R),
            ("more", 197.724, 146.720, R),
            ("words", 222.649, 146.720, R),
            ("and", 251.234, 146.720, R),
            ("ff", 270.598, 146.720, I),
            ("after.", 280.025, 146.720, R),
            ("Bold", 148.712, 158.675, R),
            ("f", 172.365, 158.675, B),
            ("then", 179.193, 158.675, R),
            ("more", 201.876, 158.675, R),
            ("words", 226.802, 158.675, R),
            ("to", 255.387, 158.675, R),
            ("see", 267.558, 158.675, R),
            ("a", 283.656, 158.675, R),
            ("shift.", 291.964, 158.675, R),
        ],
    );
}

/// `\textit` and `\emph` do add `\/`, except before `.` and `,`
/// (`\nocorrlist`): `\emph{f}b` puts `b` 5.17 bp after `f`'s origin, and
/// `\emph{f}.` puts the period 3.05 bp after it.
#[test]
fn textit_and_emph_correct_except_before_nocorrlist() {
    if !common::lm_available() {
        return;
    }
    check(
        "text commands",
        r"\documentclass{article}
\usepackage[T1]{fontenc}
\begin{document}
Upright \textit{leaf}. then text and \textit{leaf}b then text.

Upright \textit{f}, then text and \textit{f} then text.

Upright \emph{f}. then text and \emph{f}b then text and \emph{f} x.
\end{document}
",
        &[
            ("Upright", 148.712, 134.765, R),
            ("leaf", 185.819, 134.765, I),
            (".", 200.573, 134.765, R),
            ("then", 207.773, 134.765, R),
            ("text", 230.457, 134.765, R),
            ("and", 251.205, 134.765, R),
            ("leaf", 270.580, 134.765, I),
            ("b", 287.446, 134.765, R),
            ("then", 296.307, 134.765, R),
            ("text.", 318.991, 134.765, R),
            ("Upright", 148.712, 146.720, R),
            ("f", 185.819, 146.720, I),
            (",", 188.873, 146.720, R),
            ("then", 194.957, 146.720, R),
            ("text", 217.641, 146.720, R),
            ("and", 238.389, 146.720, R),
            ("f", 257.764, 146.720, I),
            ("then", 266.248, 146.720, R),
            ("text.", 288.932, 146.720, R),
            ("Upright", 148.712, 158.675, R),
            ("f", 185.819, 158.675, I),
            (".", 188.873, 158.675, R),
            ("then", 196.063, 158.675, R),
            ("text", 218.747, 158.675, R),
            ("and", 239.495, 158.675, R),
            ("f", 258.869, 158.675, I),
            ("b", 264.036, 158.675, R),
            ("then", 272.887, 158.675, R),
            ("text", 295.571, 158.675, R),
            ("and", 316.319, 158.675, R),
            ("f", 335.693, 158.675, I),
            ("x.", 344.177, 158.675, R),
        ],
    );
}

/// The same rules inside a macro body: `\textit{leaf}` closing the body
/// corrects before upright text, and not before a `.` after the call;
/// `\textsl{half}b` corrects before the body's next byte; a declaration
/// group in the body (`{\itshape Wolf}`) does not. Before: line 2 was up to
/// 9.44 bp off.
#[test]
fn italic_correction_inside_a_macro_body() {
    if !common::lm_available() {
        return;
    }
    check(
        "body corrections",
        r"\documentclass{article}
\usepackage[T1]{fontenc}
\newcommand{\x}{\textit{leaf}}
\newcommand{\vv}[1]{\emph{elf} #1 \textbf{bold words} upright}
\newcommand{\uu}{{\itshape Wolf} upright \textsl{half}b}
\begin{document}
One \x{} upright then words and \x. more words follow here.

Two \vv{argument} with more words.

Three \uu{} and more upright text and \emph{f}{} x done.
\end{document}
",
        &[
            ("One", 148.712, 134.765, R),
            ("leaf", 169.736, 134.765, I),
            ("upright", 189.930, 134.765, R),
            ("then", 225.100, 134.765, R),
            ("words", 247.784, 134.765, R),
            ("and", 276.369, 134.765, R),
            ("leaf", 295.733, 134.765, I),
            (".", 310.498, 134.765, R),
            ("more", 317.688, 134.765, R),
            ("words", 342.613, 134.765, R),
            ("follow", 371.198, 134.765, R),
            ("here.", 399.977, 134.765, R),
            ("Two", 148.712, 146.720, R),
            ("elf", 170.290, 146.720, I),
            ("argument", 185.901, 146.720, R),
            ("bold", 230.478, 146.720, B),
            ("words", 256.248, 146.720, B),
            ("upright", 288.846, 146.720, R),
            ("with", 324.016, 146.720, R),
            ("more", 346.700, 146.720, R),
            ("words.", 371.625, 146.720, R),
            ("Three", 148.712, 158.675, R),
            ("Wolf", 177.510, 158.675, I),
            ("upright", 200.710, 158.675, R),
            ("half", 235.880, 158.675, S),
            ("b", 254.126, 158.675, R),
            ("and", 262.977, 158.675, R),
            ("more", 282.351, 158.675, R),
            ("upright", 307.277, 158.675, R),
            ("text", 342.437, 158.675, R),
            ("and", 363.195, 158.675, R),
            ("f", 382.559, 158.675, I),
            ("x", 391.043, 158.675, R),
            ("done.", 399.618, 158.675, R),
        ],
    );
}
