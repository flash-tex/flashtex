//! `\\[<dimen>]` inside a `\newcommand` body is a real vertical skip.
//!
//! latex.ltx's `\\` is `\@normalcr`; `\@xnewline` turns the bracketed argument
//! into a `\vspace{<dimen>}` after the broken line. `adapter::line_break_skip`
//! recovers that argument by re-reading the source bytes that follow the
//! node's span, which is correct only for a `\\` written literally in the
//! document. Expanded out of a macro body the node's span is the *invocation*
//! (`expansion::Converter::place` places a replacement-text token at the call
//! site), so those bytes are the call's own arguments — the `{Education}` of
//! `\cvsection{Education}` — and the skip was silently dropped.
//!
//! ## Oracle
//!
//! `\showoutput` for the probe below (pdfTeX 3.141592653-2.6-1.40.27,
//! TeX Live 2025, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) prints the
//! `Education` line, the `\\[-6pt]` and the `\rule` as
//!
//! ```text
//! ...\hbox(8.28131+0.0)x360.0        % {\large\bfseries Education}
//! ...\glue -6.0                      % <- the \\[-6pt]
//! ...\penalty 300
//! ...\glue(\baselineskip) 13.0
//! ...\hbox(0.6+0.0)x360.0            % \rule{\textwidth}{0.6pt}
//! ...\glue 2.0                       % \vspace{2pt}
//! ...\glue 0.0
//! ...\glue(\parskip) 4.0
//! ...\glue(\parskip) 0.0
//! ...\glue(\baselineskip) 6.05852
//! ...\hbox(7.54149+2.12863)x360.0    % Body line.
//! ```
//!
//! so `Education`'s baseline to `Body`'s baseline is
//!
//! ```text
//! 0.0 + (-6.0) + 13.0 + 0.6    = 7.60000 pt   (to the rule's baseline)
//! 0.0 + 2.0 + 4.0 + 6.05852 + 7.54149 = 19.60001 pt
//!                              = 27.20001 pt = 27.0984 bp
//! ```
//!
//! and the same two words in that probe's `main.pdf` sit 27.099 bp apart,
//! which is where the expected number below comes from. Drop the `\glue -6.0`
//! and the separation is 33.0759 bp: 6 pt = 5.977 bp too far, one per macro
//! call. `fixtures/real-world/cv` calls `\cvsection` five times, and on
//! `main` its words form an exact staircase of 0 / 5.98 / 11.95 / 17.93 /
//! 23.90 / 29.87 bp against the committed reference — 5.45% of its aligned
//! words within 0.5 bp on `dy`. pdflatex is an oracle only and never runs in
//! the product path.
//!
//! Needs the compiler's `parser::Inline::LineBreak::skip_pt`, so the whole
//! file is behind the `linebreak-skip` feature (see `Cargo.toml`): the pinned
//! `vendor/compiler` predates that field, and while it does the byte scan
//! stays in charge and this document is 5.977 bp too tall.
#![cfg(feature = "linebreak-skip")]

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// 1 bp = 1.00375 TeX pt.
const BP: f64 = 1.00375;

/// Well inside the 5.977 bp the bug is worth, and the corpus harness's own
/// word tolerance.
const TOL: f64 = 0.5;

/// `\\[-6pt]` reached through a one-argument `\newcommand`, exactly as
/// `fixtures/real-world/cv`'s `\cvsection` does.
const VIA_MACRO: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\pagestyle{empty}
\setlength{\parindent}{0pt}
\setlength{\parskip}{4pt}
\newcommand{\cvsection}[1]{\vspace{6pt}{\large\bfseries #1}\\[-6pt]\rule{\textwidth}{0.6pt}\vspace{2pt}}
\begin{document}
Top line.

\cvsection{Education}

Body line.
\end{document}
";

/// The same document with the macro hand-expanded. pdflatex's two PDFs are
/// byte-identical, so this is the control: it already passed before the fix
/// (the byte scan can see this `[-6pt]`), and it pins that the fix did not
/// change the literal case.
const LITERAL: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\pagestyle{empty}
\setlength{\parindent}{0pt}
\setlength{\parskip}{4pt}
\begin{document}
Top line.

\vspace{6pt}{\large\bfseries Education}\\[-6pt]\rule{\textwidth}{0.6pt}\vspace{2pt}

Body line.
\end{document}
";

/// The baseline of the first glyph of each word on page 1, keyed by the
/// word's text.
fn baseline_of(text: &str, word: &str) -> f64 {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "line-break-skip", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    for item in &r.v2.pages[0].items {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.trim_start().starts_with(word) {
            if let Some(g) = run.glyphs.first() {
                return g.baseline_y.to_bp();
            }
        }
    }
    panic!("no glyph run starting `{word}` on page 1");
}

/// pdflatex: 27.20001 pt from `Education`'s baseline to `Body`'s.
const EXPECTED_BP: f64 = 27.20001 / BP;

#[test]
fn a_line_break_skip_inside_a_macro_body_is_applied() {
    if !common::lm_available() {
        eprintln!("SKIP a_line_break_skip_inside_a_macro_body_is_applied: Latin Modern not installed");
        return;
    }
    let got = baseline_of(VIA_MACRO, "Body") - baseline_of(VIA_MACRO, "Education");
    assert!(
        (got - EXPECTED_BP).abs() <= TOL,
        "`\\cvsection{{Education}}` to the next paragraph: {got:.4} bp, pdflatex {EXPECTED_BP:.4} bp \
         (off by {:+.4} bp). The `\\\\[-6pt]` in the macro body is worth {:.3} bp, so a miss of that \
         size means the skip never left the macro: the node's span is the invocation, and \
         `line_break_skip` read `{{Education}}` instead of `[-6pt]`.",
        got - EXPECTED_BP,
        6.0 / BP
    );
}

#[test]
fn the_literal_form_is_unchanged() {
    if !common::lm_available() {
        eprintln!("SKIP the_literal_form_is_unchanged: Latin Modern not installed");
        return;
    }
    let got = baseline_of(LITERAL, "Body") - baseline_of(LITERAL, "Education");
    assert!(
        (got - EXPECTED_BP).abs() <= TOL,
        "hand-expanded `\\\\[-6pt]`: {got:.4} bp, pdflatex {EXPECTED_BP:.4} bp (off by {:+.4} bp)",
        got - EXPECTED_BP
    );
}

/// The two spellings must agree, which is the property pdflatex has by
/// construction: its two PDFs are byte-identical.
#[test]
fn the_macro_and_literal_forms_agree() {
    if !common::lm_available() {
        eprintln!("SKIP the_macro_and_literal_forms_agree: Latin Modern not installed");
        return;
    }
    let via_macro = baseline_of(VIA_MACRO, "Body") - baseline_of(VIA_MACRO, "Education");
    let literal = baseline_of(LITERAL, "Body") - baseline_of(LITERAL, "Education");
    assert!(
        (via_macro - literal).abs() <= 0.001,
        "the same `\\\\[-6pt]` through a macro ({via_macro:.4} bp) and written out \
         ({literal:.4} bp) must set the same page; pdflatex's two PDFs are byte-identical"
    );
}
