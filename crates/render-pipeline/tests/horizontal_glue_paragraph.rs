//! A body whose only content is horizontal glue or kerns still sets one
//! line, and therefore one page (issue #843).
//!
//! TeX §1091 (`new_graf`): horizontal material in vertical mode starts a
//! paragraph, and a paragraph with no boxes still breaks to one line.
//! pdflatex ships a page for a body holding only `\hspace{1cm}`, `\hfill`,
//! `\quad`, `\,`, `~` or `\ `, while a genuinely empty body and
//! `\vspace{1cm}` alone (vertical material) produce no PDF. The pipeline
//! used to drop every glue-only horizontal list in `typeset::hlist`
//! (trailing glue stripped, no box left, `paragraph_block` returning
//! `None`), so the display list had no pages and the build failed outright.
//! The fix anchors such lists with the same empty-hbox `LeaveVmode` node
//! `\hrulefill` already relies on.
//!
//! Page counts below are the pdflatex oracle (TeX Live 2026, `pdfinfo`):
//! every `one_page` body is 1 page there, every `no_page` body none.
//! `\strut` belongs to the same issue but is implemented in the compiler,
//! which this crate only sees through its pinned vendor copy, so it is
//! covered by the compiler's own tests until the next re-pin — not here.

mod common;

use common::render_one;

fn pages_of_quiet(body: &str) -> usize {
    let src = format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}");
    let rendered = render_one(&src);
    let errors: Vec<_> = rendered
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_render_pipeline::display::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "{body}: unexpected diagnostics: {errors:?}"
    );
    rendered.v2.pages.len()
}

/// Issue #843: each of these bodies is one page under pdflatex.
#[test]
fn horizontal_only_bodies_set_one_page() {
    if !common::lm_available() {
        return;
    }
    for body in [
        "\\hspace{1cm}",
        "\\hfill",
        "\\hskip1cm",
        "\\quad",
        "\\qquad",
        "\\,",
        "\\thinspace",
        "\\enspace",
        "~",
        "\\ ",
    ] {
        assert_eq!(pages_of_quiet(body), 1, "body {body:?}");
    }
}

/// The guard stays honest: vertical-only and whatsit-only bodies still set
/// no page, exactly as pdflatex does.
#[test]
fn genuinely_empty_bodies_still_set_no_page() {
    if !common::lm_available() {
        return;
    }
    for body in ["", "\\vspace{1cm}", "\\label{x}"] {
        assert_eq!(pages_of_quiet(body), 0, "body {body:?}");
    }
}

/// Already-correct rows of the same table, locked in unchanged.
#[test]
fn box_only_and_text_bodies_still_set_one_page() {
    if !common::lm_available() {
        return;
    }
    for body in ["\\rule{1cm}{1cm}", "Hello world", "A\\hspace{1cm}B"] {
        assert_eq!(pages_of_quiet(body), 1, "body {body:?}");
    }
}
