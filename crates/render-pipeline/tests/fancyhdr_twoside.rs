//! fancyhdr in a `twoside` document (#1123): even pages ship the even
//! fields (`\@evenhead`, fancyhdr's `\ifodd\c@page` switch), from the `E`
//! groups of `\fancyhead`/`\fancyfoot` and the optional `[even]` argument
//! of `\lhead`...`\rfoot`, and fancyhdr's own twoside defaults
//! (`\fancyhead[el,or]{\rightmark}`, `[er,ol]{\leftmark}`).
//!
//! Oracle: pdflatex (TeX Live 2023, pdfTeX 1.40.25, fancyhdr 4.1;
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two runs), word origins read
//! with `tools/visual-oracle/pdftext.py`, in bp from the page's top-left.
//! The twoside head and foot code is the same in fancyhdr 5.2 (TeX Live
//! 2026) for fields without `\fancyheadwidth`.

mod common;

use common::{lm_available, render_one, words_of, Word};

fn find<'a>(words: &'a [Word], page: u32, text: &str) -> Option<&'a Word> {
    words.iter().find(|w| w.page == page && w.text.trim() == text)
}

fn assert_words(words: &[Word], page: u32, want: &[(&str, f64, f64)]) {
    for (text, x, top) in want {
        let got = find(words, page, text).unwrap_or_else(|| {
            panic!("page {page}: no word {text:?} in {:?}", words.iter().filter(|w| w.page == page).map(|w| (w.text.as_str(), w.x, w.baseline)).collect::<Vec<_>>())
        });
        assert!(
            (got.x - x).abs() <= 0.02 && (got.baseline - top).abs() <= 0.02,
            "page {page} {text:?}: got ({:.3}, {:.3}), pdflatex ({x:.3}, {top:.3})",
            got.x,
            got.baseline
        );
    }
}

/// #1123's falsifier: fancyhdr's defaults. Page 2 is even, so `\leftmark`
/// (`2 SECOND PART`) is the even page's right field, ending at the right
/// edge of the even page's text block; one-sided it would be the left one.
#[test]
fn twoside_defaults_put_the_left_mark_right_on_even_pages() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = "\\documentclass[twoside]{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\n\
               \\begin{document}\n\\section{Introduction}\nText on page one.\n\\subsection{Details here}\nMore.\n\
               \\newpage\n\\section{Second part}\nPage two.\n\\end{document}\n";
    let words = words_of(&render_one(doc));
    assert_words(&words, 1, &[("1", 106.869, 96.309), ("INTRODUCTION", 121.813, 96.309), ("Text", 106.869, 156.586)]);
    assert_words(&words, 2, &[("2", 416.652, 96.309), ("SECOND", 431.596, 96.309), ("PART", 477.255, 96.309), ("Page", 160.667, 156.586)]);
}

/// `E`/`O` groups and `\lhead[even]{odd}`: each page parity ships its own
/// head and foot fields, odd again on page 3.
#[test]
fn even_and_odd_fields_ship_on_their_own_pages() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = "\\documentclass[twoside]{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\n\\fancyhf{}\n\
               \\fancyhead[LE,RO]{\\thepage}\n\\fancyhead[RE]{Even right}\n\\fancyhead[LO]{Odd left}\n\
               \\fancyfoot[CE]{Even foot}\n\\fancyfoot[CO]{Odd foot}\n\\lhead[Lhead even]{Lhead odd}\n\
               \\begin{document}\nOne.\n\\newpage\nTwo.\n\\newpage\nThree.\n\\end{document}\n";
    let words = words_of(&render_one(doc));
    let odd = |page: u32, number: &'static str| {
        [("Lhead", 106.869, 96.309), ("odd", 136.892, 96.309), (number, 445.595, 96.309), ("Odd", 259.076, 702.635), ("foot", 281.213, 702.635)].into_iter().map(move |w| (page, w))
    };
    for (page, w) in odd(1, "1").chain(odd(3, "3")) {
        assert_words(&words, page, &[w]);
    }
    assert_words(&words, 2, &[("Lhead", 160.667, 96.309), ("even", 190.690, 96.309), ("Even", 458.547, 96.309), ("right", 483.597, 96.309), ("foot", 336.462, 702.635)]);
    // The odd fields are not on the even page, nor the even ones on odd pages.
    assert!(find(&words, 2, "odd").is_none() && find(&words, 2, "Odd").is_none());
    assert!(find(&words, 1, "even").is_none() && find(&words, 3, "right").is_none());
}

/// One-sided, the same commands ship the odd fields on every page, as
/// `\@oddhead` does.
#[test]
fn a_one_sided_document_ships_the_odd_fields_on_even_pages() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\n\\fancyhf{}\n\
               \\fancyhead[LE]{Even}\n\\fancyhead[LO]{Odd}\n\
               \\begin{document}\nOne.\n\\newpage\nTwo.\n\\end{document}\n";
    let words = words_of(&render_one(doc));
    assert!(find(&words, 2, "Odd").is_some(), "{:?}", words.iter().filter(|w| w.page == 2).map(|w| &w.text).collect::<Vec<_>>());
    assert!(find(&words, 2, "Even").is_none());
}
