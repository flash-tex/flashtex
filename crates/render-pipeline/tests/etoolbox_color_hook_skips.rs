//! `\AtBeginEnvironment{example}{\color{black!80}}` (etoolbox) runs
//! `\color` inside the environment's group. In vertical mode that is a
//! `\pdfcolorstack` whatsit before the `\trivlist`'s `\addvspace\@topsep`,
//! and its `\aftergroup\reset@color` is another whatsit right after
//! `\end{example}`. Each hides the skip before it from the next
//! `\addvspace` (`\lastskip` = 0), so the two skips add instead of the
//! larger winning: an `example` between two theorems sits 9pt lower, and
//! the theorem after it 18pt lower, than without the hook (v0.2.0 gate B8,
//! the `new-linalg-notes` document).
//!
//! Oracle: pdflatex 1.40.29 (TeX Live 2026, `SOURCE_DATE_EPOCH=0`), whole
//! documents (`article` 11pt, `geometry` `margin=1in`, amsthm, xcolor,
//! etoolbox, `\pagestyle{empty}`), baselines read with pymupdf.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const EX: &str = "\\begin{example}\nSample words here.\n\\end{example}\n";
const TH: &str = "\\begin{theorem}\nClaim words here.\n\\end{theorem}\n";

/// `(first word, baseline)` of every glyph run, in bp, or `None` when
/// Latin Modern is missing.
fn baselines(hook: &str, body: &str) -> Option<Vec<(String, f64)>> {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return None;
    }
    let src = format!(
        "\\documentclass[11pt]{{article}}\n\\usepackage[margin=1in]{{geometry}}\n\\usepackage{{amsmath,amsthm}}\n\\usepackage{{xcolor}}\n\\usepackage{{etoolbox}}\n\\newtheorem{{theorem}}{{Theorem}}\n\\newtheorem{{example}}[theorem]{{Example}}\n{hook}\n\\begin{{document}}\n\\pagestyle{{empty}}\n{body}\\end{{document}}\n"
    );
    let docs = [SourceDocument { path: "main.tex", text: &src }];
    let out = render(&docs, "main.tex", 1, "etoolbox-color-hook", &fonts, &RenderOptions::default());
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

/// The baselines of the runs whose text is `word`, in order.
fn ys(runs: &[(String, f64)], word: &str) -> Vec<f64> {
    runs.iter().filter(|(t, _)| t == word).map(|r| r.1).collect()
}

fn near(what: &str, got: f64, want: f64) {
    assert!((got - want).abs() <= 0.05, "{what}: FlashTeX {got:.3}bp, pdflatex {want:.3}bp");
}

#[test]
fn colour_hook_between_theorems_adds_both_skips() {
    let hook = "\\AtBeginEnvironment{example}{\\color{black!80}}";
    let Some(r) = baselines(hook, &format!("Above text.\n\n{TH}\n{EX}\n{TH}\nBody text here.\n")) else { return };
    // Before: Example 127.99, second Theorem 150.51, Body 173.02 (the
    // values without the hook).
    near("Example", ys(&r, "Example")[0], 136.956);
    near("second Theorem", ys(&r, "Theorem")[1], 168.438);
    near("Body", ys(&r, "Body")[0], 190.954);
}

#[test]
fn colour_hook_after_a_paragraph_changes_nothing() {
    // After a paragraph `\lastskip` is `\parskip` (0pt plus 1pt), which the
    // `\@topsep` beats either way: the whatsit makes no difference.
    let hook = "\\AtBeginEnvironment{example}{\\color{black!80}}";
    let Some(r) = baselines(hook, &format!("Above text.\n\n{EX}\nMiddle text.\n\n{EX}\nBody text here.\n")) else { return };
    let ex = ys(&r, "Example");
    near("first Example", ex[0], 105.474);
    near("Middle", ys(&r, "Middle")[0], 127.99);
    near("second Example", ex[1], 150.506);
    near("Body", ys(&r, "Body")[0], 173.021);
}

#[test]
fn colour_hook_on_center_between_theorems() {
    let hook = "\\AtBeginEnvironment{center}{\\color{red}}";
    let Some(r) =
        baselines(hook, &format!("Above text.\n\n{TH}\n\\begin{{center}}Centred words\\end{{center}}\n{TH}\nBody text here.\n")) else {
        return;
    };
    near("Centred", ys(&r, "Centred")[0], 139.945);
    near("second Theorem", ys(&r, "Theorem")[1], 174.416);
}

#[test]
fn without_a_colour_the_skips_still_merge() {
    // Controls: no hook, and a hook that sets no colour.
    for hook in ["", "\\AtBeginEnvironment{example}{\\relax}"] {
        let Some(r) = baselines(hook, &format!("Above text.\n\n{TH}\n{EX}\n{TH}\nBody text here.\n")) else { return };
        near(&format!("{hook:?} Example"), ys(&r, "Example")[0], 127.99);
        near(&format!("{hook:?} second Theorem"), ys(&r, "Theorem")[1], 150.506);
    }
}
