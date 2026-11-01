//! GH-897: a `proof` nested in a list item is its own `\trivlist`, so it
//! keeps its italic head and its `\topsep` above and below — exactly like a
//! top-level proof.
//!
//! The compiler reports the nested proof as a label-less `ListItem`, and the
//! pipeline used to exclude every `ListItem` from the theorem-open scan in
//! `adapter`. Without the scan the paragraph took the source-scoped weight
//! (upright `Proof.` instead of italic) and neither the opening `\@topsep`
//! nor the closing `\@topsepadd` was set.
//!
//! ## Oracle
//!
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2026), `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`, on [`PROBE`] (10pt article). `pdflatex` sets the
//! nested head in `CMTI10` in both places, and the shipped-box trace shows
//! why each boundary is one `\topsep` wider than a plain paragraph break:
//!
//! ```text
//! ...\hbox(6.94444+1.94444)x444.75499   % item body ("here.")
//! ...\penalty -51
//! ...\glue 12.0 plus 9.0 minus 2.0
//! ...\glue -12.0 plus -9.0 minus -2.0   % nets to zero
//! ...\glue 8.0 plus 7.0 minus 1.0       % proof's \@topsep (6pt + \partopsep)
//! ...\glue(\parskip) 4.0 plus 2.0 minus 1.0
//! ...\glue(\parskip) 0.0
//! ...\glue(\baselineskip) 3.11111
//! ...\hbox(6.94444+1.94444)x444.75499   % "Proof." line
//! ```
//!
//! so item body to `Proof.`, baseline to baseline, is the previous depth
//! (1.94) + `\@topsep` (8.0) + outer `\parskip` (4.0) + the
//! `\baselineskip` remainder (3.11) + the head height (6.94) = 24.0pt (the
//! PDF reads 23.91bp). The close side nets the same way: `\@endparenv`'s
//! `\@topsepadd` (8.0) survives the take-off/put-back dance whole, the
//! outer `\itemsep` loses the `\addvspace` maximum to it, and the outer
//! `\parskip` (4.0) stays — 24.0pt again. The second `\item` down to the
//! top-level `Proof.` below the list is the outer `\@topsepadd` (10.0) kept
//! over the proof's own `\@topsep`, plus `\parskip`: 22.0pt.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// 1 bp = 1.00375 TeX pt.
const BP: f64 = 1.00375;

/// The corpus harness's own word tolerance.
const TOL: f64 = 0.5;

/// Issue #897's repro, in the house 10pt shape (one line per paragraph, so
/// no line breaking or `\lineskip` mode enters the numbers).
const PROBE: &str = r"\documentclass{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{amsthm}
\pagestyle{empty}
\begin{document}
\begin{enumerate}
\item First item body text goes here.
\begin{proof}
Proof body nested inside the list item.
\end{proof}
\item Second item body text goes here.
\end{enumerate}

\begin{proof}
Proof body at top level, outside any list.
\end{proof}
\end{document}
";

/// Every word run on page 1 as `(text, postscript font name, baseline bp)`.
fn runs() -> Vec<(String, String, f64)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text: PROBE }];
    let r = render(&docs, "main.tex", 1, "proof-in-list", &fonts, &RenderOptions::default());
    assert_eq!(r.v2.pages.len(), 1, "expected a one-page document");
    assert!(r.v2.diagnostics.is_empty(), "probe must stay silent: {:?}", r.v2.diagnostics);
    let names: std::collections::HashMap<&str, &str> =
        r.v2.fonts.iter().map(|f| (f.font_id.as_ref(), f.postscript_name.as_str())).collect();
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        let Some(g) = run.glyphs.first() else { continue };
        let name = names.get(run.font_id.as_ref()).copied().unwrap_or("?").to_string();
        out.push((run.text.clone(), name, g.baseline_y.to_bp()));
    }
    out
}

/// Baseline of the nth word run whose text is exactly `word`.
fn baseline_at(word: &str, n: usize) -> f64 {
    runs()
        .iter()
        .filter(|(text, _, _)| text == word)
        .nth(n)
        .unwrap_or_else(|| panic!("no {n}th run {word:?}"))
        .2
}

/// Postscript font name of the nth word run whose text is exactly `word`.
fn font_at(word: &str, n: usize) -> String {
    runs()
        .iter()
        .filter(|(text, _, _)| text == word)
        .nth(n)
        .unwrap_or_else(|| panic!("no {n}th run {word:?}"))
        .1
        .clone()
}

fn check(label: &str, got: f64, expect_pt: f64) {
    let expect = expect_pt / BP;
    assert!(
        (got - expect).abs() <= TOL,
        "{label}: {got:.4} bp, pdflatex {expect:.4} bp (off by {:+.4} bp)",
        got - expect
    );
}

/// The nested head sets italic, exactly like the top-level one; the bodies
/// stay upright. `pdflatex` uses CMTI10 for both heads.
#[test]
fn a_proof_nested_in_a_list_item_keeps_its_italic_head() {
    if !common::lm_available() {
        eprintln!("SKIP a_proof_nested_in_a_list_item_keeps_its_italic_head: Latin Modern not installed");
        return;
    }
    let nested = font_at("Proof.", 0);
    let top = font_at("Proof.", 1);
    assert!(
        nested.contains("Italic"),
        "nested Proof. head must be italic, got {nested}"
    );
    assert_eq!(
        nested, top,
        "nested and top-level Proof. heads must use the same italic face"
    );
    for (word, n) in [("Proof", 0), ("body", 0), ("nested", 0), ("Second", 0)] {
        let name = font_at(word, n);
        assert!(
            !name.contains("Italic"),
            "body word {word:?} must stay upright, got {name}"
        );
    }
}

/// Both `\topsep` boundaries of the nested proof match `pdflatex`: 24.0pt
/// above (item body to `Proof.`) and 24.0pt below (`Proof.` line to the next
/// `\item`), against 16.0pt and 20.0pt before the fix. The second `\item`
/// down to the top-level `Proof.` is the outer `\@topsepadd` kept over the
/// proof's own `\@topsep`: 22.0pt.
#[test]
fn a_proof_nested_in_a_list_item_keeps_its_topsep() {
    if !common::lm_available() {
        eprintln!("SKIP a_proof_nested_in_a_list_item_keeps_its_topsep: Latin Modern not installed");
        return;
    }
    let first = baseline_at("First", 0);
    let nested_proof = baseline_at("Proof.", 0);
    let second = baseline_at("Second", 0);
    let top_proof = baseline_at("Proof.", 1);
    check("item body -> nested Proof.", nested_proof - first, 24.0);
    check("nested Proof. line -> next item", second - nested_proof, 24.0);
    check("second item -> top-level Proof.", top_proof - second, 22.0);
}
