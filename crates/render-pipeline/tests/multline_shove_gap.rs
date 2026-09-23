//! amsmath `multline` row placement: `\shoveleft`/`\shoveright` overrides and
//! the `\multlinegap` indent.
//!
//! ## Oracle
//!
//! TeX Live 2026 `/Library/TeX/texbin/pdflatex`
//! (`\documentclass{article}\usepackage{amsmath}`), glyph origins from
//! `python3 -c "import pymupdf"` over the one-page PDFs (`get_text("rawdict")`
//! char origins, in bp):
//!
//! * `\begin{multline*}aaaa\\\shoveleft{bb}\\cc\end{multline*}`: first `a`
//!   at x=143.731, first `b` at x=143.731 (flush left like the first row),
//!   first `c` at x=458.894.
//! * Same with `\shoveright{bb}`: first `b` at x=458.966 (flush right).
//! * With `\setlength{\multlinegap}{0pt}` and rows `aaaa`/`bb`/`cc`: first
//!   `a` at x=133.768 (the bare text-block edge), centred first `b` at
//!   x=301.348, last-row first `c` at x=468.857.
//! * Without amsmath pdflatex rejects the source outright:
//!   `! LaTeX Error: Environment multline* undefined.` and
//!   `! Undefined control sequence.` for `\shoveleft`.

mod common;

use common::*;

/// Left edge x of the `aaaa` / `bb` / `cc` rows, top to bottom (each row is
/// one glyph run; the page number is not a math row).
fn row_xs(source: &str) -> (Vec<f64>, Vec<flashtex_render_pipeline::display::Diagnostic>) {
    let r = render_one(source);
    let diags = r.v2.diagnostics.clone();
    let words = words_of(&r);
    let mut rows: Vec<(f64, &str, f64)> = words
        .iter()
        .filter(|w| ["aaaa", "bb", "cc"].contains(&w.text.as_str()))
        .map(|w| (w.baseline, w.text.as_str(), w.x))
        .collect();
    rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    assert_eq!(rows.len(), 3, "three math rows: {words:?}");
    assert_eq!([rows[0].1, rows[1].1, rows[2].1], ["aaaa", "bb", "cc"], "{words:?}");
    (rows.into_iter().map(|(_, _, x)| x).collect(), diags)
}

fn no_errors(diags: &[flashtex_render_pipeline::display::Diagnostic], words: &[f64]) {
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == flashtex_render_pipeline::display::Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    assert!(errors.is_empty(), "unexpected errors at {words:?}: {errors:?}");
}

const SHOVELEFT_MULTLINE: &str = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{multline*}
aaaa\\
\shoveleft{bb}\\
cc
\end{multline*}
\end{document}
";

/// `\shoveleft` sets the row flush left, exactly where the first row sits.
#[test]
fn multline_shoveleft_row_is_flush_left_like_the_first_row() {
    let (xs, diags) = row_xs(SHOVELEFT_MULTLINE);
    no_errors(&diags, &xs);
    assert!((xs[0] - 143.731).abs() < 0.1, "first row at {} (want 143.731)", xs[0]);
    assert!((xs[1] - 143.731).abs() < 0.1, "shoved row at {} (want 143.731)", xs[1]);
    assert!((xs[2] - 458.894).abs() < 0.1, "last row at {} (want 458.894)", xs[2]);
}

const SHOVERIGHT_MULTLINE: &str = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{multline*}
aaaa\\
\shoveright{bb}\\
cc
\end{multline*}
\end{document}
";

/// `\shoveright` sets the row flush right, against `\multlinegap`.
#[test]
fn multline_shoveright_row_is_flush_right() {
    let (xs, diags) = row_xs(SHOVERIGHT_MULTLINE);
    no_errors(&diags, &xs);
    assert!((xs[0] - 143.731).abs() < 0.1, "first row at {} (want 143.731)", xs[0]);
    assert!((xs[1] - 458.966).abs() < 0.1, "shoved row at {} (want 458.966)", xs[1]);
    assert!((xs[2] - 458.894).abs() < 0.1, "last row at {} (want 458.894)", xs[2]);
}

const GAPLESS_MULTLINE: &str = r"\documentclass{article}
\usepackage{amsmath}
\setlength{\multlinegap}{0pt}
\begin{document}
\begin{multline*}
aaaa\\
bb\\
cc
\end{multline*}
\end{document}
";

/// `\setlength{\multlinegap}{0pt}` removes the first/last-row indent.
#[test]
fn multline_gap_length_controls_first_and_last_row_indent() {
    let (xs, diags) = row_xs(GAPLESS_MULTLINE);
    no_errors(&diags, &xs);
    assert!((xs[0] - 133.768).abs() < 0.1, "first row at {} (want 133.768)", xs[0]);
    assert!((xs[1] - 301.348).abs() < 0.1, "middle row at {} (want 301.348)", xs[1]);
    assert!((xs[2] - 468.857).abs() < 0.1, "last row at {} (want 468.857)", xs[2]);
}

const NO_AMSMATH_SHOVE: &str = r"\documentclass{article}
\begin{document}
\begin{multline*}
aaaa\\
\shoveleft{bb}\\
cc
\end{multline*}
\end{document}
";

const OP_SHOVELEFT_MULTLINE: &str = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{multline*}
aaaa\\
\shoveleft{+bb}\\
cc
\end{multline*}
\end{document}
";

/// A shoved row starting with a Bin keeps amsmath's `.5(\wd\@ne-\wdz@)`
/// correction: pdflatex sets `+` at x=143.731 and `bb` at x=151.482.
#[test]
fn multline_shoveleft_operator_row_keeps_prefix_correction() {
    let r = render_one(OP_SHOVELEFT_MULTLINE);
    let diags = r.v2.diagnostics.clone();
    let words = words_of(&r);
    let at = |text: &str| -> f64 {
        let v: Vec<f64> = words.iter().filter(|w| w.text == text).map(|w| w.x).collect();
        assert_eq!(v.len(), 1, "one {text:?} run: {words:?}");
        v[0]
    };
    no_errors(&diags, &[at("aaaa")]);
    assert!((at("aaaa") - 143.731).abs() < 0.1, "first row at {}", at("aaaa"));
    assert!((at("+") - 143.731).abs() < 0.1, "shoved + at {}", at("+"));
    assert!((at("bb") - 151.482).abs() < 0.1, "shoved bb at {}", at("bb"));
    assert!((at("cc") - 458.894).abs() < 0.1, "last row at {}", at("cc"));
}

const NUMBERED_SHOVERIGHT_MULTLINE: &str = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{multline}
aaaa\\
\shoveright{bb}\\
cc
\end{multline}
\end{document}
";

/// In a numbered display `\shoveright` reserves the tag (`\iftag@`):
/// pdflatex sets the shoved `bb` at x=446.235 and the last-row `cc` at
/// x=446.164, the `(1)` tag at x=464.754.
#[test]
fn multline_numbered_shoveright_reserves_the_tag() {
    let (xs, diags) = row_xs(NUMBERED_SHOVERIGHT_MULTLINE);
    no_errors(&diags, &xs);
    assert!((xs[0] - 143.731).abs() < 0.1, "first row at {} (want 143.731)", xs[0]);
    assert!((xs[1] - 446.235).abs() < 0.1, "shoved row at {} (want 446.235)", xs[1]);
    assert!((xs[2] - 446.164).abs() < 0.1, "last row at {} (want 446.164)", xs[2]);
}

const EDGE_SHOVE_MULTLINE: &str = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
\begin{multline*}
solo
\end{multline*}
\begin{multline*}
\shoveleft{aa}\\
bb\\
\shoveright{cc}
\end{multline*}
\end{document}
";

/// Degenerate glue (no net fil): a single-row display sits at
/// `\multlinegap` (pdflatex: `solo` at x=143.731), and a shove onto the
/// first/last row double-counts one side (pdflatex: `aa` at x=153.694,
/// `cc` at x=133.768).
#[test]
fn multline_single_row_and_edge_shoves_match_pdflatex() {
    let r = render_one(EDGE_SHOVE_MULTLINE);
    let diags = r.v2.diagnostics.clone();
    let words = words_of(&r);
    let at = |text: &str| -> f64 {
        let v: Vec<f64> = words.iter().filter(|w| w.text == text).map(|w| w.x).collect();
        assert_eq!(v.len(), 1, "one {text:?} run: {words:?}");
        v[0]
    };
    no_errors(&diags, &[at("solo")]);
    assert!((at("solo") - 143.731).abs() < 0.1, "single row at {}", at("solo"));
    assert!((at("aa") - 153.694).abs() < 0.1, "shoved first row at {}", at("aa"));
    assert!((at("bb") - 301.348).abs() < 0.1, "middle row at {}", at("bb"));
    assert!((at("cc") - 133.768).abs() < 0.1, "shoved last row at {}", at("cc"));
}

/// Without amsmath pdflatex rejects `\shoveleft` (`Undefined control
/// sequence`): FlashTeX diagnoses `\shoveleft requires
/// \usepackage{amsmath}` and the row keeps the display's default
/// (centred) placement instead of shoving.
#[test]
fn shoveleft_without_amsmath_is_rejected() {
    let (xs, diags) = row_xs(NO_AMSMATH_SHOVE);
    let messages: Vec<_> = diags.iter().map(|d| d.message.clone()).collect();
    assert!(
        messages.iter().any(|m| m.contains("\\shoveleft") && m.contains("amsmath")),
        "a diagnostic should gate \\shoveleft on amsmath: {messages:?}"
    );
    assert!(
        (xs[1] - 301.348).abs() < 0.1,
        "ungated row must keep the centred placement at {} (want 301.348)",
        xs[1]
    );
}
