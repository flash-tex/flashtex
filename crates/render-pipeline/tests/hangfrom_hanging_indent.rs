//! `\hangfrom{label}` hangs the paragraph's continuation lines under the
//! text after the label (ltsect.dtx `\@hangfrom`: `\hangindent` after the
//! label, then `\noindent` with the label text).
//!
//! Every expected coordinate below was checked against pdfTeX
//! 3.141592653-2.6-1.40.27 (TeX Live 2026) on the same body text with
//! `\makeatletter\@hangfrom{Label. }` (bare `\hangfrom` is not a LaTeX
//! command), measured with `tools/visual-oracle/pdftext.py`: pdfTeX sets
//! the label at 72.000 and every continuation line at 106.394. The one
//! known divergence is the body's start on the first line (107.542 here
//! against 106.392 there): pdfTeX freezes the label's gap inside its
//! `\hbox`, while this pipeline keeps it as flowing interword glue that
//! stretches with the justified line.

mod common;

use common::*;

const TOL: f64 = 0.1;

const SOURCE: &str = concat!(
    "\\documentclass[11pt]{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage{lmodern}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\pagestyle{empty}\n",
    "\\begin{document}\n",
    "\\hangfrom{Label. }This body text is long enough to wrap onto a second line so that the continuation indent can be measured against the first line exactly, and then onto a third line for good measure here.\n",
    "\\end{document}\n",
);

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs {expected}");
}

#[test]
fn hangfrom_hangs_continuation_lines_under_the_body_text() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SOURCE);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    // The missing-hang warning is gone: the hang is set.
    assert!(
        !r.v2.diagnostics.iter().any(|d| d.message.contains("hanging indent")),
        "{:?}",
        r.v2.diagnostics
    );
    let words = words_of(&r);

    // The first line starts at the left margin with the label (`\noindent`).
    let label = word(&words, "Label.");
    close(label.x, 72.000, "label at the left margin");
    // The body follows the label on the first line.
    let body = word(&words, "This");
    assert_eq!(body.baseline, label.baseline, "body shares the first line");
    assert!(body.x > label.x, "body follows the label: {} vs {}", body.x, label.x);

    // Every continuation line hangs the label's own width in
    // (`\hangindent`): 72.000 + 34.394, exactly pdfTeX's 106.394.
    let second = word(&words, "can");
    assert!(second.baseline > label.baseline, "second line below the first");
    close(second.x, 106.394, "second line hangs the label width in");
    // ... and the lines after it too.
    let third = word(&words, "here.");
    assert!(third.baseline > second.baseline, "third line below the second");
    close(third.x, 106.394, "third line hangs like the second");

    // The hang covers the label's trailing gap, not just its words:
    // the indent past the margin exceeds the label run's own width.
    assert!(
        second.x - 72.000 > label.width,
        "hang holds the trailing space: {} vs label width {}",
        second.x - 72.000,
        label.width
    );
}
