//! fancyhdr's running head and foot on the exact route, and the other
//! constructs of the 21-242 proof-practice fixture
//! (`fixtures/real-world/proof-practice-21242`) that are not its
//! fixed-height minipages.
//!
//! Oracle for every number here: pdflatex, TeX Live 2026
//! (`/usr/local/texlive/2026/bin/universal-darwin/pdflatex`,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two runs), word origins read
//! from its PDF with `tools/visual-oracle/pdftext.py`, in bp from the page's
//! top-left corner. pdfTeX strokes a thin rule as a line; its `(x, top,
//! width, height)` below is the stroke's box.
//!
//! - `FANCY` sets the fixture's own head and foot. Its head names
//!   `\topicshort` before `\newcommand` defines it and the body
//!   `\renewcommand`s it: fancyhdr expands the fields at shipout, so page 1
//!   reads `Problem collection` and page 2 `Axioms and abstraction`. The
//!   head's `[b]` parboxes end in `\strut`, so the text sits the `\small`
//!   strut's depth (3.6pt) above the head rule's top; the foot is at the
//!   class's foot baseline. microtype protrudes the foot's `T` 0.359bp into
//!   the left margin; the page numbers 7 and 8 do not protrude on the right
//!   (the field ends in the strut's box).
//! - `RULED` is the fixture's `\ruled{7}`: a group with `\parskip=0pt` and
//!   `\baselineskip=17pt`, a `\loop` of seven `\textcolor{black!17}` rules,
//!   16.937bp apart, gray 0.83.

mod common;

use common::{lm_available, render_one, rules_of, words_of, Word};
use flashtex_render_pipeline::display::Item;

const FANCY: &str = "\\documentclass[11pt,letterpaper]{article}\n\
\\usepackage[margin=.78in,headheight=15pt]{geometry}\n\
\\usepackage[T1]{fontenc}\n\
\\usepackage{lmodern,microtype,fancyhdr,hyperref}\n\
\\pagestyle{fancy}\\fancyhf{}\\fancyhead[L]{\\small\\sffamily 21-242 / PROOF PRACTICE}\\fancyhead[R]{\\small\\sffamily\\topicshort}\\fancyfoot[L]{\\small Through change of basis}\\fancyfoot[R]{\\thepage}\n\
\\newcommand{\\topicshort}{Problem collection}\n\
\\begin{document}\n\
\\setcounter{page}{7}\n\
Body one.\n\
\\newpage\n\
\\renewcommand{\\topicshort}{Axioms and abstraction}\n\
\\pdfbookmark[1]{1. Fields}{topic1}\n\
Body two.\n\
\\end{document}\n";

/// pdflatex's words, per page: (text, x, top).
const FANCY_PAGE_1: &[(&str, f64, f64)] = &[
    ("21-242", 56.160, 27.667),
    ("PROOF", 96.013, 27.667),
    ("PRACTICE", 132.471, 27.667),
    ("Problem", 478.769, 27.667),
    ("collection", 516.708, 27.667),
    ("Body", 73.096, 67.119),
    ("one.", 102.033, 67.119),
    ("Through", 55.801, 765.728),
    ("change", 96.781, 765.728),
    ("of", 129.718, 765.728),
    ("basis", 141.060, 765.728),
    ("7", 550.386, 765.728),
];

const FANCY_PAGE_2: &[(&str, f64, f64)] = &[
    ("21-242", 56.160, 27.667),
    ("PROOF", 96.013, 27.667),
    ("PRACTICE", 132.471, 27.667),
    ("Axioms", 457.708, 27.667),
    ("and", 491.356, 27.667),
    ("abstraction", 509.767, 27.667),
    ("Body", 73.096, 67.119),
    ("two.", 102.033, 67.119),
    ("Through", 55.801, 765.728),
    ("change", 96.781, 765.728),
    ("of", 129.718, 765.728),
    ("basis", 141.060, 765.728),
    ("8", 550.386, 765.728),
];

/// The head rule on both pages: `\headrulewidth` 0.4pt under the head box.
const HEAD_RULE: (f64, f64, f64, f64) = (56.160, 31.254, 499.680, 0.398);

const RULED: &str = "\\documentclass[11pt,letterpaper]{article}\n\
\\usepackage{xcolor}\n\
\\setlength{\\parindent}{0pt}\\setlength{\\parskip}{6pt}\n\
\\newcommand{\\ruled}[1]{\\par\\begingroup\\parskip=0pt\\baselineskip=17pt\\count255=0\\loop\\ifnum\\count255<#1\\noindent\\textcolor{black!17}{\\rule{\\linewidth}{.25pt}}\\par\\advance\\count255 by1\\repeat\\endgroup}\n\
\\begin{document}\n\
Before the rules.\n\
\\ruled{7}\n\
After the rules.\n\
\n\
Second paragraph.\n\
\\end{document}\n";

const RULED_WORDS: &[(&str, f64, f64)] = &[
    ("Before", 125.798, 140.738),
    ("After", 125.798, 278.824),
    ("Second", 125.798, 298.349),
];

/// The seven rules' tops (x 125.798, width 358.655, height 0.249).
const RULED_TOPS: [f64; 7] = [157.429, 174.366, 191.303, 208.240, 225.175, 242.112, 259.048];

fn find<'a>(words: &'a [Word], page: u32, text: &str, nth: usize) -> Option<&'a Word> {
    words.iter().filter(|w| w.page == page && w.text.trim() == text).nth(nth)
}

fn assert_words(words: &[Word], page: u32, want: &[(&str, f64, f64)], tol: f64) {
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (text, x, top) in want {
        let n = seen.entry(text).or_insert(0);
        let got = find(words, page, text, *n).unwrap_or_else(|| {
            panic!(
                "page {page}: no word {text:?} in {:?}",
                words.iter().filter(|w| w.page == page).map(|w| w.text.as_str()).collect::<Vec<_>>()
            )
        });
        *n += 1;
        assert!(
            (got.x - x).abs() <= tol && (got.baseline - top).abs() <= tol,
            "page {page} {text:?}: got ({:.3}, {:.3}), pdflatex ({x:.3}, {top:.3})",
            got.x,
            got.baseline
        );
    }
}

#[test]
fn fancy_head_and_foot_match_pdflatex_with_the_field_macro_in_force_at_each_page() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let rendered = render_one(FANCY);
    let errors: Vec<_> = rendered
        .v2
        .diagnostics
        .iter()
        .filter(|d| format!("{d:?}").contains("Error"))
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(rendered.v2.pages.len(), 2);
    let words = words_of(&rendered);
    assert_words(&words, 1, FANCY_PAGE_1, 0.02);
    assert_words(&words, 2, FANCY_PAGE_2, 0.02);
    // The first definition never reaches page 2, nor the second page 1.
    assert!(find(&words, 2, "Problem", 0).is_none());
    assert!(find(&words, 1, "Axioms", 0).is_none());
    // `\pdfbookmark` typesets nothing: no bookmark text, no `[1]`.
    assert!(!words.iter().any(|w| w.text.contains("Fields") || w.text.contains("topic1") || w.text.contains("[1]")), "{words:?}");
    // The head rule, once per page.
    for (i, page) in rules_of(&rendered).iter().enumerate() {
        let near: Vec<_> = page
            .iter()
            .filter(|r| (r.0 - HEAD_RULE.0).abs() < 0.02 && (r.1 - HEAD_RULE.1).abs() < 0.02 && (r.2 - HEAD_RULE.2).abs() < 0.02 && (r.3 - HEAD_RULE.3).abs() < 0.02)
            .collect();
        assert_eq!(near.len(), 1, "page {}: {page:?}", i + 1);
    }
}

#[test]
fn a_page_style_switch_away_from_fancy_drops_its_chrome() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `\thispagestyle{plain}` on page 1 only: the plain foot (a centred
    // page number) there, fancyhdr's head and foot from page 2 on.
    let src = FANCY.replace("\\setcounter{page}{7}\n", "\\setcounter{page}{7}\\thispagestyle{plain}\n");
    let rendered = render_one(&src);
    let words = words_of(&rendered);
    assert!(find(&words, 1, "PROOF", 0).is_none(), "{words:?}");
    assert!(find(&words, 1, "Through", 0).is_none(), "{words:?}");
    assert!(find(&words, 2, "PROOF", 0).is_some(), "{words:?}");
}

#[test]
fn ruled_draws_seven_grey_rules_17pt_apart_like_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let rendered = render_one(RULED);
    assert!(
        !rendered.v2.diagnostics.iter().any(|d| format!("{d:?}").contains("not implemented")),
        "{:?}",
        rendered.v2.diagnostics
    );
    let words = words_of(&rendered);
    assert_words(&words, 1, RULED_WORDS, 0.02);
    let rules: Vec<_> = rendered.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|it| match it {
            Item::Rule(rule) => Some(rule.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(rules.len(), 7, "{rules:?}");
    for (rule, top) in rules.iter().zip(RULED_TOPS) {
        assert!((rule.x.to_bp() - 125.798).abs() < 0.02, "{rule:?}");
        assert!((rule.top.to_bp() - top).abs() < 0.02, "top {} vs pdflatex {top}", rule.top.to_bp());
        assert!((rule.width.to_bp() - 358.655).abs() < 0.02, "{rule:?}");
        assert!((rule.height.to_bp() - 0.249).abs() < 0.02, "{rule:?}");
        // `black!17`: pdfTeX's `0.83 g`.
        assert!((rule.paint.r - 0.83).abs() < 0.005 && (rule.paint.g - 0.83).abs() < 0.005, "{:?}", rule.paint);
    }
}
