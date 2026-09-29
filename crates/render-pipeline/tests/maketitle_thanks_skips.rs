//! `\maketitle` issues `\@thanks` (the `\footnotetext`s of every
//! `\thanks`) in vertical mode after `\@maketitle`'s closing
//! `\vskip 1.5em`. The footnote inserts end the vertical list, so
//! `\lastskip` is 0 at the next `\addvspace`: an abstract's, a list's or a
//! theorem's `\@topsep` and a heading's before-skip are added whole, where
//! without a `\thanks` the 1.5em absorbs them.
//!
//! Oracle: pdflatex 1.40.29 (TeX Live 2026, `SOURCE_DATE_EPOCH=0`), whole
//! documents (`article` 11pt, `geometry` `margin=1in`, amsmath, amssymb,
//! amsthm, `\newtheorem{problem}{Problem}`, title, author and date),
//! baselines read with pymupdf.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// `(first word, baseline)` of every glyph run, in bp, or `None` when
/// Latin Modern is missing.
fn baselines(thanks: &str, body: &str) -> Option<Vec<(String, f64)>> {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return None;
    }
    let src = format!(
        "\\documentclass[11pt]{{article}}\n\\usepackage[margin=1in]{{geometry}}\n\\usepackage{{amsmath,amssymb,amsthm}}\n\\newtheorem{{problem}}{{Problem}}\n\\title{{Real Analysis}}\n\\author{{Jordan Lee{thanks}}}\n\\date{{Due October 3, 2026}}\n\\begin{{document}}\n\\maketitle\n{body}\n\\end{{document}}\n"
    );
    let docs = [SourceDocument { path: "main.tex", text: &src }];
    let out = render(&docs, "main.tex", 1, "maketitle-thanks", &fonts, &RenderOptions::default());
    assert_eq!(out.v2.pages.len(), 1);
    Some(
        out.v2.pages[0]
            .resident_items()
            .iter()
            .filter_map(|i| match i {
                Item::GlyphRun(r) => r.glyphs.first().map(|g| (r.text.clone(), g.baseline_y.to_bp())),
                _ => None,
            })
            .collect(),
    )
}

fn y(runs: &[(String, f64)], word: &str) -> f64 {
    runs.iter().find(|(t, _)| t == word).unwrap_or_else(|| panic!("no run {word:?} in {runs:?}")).1
}

fn near(what: &str, got: f64, want: f64) {
    assert!((got - want).abs() <= 0.05, "{what}: FlashTeX {got:.3}bp, pdflatex {want:.3}bp");
}

const THANKS: &str = "\\thanks{Collaborated.}";

#[test]
fn section_after_a_thanks_title_keeps_its_whole_before_skip() {
    let Some(r) = baselines(THANKS, "\\section{Intro}\nAfter text.") else { return };
    near("Intro", y(&r, "Intro"), 244.156);
    near("After", y(&r, "After"), 268.508);
    let Some(r) = baselines("", "\\section{Intro}\nAfter text.") else { return };
    near("Intro without thanks", y(&r, "Intro"), 227.792);
}

#[test]
fn abstract_after_a_thanks_title_keeps_its_topsep() {
    let body = "\\begin{abstract}\nAbstract words.\n\\end{abstract}\n\nAfter text.";
    let Some(r) = baselines(THANKS, body) else { return };
    near("Abstract head", y(&r, "Abstract"), 233.694);
    near("After", y(&r, "After"), 274.391);
    let Some(r) = baselines("", body) else { return };
    near("Abstract head without thanks", y(&r, "Abstract"), 221.739);
    near("After without thanks", y(&r, "After"), 262.436);
}

#[test]
fn theorem_and_list_after_a_thanks_title() {
    let thm = "\\begin{problem}\nWords here.\n\\end{problem}\n\nAfter text.";
    let Some(r) = baselines(THANKS, thm) else { return };
    near("Problem", y(&r, "Problem"), 232.299);
    near("After", y(&r, "After"), 254.815);
    let Some(r) = baselines("", thm) else { return };
    near("Problem without thanks", y(&r, "Problem"), 223.333);
    let Some(r) = baselines(THANKS, "\\begin{itemize}\n\\item One item\n\\end{itemize}\nAfter text.") else { return };
    near("One", y(&r, "One"), 235.288);
    near("After list", y(&r, "After"), 260.792);
    // A paragraph adds no `\addvspace`: nothing changes.
    let Some(r) = baselines(THANKS, "Words here.\n\nAfter text.") else { return };
    near("Words", y(&r, "Words"), 223.333);
}
