//! `description` is a `\list`, not free-standing paragraphs.
//!
//! article.cls builds it as
//!
//! ```tex
//! \newenvironment{description}
//!   {\list{}{\labelwidth\z@ \itemindent-\leftmargin
//!            \let\makelabel\descriptionlabel}}
//!   {\endlist}
//! \newcommand*\descriptionlabel[1]{\hspace\labelsep
//!   \normalfont\bfseries #1}
//! ```
//!
//! so it carries exactly the `\topsep`/`\partopsep`/`\itemsep`/`\parsep` of
//! `itemize` and the same closing `\@endparenv` skip, its items hang
//! `\leftmargin` in from the second line on while the first line is flush at
//! the text margin, and its label is bold with `\labelsep` before it.
//!
//! Every expected coordinate below was measured with
//! `tools/visual-oracle/pdftext.py` on the PDF pdfTeX 3.141592653-2.6-1.40.27
//! (TeX Live 2025, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) produced from
//! the same source; the oracle is not run here. Coordinates are bp from the
//! page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.3;

const SOURCE: &str = concat!(
    "\\documentclass[11pt]{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage{lmodern}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\pagestyle{empty}\n",
    "\\begin{document}\n",
    "Alpha paragraph one line only here.\n",
    "\n",
    "\\begin{description}\n",
    "  \\item[First label] Description item body text here which is long enough to\n",
    "  wrap onto a second line so that the continuation indent can be measured\n",
    "  against the reference output of pdflatex exactly.\n",
    "  \\item[Second label] Another item body, also long enough to wrap onto a\n",
    "  second line so the hanging indent shows up twice in this probe document.\n",
    "\\end{description}\n",
    "\n",
    "Bravo paragraph one line only here.\n",
    "\n",
    "\\begin{description}\n",
    "  \\item[Solo] Short.\n",
    "\\end{description}\n",
    "\n",
    "Charlie paragraph.\n",
    "\\end{document}\n",
);

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

#[test]
fn description_items_carry_the_list_skips_the_hanging_indent_and_a_bold_label() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SOURCE);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let words = words_of(&r);

    // `\@trivlist` opens the list with `\topsep` (9pt) + `\partopsep` (3pt),
    // because the `\begin{description}` was read in vertical mode: 13.6pt of
    // `\baselineskip` plus 12pt, 25.504 bp below "Alpha".
    close(word(&words, "Alpha").baseline, 82.959, "paragraph above the list");
    close(word(&words, "First").baseline, 108.463, "first item is \\topsep + \\partopsep below");
    // `\itemindent-\leftmargin`: the item's first line is flush at the text
    // margin, and `\descriptionlabel`'s `\hspace\labelsep` is inside the
    // label box, so the bold label starts exactly on the margin.
    close(word(&words, "First").x, 72.000, "label flush at the text margin");
    // The label is bold, which is what puts the body text at 134.140 rather
    // than the ~4 bp further left a medium-weight label would leave.
    close(word(&words, "Description").x, 134.140, "body follows the bold label by \\labelsep");
    // Continuation lines hang `\leftmargini` (2.5em = 27.375pt) in.
    close(word(&words, "that").x, 99.273, "continuation line hangs \\leftmargin in");
    close(word(&words, "that").baseline, 122.012, "continuation baseline");

    // `\itemsep + \parsep` between items (4.5pt each at 11pt).
    close(word(&words, "Second").baseline, 144.528, "second item baseline");
    close(word(&words, "Second").x, 72.000, "second label flush at the margin");
    close(word(&words, "Another").x, 146.277, "second body after its bold label");

    // `\endlist` -> `\@endparenv`'s `\addvspace\@topsepadd`: the same 12pt
    // below the list as above it.
    close(word(&words, "Bravo").baseline, 183.582, "paragraph after \\end{description}");

    // A one-line item: `\labelwidth` is zero, so a short label is never
    // padded out and the body still follows it by `\labelsep` alone.
    close(word(&words, "Solo").baseline, 209.086, "second list opens with \\topsep + \\partopsep");
    close(word(&words, "Solo").x, 72.000, "short label is not padded to a \\labelwidth");
    close(word(&words, "Short.").x, 100.454, "body after a short bold label");
}
