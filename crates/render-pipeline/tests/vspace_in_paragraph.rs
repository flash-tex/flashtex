//! `\vspace` inside a paragraph is latex.ltx's
//! `\@bsphack\vadjust{\vskip<glue>}\@esphack`: the paragraph goes on and the
//! glue lands below the line the command is set on.
//!
//! ## Oracle
//!
//! Word origins of the reference PDFs for the two probes below (pdfTeX,
//! TeX Live 2026, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`), relative to
//! the first word. pdflatex is an oracle only and never runs in the product
//! path.
//!
//! * [`MID`]: `in` is on the first line, 49.188bp right of `Some`; the
//!   second line (`second`) is 21.917bp below it (12pt `\baselineskip` plus
//!   the 10pt). Breaking the paragraph at the command put `in` at the start
//!   of a line of its own, 47.71bp left of pdflatex's.
//! * [`AFTER_NEWLINE`]: `\\`'s `\@ifnextchar[` takes the space before
//!   `\vspace`, so the space after it is glue *behind* the adjust node,
//!   which is not discarded at the break: `second` is 3.321bp (one
//!   interword space) right of `First`, 11.955bp (12pt) below it, and the
//!   10pt lands below `second`'s line: `Third` is 21.918bp below `second`.
//!
//! Needs the compiler's `parser::Inline::VAdjustSkip`, so the whole file is
//! behind the `vadjust-skip` feature (see `Cargo.toml`): the pinned
//! `vendor/compiler` predates that node and still breaks the paragraph.
#![cfg(feature = "vadjust-skip")]

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const TOL: f64 = 0.05;

const MID: &str = r"\documentclass{article}
\pagestyle{empty}
\begin{document}
\noindent Some text \vspace{10pt} in the middle of a paragraph that goes on long enough to
need a second line, which starts with the word Second after the break.
\end{document}
";

const AFTER_NEWLINE: &str = r"\documentclass{article}
\pagestyle{empty}
\begin{document}
\noindent First\\ \vspace{10pt} second

Third
\end{document}
";

/// `(x, baseline)` in bp of the first glyph run whose text is `word`.
fn origin(text: &str, word: &str) -> (f64, f64) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "vspace-in-paragraph", &fonts, &RenderOptions::default());
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        if run.text == word {
            if let Some(g) = run.glyphs.first() {
                return (g.origin_x.to_bp(), g.baseline_y.to_bp());
            }
        }
    }
    panic!("no glyph run `{word}` on page 1");
}

fn assert_near(what: &str, got: f64, want: f64) {
    assert!((got - want).abs() <= TOL, "{what}: {got:.3} bp, pdflatex {want:.3} bp");
}

#[test]
fn vspace_mid_paragraph_keeps_the_line_and_adds_space_below_it() {
    if !common::lm_available() {
        eprintln!("SKIP vspace_mid_paragraph_keeps_the_line_and_adds_space_below_it: Latin Modern not installed");
        return;
    }
    let some = origin(MID, "Some");
    let within = origin(MID, "in");
    let second = origin(MID, "second");
    assert_near("`in` baseline - `Some` baseline", within.1 - some.1, 0.0);
    assert_near("`in` x - `Some` x", within.0 - some.0, 49.188);
    assert_near("second line - first line", second.1 - some.1, 21.917);
}

#[test]
fn vspace_after_a_line_break_goes_below_the_next_line() {
    if !common::lm_available() {
        eprintln!("SKIP vspace_after_a_line_break_goes_below_the_next_line: Latin Modern not installed");
        return;
    }
    let first = origin(AFTER_NEWLINE, "First");
    let second = origin(AFTER_NEWLINE, "second");
    let third = origin(AFTER_NEWLINE, "Third");
    assert_near("`second` x - `First` x", second.0 - first.0, 3.321);
    assert_near("`second` - `First` baselines", second.1 - first.1, 11.955);
    assert_near("`Third` - `second` baselines", third.1 - second.1, 21.918);
}
