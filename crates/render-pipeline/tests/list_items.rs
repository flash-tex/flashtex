//! `\item` bodies that start with a display, displays inside lists and the
//! glue LaTeX puts around them:
//!
//! * `\item \[ ... \]` — `\@item` holds the label in `\@labels` and sets it
//!   from `\everypar` when the display opens the paragraph, so the label
//!   stands on a line of its own and the display follows. Without amsmath
//!   `\[` first sets `\nointerlineskip\makebox[.6\linewidth]{}` (a wide
//!   pre-display line: the long `\abovedisplayskip`); under amsmath `\[`
//!   is `\begin{equation*}`, a bare `$$` (pre-display size
//!   `\@totalleftmargin + 2em`: the short skip, i.e. one `\baselineskip`).
//! * a display in a list is centred in `\linewidth` (`\displaywidth`),
//!   `\@totalleftmargin` (`\displayindent`) in from the margin — also
//!   inside a nested list.
//! * `\@item`'s `\addvspace\@topsep` tops up the previous display's
//!   `\belowdisplayskip` and `\addvspace{-\parskip}` then takes `\parsep`
//!   off; `\endtrivlist` changes a positive trailing skip by `\parsep -
//!   \@outerparskip`; `\@startsection`'s `\addvspace` replaces a smaller
//!   trailing skip instead of adding to it.
//!
//! Every expected coordinate below was measured on the PDF pdflatex
//! (MacTeX 2026 full, pdfTeX 1.40.29, `SOURCE_DATE_EPOCH=0`) produced from
//! the same source, with `tools/visual-oracle/pdftext.py`; the oracle is
//! not run here. Coordinates are bp from the page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.3;

fn render(src: &str) -> Vec<Word> {
    let r = render_one(src);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    words_of(&r)
}

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

/// Left edge, right edge and baseline of the line whose baseline lies
/// strictly between `above` and `below`.
fn line_between(words: &[Word], above: f64, below: f64) -> (f64, f64, f64) {
    let line: Vec<&Word> = words.iter().filter(|w| w.baseline > above + 0.5 && w.baseline < below - 0.5).collect();
    assert!(!line.is_empty(), "no line between {above} and {below}: {words:?}");
    let baseline = line[0].baseline;
    assert!(line.iter().all(|w| (w.baseline - baseline).abs() < 0.05), "one line expected: {line:?}");
    let left = line.iter().map(|w| w.x).fold(f64::INFINITY, f64::min);
    let right = line.iter().map(|w| w.x + w.width).fold(0.0, f64::max);
    (left, right, baseline)
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

#[test]
fn display_first_item_sets_the_label_on_its_own_line() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // 11pt article, no amsmath: `\[` sets the .6\linewidth box with
    // \nointerlineskip, so the label line sits \topsep+\parskip-\parsep
    // +\parsep = 9pt below "Intro text." with no interline glue (16.06pt =
    // 9pt + the label's 7.06pt height) and the display follows on the long
    // \abovedisplayskip: 11pt + 13.6pt. Item 2 comes 11pt (below skip,
    // absorbing \itemsep 4.5pt) + \parsep 4.5pt + 13.6pt later.
    let words = render(
        "\\documentclass[11pt]{article}\n\\begin{document}\nIntro text.\n\\begin{enumerate}\n\\item \\[ x = y \\]\n\\item Then\n\\[ a = b \\]\nafter.\n\\end{enumerate}\nOutro text.\n\\end{document}\n",
    );
    let intro = word(&words, "Intro");
    let one = word(&words, "1.");
    let two = word(&words, "2.");
    let after = word(&words, "after.");
    let outro = word(&words, "Outro");
    close(intro.baseline, 140.74, "intro baseline");
    close(one.x, 139.13, "label 1. x");
    close(one.baseline, 156.74, "label 1. on its own line");
    let (left, right, display) = line_between(&words, one.baseline, two.baseline);
    close(display, 181.25, "display baseline after the label line");
    // Centred in \linewidth, \leftmargini in: pdflatex's `x = y` spans
    // 305.50..331.64.
    close((left + right) / 2.0, (305.50 + 331.64) / 2.0, "display centre");
    close(two.x, 139.13, "label 2. x");
    close(two.baseline, 210.24, "second item baseline");
    let (_, _, display2) = line_between(&words, two.baseline, after.baseline);
    close(display2, 223.79, "second display baseline");
    close(after.x, 153.07, "continuation line starts \\leftmargini in");
    close(after.baseline, 243.81, "continuation baseline");
    close(outro.baseline, 266.33, "paragraph after the list");
}

#[test]
fn amsmath_display_first_item_uses_the_short_skip_and_lists_meet_headings_with_addvspace() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // HW2's preamble: amsmath's `\[` is a bare `$$`, so the display sits
    // one \baselineskip (13.6pt) under its label line; the `(b)` label
    // line is \belowdisplayshortskip 6.5pt (absorbing \itemsep 4.95pt) +
    // \parsep 4.5pt + 13.6pt below the display; `\subsection*` after the
    // last display replaces the 6.5pt trailing skip (adjusted by
    // \endtrivlist to 3.85pt) with its own 3.25ex \addvspace.
    let words = render(
        "\\documentclass[11pt]{article}\n\\usepackage{amsmath}\n\\usepackage[shortlabels]{enumitem}\n\\setlength{\\parindent}{0pt}\n\\setlength{\\parskip}{0.65em}\n\\setlist[enumerate]{leftmargin=*,itemsep=0.45em,topsep=0.35em}\n\\begin{document}\nIntro text.\n\\begin{enumerate}[(a)]\n\\item \\[ x = y \\]\n\\item Then\n\\[ a = b \\]\n\\end{enumerate}\n\\subsection*{Next}\nOutro text.\n\\end{document}\n",
    );
    let intro = word(&words, "Intro");
    let a = word(&words, "(a)");
    let b = word(&words, "(b)");
    let next = word(&words, "Next");
    let outro = word(&words, "Outro");
    close(intro.baseline, 140.74, "intro baseline");
    close(a.x, 129.44, "label (a) x");
    close(a.baseline, 165.20, "label (a) on its own line: \\topsep + \\parskip");
    let (left, right, display) = line_between(&words, a.baseline, b.baseline);
    close(display, 178.75, "display one \\baselineskip under the label (short skip)");
    close((left + right) / 2.0, (303.38 + 329.51) / 2.0, "display centre in \\linewidth");
    close(b.x, 128.83, "label (b) x");
    close(b.baseline, 203.26, "label (b) baseline");
    let (_, _, display2) = line_between(&words, b.baseline, next.baseline);
    close(display2, 216.81, "second display baseline");
    close(next.baseline, 253.11, "heading after a list ending in a display");
    close(outro.baseline, 280.80, "paragraph after the heading");
}

#[test]
fn nested_list_display_is_indented_by_both_margins() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // itemize > enumerate: the inner label hangs off \leftmargini +
    // \leftmarginii, the display is centred in the remaining width, the
    // inner item's \parskip is the level-2 \parsep (2pt) and the first
    // inner item's \@topsep uses the outer list's \parsep as its
    // \parskip. Closing both lists after the display: \endtrivlist turns
    // the 11pt below skip into 11 + 2 - 4.5 and then + 4.5 - 0 = 13pt.
    let words = render(
        "\\documentclass[11pt]{article}\n\\begin{document}\nIntro text.\n\\begin{itemize}\n\\item Outer\n\\begin{enumerate}\n\\item \\[ x = y \\]\n\\end{enumerate}\n\\end{itemize}\nOutro text.\n\\end{document}\n",
    );
    let outer = word(&words, "Outer");
    let one = word(&words, "1.");
    let outro = word(&words, "Outro");
    close(outer.baseline, 163.26, "outer item baseline");
    close(one.x, 163.13, "inner label x");
    close(one.baseline, 179.25, "inner label on its own line");
    let (left, right, display) = line_between(&words, one.baseline, outro.baseline);
    close(display, 203.76, "display baseline");
    close((left + right) / 2.0, (317.50 + 343.64) / 2.0, "display centre in the nested \\linewidth");
    close(outro.baseline, 230.26, "paragraph after both lists close");
}

#[test]
fn hw2_sets_three_pages_with_the_reference_breaks() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // fixtures/real-world/hw2: pdflatex breaks after Problem 3's first
    // display (page 1, last baseline 697.33) and after the hint's last
    // line (page 2, 717.92); the bonus problem is page 3 (`\newpage`).
    // Glyph widths differ (missing math glyphs are compiler-owned), so
    // only the page count and the per-page last baselines are checked.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/hw2/HW2.tex")).expect("HW2.tex");
    let words = render(&src);
    let pages = words.iter().map(|w| w.page).max().unwrap_or(0);
    assert_eq!(pages, 3, "HW2 page count");
    let last = |page: u32| words.iter().filter(|w| w.page == page).map(|w| w.baseline).fold(0.0, f64::max);
    assert!((last(1) - 697.33).abs() < 1.0, "page 1 last baseline {}", last(1));
    assert!((last(2) - 717.92).abs() < 1.0, "page 2 last baseline {}", last(2));
    // Problem 1's display-first items keep their labels.
    let labels: Vec<&Word> = words.iter().filter(|w| w.page == 1 && ["(a)", "(b)", "(c)"].contains(&w.text.as_str())).collect();
    assert!(labels.len() >= 5, "labels on page 1: {labels:?}");
}
