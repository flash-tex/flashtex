//! `\item[<label>]` label and body origins, measured against pdflatex
//! (GH-676 point 3).
//!
//! The default markers are already pinned by `itemize_label_offset.rs`. What
//! is pinned here is the *explicit* label — the one the issue calls "not 1:1
//! all of the time" — in all three list environments at 10pt, with a label
//! that fits `\labelwidth` and one that does not:
//!
//!   * `itemize`/`enumerate` set the label through `\makelabel` =
//!     `\hbox to\labelwidth{\hss #1}` inside `\llap`, so a short label is
//!     **right-aligned**, ending `\labelsep` before the body edge, and a wide
//!     one overhangs to the **left** without moving the body (`Ix` at
//!     144.838 and `IwideLabelHere` at 85.727 both end at the same edge, and
//!     `IA`/`IB` start at the same place).
//!   * `description` uses `\descriptionlabel` =
//!     `\hspace\labelsep\normalfont\bfseries #1` with `\labelwidth\z@` and
//!     `\itemindent-\leftmargin`, so both labels are **left-flushed** at the
//!     item's indent (133.768 for the short and the wide one alike) and the
//!     body follows on the same line, `\labelsep` after the label ends —
//!     which is why `DA` and `DB` do *not* share an x.
//!
//! The PDF coordinates come from `/Library/TeX/texbin/pdflatex` on
//! `fixtures/item-label-explicit/article-10.tex`, read with PyMuPDF's
//! `get_texttrace()` glyph origins. The issue's gate is 0.5 bp; the tolerance
//! here is the suite's usual 0.1 bp, which the engine already meets.

mod common;

use common::{lm_available, render_one, words_of, Word};

const TOL: f64 = 0.1;

struct Expected {
    page: u32,
    label: &'static str,
    body: &'static str,
    label_x: f64,
    body_x: f64,
}

const CASES: [Expected; 6] = [
    // itemize: short label right-aligned, body at the text edge.
    Expected { page: 1, label: "Ix", body: "IA", label_x: 144.838000, body_x: 158.676000 },
    // itemize: the over-wide label overhangs left; the body does not move.
    Expected { page: 1, label: "IwideLabelHere", body: "IB", label_x: 85.727000, body_x: 158.703100 },
    // enumerate: the same `\makelabel`, one `\labelwidth` narrower.
    Expected { page: 2, label: "Ex", body: "EA", label_x: 141.655000, body_x: 158.681100 },
    Expected { page: 2, label: "EwideLabelHere", body: "EB", label_x: 82.544000, body_x: 158.708100 },
    // description: both labels flush at the item's indent, body after them.
    Expected { page: 3, label: "Dx", body: "DA", label_x: 133.768000, body_x: 153.583600 },
    Expected { page: 3, label: "DwideLabelHere", body: "DB", label_x: 133.768000, body_x: 222.395300 },
];

fn word<'a>(words: &'a [Word], page: u32, text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.page == page && w.text == text)
        .unwrap_or_else(|| panic!("no page {page} word {text:?}: {words:?}"))
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOL,
        "{what}: {actual} vs pdflatex {expected}"
    );
}

#[test]
fn explicit_item_label_origins_match_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/item-label-explicit/article-10.tex"
    ))
    .expect("read the explicit-label fixture");
    let rendered = render_one(&source);
    assert_eq!(rendered.v2.pages.len(), 3, "one environment per page");
    let words = words_of(&rendered);
    for case in &CASES {
        close(
            word(&words, case.page, case.label).x,
            case.label_x,
            case.label,
        );
        close(word(&words, case.page, case.body).x, case.body_x, case.body);
    }

    // The two properties the environments differ on, stated directly rather
    // than left implicit in the numbers above.
    let same_edge = |a: &str, b: &str, page| {
        (word(&words, page, a).x - word(&words, page, b).x).abs() < TOL
    };
    assert!(
        same_edge("IA", "IB", 1) && same_edge("EA", "EB", 2),
        "an over-wide \\makelabel overhangs left and never moves the body"
    );
    assert!(
        same_edge("Dx", "DwideLabelHere", 3),
        "\\descriptionlabel flushes every label at the item's indent"
    );
    assert!(
        !same_edge("DA", "DB", 3),
        "a description body follows its label on the same line"
    );
}
