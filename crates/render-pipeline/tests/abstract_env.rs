//! The `abstract` environment and the `\addvspace` rule that opens every
//! `\list`/`\trivlist`.
//!
//! Every number below is read off pdfTeX's own vertical list (`\showoutput`,
//! TeX Live 2025, `pdfTeX 3.141592653-2.6-1.40.27`) for the probe quoted in
//! the test, or off the word origins of the reference PDF that probe
//! produces. pdflatex is an oracle only and never runs in the product path.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[derive(Debug, Clone)]
struct Word {
    text: String,
    x: f64,
    baseline: f64,
    size: f64,
}

fn layout(text: &str) -> (Vec<String>, Vec<Word>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    let mut words = Vec::new();
    for page in &r.v2.pages {
        for it in &page.items {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                words.push(Word {
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    size: run.font_size.to_bp(),
                });
            }
        }
    }
    (v1.diagnostics.iter().map(|d| d.code.clone()).collect(), words)
}

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.text == text)
        .unwrap_or_else(|| panic!("no word {text:?} in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
}

/// TeX points to PDF points, the unit of every v2 coordinate.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

const TITLE_ABSTRACT: &str = "\\documentclass[SIZE]{article}\n\\usepackage[margin=1in]{geometry}\n\
\\title{A Title}\n\\author{An Author}\n\\date{March 14, 2025}\n\\begin{document}\n\\maketitle\n\n\
\\begin{abstract}\nBody of the abstract, long enough to need a second line of its own when it is set \
at the narrower quotation measure of the environment.\n\\end{abstract}\n\n\\section{Intro}\nText.\n\\end{document}";

/// `\@maketitle` ends `\par \vskip 1.5em`, and `\begin{abstract}` opens with
/// `\addvspace{\@topsep}` — `\@xaddvskip`, not `\vskip`. At a 10pt base
/// `1.5em` is 15pt and `\topsep + \partopsep` is 8+2 = 10pt; at 11pt they
/// are 16.425pt (`1.5em` of cmr10 at 10.95pt) and 9+3 = 12pt. The larger
/// `\lastskip` wins both times, so the environment contributes nothing of
/// its own and the date-to-head distance is
///
///   `\@topsepadd` (`\end{center}` of `\@maketitle`) + `1.5em` + `\small`'s
///   own `\baselineskip`
///
/// = 10 + 15 + 11 = **36pt** at a 10pt base and 12 + 16.425 + 12 =
/// **40.425pt** at 11pt. Those are the distances pdfTeX puts between the
/// two baselines: 35.866 bp and 40.274 bp in the reference PDFs of
/// `fixtures/real-world/plain-article` and `.../lab-report`.
#[test]
fn maketitle_vskip_absorbs_the_abstract_opening_addvspace() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    for (size, expected_pt) in [("10pt", 36.0), ("11pt", 12.0 + 1.5 * 10.95003 + 12.0)] {
        let (_, words) = layout(&TITLE_ABSTRACT.replace("SIZE", size));
        let date = word(&words, "March");
        let head = word(&words, "Abstract");
        let gap = head.baseline - date.baseline;
        assert!(
            (gap - bp(expected_pt)).abs() < 0.02,
            "{size}: date to abstract head {gap} bp, pdfTeX {} bp",
            bp(expected_pt)
        );
    }
}

/// The head is `{\bfseries \abstractname}` at `\small`, centred at the full
/// measure (`center` is a `\trivlist`, so `\leftmargin` is 0), and the body
/// is a `quotation`: `\leftmargini` in from both sides. `\small` at an 11pt
/// base is 10pt.
#[test]
fn the_head_is_small_and_centred_and_the_body_is_a_quotation() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (codes, words) = layout(&TITLE_ABSTRACT.replace("SIZE", "11pt"));
    assert!(
        !codes.iter().any(|c| c == "compiler"),
        "the environment is set, so its compiler limitation is superseded: {codes:?}"
    );
    let s = flashtex_render_pipeline::Stylesheet::article(11, flashtex_render_pipeline::fonts::Family::LatinModern, None);
    let head = word(&words, "Abstract");
    assert!((head.size - bp(10.0)).abs() < 0.01, "head at \\small = 10pt: {}", head.size);
    // `margin=1in`: the text area starts at exactly 72 bp.
    const TEXT_X: f64 = 72.0;
    assert!(head.x > TEXT_X + 0.25 * bp(s.text_width_pt), "head centred, not at the margin: {}", head.x);
    // The body's second line starts at `\leftmargini`; the first is further
    // in by `\listparindent` (1.5em of `\small`), so take the minimum.
    let left = words
        .iter()
        .filter(|w| w.baseline > head.baseline && w.size < bp(10.5))
        .map(|w| w.x)
        .fold(f64::MAX, f64::min);
    assert!(
        (left - (TEXT_X + bp(s.leftmargini_pt))).abs() < 0.02,
        "quotation body at \\leftmargini: {left} vs {}",
        TEXT_X + bp(s.leftmargini_pt)
    );
}

/// `\if@twocolumn` (article.cls 378-379) is `\section*{\abstractname}`: an
/// unnumbered `\Large\bfseries` head at the *column* measure and the body as
/// ordinary `\normalsize` paragraphs — no `\small`, no `quotation`, no
/// centring. In the MacTeX reference of `fixtures/real-world/conf-paper`
/// that head is `SFBX1440` (14.35 bp) at x = 54.00, the column's left edge,
/// and the body is `SFRM1000` (9.96 bp).
#[test]
fn the_two_column_branch_is_an_unnumbered_section() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[10pt,twocolumn]{article}\n\\title{A Title}\\author{An Author}\\date{}\n\
\\begin{document}\n\\maketitle\n\n\\begin{abstract}\nBody of the abstract.\n\\end{abstract}\n\n\
\\section{Intro}\nText.\n\\end{document}";
    let (codes, words) = layout(src);
    assert!(!codes.iter().any(|c| c == "compiler"), "the branch is set: {codes:?}");
    let head = word(&words, "Abstract");
    // `\Large` at a 10pt base is 14.4pt, and `\section*` is left-aligned.
    assert!((head.size - bp(14.4)).abs() < 0.01, "head at \\Large = 14.4pt: {}", head.size);
    let body = word(&words, "Body");
    assert!((body.size - bp(10.0)).abs() < 0.01, "body at \\normalsize: {}", body.size);
    assert!((body.x - head.x).abs() < 0.02, "head and body share the column's left edge: {} vs {}", head.x, body.x);
}

/// `\@item` opens a list with `\addvspace{\@topsep}` only when `\if@nobreak`
/// is false. Right after a heading `\@afterheading` has set `\@nobreaktrue`,
/// so `\@nbitem` runs instead — `\addvspace{\@outerparskip - \parskip}`
/// followed by the list's own `\parskip` leaves exactly `\lastskip`, the
/// heading's own after-skip. pdfTeX's list for the probe below, 12pt:
///
/// ```text
/// \glue 7.74811 plus 1.03305          \subsection's 1.5ex after-skip
/// \glue -7.74811 plus -1.03305        \@nbitem's \addvspace removes it
/// \glue 2.74811 plus -0.46695 minus -1.0
/// \glue(\parskip) 5.0 plus 2.5 minus 1.0
/// ```
///
/// net 7.74811pt, so the quote's first baseline is 7.74811 + `\baselineskip`
/// 14.5 = 22.248pt below the heading's — 22.165 bp, which is exactly the
/// distance in the reference PDF. `\topsep + \partopsep` (13pt at 12pt)
/// never appears.
#[test]
fn a_list_right_after_a_heading_adds_no_topsep() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[12pt]{article}\n\\begin{document}\n\\section{Alpha}\nBeta gamma delta.\n\n\
\\subsection{A worked limit}\n\n\\begin{quote}\nQuoted material here on one line only.\n\\end{quote}\n\n\
Following paragraph.\n\\end{document}";
    let (_, words) = layout(src);
    let heading = word(&words, "worked");
    let quoted = word(&words, "Quoted");
    let gap = quoted.baseline - heading.baseline;
    assert!((gap - bp(7.74811 + 14.5)).abs() < 0.02, "heading to quote {gap} bp, pdfTeX {} bp", bp(7.74811 + 14.5));
}
