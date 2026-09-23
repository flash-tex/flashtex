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
//! Three more from the 2026-09-19T230000Z ranking (main `608ee20ca`):
//!
//! * `min5-url-break`: a `\url` is a math list with url.sty's penalties
//!   between its runs (listings-manual p.1).
//! * `min5-description-label`: `-{}-` stays two hyphens and `\emph`'s
//!   trailing `\/` stays in a label box (listings-manual p.3).
//! * `min5-emph-punct-hyphen`: a word followed by punctuation in another
//!   font is still hyphenated, and a tie next to an input ligature is still
//!   a tie (article-twocolumn p.2).
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

/// url.sty sets a URL as a math list whose muskips are 0mu, so it breaks at
/// TeX's own math-list penalties: `\binoppenalty` (700) after a `\UrlBreaks`
/// character that an ordinary one precedes, `\relpenalty` (500) after `:`
/// (tex.web §761 with the §728-729 Bin→Ord demotions). The pipeline merged
/// the compiler's runs back into one unbreakable word, so a URL that did
/// not fit went down whole (listings-manual p.1: `rather` 37.44 bp left).
/// pdflatex's `\tracingparagraphs`: `https : / / example . org / ftxc /`
/// then `@\penalty via @@1 b=7 p=700 d=490289`; for the second paragraph
/// `... / items ?` then `@\penalty via @@0 b=0 p=700 d=490100`.
#[test]
fn a_url_breaks_at_its_math_list_penalties() {
    if !common::lm_available() {
        return;
    }
    let tex = probe_source("min5-url-break");
    // `issues` opens line 2 of paragraph 1 (was the whole URL on line 2 and
    // `rather` at 72.000); `filter=` opens line 2 of paragraph 2.
    check("min5-url-break", &tex, "https:", 399.065, 96.508);
    check("min5-url-break", &tex, "issues", 72.000, 110.057);
    check("min5-url-break", &tex, "rather", 109.442, 110.057);
    check("min5-url-break", &tex, "directly.", 207.074, 110.057);
    check("min5-url-break", &tex, "https:", 325.790, 123.606);
    check("min5-url-break", &tex, "filter=", 72.000, 137.156);
    check("min5-url-break", &tex, "which", 228.239, 137.156);
}

/// `\emph{Software: Practice and Experience},` is one word of two segments
/// here (the letters in `cmti`, the `,` in `cmr`); TeX ends the word at the
/// font change (§898) and skips the `,` to the glue (§899), so `Experience`
/// is hyphenated like any other word. The pipeline refused every word of
/// more than one segment, and the second `\bibitem` then broke after
/// `paragraphs` and `Experience,` (badness 1742) instead of `para-` and
/// `Experi-` (pdflatex `@@10: line 3.2- t=498525`), every word of its
/// middle line 33.8 bp off (article-twocolumn p.2). The same entry's
/// `pp.~1119--1184,` also lost its tie to the input-ligature length test
/// (`1981.` 2.76 bp right).
#[test]
fn a_word_followed_by_punctuation_in_another_font_is_hyphenated() {
    if !common::lm_available() {
        return;
    }
    let tex = probe_source("min5-emph-punct-hyphen");
    check("min5-emph-punct-hyphen", &tex, "para-", 274.281, 138.649);
    check("min5-emph-punct-hyphen", &tex, "graphs", 69.495, 150.604);
    check("min5-emph-punct-hyphen", &tex, "Software:", 156.874, 150.604);
    check("min5-emph-punct-hyphen", &tex, "Experi-", 265.641, 150.604);
    check("min5-emph-punct-hyphen", &tex, "ence", 69.495, 162.559);
    check("min5-emph-punct-hyphen", &tex, "1981.", 229.990, 162.559);
}

/// A `description` label of `\texttt{-{}-set \emph{key}=\emph{value}}`:
/// the empty group keeps `--` from becoming `ectt1095`'s en-dash ligature
/// (`LIG O 55 O 25`), and `\emph`'s closing `\/` after `value` is a kern
/// that stays at the end of `\descriptionlabel`'s `\hbox`. pdflatex's
/// `\showbox` of the label: 15 characters of 5.65837 pt plus two
/// `\kern 1.90057` = 88.67671 pt. The pipeline shaped the two hyphens as
/// one en dash and dropped the trailing correction with the trailing glue
/// (listings-manual p.3: `Override` 7.54 bp left, `Rebuild` 5.64).
#[test]
fn a_description_label_keeps_its_empty_group_hyphens_and_trailing_italic_correction() {
    if !common::lm_available() {
        return;
    }
    let tex = probe_source("min5-description-label");
    check("min5-description-label", &tex, "Rebuild", 116.912, 122.012);
    check("min5-description-label", &tex, "Override", 165.805, 144.528);
    check("min5-description-label", &tex, "Select", 146.997, 167.044);
    check("min5-description-label", &tex, "Limit", 124.450, 189.559);
    // `shelf{}ful` and `f{}ine` in the body: no `ff`/`fi` ligature, so the
    // words that follow sit where pdflatex puts them.
    check("min5-description-label", &tex, "of", 138.116, 215.064);
    check("min5-description-label", &tex, "print:", 171.269, 215.064);
}
