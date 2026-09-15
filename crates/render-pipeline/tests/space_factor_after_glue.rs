//! Explicit glue does not reset the space factor.
//!
//! TeX sets `\spacefactor` only when it appends a character, a box in
//! horizontal mode, a rule or math (tex.web §1034-§1044). `\hfill`, `\quad`
//! (`\hskip1em\relax`), the leaders of `\hrulefill`/`\dotfill`, their closing
//! `\kern\z@` and the `\leavevmode` (`\unhbox` of a void box) leave it where
//! the last character put it. So in `Name: \hrulefill{} Date:` the blank after
//! `{}` still follows the colon's 2000 and gets `\fontdimen7` (1.11111 pt in
//! `ecrm1000`) as well as `\fontdimen2`. The pipeline reset the factor to 1000
//! after every glue inline, so that blank was 1.111 pt short and, with the two
//! fills sharing the leftover width, `Date:` landed 0.554 bp left of pdfLaTeX
//! in `fixtures/divergence-probes/min-hrulefill`.
//!
//! ## Oracle
//!
//! Word origins (first glyph x, baseline y from the page top, bp) read by
//! `tools/visual-oracle/pdftext.py` from pdfLaTeX's output for each document:
//! pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, MacTeX),
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two passes. The committed
//! TeX Live 2025 `reference.pdf` of `min-hrulefill` puts `Date:` at the same
//! 317.830. pdflatex is an oracle only, never in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// The corpus harness's own word tolerance; the bug was 0.554 bp.
const TOL: f64 = 0.1;

fn probe(line: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage[margin=1in]{{geometry}}\n\\begin{{document}}\n\n{line}\n\nA second line of ordinary text so the baseline below the rule can be compared.\n\\end{{document}}\n"
    )
}

/// x of the first glyph of the first run on the first line whose text is `word`.
fn x_of(text: &str, word: &str) -> f64 {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "space-factor-after-glue", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.trim() == word {
            if let Some(g) = run.glyphs.first() {
                assert!((g.baseline_y.to_bp() - 81.963).abs() <= TOL, "`{word}` is not on the first line");
                return g.origin_x.to_bp();
            }
        }
    }
    panic!("no glyph run `{word}` on page 1");
}

/// (line, word, pdflatex x in bp). Every case starts `Name:` at 86.944.
const CASES: &[(&str, &str, f64)] = &[
    // `fixtures/divergence-probes/min-hrulefill` verbatim (was 317.276).
    (r"Name: \hrulefill{} Date: \hrulefill", "Date:", 317.830),
    // The same through the dot leaders (was 0.282 bp off).
    (r"Name: \dotfill{} Date: \dotfill", "Date:", 317.833),
    // `\quad` is `\hskip1em\relax`: the `.`'s 3000 survives it too (was
    // 2.217 bp off at `x`).
    (r"Name:\quad{} Date. \quad{} x", "Date.", 129.273),
    (r"Name:\quad{} Date. \quad{} x", "x", 171.748),
];

#[test]
fn the_space_factor_survives_explicit_glue() {
    if !common::lm_available() {
        return;
    }
    for &(line, word, expected) in CASES {
        let tex = probe(line);
        assert!((x_of(&tex, "Name:") - 86.944).abs() <= TOL);
        let got = x_of(&tex, word);
        assert!(
            (got - expected).abs() <= TOL,
            "{line}: `{word}` at x {got:.3} bp, pdflatex {expected:.3} bp (off by {:+.3} bp)",
            got - expected
        );
    }
}
