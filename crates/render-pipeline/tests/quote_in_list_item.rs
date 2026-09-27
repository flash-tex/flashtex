//! `quotation` (and `quote`) inside an `itemize`/`enumerate` item.
//!
//! pdflatex oracle (TeX Live 2026, article 11pt, US Letter, no geometry):
//! ```
//! \documentclass[11pt]{article}
//! \begin{document}
//! \begin{itemize}
//! \item Item text.
//! \begin{quotation}
//! Quotation inside item.
//!
//! Second in item.
//! \end{quotation}
//! \end{itemize}
//! \end{document}
//! ```
//! compiled with `pdflatex -interaction=nonstopmode main.tex`; word boxes
//! read with pymupdf (`page.get_text("words")`):
//! `Quotation x0=193.435`, `Second x0=193.435` (bp from the page's left
//! edge). Per `\showoutput`, the quotation lines are
//! `\hbox(...)x284.44489, shifted 51.46509` (the item's `\@totalleftmargin`
//! 27.37506pt plus the nested list's own `\leftmargin` 24.09003pt, i.e.
//! `\@listii`'s `\leftmarginii` 2.2em) opening with the `\listparindent`
//! 1.5em = 16.42503pt box, so both paragraphs start at the same x.

mod common;

use common::*;

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

/// TeX points to PDF points, the unit of every v2 coordinate.
fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

#[test]
fn quotation_inside_itemize_item_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass[11pt]{article}\\begin{document}\n\\begin{itemize}\n\\item Item text.\n\\begin{quotation}\nQuotation inside item.\n\nSecond in item.\n\\end{quotation}\n\\end{itemize}\n\\end{document}";
    let r = render_one(src);
    assert_ne!(r.v2.pages.len(), 0, "expected a page: {:?}", r.v2.diagnostics);
    let words = words_of(&r);
    let q = word(&words, "Quotation");
    let s = word(&words, "Second");
    // Measured from pdflatex's PDF (see module docs): tight 0.1bp tolerance.
    assert!((q.x - 193.435).abs() < 0.1, "Quotation at pdflatex x 193.435bp, got {}bp ({q:?})", q.x);
    assert!((s.x - 193.435).abs() < 0.1, "Second at pdflatex x 193.435bp, got {}bp ({s:?})", s.x);
}

#[test]
fn top_level_quote_is_unchanged() {
    if !lm_available() {
        return;
    }
    // A top-level `quote` still starts at `\leftmargini` from the text edge:
    // the enclosing-item margin must not leak into it.
    let src = "\\documentclass[11pt]{article}\\begin{document}\nBefore.\n\\begin{quote}\nQuoted line here.\n\\end{quote}\n\\end{document}";
    let r = render_one(src);
    let words = words_of(&r);
    let s = flashtex_render_pipeline::Stylesheet::article(
        11,
        flashtex_render_pipeline::fonts::Family::LatinModern,
        None,
    );
    let text_x = bp(s.text_x_pt);
    let quoted = word(&words, "Quoted");
    assert!(
        (quoted.x - (text_x + bp(s.leftmargini_pt))).abs() < 0.1,
        "top-level quote at text edge + \\leftmargini: {} vs {}",
        quoted.x,
        text_x + bp(s.leftmargini_pt)
    );
}
