//! `\makebox[<w>][<pos>]`, `\llap`, `\rlap` (compiler `HBox::width_pt` and
//! `align`) and a vertical-mode `\hrule` with a rule spec (compiler
//! `Block::Rule` width/height/depth): the renderer halves that came with
//! algorithm2e's compiler model.
//!
//! ## Oracle
//!
//! Every x is a word origin (first glyph, bp) read by
//! `tools/visual-oracle/pdftext.py` from pdfLaTeX's output for the same
//! document: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, MacTeX). The
//! rule is pdflatex's `re` operand (85.039 x 2.989 bp). pdflatex is an
//! oracle only, never in the product path.

mod common;

use flashtex_render_pipeline::display::Item;

/// The corpus harness's exact-route tolerance for a word origin.
const TOL: f64 = 0.01;

const DOC: &str = "\\documentclass{article}\n\
\\begin{document}\n\
Alpha\\llap{Lft}Beta and Gamma\\rlap{Rgt}Delta.\n\n\
\\makebox[4cm][l]{Ell}Next1\n\n\
\\makebox[4cm][r]{Arr}Next2\n\n\
\\makebox[4cm]{Cee}Next3\n\n\
\\makebox[4cm][s]{Ess\\hfill Tee}Next4\n\n\
\\makebox[1cm][c]{Overfullwidetext}Next5\n\n\
\\hrule width 3cm height 2pt depth 1pt\n\
After rule text.\n\
\\end{document}\n";

#[test]
fn fixed_width_boxes_place_their_content_like_pdflatex() {
    if !common::lm_available() {
        return;
    }
    let r = common::render_one(DOC);
    assert_eq!(r.v2.pages.len(), 1);
    let mut runs = Vec::new();
    let mut rules = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        match item {
            Item::GlyphRun(run) => {
                if let Some(g) = run.glyphs.first() {
                    runs.push((run.text.trim().to_string(), g.origin_x.to_bp()));
                }
            }
            Item::Rule(rule) => rules.push((rule.width.to_bp(), rule.height.to_bp())),
            _ => {}
        }
    }
    let x_of = |word: &str| {
        runs.iter()
            .find(|(text, _)| text.starts_with(word))
            .map(|(_, x)| *x)
            .unwrap_or_else(|| panic!("no run starting {word}: {runs:?}"))
    };
    for (word, want) in [
        // `\llap`: the text ends where `Alpha` ends; `\rlap` takes no width.
        ("Lft", 161.853),
        ("Delta.", 252.419),
        ("Ell", 148.712),
        ("Next1", 262.098),
        ("Arr", 246.822),
        ("Cee", 197.380),
        ("Next3", 262.097),
        ("Ess", 148.712),
        // `s`: the `\hfill` takes the excess.
        ("Tee", 246.877),
        // Overfull and centred: the `\hss` on both sides shrink.
        ("Overfullwidetext", 126.619),
        ("Next5", 177.055),
    ] {
        let x = x_of(word);
        assert!((x - want).abs() <= TOL, "{word}: x {x} vs pdflatex {want} ({runs:?})");
    }
    assert!(
        rules.iter().any(|(w, h)| (w - 85.039).abs() <= TOL && (h - 2.989).abs() <= TOL),
        "no 85.039 x 2.989 rule: {rules:?}"
    );
}
