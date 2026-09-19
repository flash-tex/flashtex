//! Line breaks pinned to pdflatex's for four causes the visual-oracle ranking
//! of 2026-09-19 (`docs/evidence/visual-oracle-2026-09-19T210411Z`) put at
//! the top of the corpus: each was a paragraph where one item of the
//! horizontal list handed to the breaker differed from pdfTeX's and moved
//! exactly one break.
//!
//! * `min4-mapsto-rel`: `\mapsto` is `\mapstochar\rightarrow` (fontmath.ltx
//!   340-341), a relation whose box is cmsy "21's (1 em) with `\thickmuskip`
//!   either side. math-layout's `default_class` has no row for U+21A6, so the
//!   pipeline set an Ord from Latin Modern Math's 0.977 em glyph with no
//!   spaces: `$x \mapsto |x|$` 6.3 pt short, and `and` moved up a line
//!   (inline-math p.2, `\tracingparagraphs` @@21 t=3192 vs @@22 t=4467).
//! * `min4-small-math`: `\DeclareMathSizes` (fontmath.ltx 75-82) selects
//!   the math fonts of the current text size; the pipeline sized only the
//!   footnote of the 10pt/12pt classes and set every other size with the
//!   body's metrics. In the 11pt class the abstract (`\small`, 10 pt) and the
//!   notes (9 pt) ran 9.5% and 21.7% wide in every formula (lab-report p.1:
//!   `we` and `assumes` each a line late).
//! * `min4-lig-hyphen`: a Liang point inside a ligature (`traf-fic`, the
//!   `ffi` glyph) was dropped; pdfTeX breaks there with the discretionary
//!   its reconstitution builds (tex.web §903-918, `\discretionary replacing 1
//!   {f-}{fi}` before the `ffi` node). Without it the second pass ends the
//!   line at `par-` instead of `parti-` and every later line of the
//!   paragraph differs (conf-paper p.1).
//! * `min4-bib-sfcode`: `thebibliography` runs `\sloppy` and `\sfcode`\.\@m`
//!   after its `\list` (article.cls 576-580; natbib.sty 1074-1075), and
//!   `\newblock` is `\hskip .11em \@plus.33em \@minus.07em` (article.cls
//!   584) which the compiler lowers to a bare space token. Two sentence
//!   spaces (`\fontdimen7`, 1.3 pt each at 12 pt) that pdfTeX does not set
//!   pushed `Practice` to the next line (input-bibliography p.2, first pass
//!   @@1 b=99), and the missing `\newblock` glue moved every word after a
//!   block boundary (natbib-review p.3, thesis-chapter p.5).
//!
//! ## Oracle
//!
//! Word origins (first glyph x, baseline y from the page top, bp) read by
//! `tools/visual-oracle/pdftext.py` from each probe's committed
//! `reference.pdf`: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, MacTeX),
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, run to convergence by
//! `tools/real-world-corpus/run.py`. pdflatex is an oracle only, never in
//! the product path. Before the fixes, main `e4de97e9` put the words marked
//! `was` where noted.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// The corpus harness's own word tolerance; every reflow here was tens or
/// hundreds of bp, and the widths behind them 1.2-6.3 pt.
const TOL: f64 = 0.1;

fn probe_source(id: &str) -> String {
    let path = format!("{}/../../fixtures/divergence-probes/{id}/main.tex", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// `(x, baseline_y)` in bp of every glyph run on page 1 whose trimmed text
/// is `word`, in item order.
fn positions(text: &str, word: &str) -> Vec<(f64, f64)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "linebreak-probes", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.trim() == word {
            if let Some(g) = run.glyphs.first() {
                out.push((g.origin_x.to_bp(), g.baseline_y.to_bp()));
            }
        }
    }
    out
}

/// Asserts that some run of `word` sits at pdflatex's `(x, y)`.
fn check(id: &str, text: &str, word: &str, x: f64, y: f64) {
    let got = positions(text, word);
    assert!(!got.is_empty(), "{id}: no glyph run `{word}` on page 1");
    let near = got.iter().any(|(gx, gy)| (gx - x).abs() <= TOL && (gy - y).abs() <= TOL);
    assert!(near, "{id}: `{word}` at {got:?} bp, pdflatex ({x:.3}, {y:.3}) bp");
}

#[test]
fn mapsto_is_a_relation_of_cmsy_width() {
    if !common::lm_available() {
        return;
    }
    let tex = probe_source("min4-mapsto-rel");
    // Line 1 ends `1-Lipschitz` (b=1 stretched); `and` opens line 2. Was:
    // `and` at (522.424, 82.959), line 1 shrunk around the narrow arrow.
    check("min4-mapsto-rel", &tex, "Triangle", 72.000, 82.959);
    check("min4-mapsto-rel", &tex, "-Lipschitz", 493.424, 82.959);
    check("min4-mapsto-rel", &tex, "and", 72.000, 96.508);
    check("min4-mapsto-rel", &tex, "continuous", 156.302, 96.508);
}

#[test]
fn small_and_footnote_math_take_their_sizes_metrics() {
    if !common::lm_available() {
        return;
    }
    let tex = probe_source("min4-small-math");
    // `\small` abstract: `we` closes line 2 (was first on line 3 at
    // 99.273, 125.051) and `systematic` closes line 3 (was on line 4).
    check("min4-small-math", &tex, "We", 114.217, 101.141);
    check("min4-small-math", &tex, "we", 501.381, 113.096);
    check("min4-small-math", &tex, "obtained", 99.273, 125.051);
    check("min4-small-math", &tex, "systematic", 466.950, 125.051);
    // `\footnotesize` note: `assumes` closes line 1 (was first on line 2).
    check("min4-small-math", &tex, "This", 88.588, 708.985);
    check("min4-small-math", &tex, "assumes", 507.592, 708.985);
}

#[test]
fn a_hyphenation_point_inside_a_ligature_is_a_discretionary() {
    if !common::lm_available() {
        return;
    }
    let tex = probe_source("min4-lig-hyphen");
    // `parti-|tioning` (was `par-|titioning`), then `traf-|fic` through the
    // `ffi` ligature (was `traffic` unbroken), `adap-|tive`, and the last
    // line starting `intervention` (was `operator`).
    check("min4-lig-hyphen", &tex, "Consider", 54.000, 63.963);
    check("min4-lig-hyphen", &tex, "parti-", 272.624, 99.828);
    check("min4-lig-hyphen", &tex, "tioning", 54.000, 111.783);
    check("min4-lig-hyphen", &tex, "traf-", 277.882, 111.783);
    check("min4-lig-hyphen", &tex, "fic", 54.000, 123.739);
    check("min4-lig-hyphen", &tex, "adap-", 272.654, 123.739);
    check("min4-lig-hyphen", &tex, "tive", 54.000, 135.694);
    check("min4-lig-hyphen", &tex, "intervention", 54.000, 171.559);
}

#[test]
fn bibliography_entries_use_sfcode_period_sloppy_and_newblock_glue() {
    if !common::lm_available() {
        return;
    }
    let tex = probe_source("min4-bib-sfcode");
    // [1]: no sentence space after `Knuth.` and `book.` (`1984.` was at
    // 350.014). [2]: `Practice` closes line 1 (was first on line 2). [3]:
    // `\newblock` glue after `Liang.` and `Computer.` fills the line exactly
    // (`1983.` was alone on a second line; with the `\sfcode` fix but no
    // `\newblock` glue it sat 2.4 bp left of pdflatex).
    check("min4-bib-sfcode", &tex, "[1]", 72.000, 150.166);
    check("min4-bib-sfcode", &tex, "1984.", 347.409, 150.166);
    check("min4-bib-sfcode", &tex, "[2]", 72.000, 174.575);
    check("min4-bib-sfcode", &tex, "Practice", 498.758, 174.575);
    check("min4-bib-sfcode", &tex, "[3]", 72.000, 213.429);
    check("min4-bib-sfcode", &tex, "PhD", 347.178, 213.429);
    check("min4-bib-sfcode", &tex, "1983.", 513.345, 213.429);
}
