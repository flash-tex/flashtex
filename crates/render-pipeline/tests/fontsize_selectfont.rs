//! NFSS `\fontsize{<size>}{<skip>}\selectfont` sets its exact size and
//! baselineskip (the compiler's `FontSizeLevel::Explicit`), with the font
//! the family's `.fd` loads for that size. Expected positions are
//! pdflatex's (TeX Live 2026, MacTeX; oracle only), read with
//! `tools/visual-oracle/pdftext.py`: word origin x and baseline y, in bp.
//!
//! - OT1 `cmr` has no 13pt: LaTeX substitutes `cmr12` (11.955 bp glyphs),
//!   and the paragraph's lines are 15pt (14.944 bp) apart.
//! - `lmodern` in T1 scales: `ec-lmr12` at 13pt (12.951 bp).

mod common;

use flashtex_render_pipeline::display::Item;

const PARA: &str = "{\\fontsize{13}{15}\\selectfont Some larger words that run on for a while so that the paragraph wraps over at least two lines of the page here.\\par}";

/// `(text, x, baseline, font size)` of every glyph run on page 1, in bp.
fn runs(preamble: &str) -> Vec<(String, f64, f64, f64)> {
    runs_of(&format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\nBefore text here.\n\n{PARA}\n\nAfter text here.\n\\end{{document}}\n"))
}

fn runs_of(source: &str) -> Vec<(String, f64, f64, f64)> {
    let r = common::render_docs(&[("main.tex", source)], "main.tex");
    assert!(r.v2.diagnostics.is_empty(), "{:?}", r.v2.diagnostics);
    r.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|item| match item {
            Item::GlyphRun(run) => run.glyphs.first().map(|g| (run.text.clone(), g.origin_x.to_bp(), g.baseline_y.to_bp(), run.font_size.to_bp())),
            _ => None,
        })
        .collect()
}

fn check(runs: &[(String, f64, f64, f64)], word: &str, x: f64, y: f64, size: f64) {
    let (_, gx, gy, gs) = runs.iter().find(|(t, ..)| t == word).unwrap_or_else(|| panic!("no run {word:?} in {runs:?}"));
    assert!((gx - x).abs() < 0.01 && (gy - y).abs() < 0.01, "{word}: ({gx:.3}, {gy:.3}), pdflatex ({x:.3}, {y:.3})");
    assert!((gs - size).abs() < 0.01, "{word}: size {gs:.3} bp, pdflatex {size:.3}");
}

#[test]
fn ot1_cmr_substitutes_twelve_point_at_a_fifteen_point_leading() {
    assert!(common::lm_available());
    let runs = runs("");
    check(&runs, "Some", 148.712, 149.709, 11.955);
    check(&runs, "paragraph", 425.449, 149.709, 11.955);
    check(&runs, "wraps", 133.768, 164.653, 11.955);
    check(&runs, "After", 148.712, 176.608, 9.963);
}

#[test]
fn latin_modern_sets_the_exact_thirteen_points() {
    assert!(common::lm_available());
    let runs = runs("\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n");
    check(&runs, "Some", 148.712, 149.709, 12.951);
    check(&runs, "least", 249.030, 164.653, 12.951);
    check(&runs, "page", 367.880, 164.653, 12.951);
}

/// `\@sect` ends the title with `#8\@@par` inside the heading's group, so a
/// size the title selects sets the heading's `\baselineskip` (15 pt here,
/// not `\Large`'s 18 pt; `\small`'s 11 pt). `\@hangfrom` hangs the later
/// lines by the number box (`1\quad` in `cmbx12` at 14.4 pt).
#[test]
fn a_size_selected_in_a_section_title_sets_its_leading() {
    assert!(common::lm_available());
    let runs = runs_of(
        "\\documentclass{article}\n\\begin{document}\nBefore text here.\n\\section{\\fontsize{13}{15}\\selectfont Head that is long enough to wrap onto two lines of the page with more words here to go}\nAfter text here.\n\\section{\\small Small head that is long enough to wrap onto two lines of the page with more words here to go on}\nAfter small.\n\\section{Plain head}\nTail.\n\\end{document}\n",
    );
    check(&runs, "Head", 157.977, 164.722, 11.955);
    check(&runs, "page", 157.978, 179.666, 11.955);
    check(&runs, "After", 133.768, 201.487, 9.963);
    check(&runs, "Small", 157.977, 227.459, 8.966);
    check(&runs, "on", 290.396, 238.418, 8.966);
    check(&runs, "small.", 159.806, 260.238, 9.963);
    check(&runs, "Tail.", 133.768, 315.005, 9.963);
}

/// `{\LARGE \@title \par}`: the title's lines are set 24 pt apart under
/// `\fontsize{20}{24}\selectfont` (`cmr17` at 20.74 pt), not `\LARGE`'s 22.
#[test]
fn a_size_selected_in_the_title_sets_its_leading() {
    assert!(common::lm_available());
    let runs = runs_of(
        "\\documentclass{article}\n\\title{\\fontsize{20}{24}\\selectfont A title that is long enough to wrap over two lines of the page here}\n\\author{Someone}\n\\date{}\n\\begin{document}\n\\maketitle\nBody text here.\n\\end{document}\n",
    );
    check(&runs, "A", 140.338, 178.600, 20.659);
    check(&runs, "two", 199.921, 202.511, 20.659);
    check(&runs, "Someone", 283.187, 231.402, 11.955);
    check(&runs, "Body", 148.712, 268.264, 9.963);
}
