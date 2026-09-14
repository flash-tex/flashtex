//! article.cls's `description` (lines 400-404):
//!
//! ```tex
//! \newenvironment{description}
//!   {\list{}{\labelwidth\z@ \itemindent-\leftmargin
//!            \let\makelabel\descriptionlabel}}{\endlist}
//! \newcommand*\descriptionlabel[1]{\hspace\labelsep\normalfont\bfseries #1}
//! ```
//!
//! Its `\leftmargin` is the class's `\leftmargin<i>` (2.5 em = 25 pt at
//! 10 pt), so the *continuation* lines of an item hang there, while
//! `\itemindent-\leftmargin` pulls the first line back out so the term sets
//! flush at the enclosing margin. The pipeline used to know only
//! `itemize`/`enumerate`/`thebibliography`, so a `description` item got an
//! empty margin stack — no hanging indent, and the list's own `\topsep`
//! never contributed either.
//!
//! Every expected coordinate below was measured with
//! `tools/visual-oracle/pdftext.py` on the PDF pdflatex (TeX Live 2025,
//! pdfTeX 3.141592653-2.6-1.40.27, `SOURCE_DATE_EPOCH=0
//! FORCE_SOURCE_DATE=1`) produced from exactly this source — it is
//! `fixtures/divergence-probes/min-description/main.tex`, whose committed
//! `reference.pdf` is that PDF. The oracle is not run here; pdflatex is an
//! oracle only and never in the product path. Coordinates are bp from the
//! page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.3;

const SRC: &str = concat!(
    "\\documentclass{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\begin{document}\n\n",
    "Body text before the list.\n\n",
    "\\begin{description}\n",
    "  \\item[First term] its explanation, which runs on for long enough to wrap\n",
    "    onto a second line and show the hanging indentation.\n",
    "  \\item[A much longer term] its explanation.\n",
    "\\end{description}\n\n",
    "Body text after the list.\n",
    "\\end{document}\n",
);

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

#[test]
fn description_hangs_its_body_at_leftmargin_and_sets_the_term_flush() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SRC);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let w = words_of(&r);

    // The paragraph around the list, at the text margin (72) plus
    // `\parindent` (15 pt = 14.944 bp).
    close(word(&w, "Body").x, 86.944, "body before x");
    close(word(&w, "Body").baseline, 81.963, "body before baseline");

    // `\itemindent-\leftmargin`: the term is flush at the text margin, not
    // `\leftmargin` in and not `\labelsep` out to its left.
    let first = word(&w, "First");
    close(first.x, 72.000, "term x");
    close(first.baseline, 103.880, "first item baseline");
    // `\descriptionlabel` is one `\hbox`, so its interword gap is TeX's glue
    // at natural width -- not the T1 `visiblespace` glyph, which is 1.91 bp
    // wider on `ecbx1000` and used to push the rest of the line right.
    close(word(&w, "term").x, 99.894, "second word of the term");
    // The body resumes `\labelsep` after the term box.
    close(word(&w, "its").x, 128.843, "item text after the term");

    // The continuation line hangs at `\leftmargin` (2.5 em = 25 pt).
    let cont = word(&w, "indentation.");
    close(cont.x, 96.907, "continuation line x");
    close(cont.baseline, 115.836, "continuation line baseline");

    // Second item: `\itemsep + \parsep` below the first item's last line,
    // its term flush again, and its own words at the natural interword gap.
    let second = word(&w, "A");
    close(second.x, 72.000, "second term x");
    close(second.baseline, 135.761, "second item baseline");
    close(word(&w, "much").x, 84.475, "second word of the second term");
    close(word(&w, "longer").x, 115.025, "third word of the second term");

    // `\endlist`'s `\addvspace\@topsepadd` after the list: the paragraph
    // below sits 21.918 bp under the last item, not 9.96 bp.
    let after: Vec<&Word> = w.iter().filter(|x| x.text == "Body").collect();
    assert_eq!(after.len(), 2, "two `Body` words: {w:?}");
    close(after[1].x, 86.944, "body after x");
    close(after[1].baseline, 157.679, "body after baseline");
}
