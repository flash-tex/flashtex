//! `\lstset` and the `lstlisting` keys (`crate::listings`).
//!
//! Every number below is read off pdfTeX itself — `\showoutput` on the probe
//! quoted in the test, or the content stream of
//! `fixtures/real-world/listings-manual/reference.pdf` — with TeX Live 2025
//! (`pdfTeX 3.141592653-2.6-1.40.27`) and listings v1.10c. pdflatex is an
//! oracle only and never runs in the product path.
//!
//! Every test but `environment_keys_do_not_escape_their_listing` (which
//! asserts an absence) fails on main at b529e9d3, where the compiler's
//! plain typewriter lowering is all there is. Built from a `git archive` of
//! `origin/main:crates/render-pipeline` with this file dropped in, main
//! reports:
//!
//! - `code_lines_use_basicstyle_size_and_that_sizes_baselineskip`:
//!   `three code lines: []` — nothing is set at 10pt, because the block is
//!   set at the 10.95pt body size with the 13.6pt body `\baselineskip`;
//! - `caption_line_is_set_above_the_listing_at_the_body_size`:
//!   ``\fnum@lstlisting: ` is `Listing 1: ``` — there is no caption line;
//! - `line_numbers_are_right_aligned_a_numbersep_before_the_margin`:
//!   ``numberstyle=\tiny` is 6pt at an 11pt base: 10.909090995788574 bp` —
//!   the only `1` on the page is the folio;
//! - `a_token_is_centred_in_its_fixed_columns`:
//!   `index out of bounds: the len is 0 but the index is 0`;
//! - `a_listing_label_resolves_to_the_listing_number`:
//!   ``\ref{lst:demo}` is unresolved: [... "Listing", "??", ...]`;
//! - `the_limitation_names_every_key_that_was_not_applied`:
//!   ``` "`frame` rules are not drawn" missing from "lstlisting (3 line(s))
//!   set as a flush-left typewriter paragraph with forced line breaks: the
//!   listings keys are not applied, so `basicstyle` (its font size),
//!   `frame`, `numbers` and `caption` are missing" ```.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// One glyph run: its text, the first glyph's origin, and the pen position
/// after its last glyph.
#[derive(Debug, Clone)]
struct Run {
    text: String,
    x: f64,
    end: f64,
    baseline: f64,
    size: f64,
}

fn runs(text: &str) -> (Vec<String>, Vec<Run>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for it in &page.items {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                let last = run.glyphs.last().expect("a run with a first glyph has a last");
                out.push(Run {
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    end: last.origin_x.to_bp() + last.advance_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    size: run.font_size.to_bp(),
                });
            }
        }
    }
    (r.v2.diagnostics.iter().map(|d| d.message.clone()).collect(), out)
}

fn run_of<'a>(runs: &'a [Run], text: &str) -> &'a Run {
    runs.iter().find(|r| r.text == text).unwrap_or_else(|| {
        panic!("no run {text:?} in {:?}", runs.iter().map(|r| &r.text).collect::<Vec<_>>())
    })
}

/// TeX points to PDF points, the unit of every v2 coordinate.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

/// The baselines of the code lines, in document order: every run set at the
/// `basicstyle` size, one entry per distinct baseline.
fn code_baselines(runs: &[Run], size_pt: f64) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for r in runs.iter().filter(|r| (r.size - bp(size_pt)).abs() < 0.01) {
        if out.last().is_none_or(|last: &f64| (last - r.baseline).abs() > 0.01) {
            out.push(r.baseline);
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    out.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    out
}

/// The probe: `fixtures/real-world/listings-manual`'s own `\lstset`, one
/// captioned `language=C` listing, and a paragraph on either side.
const PROBE: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{listings}
\lstset{
  basicstyle=\ttfamily\small,
  numbers=left,
  numberstyle=\tiny,
  frame=single,
  breaklines=true,
  showstringspaces=false,
  tabsize=2
}
\begin{document}
Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi.

\begin{lstlisting}[language=C,caption={Demo caption},label={lst:demo}]
int main(void) {
    return 0;
}
\end{lstlisting}

Omicron pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta.

See Listing~\ref{lst:demo} for details.
\end{document}";

/// `\lst@Init` runs the `basicstyle` declarations before the first line, so
/// the block is set at that size *and at that size's own `\baselineskip`*:
/// `\ttfamily\small` in an 11pt article is `\fontsize{10}{12}`. The
/// reference PDF sets its code in `SFTT1000` at 9.9626 bp with 11.955 bp
/// between baselines, and `\showoutput` gives every line as
/// `\hbox(8.39996+3.60004)` — LaTeX's strut, 0.7 and 0.3 of that 12 pt.
#[test]
fn code_lines_use_basicstyle_size_and_that_sizes_baselineskip() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (_, runs) = runs(PROBE);
    let baselines = code_baselines(&runs, 10.0);
    assert_eq!(baselines.len(), 3, "three code lines: {baselines:?}");
    for pair in baselines.windows(2) {
        assert!(
            (pair[1] - pair[0] - bp(12.0)).abs() < 0.02,
            "code leading {} bp, pdfTeX sets 11.955 ({baselines:?})",
            pair[1] - pair[0]
        );
    }
}

/// `\lst@MakeCaption t` is `\@makecaption{\lstlistingname~\thelstlisting}`
/// at `\normalsize\normalfont`, centred at the full measure, and the
/// vertical list between it and the first code line is
/// `\belowcaptionskip + \lineskip + \ht(frame rule box) - \baselineskip`,
/// the interline glue, and the rule box — 3 + 1 + 0.4 + 3 + 8.4 = **15.8
/// pt** from baseline to baseline. The reference PDF measures 15.75 bp
/// between `Listing 1: Demo caption` and the line under it.
#[test]
fn caption_line_is_set_above_the_listing_at_the_body_size() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (_, runs) = runs(PROBE);
    let listing = run_of(&runs, "Listing");
    assert!(
        (listing.size - bp(10.95003)).abs() < 0.02,
        "the caption is `\\normalsize`, not the basicstyle: {} bp",
        listing.size
    );
    let number = runs
        .iter()
        .find(|r| r.text == "1:" && (r.baseline - listing.baseline).abs() < 0.01)
        .expect("`\\fnum@lstlisting: ` is `Listing 1: `");
    assert!(number.x > listing.end, "the number follows the name");
    let first_code = code_baselines(&runs, 10.0)[0];
    assert!(
        (first_code - listing.baseline - bp(15.8)).abs() < 0.03,
        "caption to first code baseline {} bp, pdfTeX sets 15.75",
        first_code - listing.baseline
    );
}

/// `numbers=left` is `\llap{\normalfont \lst@numberstyle{\thelstnumber}
/// \kern\lst@numbersep}` (lstmisc.sty 1183-1186). `\normalfont` makes the
/// numbers roman even under a typewriter basicstyle — the reference sets
/// them in `SFRM0600`, not `SFTT1000` — and `\llap` right-aligns them, so
/// the number's *right edge* plus `numbersep` (10 pt) lands on the text
/// margin. In the reference the `1` starts at x = 58.385 bp and is 3.652 bp
/// wide: 58.385 + 3.652 + 9.963 = 72.0.
#[test]
fn line_numbers_are_right_aligned_a_numbersep_before_the_margin() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (_, runs) = runs(PROBE);
    let margin = 72.0;
    for n in ["1", "2", "3"] {
        let number = run_of(&runs, n);
        assert!(
            (number.size - bp(6.0)).abs() < 0.02,
            "`numberstyle=\\tiny` is 6pt at an 11pt base: {} bp",
            number.size
        );
        assert!(
            (number.end + bp(10.0) - margin).abs() < 0.03,
            "number {n} ends at {} bp, a numbersep short of the {margin} bp margin",
            number.end
        );
    }
}

/// `columns=[c]fixed`: an output token of `n` characters is set in `n` cells
/// of `basewidth`, centred, so each of the `n + 1` `\hss` gets
/// `n(W - w)/(n + 1)`. For `\ttfamily\small` that is `n · 0.1em/(n + 1)` of
/// `ec-lmtt10` (quad 1.05, every character 0.525). pdfTeX writes exactly
/// those kerns: the fixture's `def` gets `-78`/`-79` and its 20-character
/// `write_default_config` `-100`. So a line opening with the three-letter
/// token `int` starts `3 × 1.0498/4 = 0.78735 pt` right of the margin.
#[test]
fn a_token_is_centred_in_its_fixed_columns() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (_, runs) = runs(PROBE);
    let first_code = code_baselines(&runs, 10.0)[0];
    let opening = runs
        .iter()
        .filter(|r| (r.baseline - first_code).abs() < 0.01 && (r.size - bp(10.0)).abs() < 0.01)
        .min_by(|a, b| a.x.partial_cmp(&b.x).expect("finite"))
        .expect("the first code line has glyphs");
    let quad = 1.05 * 10.0;
    let hss = 3.0 * (0.6 - 0.5) * quad / 4.0;
    assert!(
        (opening.x - (72.0 + bp(hss))).abs() < 0.03,
        "`int` opens at {} bp, {} expected ({}pt of fill before it)",
        opening.x,
        72.0 + bp(hss),
        hss
    );
}

/// `\lstset` is global from its point of use; a key in the environment's own
/// `[...]` applies to that listing alone. Here the second listing is set
/// without `numbers`, so its first line starts at the margin plus its own
/// centring fill and nothing hangs in the margin.
#[test]
fn environment_keys_do_not_escape_their_listing() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let source = PROBE.replace(
        "\\begin{lstlisting}[language=C,caption={Demo caption},label={lst:demo}]",
        "\\begin{lstlisting}[numbers=none]",
    );
    let (_, runs) = runs(&source);
    assert!(
        !runs.iter().any(|r| (r.size - bp(6.0)).abs() < 0.02),
        "`numbers=none` sets no `\\tiny` number: {:?}",
        runs.iter().map(|r| (&r.text, r.size)).collect::<Vec<_>>()
    );
    assert!(
        runs.iter().filter(|r| (r.size - bp(10.0)).abs() < 0.01).all(|r| r.x >= 72.0 - 0.01),
        "nothing hangs left of the margin without `numbers`"
    );
}

/// `\label` given in an `lstlisting`'s keys resolves to the listing's own
/// number: `\lst@MakeCaption t` steps `lstlisting` and runs
/// `\label{\lst@label}` for every displayed listing (listings.sty
/// 1638-1641). On main `Listing~\ref{lst:demo}` rendered as `??`.
#[test]
fn a_listing_label_resolves_to_the_listing_number() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (_, runs) = runs(PROBE);
    assert!(
        !runs.iter().any(|r| r.text.contains("??")),
        "`\\ref{{lst:demo}}` is unresolved: {:?}",
        runs.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
}

/// The limitation must keep naming what is still missing. A half-built
/// feature that goes quiet is worse than an honest report.
#[test]
fn the_limitation_names_every_key_that_was_not_applied() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let (messages, _) = runs(PROBE);
    let listing = messages
        .iter()
        .find(|m| m.starts_with("lstlisting ("))
        .unwrap_or_else(|| panic!("no lstlisting limitation in {messages:?}"));
    for expected in ["`frame` rules are not drawn", "language=C", "breaklines"] {
        assert!(listing.contains(expected), "{expected:?} missing from {listing:?}");
    }
    // The probe sets `showstringspaces=false`, which is what this module
    // does anyway, so it is not a gap; with the package default (true) it is.
    assert!(!listing.contains("showstringspaces"), "{listing:?}");
    let (on, _) = runs(&PROBE.replace("showstringspaces=false", "showstringspaces=true"));
    assert!(
        on.iter().any(|m| m.contains("showstringspaces")),
        "the default `showstringspaces` is a gap and must be named: {on:?}"
    );
    for applied in ["`basicstyle`", "`columns=[c]fixed`", "`numbers=left`", "`caption`"] {
        assert!(listing.contains(applied), "{applied:?} missing from {listing:?}");
    }
    assert!(
        !messages.iter().any(|m| m.contains("not supported in the document preamble") && m.contains("lstset")),
        "`\\lstset` is read here, so the compiler's preamble error is superseded: {messages:?}"
    );
}
