//! enumitem `style=nextline` on a `description` list against pdflatex.
//!
//! `style=nextline` (`enumitem.sty` `\enit@style@nextline`) sets
//! `\enit@nextline` and computes `\labelwidth` from the narrowest fit, so
//! `\enit@postlabel@i` breaks after every label in practice: each label
//! sits alone on its line flush at the margin and the body starts on the
//! next line — one `\baselineskip` below, at the hanging indent where a
//! wrapped continuation line also starts.
//!
//! Every expected coordinate below was measured with
//! `tools/visual-oracle/pdftext.py` on the PDF pdfTeX 3.141592653-2.6-1.40.27
//! (TeX Live 2026) produced from the same source; the oracle is not run
//! here. Coordinates are bp from the page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.02;

const SOURCE: &str = concat!(
    "\\documentclass[11pt]{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage{lmodern}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\usepackage{enumitem}\n",
    "\\pagestyle{empty}\n",
    "\\begin{document}\n",
    "Alpha paragraph one line only here.\n",
    "\n",
    "\\begin{description}[style=nextline]\n",
    "  \\item[First label] Description item body text here which is long enough to\n",
    "  wrap onto a second line so that the continuation indent can be measured\n",
    "  against the reference output of pdflatex exactly.\n",
    "  \\item[Second label] Another item body, also long enough to wrap onto a\n",
    "  second line so the hanging indent shows up twice in this probe document.\n",
    "\\end{description}\n",
    "\n",
    "Bravo paragraph one line only here.\n",
    "\\end{document}\n",
);

/// The same list driven by `\setlist` instead of the `\begin` optional
/// argument (structural assertions only: no second oracle run).
const SETLIST_SOURCE: &str = concat!(
    "\\documentclass[11pt]{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage{lmodern}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\usepackage{enumitem}\n",
    "\\setlist[description]{style=nextline}\n",
    "\\pagestyle{empty}\n",
    "\\begin{document}\n",
    "Alpha paragraph one line only here.\n",
    "\n",
    "\\begin{description}\n",
    "  \\item[First label] Description item body text here which is long enough to\n",
    "  wrap onto a second line so that the continuation indent can be measured\n",
    "  against the reference output of pdflatex exactly.\n",
    "\\end{description}\n",
    "\n",
    "Bravo paragraph one line only here.\n",
    "\\end{document}\n",
);

/// `style=sameline` is the default same-line label: no break is forced.
const SAMELINE_SOURCE: &str = concat!(
    "\\documentclass[11pt]{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage{lmodern}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\usepackage{enumitem}\n",
    "\\pagestyle{empty}\n",
    "\\begin{document}\n",
    "\\begin{description}[style=sameline]\n",
    "  \\item[First label] Short body here.\n",
    "\\end{description}\n",
    "\\end{document}\n",
);

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

#[test]
fn nextline_puts_each_body_on_its_own_line_at_the_hanging_indent() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SOURCE);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let words = words_of(&r);

    close(word(&words, "Alpha").baseline, 82.959, "paragraph above the list");
    // The label keeps its ordinary placement: bold, flush at the margin.
    close(word(&words, "First").baseline, 108.463, "first label baseline");
    close(word(&words, "First").x, 72.000, "first label flush at the text margin");
    // The body does NOT follow the label on the same line: it starts one
    // `\baselineskip` (13.6pt at 11pt) below, at the hanging indent.
    close(word(&words, "Description").baseline, 122.012, "body starts one line below its label");
    close(word(&words, "Description").x, 99.273, "body starts at the hanging indent");
    // ... which is exactly where a wrapped continuation line starts.
    close(word(&words, "continuation").baseline, 135.562, "wrapped continuation baseline");
    close(word(&words, "continuation").x, 99.273, "continuation hangs at the same indent");

    // The second item breaks the same way regardless of label width.
    close(word(&words, "Second").baseline, 158.077, "second label baseline");
    close(word(&words, "Second").x, 72.000, "second label flush at the text margin");
    close(word(&words, "Another").baseline, 171.626, "second body starts one line below its label");
    close(word(&words, "Another").x, 99.273, "second body starts at the hanging indent");
    close(word(&words, "up").baseline, 185.176, "second wrapped continuation baseline");
    close(word(&words, "up").x, 99.273, "second continuation hangs at the same indent");

    close(word(&words, "Bravo").baseline, 210.680, "paragraph after \\end{description}");
}

#[test]
fn nextline_via_setlist_breaks_after_the_label() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SETLIST_SOURCE);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let words = words_of(&r);

    let (label, body, wrapped) = (word(&words, "First"), word(&words, "Description"), word(&words, "continuation"));
    assert!(body.baseline > label.baseline + 5.0, "body below its label: {} vs {}", body.baseline, label.baseline);
    assert!(
        (body.baseline - label.baseline - 13.6).abs() < 1.0,
        "body one line below its label: {} vs {}",
        body.baseline,
        label.baseline
    );
    assert!((body.x - wrapped.x).abs() < TOL, "body at the hanging indent: {} vs {}", body.x, wrapped.x);
}

#[test]
fn sameline_keeps_the_body_beside_the_label() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SAMELINE_SOURCE);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let words = words_of(&r);

    let (label, body) = (word(&words, "First"), word(&words, "Short"));
    assert!(
        (body.baseline - label.baseline).abs() < 0.01,
        "body beside its label: {} vs {}",
        body.baseline,
        label.baseline
    );
    assert!(body.x > label.x, "body after its label: {} vs {}", body.x, label.x);
}
