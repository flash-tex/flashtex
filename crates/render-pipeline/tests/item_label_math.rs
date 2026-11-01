//! `\item[<label>]` whose label holds math (issue #676).
//!
//! The compiler parses the bracketed argument into inlines, so `\item[this
//! is $2x$]` carries an `Inline::Math`; the typesetter used to set only the
//! label's flattened text through `word_box`, which cannot set math, so
//! `this is $2x$` came out as `this is` and `\item[$\alpha$]` as nothing.
//! `ListGeom::label_items` now carries the label's own items and
//! `label_box_items` sets them as one horizontal list, exactly as
//! `\@item`'s `\hbox` does.
//!
//! Expected coordinates measured with `tools/visual-oracle/pdftext.py` on
//! the PDF pdflatex (TeX Live 2026) produced from exactly this source; the
//! oracle is not run here. Coordinates are bp from the page's top-left
//! corner.

mod common;

use common::*;

const TOL: f64 = 0.3;

const SRC: &str = concat!(
    "\\documentclass{article}\n",
    "\\begin{document}\n",
    "\\begin{description}\n",
    "\\item[this is $2x$] Text.\n",
    "\\item[$\\alpha$] Alpha.\n",
    "\\item[plain] Plain.\n",
    "\\end{description}\n",
    "\\begin{itemize}\n",
    "\\item[($2x$)] Paren math label.\n",
    "\\end{itemize}\n",
    "\\end{document}\n",
);

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

#[test]
fn explicit_item_labels_keep_their_math() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(SRC);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let w = words_of(&r);

    // `this is $2x$`: the words are bold, the math follows the interword
    // glue, and the body follows the whole label by `\labelsep`.
    close(word(&w, "this").x, 133.768, "label flush at the margin");
    close(word(&w, "is").x, 156.105, "second label word");
    let two: Vec<&Word> = w.iter().filter(|x| x.text == "2").collect();
    assert_eq!(two.len(), 2, "one `2` per math label: {w:?}");
    close(two[0].x, 167.632, "math digit in the description label");
    close(two[0].baseline, 134.765, "math on the label's baseline");
    let xs: Vec<&Word> = w.iter().filter(|x| x.text == "x").collect();
    assert_eq!(xs.len(), 2, "one `x` per math label: {w:?}");
    close(xs[0].x + xs[0].width, 178.307, "math letter ends the label");
    close(word(&w, "Text.").x, 183.288, "body after the math label");

    // `$\alpha$` alone: the label is the math box.
    let alpha = word(&w, "α");
    close(alpha.x, 133.768, "alpha flush at the margin");
    close(alpha.width, 6.373, "alpha width");
    close(alpha.baseline, 154.690, "alpha baseline");
    close(word(&w, "Alpha.").x, 145.162, "body after the alpha label");

    // A plain label is unchanged.
    close(word(&w, "plain").x, 133.768, "plain label");
    close(word(&w, "Plain.").x, 163.413, "body after the plain label");

    // itemize: `\llap`, the label's right edge `\labelsep` before the text.
    close(word(&w, "(").x, 135.270, "paren opens the itemize label");
    close(two[1].x, 139.144, "math digit in the itemize label");
    close(word(&w, ")").x + word(&w, ")").width, 153.694, "paren closes the label");
    close(word(&w, "Paren").x, 158.675, "body after the itemize label");
}
