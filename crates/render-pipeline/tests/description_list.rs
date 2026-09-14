//! `description` geometry (article.cls lines 360-365):
//!
//! ```tex
//! \newenvironment{description}
//!     {\list{}{\labelwidth\z@ \itemindent-\leftmargin
//!              \let\makelabel\descriptionlabel}}
//!     {\endlist}
//! \newcommand*\descriptionlabel[1]{\hspace\labelsep
//!     \normalfont\bfseries #1}
//! ```
//!
//! `\list` still sets `\parshape` so every line hangs at
//! `\@totalleftmargin` (`\leftmargini` = 2.5 em at the outermost level),
//! but `\itemindent-\leftmargin` takes that whole margin back off the
//! first line, `\labelwidth\z@` means the label box is never padded, and
//! `\descriptionlabel`'s leading `\hspace\labelsep` cancels `\@item`'s
//! `\hskip-\labelsep`. The term therefore starts flush on the left margin
//! in the bold body face and the item text follows one `\labelsep` after
//! it.
//!
//! Expected coordinates were measured on the pdflatex PDF of
//! `fixtures/divergence-probes/min-description` (TeX Live 2025,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) with
//! `tools/visual-oracle/pdftext.py`; the oracle is not run here.
//! Coordinates are bp from the page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.05;

const SRC: &str = "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage[margin=1in]{geometry}\n\
\\begin{document}\n\nBody text before the list.\n\n\\begin{description}\n\
  \\item[First term] its explanation, which runs on for long enough to wrap\n\
    onto a second line and show the hanging indentation.\n\
  \\item[A much longer term] its explanation.\n\\end{description}\n\n\
Body text after the list.\n\\end{document}\n";

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

#[test]
fn description_terms_start_on_the_left_margin_and_the_body_hangs_at_leftmargini() {
    if !lm_available() {
        return;
    }
    let r = render_one(SRC);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    let words = words_of(&r);

    // `\itemindent-\leftmargin`: the term is flush on the text-area's left
    // edge, one whole `\leftmargini` left of the item's own margin.
    assert!((word(&words, "First").x - 72.000).abs() < TOL, "{:?}", word(&words, "First"));
    assert!((word(&words, "A").x - 72.000).abs() < TOL, "{:?}", word(&words, "A"));

    // The term's own words are one `\fontdimen2` of the bold face apart.
    assert!((word(&words, "term").x - 99.894).abs() < TOL, "{:?}", word(&words, "term"));

    // `\hskip\labelsep` after the label box, then the item text.
    assert!((word(&words, "its").x - 128.843).abs() < TOL, "{:?}", word(&words, "its"));

    // The wrapped line hangs at `\@totalleftmargin` = `\leftmargini`
    // (2.5 em of a 10 pt body = 25 pt = 24.907 bp).
    assert!((word(&words, "indentation.").x - 96.907).abs() < TOL, "{:?}", word(&words, "indentation."));
}

#[test]
fn description_terms_are_set_in_the_bold_body_face() {
    if !lm_available() {
        return;
    }
    let fonts = flashtex_render_pipeline::FontSet::with_default_dirs(&[]);
    let r = render_one_with(SRC, &fonts);
    let mut term = None;
    let mut body = None;
    for page in &r.v2.pages {
        for it in &page.items {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let name = fonts.by_font_id(&run.font_id).map(|f| f.name.clone()).unwrap_or_default();
                if run.text == "First" {
                    term = Some(name);
                } else if run.text == "explanation," {
                    body = Some(name);
                }
            }
        }
    }
    let term = term.expect("the term is set");
    let body = body.expect("the item body is set");
    assert!(term.to_lowercase().contains("bold") || term.contains("bx"), "the term is `\\bfseries`: {term}");
    assert!(!(body.to_lowercase().contains("bold") || body.contains("bx")), "the item body is not: {body}");
}
