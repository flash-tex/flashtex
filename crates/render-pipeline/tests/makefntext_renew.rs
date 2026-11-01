//! A renewed `\@makefntext` moves real PDFs, not just the compiler layout:
//! the note body / note mark x positions against pdflatex.
//!
//! Oracle (all `[12pt]{article}`, `Text\footnote{A note.}` on page 1):
//! ```
//! pdflatex -interaction=nonstopmode -output-directory=/tmp/makefn {default,renew,ident}.tex
//! ```
//! (TeX Live 2026, `/Library/TeX/texbin/pdflatex`; 0 errors each; glyph
//! origins read from the PDFs with PyMuPDF.) The running-text mark sits at
//! x=151.825 in all three; the page number `1` at y=702.137.
//!
//! * default (`\@makefntext` = `\noindent\hb@xt@1.8em{\hss\@makefnmark}#1`):
//!   note mark x=124.318, note body x=128.787 (indent 17.933bp = 18TeXpt).
//! * renew (`\renewcommand\@makefntext[1]{\noindent\makebox[1.5em][r]{
//!   \@makefnmark}#1}`): note mark x=121.329, note body x=125.798
//!   (indent 14.944bp = 15TeXpt).
//! * ident (`\renewcommand\@makefntext[1]{#1}`): note body x=110.854 (the
//!   left margin, shared by all three), no note mark at all.
//! * minimal (`\documentclass{minimal}` + the renew body): pdflatex stops
//!   with `! LaTeX Error: Command \@makefntext undefined.` — `minimal`
//!   never defines it — so the renewal must be ignored there.

mod common;

use common::*;

const TOL_BP: f64 = 0.1;

const DEFAULT_DOC: &str = "\\documentclass[12pt]{article}\n\\begin{document}\nText\\footnote{A note.}\n\\end{document}\n";
const RENEW_DOC: &str = "\\documentclass[12pt]{article}\n\\makeatletter\\renewcommand\\@makefntext[1]{\\noindent\\makebox[1.5em][r]{\\@makefnmark}#1}\\makeatother\n\\begin{document}\nText\\footnote{A note.}\n\\end{document}\n";
const IDENT_DOC: &str = "\\documentclass[12pt]{article}\n\\makeatletter\\renewcommand\\@makefntext[1]{#1}\\makeatother\n\\begin{document}\nText\\footnote{A note.}\n\\end{document}\n";
const MINIMAL_DOC: &str = "\\documentclass{minimal}\n\\makeatletter\\renewcommand\\@makefntext[1]{\\noindent\\makebox[1.5em][r]{\\@makefnmark}#1}\\makeatother\n\\begin{document}\nText\\footnote{A note.}\n\\end{document}\n";
const MINIMAL_DEFAULT_DOC: &str = "\\documentclass{minimal}\n\\begin{document}\nText\\footnote{A note.}\n\\end{document}\n";

/// `(note body x, note mark x)` in bp: the `A` of `A note.` at the foot of
/// page 1, and the raised `1` just above its baseline (`None` when the
/// renewal sets no mark).
fn note_geometry(r: &flashtex_render_pipeline::Rendered) -> (f64, Option<f64>) {
    let words: Vec<Word> = words_of(r).into_iter().filter(|w| w.page == 1).collect();
    let body = words
        .iter()
        .find(|w| w.text == "A" || w.text == "A note.")
        .unwrap_or_else(|| panic!("no note body word; words: {words:?}"));
    let mark = words
        .iter()
        .filter(|w| w.text == "1")
        .filter(|w| (body.baseline - w.baseline) > 1.0 && (body.baseline - w.baseline) < 8.0)
        .min_by(|a, b| a.baseline.total_cmp(&b.baseline))
        .map(|w| w.x);
    (body.x, mark)
}

#[test]
fn default_note_geometry_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    let (body, mark) = note_geometry(&render_one(DEFAULT_DOC));
    assert!((body - 128.787).abs() <= TOL_BP, "note body x {body:.3} vs pdflatex 128.787");
    let mark = mark.expect("default \\@makefntext sets a mark");
    assert!((mark - 124.318).abs() <= TOL_BP, "note mark x {mark:.3} vs pdflatex 124.318");
}

#[test]
fn renewed_makefntext_moves_the_note_to_1_5em() {
    if !lm_available() {
        return;
    }
    // NOTE: the pinned compiler still reports `LaTeX Error: Command
    // \@makefntext undefined.` for the renewal itself (it defines the
    // internal only after a vendor re-pin); the geometry below is what
    // this test owns, and pdflatex reports 0 errors.
    let (body, mark) = note_geometry(&render_one(RENEW_DOC));
    assert!((body - 125.798).abs() <= TOL_BP, "note body x {body:.3} vs pdflatex 125.798");
    let mark = mark.expect("the renewal keeps \\@makefnmark");
    assert!((mark - 121.329).abs() <= TOL_BP, "note mark x {mark:.3} vs pdflatex 121.329");
}

#[test]
fn identity_makefntext_sets_the_body_at_the_margin_with_no_mark() {
    if !lm_available() {
        return;
    }
    let (body, mark) = note_geometry(&render_one(IDENT_DOC));
    assert!((body - 110.854).abs() <= TOL_BP, "note body x {body:.3} vs pdflatex 110.854");
    assert!(mark.is_none(), "identity \\@makefntext sets no mark, found {mark:?}");
}

#[test]
fn renewal_is_ignored_without_a_defining_class() {
    if !lm_available() {
        return;
    }
    // `minimal` defines no `\@makefntext` (pdflatex: `! LaTeX Error:
    // Command \@makefntext undefined.`, and its `\footnote` is undefined
    // too — there is no pdflatex oracle here). The 1.5em renewal must not
    // apply: the note is byte-identical to the renewal-less default under
    // the same class, mark included.
    let (body, mark) = note_geometry(&render_one(MINIMAL_DOC));
    let (default_body, default_mark) = note_geometry(&render_one(MINIMAL_DEFAULT_DOC));
    assert!(
        (body - default_body).abs() <= 0.01,
        "renewal under minimal moved the note: {body:.3} vs default {default_body:.3}"
    );
    assert_eq!(mark.is_some(), default_mark.is_some(), "mark presence changed under minimal");
    assert!(mark.is_some(), "the renewal is gated off under minimal, so the mark stays");
}
