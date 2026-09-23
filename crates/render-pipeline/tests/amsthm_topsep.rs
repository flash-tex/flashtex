//! A theorem-like environment is a `\trivlist`: it has `\topsep` above and
//! below, and `\addvspace` combines those with what is already there.
//!
//! The pipeline knew amsthm's head separator and its unindented first line
//! (`crate::amsthm`, `adapter::opens_theorem_item`) but gave the environment
//! no vertical space at all, so every `\begin{theorem}`/`\end{theorem}`
//! boundary was one `\topsep` too tight: 8 / 9 / 10 pt at a 10 / 11 / 12 pt
//! base. In `fixtures/real-world/lecture-notes` that is eight boundaries on
//! page 1 alone, and the page's words formed a staircase reaching −49.6 bp
//! against the committed reference.
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025), `\showoutput`,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, on [`PROBE`] at an 11 pt base.
//! Between the last line of one environment and the first line of the next:
//!
//! ```text
//! ...\hbox(8.2125+2.73749)x469.75502        % last line of the definition
//! ...\penalty -51
//! ...\glue 9.0 plus 3.0 minus 5.0           % \@endparenv's \@topsepadd
//! ...\glue -11.73749 plus -3.0 minus -5.0   % \endtrivlist: -(that + depth)
//! ...\penalty -51
//! ...\glue 2.73749                          % + the depth back
//! ...\glue 9.0 plus 3.0 minus 5.0           % \@item's \@topsep for the next
//! ...\glue(\parskip) 0.0 plus 1.0
//! ...\glue(\parskip) 0.0
//! ...\glue(\baselineskip) 2.65002
//! ...\hbox(8.2125+2.73749)x469.75502        % first line of the lemma
//! ```
//!
//! The four explicit glues net to **one** `\topsep`, so the separation is
//! `\baselineskip` + `\topsep` = 13.6 + 9.0 = 22.6 pt, and the same trace at
//! a 10 pt base gives `8.0 plus 2.0 minus 4.0` (12.0 + 8.0 = 20.0 pt) and at
//! 12 pt `10.0 plus 4.0 minus 6.0` (14.5 + 10.0 = 24.5 pt).
//!
//! The value is `\topsep` alone — never `\partopsep`, never `\parskip` —
//! because `\@thm` assigns `\@topsep\thm@preskip` and
//! `\@topsepadd\thm@postskip` outright (amsthm.sty 140–141) and
//! `\thm@space@setup` sets both to `\topsep`; the `\@trivlist` derivation
//! that `center`/`quote` go through never runs for a theorem.
//!
//! `proof` is not a `\@thm`: it is a plain `\trivlist` under amsthm's own
//! `\topsep6\p@\@plus6\p@` entered from `\par`, so its *closing*
//! `\@topsepadd` is `6pt` + `\partopsep` — the trace prints
//! `8.0 plus 7.0 minus 1.0`, `9.0 plus 7.0 minus 1.0` and
//! `9.0 plus 8.0 minus 2.0` at a 10 / 11 / 12 pt base.
//!
//! And when the environment's last thing is a display, `\addvspace` keeps the
//! *larger* of the two and not their sum — `lecture-notes`' page 1 again:
//!
//! ```text
//! ...\hbox(7.60416+2.12917)x136.07909, shifted 166.83797, display
//! ...\penalty 0
//! ...\glue(\belowdisplayskip) 11.0 plus 3.0 minus 6.0
//! ...\glue -11.0 plus -3.0 minus -6.0       % \@endparenv takes the skip off
//! ...\glue 11.0 plus 3.0 minus 6.0          % and puts back the larger of
//! ...                                       % the two, not 11.0 + 9.0
//! ```
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// 1 bp = 1.00375 TeX pt.
const BP: f64 = 1.00375;

/// The corpus harness's own word tolerance, and a fifth of the smallest
/// `\topsep` this file asserts.
const TOL: f64 = 0.5;

/// `\newtheorem` environments of both amsthm styles, a `proof`, and ordinary
/// paragraphs before and after, so every boundary a theorem can have is one
/// measurement away. Each body is one short line, so no line breaking or
/// `\lineskip` mode enters the numbers.
const PROBE: &str = r"\documentclass[SIZEpt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{amsthm}
\pagestyle{empty}
\newtheorem{lemma}{Lemma}
\theoremstyle{definition}
\newtheorem{definition}[lemma]{Definition}
\begin{document}
Alpha before.

\begin{definition}
Beta body of the definition.
\end{definition}

\begin{lemma}
Gamma body of the lemma.
\end{lemma}

\begin{proof}
Delta body of the proof.
\end{proof}

Epsilon after.
\end{document}
";

/// A theorem whose last thing is a display, then another environment: the
/// `\belowdisplayskip` must absorb the closing `\topsep`, not add to it.
const DISPLAY_PROBE: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{amsmath,amsthm}
\pagestyle{empty}
\newtheorem{theorem}{Theorem}
\begin{document}
\begin{theorem}
There exist unique $q$ and $r$ with
\[
  a = qb + r.
\]
\end{theorem}

\begin{proof}
Existence is by well-ordering.
\end{proof}
\end{document}
";

/// The baseline, in bp from the page top, of the glyph run that starts with
/// `word` on page 1.
fn baseline_of(text: &str, word: &str) -> f64 {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "amsthm-topsep", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.trim_start().starts_with(word) {
            if let Some(g) = run.glyphs.first() {
                return g.baseline_y.to_bp();
            }
        }
    }
    panic!("no glyph run starting `{word}` on page 1");
}

/// (class size, `\baselineskip`, `\topsep`, `proof`'s closing `\@topsepadd`),
/// all in TeX points, from `size1x.clo` and the traces quoted above.
const SIZES: &[(u32, f64, f64, f64)] = &[
    (10, 12.0, 8.0, 8.0),
    (11, 13.6, 9.0, 9.0),
    (12, 14.5, 10.0, 9.0),
];

fn probe(size: u32) -> String {
    PROBE.replace("SIZE", &size.to_string())
}

fn check(label: &str, got: f64, expect_pt: f64) {
    let expect = expect_pt / BP;
    assert!(
        (got - expect).abs() <= TOL,
        "{label}: {got:.4} bp, pdflatex {expect:.4} bp (off by {:+.4} bp)",
        got - expect
    );
}

/// The boundary between an ordinary paragraph and a theorem, and between two
/// theorems, is `\baselineskip` + `\topsep` — one `\topsep`, not two and not
/// none.
#[test]
fn a_theorem_has_one_topsep_above_and_below_it() {
    if !common::lm_available() {
        eprintln!("SKIP a_theorem_has_one_topsep_above_and_below_it: Latin Modern not installed");
        return;
    }
    for &(size, baselineskip, topsep, proof_close) in SIZES {
        let tex = probe(size);
        let y = |w: &str| baseline_of(&tex, w);
        let (alpha, def, lemma, proof, epsilon) =
            (y("Alpha"), y("Definition"), y("Lemma"), y("Proof"), y("Epsilon"));
        check(&format!("{size}pt paragraph -> definition"), def - alpha, baselineskip + topsep);
        check(&format!("{size}pt definition -> lemma"), lemma - def, baselineskip + topsep);
        check(&format!("{size}pt lemma -> proof"), proof - lemma, baselineskip + topsep);
        // amsthm's `proof` sets `\topsep6\p@\@plus6\p@` and is entered from
        // `\par`, so its closing `\@topsepadd` is 6pt + `\partopsep`.
        check(&format!("{size}pt proof -> paragraph"), epsilon - proof, baselineskip + proof_close);
    }
}

/// `\addvspace` keeps the larger of the closing `\topsep` and whatever the
/// environment already left, not their sum.
///
/// Unlike the test above, this one *passes* on a tree without the closing
/// `\topsep` — there is nothing there to be added twice. It is the guard that
/// caught the first version of that change adding the two: with `\topsep`
/// summed on top of `\belowdisplayskip` it reports `+8.97 bp`, and four
/// unrelated corpus fixtures (`hyperref-toc`, `lab-report`, `natbib-review`,
/// `plain-article`) lost 25–40 points of `dy` to the same sum applied to their
/// `abstract`.
#[test]
fn a_closing_topsep_is_absorbed_by_a_larger_belowdisplayskip() {
    if !common::lm_available() {
        eprintln!("SKIP a_closing_topsep_is_absorbed_by_a_larger_belowdisplayskip: Latin Modern not installed");
        return;
    }
    let there = baseline_of(DISPLAY_PROBE, "There");
    let proof = baseline_of(DISPLAY_PROBE, "Proof");
    // The theorem's own line down to the display is
    // 2.12917 + \abovedisplayskip 11.0 + 3.86668 + 7.60416 = 24.60001 pt,
    // and the display down to `Proof.` is
    // 2.12917 + 11.0 + 3.92935 + 7.54149 = 24.60001 pt — the second 11.0
    // being the *whole* of the glue between them, `\belowdisplayskip` with
    // the closing `\topsep` absorbed into it. Adding the two would put
    // `Proof.` 9 pt (8.97 bp) further down.
    check("theorem line -> proof, across the display", proof - there, 2.0 * 24.60001);
}

/// The baseline, in bp from the page top, of the glyph run that starts with
/// `word` on page 1 of a document that may run to more pages.
fn baseline_on_page_one(text: &str, word: &str) -> f64 {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "amsthm-topsep", &fonts, &RenderOptions::default());
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        if run.text.trim_start().starts_with(word) {
            if let Some(g) = run.glyphs.first() {
                return g.baseline_y.to_bp();
            }
        }
    }
    panic!("no glyph run starting `{word}` on page 1");
}

/// A `proof` of two paragraphs closes with *its* `\@topsepadd` (6pt plus
/// 6pt + `\partopsep` = 9pt plus 7pt minus 1pt at 11pt), not with the
/// `\trivlist` derivation the second paragraph would get on its own (9pt
/// plus 3pt minus 5pt). The natural values agree, so only a page set with
/// shrink tells them apart: `fixtures/divergence-probes/min5-proof-close-
/// shrink` has `\textheight` 120pt and pdflatex's `\tracingpages` breaks it
/// at the `\section` with `t=123.60004 plus 16.0 minus 13.0 g=120.0 b=2
/// p=-300 c=-298`, a shrink ratio of 3.6/13. With the proof's skips dropped
/// between its blocks the builder had `minus 17.0`, ratio 3.6/17, and the
/// proof's lines sat 0.78 bp low (`lecture-notes` page 1: 0.72 bp median
/// drift, 1.08 at the foot). The same probe pins amsthm's `\qed` list
/// (`\unskip\penalty9999 \hbox{}\nobreak\hfill\quad\hbox{\qedsymbol}`):
/// with the source blank kept and the `\quad` missing, `hence by zero.`
/// stayed on one shrunk line where pdflatex breaks after `by`. Reference
/// word origins from the committed `reference.pdf` (pdfTeX
/// 3.141592653-2.6-1.40.29, TeX Live 2026).
#[test]
fn a_two_paragraph_proof_closes_with_its_own_topsepadd() {
    if !common::lm_available() {
        eprintln!("SKIP a_two_paragraph_proof_closes_with_its_own_topsepadd: Latin Modern not installed");
        return;
    }
    let path = format!("{}/../../fixtures/divergence-probes/min5-proof-close-shrink/main.tex", env!("CARGO_MANIFEST_DIR"));
    let tex = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let y = |w: &str| baseline_on_page_one(&tex, w);
    // Reference baselines in bp; the corpus tolerance is 0.1 bp.
    for (word, expect) in [("Theorem", 82.959), ("Proof.", 128.664), ("Uniqueness.", 155.763), ("zero.", 169.312), ("Corollary", 191.552)] {
        let got = y(word);
        assert!((got - expect).abs() <= 0.1, "`{word}`: {got:.3} bp, pdflatex {expect:.3} bp");
    }
}
