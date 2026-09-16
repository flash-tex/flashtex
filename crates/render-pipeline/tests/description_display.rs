//! A `description` item whose body opens with a display.
//!
//! `\@item` holds the label in `\@labels` and `\everypar` sets it when the
//! paragraph opens, so the term stands on a line of its own and the display
//! follows. That line is built by `display_opener_block`, which is the one
//! path `description`'s `\labelwidth\z@ \itemindent-\leftmargin` had not
//! reached: it still placed the term at `\leftmargin - \labelsep`, i.e.
//! 19.93 bp right of the margin instead of flush on it.
//!
//! Expected coordinates measured with `tools/visual-oracle/pdftext.py` on
//! the PDF pdflatex (TeX Live 2025, pdfTeX 3.141592653-2.6-1.40.27,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) produced from exactly this
//! source. The oracle is not run here; pdflatex is an oracle only and never
//! in the product path. Coordinates are bp from the page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.3;

const SRC: &str = concat!(
    "\\documentclass{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\begin{document}\n",
    "Body text before the list.\n\n",
    "\\begin{description}\n",
    "\\item[A long term] \\[ x + y = z \\]\n",
    "  and the explanation after the display, which wraps onto a second line so the\n",
    "  hanging indent is visible.\n",
    "\\item[Second term] plain body text that also wraps onto a second line for the\n",
    "  same reason as the first item above.\n",
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
fn a_description_item_opening_with_a_display_sets_its_term_flush() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SRC);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let w = words_of(&r);

    // The term is flush at the text margin, not `\leftmargin - \labelsep` in.
    let term = word(&w, "A");
    close(term.x, 72.000, "term x");
    close(term.baseline, 100.737, "term baseline");
    close(word(&w, "long").x, 84.475, "second word of the term");
    close(word(&w, "term").x, 109.291, "third word of the term");

    // The display itself is centred in `\linewidth`, `\@totalleftmargin` in.
    close(word(&w, "x").x, 297.720, "display x");
    close(word(&w, "x").baseline, 122.655, "display baseline");

    // The text after the display hangs at `\leftmargin` (2.5 em = 25 pt at
    // 10 pt), and an item that opens with text takes the same term rule.
    close(word(&w, "and").x, 96.907, "body after the display hangs at \\leftmargin");
    close(word(&w, "and").baseline, 144.573, "body after the display, baseline");
    close(word(&w, "Second").x, 72.000, "second item's term x");
    close(word(&w, "Second").baseline, 164.498, "second item's baseline");

    // The paragraphs around the list are unmoved.
    let body: Vec<&Word> = w.iter().filter(|x| x.text == "Body").collect();
    assert_eq!(body.len(), 2, "two `Body` words: {w:?}");
    close(body[0].x, 86.944, "body before x");
    close(body[1].x, 86.944, "body after x");
}
