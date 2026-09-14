//! `\baselinestretch` (latex.ltx `\def\baselinestretch{1}`).
//!
//! `\@setfontsize` ends with
//! `\baselineskip\baselinestretch\baselineskip`, so the factor in force
//! multiplies *that size's* leading at every font-size selection -- the
//! body, the section headings, the footnotes, `\maketitle`. It leaves
//! `\lineskip`/`\lineskiplimit` (the `.clo`s' `\normallineskip`) and the
//! fixed `\setlength`s such as `\footnotesep` alone.
//!
//! `\linespread{f}` is latex.ltx's
//! `\renewcommand{\baselinestretch}{f}\@currsize`, so it is the same thing.
//!
//! The 1.5 expectations are the pdflatex geometry of
//! `fixtures/divergence-probes/min3-baselinestretch` (TeX Live 2025,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) read with
//! `tools/visual-oracle/pdftext.py`: consecutive body baselines 17.932 bp
//! apart (1.5 x 12 pt = 18 pt), the first one still at 81.963 because
//! `\topskip` sets it.

mod common;

use common::*;

const TOL: f64 = 0.02;

const BODY: &str = "\nA paragraph of body text set with one-and-a-half line spacing, long enough to\n\
wrap over several lines so the leading is visible: the quick brown fox jumps\n\
over the lazy dog while the printer waits for a page that never comes.\n";

fn doc(preamble: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage[margin=1in]{{geometry}}\n{preamble}\\begin{{document}}\n{BODY}\\end{{document}}\n")
}

/// The distinct baselines of the page, in order.
fn baselines(src: &str) -> Vec<f64> {
    let r = render_one(src);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let mut out: Vec<f64> = Vec::new();
    for w in words_of(&r) {
        if !out.iter().any(|b| (b - w.baseline).abs() < 0.05) {
            out.push(w.baseline);
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    out
}

#[test]
fn renewcommand_baselinestretch_scales_the_body_leading() {
    if !lm_available() {
        return;
    }
    let plain = baselines(&doc(""));
    let wide = baselines(&doc("\\renewcommand{\\baselinestretch}{1.5}\n"));
    assert!(plain.len() >= 3 && wide.len() >= 3, "{plain:?} / {wide:?}");
    // `\topskip` puts the first baseline in the same place either way.
    assert!((wide[0] - plain[0]).abs() < TOL, "first baseline moved: {plain:?} / {wide:?}");
    let pitch = wide[1] - wide[0];
    assert!((pitch - 17.932).abs() < TOL, "1.5 x 12 pt expected, got {pitch}: {wide:?}");
    assert!(((plain[1] - plain[0]) - 11.955).abs() < TOL, "unstretched 12 pt expected: {plain:?}");
}

#[test]
fn linespread_is_the_same_command() {
    if !lm_available() {
        return;
    }
    assert_eq!(
        format!("{:.3?}", baselines(&doc("\\linespread{1.5}\n"))),
        format!("{:.3?}", baselines(&doc("\\renewcommand{\\baselinestretch}{1.5}\n"))),
    );
}

#[test]
fn an_unbraced_first_argument_is_the_same_call() {
    if !lm_available() {
        return;
    }
    assert_eq!(
        format!("{:.3?}", baselines(&doc("\\renewcommand\\baselinestretch{1.5}\n"))),
        format!("{:.3?}", baselines(&doc("\\renewcommand{\\baselinestretch}{1.5}\n"))),
    );
}

#[test]
fn the_last_setting_in_the_source_wins() {
    if !lm_available() {
        return;
    }
    assert_eq!(
        format!("{:.3?}", baselines(&doc("\\renewcommand{\\baselinestretch}{2}\n\\linespread{1.5}\n"))),
        format!("{:.3?}", baselines(&doc("\\linespread{1.5}\n"))),
    );
}

#[test]
fn headings_and_footnotes_stretch_with_the_body() {
    if !lm_available() {
        return;
    }
    let src = |pre: &str| {
        format!(
            "\\documentclass{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage[margin=1in]{{geometry}}\n{pre}\
             \\begin{{document}}\n\\section{{A heading long enough to wrap over two whole lines of the \
             text measure so its own leading is visible}}\nBody text.\\footnote{{A footnote long enough \
             to wrap over two lines at the bottom of the page so that its leading is visible too, which \
             takes a fair number of words indeed, rather more than one line of the text measure holds \
             at footnote size, so here are a good many more of them to be sure of a second line and a \
             third one after it: alpha beta gamma delta epsilon zeta eta theta iota kappa lambda done.}}\n\
             \\end{{document}}\n"
        )
    };
    let plain = baselines(&src(""));
    let wide = baselines(&src("\\renewcommand{\\baselinestretch}{1.5}\n"));
    // The heading's two lines: the second baseline minus the first.
    let (hp, hw) = (plain[1] - plain[0], wide[1] - wide[0]);
    assert!((hw / hp - 1.5).abs() < 0.01, "heading leading {hp} -> {hw}");
    // The footnote's own two lines, found by a word on each.
    let pitch = |s: &str| {
        let w = words_of(&render_one(s));
        let at = |t: &str| w.iter().find(|x| x.text == t).unwrap_or_else(|| panic!("no {t:?} in {w:?}")).baseline;
        at("done.") - at("footnote")
    };
    let (fp, fw) = (pitch(&src("")), pitch(&src("\\renewcommand{\\baselinestretch}{1.5}\n")));
    assert!((fw / fp - 1.5).abs() < 0.01, "footnote leading {fp} -> {fw}");
}

#[test]
fn no_setting_leaves_every_leading_untouched() {
    if !lm_available() {
        return;
    }
    // A `\baselinestretch` that is not a number, or one for another macro,
    // must not be picked up.
    assert_eq!(
        format!("{:.3?}", baselines(&doc("\\renewcommand{\\arraystretch}{1.5}\n"))),
        format!("{:.3?}", baselines(&doc(""))),
    );
}
