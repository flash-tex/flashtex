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

/// fancyhdr's defaults (`\f@nch@initialise`, one-sided article): the left
/// head `\slshape\rightmark`, the right head `\slshape\leftmark`, the centre
/// foot `\rmfamily\thepage`, and `\sectionmark` marking both sides with the
/// uppercased `\thesection\quad` title. pdflatex, TeX Live 2026: page 1's
/// head reads `1 INTRODUCTION` from x 381.17bp (the right mark: the
/// `\subsection`'s `\markright` came later on that page, but `\leftmark` is
/// the section's), page 2's `2 SECOND PART` from 389.75bp; both page numbers
/// at x 303.13bp, 702.63bp down.
const FANCY_DEFAULTS: &str = "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\n\
\\begin{document}\n\\section{Introduction}\nText on page one.\n\\subsection{Details here}\nMore.\n\
\\newpage\n\\section{Second part}\nPage two.\n\\end{document}\n";

#[test]
fn bare_pagestyle_fancy_sets_fancyhdrs_default_marks_and_page_number() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let rendered = render_one(FANCY_DEFAULTS);
    let words = words_of(&rendered);
    assert_words(&words, 1, &[("1", 381.17, 96.31), ("INTRODUCTION", 396.12, 96.31), ("Text", 133.77, 156.59)], 0.02);
    assert_words(&words, 2, &[("2", 389.75, 96.31), ("SECOND", 404.70, 96.31), ("PART", 450.36, 96.31)], 0.02);
    // The centred page number, after the body's words.
    let foot = |page: u32| words.iter().filter(|w| w.page == page && (w.baseline - 702.63).abs() < 0.02).map(|w| (w.text.clone(), w.x)).collect::<Vec<_>>();
    assert_eq!(foot(1).len(), 1, "{:?}", foot(1));
    assert!((foot(1)[0].1 - 303.13).abs() < 0.02 && foot(1)[0].0.trim() == "1", "{:?}", foot(1));
    assert!((foot(2)[0].1 - 303.13).abs() < 0.02 && foot(2)[0].0.trim() == "2", "{:?}", foot(2));
}

#[test]
fn a_field_follows_a_macro_reached_through_another_and_a_grouped_field_ends_with_its_group() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // pdflatex: `Topic: First` at 425.23bp on page 1, `Topic: Second` at
    // 415.77bp on page 2 (right-aligned).
    let rendered = render_one(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\\fancyhf{}\\fancyhead[R]{\\myhead}\\fancyfoot[C]{\\thepage}\n\
         \\newcommand{\\topicshort}{First}\\newcommand{\\myhead}{Topic: \\topicshort}\n\
         \\begin{document}\nOne.\n\\newpage\n\\renewcommand{\\topicshort}{Second}\nTwo.\n\\end{document}\n",
    );
    let words = words_of(&rendered);
    assert_words(&words, 1, &[("Topic:", 425.23, 96.31), ("First", 456.50, 96.31)], 0.02);
    assert_words(&words, 2, &[("Topic:", 415.77, 96.31), ("Second", 447.04, 96.31)], 0.02);
    // pdflatex: page 1 shipped inside the group, `Inner` at 294.12bp;
    // pages 2 and 3 `Outer` at 292.88bp.
    let rendered = render_one(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\\fancyhf{}\\fancyhead[C]{Outer}\n\
         \\begin{document}\n{\\fancyhead[C]{Inner}One.\\newpage}\nTwo.\n\\newpage\nThree.\n\\end{document}\n",
    );
    let words = words_of(&rendered);
    assert_words(&words, 1, &[("Inner", 294.12, 96.31)], 0.02);
    assert_words(&words, 2, &[("Outer", 292.88, 96.31)], 0.02);
    assert_words(&words, 3, &[("Outer", 292.88, 96.31)], 0.02);
}

#[test]
fn a_body_parskip_counts_where_its_paragraph_starts_and_only_from_there_on() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // pdflatex, TeX Live 2026: `{\parskip=12pt Second}\par` takes 12pt (TeX
    // appends `\parskip` when the paragraph starts, inside the group);
    // `Third` after the group 0pt; `{\parskip=30pt\par}` sets nothing; the
    // document-level `\parskip=20pt` moves `Fifth` and `Sixth` and none of
    // the paragraphs before it.
    let rendered = render_one(
        "\\documentclass{article}\n\\setlength{\\parskip}{0pt}\n\\begin{document}\nFirst paragraph.\n\n\
         {\\parskip=12pt Second paragraph.}\\par\nThird paragraph.\n\n{\\parskip=30pt\\par}\n\
         Fourth after a group that ended with par.\n\n\\parskip=20pt\nFifth.\n\nSixth.\n\\end{document}\n",
    );
    let words = words_of(&rendered);
    assert_words(
        &words,
        1,
        &[
            ("First", 148.71, 134.76),
            ("Second", 148.71, 158.67),
            ("Third", 148.71, 170.63),
            ("Fourth", 148.71, 182.59),
            ("Fifth.", 148.71, 214.47),
            ("Sixth.", 148.71, 246.35),
        ],
        0.02,
    );
}

#[test]
fn fields_read_the_sectioning_counters_of_the_page_they_ship_on() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // pdflatex: `No. <n>` left, `Section <n>` right, `<page> of 5` centred,
    // n = 1, 2, 3 on pages 1-3 (page 2's `\section{Two}` comes after
    // `Still one.`, and the head reads the counter when the page ships).
    let rendered = render_one(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\\fancyhf{}\\fancyhead[R]{Section \\thesection}\\fancyhead[L]{No. \\arabic{section}}\\fancyfoot[C]{\\thepage\\ of 5}\n\
         \\begin{document}\n\\section{One}\nText one.\n\\newpage\nStill one.\n\\section{Two}\nText two.\n\\newpage\n\
         \\section{Three}\n\\subsection{Sub}\nText three.\n\\end{document}\n",
    );
    let errors: Vec<_> = rendered.v2.diagnostics.iter().filter(|d| format!("{d:?}").contains("Error")).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let words = words_of(&rendered);
    for (page, n) in [(1u32, "1"), (2, "2"), (3, "3")] {
        let head: Vec<(String, f64)> = words
            .iter()
            .filter(|w| w.page == page && (w.baseline - 96.31).abs() < 0.02)
            .map(|w| (w.text.trim().to_string(), w.x))
            .collect();
        let text: Vec<&str> = head.iter().map(|(t, _)| t.as_str()).collect();
        assert!(text.contains(&n) && text.iter().any(|t| t.starts_with("No.")) && text.iter().any(|t| t.starts_with("Section")), "page {page}: {head:?}");
        let foot: Vec<(String, f64)> = words
            .iter()
            .filter(|w| w.page == page && (w.baseline - 702.63).abs() < 0.02)
            .map(|w| (w.text.trim().to_string(), w.x))
            .collect();
        assert!(foot.first().is_some_and(|(t, x)| t.starts_with(n) && (x - 293.31).abs() < 0.02), "page {page}: {foot:?}");
    }
    // The right field ends at the same edge on every page (one-digit
    // numbers): pdflatex's `Section` at 437.63bp.
    assert!(words.iter().any(|w| w.page == 3 && w.text.starts_with("Section") && (w.x - 437.63).abs() < 0.02), "{words:?}");
}

#[test]
fn a_field_wider_than_the_head_wraps_and_the_head_moves_down_by_the_excess() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // pdflatex: fancyhdr's default right head `\slshape\leftmark` of a long
    // section breaks into three `\raggedleft` lines of `\headwidth`; the
    // 34.43pt box exceeds `\headheight` (12pt), fancyhdr warns and the head
    // (rule included, top 122.245bp) sits 22.35bp lower.
    let rendered = render_one(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\n\\begin{document}\n\
         \\section{A very long section title that goes on and on and on across the whole head line and further still to force wrapping}\n\
         \\subsection{And an equally long subsection title that keeps going and going and going until it wraps as well}\nText.\n\\end{document}\n",
    );
    let words = words_of(&rendered);
    assert_words(
        &words,
        1,
        &[("1", 137.37, 94.75), ("VERY", 163.10, 94.75), ("GOES", 342.22, 94.75), ("ACROSS", 143.45, 106.70), ("FORCE", 442.47, 106.70), ("WRAPPING", 419.99, 118.66)],
        0.02,
    );
    let rules = rules_of(&rendered);
    assert!(rules[0].iter().any(|r| (r.1 - 122.245).abs() < 0.02 && (r.2 - 343.711).abs() < 0.02), "{:?}", rules[0]);
}

#[test]
fn a_field_command_inside_a_paragraph_heads_the_page_that_paragraph_starts() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // pdflatex, TeX Live 2026: TeX reads the whole paragraph `A
    // \fancyhead[C]{After}\newpage` before page 1 ships, so page 1 is headed
    // `After` (294.26bp), and so is page 2; a `\fancyhead` in the middle of
    // page 3's only paragraph heads page 3 (`Third`, 293.16bp).
    let rendered = render_one(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\\fancyhf{}\\fancyhead[C]{Before}\n\
         \\begin{document}\nA \\fancyhead[C]{After}\\newpage B\n\\newpage\n\
         Some text \\fancyhead[C]{Third} in the middle of a paragraph.\n\\end{document}\n",
    );
    let words = words_of(&rendered);
    assert_words(&words, 1, &[("After", 294.26, 96.31), ("A", 148.71, 134.76)], 0.02);
    assert_words(&words, 2, &[("After", 294.26, 96.31), ("B", 148.71, 134.76)], 0.02);
    assert_words(&words, 3, &[("Third", 293.16, 96.31), ("Some", 148.71, 134.76), ("text", 175.28, 134.76)], 0.02);
    assert!(find(&words, 1, "Before", 0).is_none(), "{words:?}");
}

#[test]
fn a_body_parskip_reaches_displays_rows_and_headings_too() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // pdflatex: after `\setlength{\parskip}{20pt}` in the body, a paragraph
    // opened by `\[..\]`, one opened by `align`, and a `\section` all take
    // the 20pt, as prose does.
    let rendered = render_one(
        "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\nFirst paragraph.\n\n\
         \\setlength{\\parskip}{20pt}\nProse after the change.\n\nBefore a display\n\\[ a = b \\]\nafter it.\n\n\
         \\[ c = d \\]\nA display that starts the paragraph.\n\n\\begin{align} x &= y \\end{align}\nText after align.\n\n\
         \\section{Heading}\nAfter heading.\n\nNext paragraph.\n\\end{document}\n",
    );
    let words = words_of(&rendered);
    assert_words(
        &words,
        1,
        &[
            ("Prose", 148.71, 166.64),
            ("Before", 148.71, 198.53),
            ("it.", 157.32, 228.41),
            ("A", 133.77, 290.18),
            ("Text", 133.77, 365.90),
            ("Heading", 157.98, 418.77),
            ("After", 133.77, 460.51),
            ("Next", 148.71, 492.40),
        ],
        0.02,
    );
}

#[test]
fn a_page_number_takes_the_style_in_force_and_thepart_reads_the_part() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // pdflatex, TeX Live 2026: `\small\thepage` centred in cmr9 (8.97bp)
    // at 303.32bp, `\sffamily\bfseries\thepage` left in cmssbx10, `Page
    // \thepage\ of 5` right; `\thepart` (I, II) and `\arabic{part}` in the
    // head.
    let rendered = render_one(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\\fancyhf{}\n\
         \\fancyfoot[C]{\\small\\thepage}\n\\fancyfoot[L]{\\sffamily\\bfseries\\thepage}\n\\fancyfoot[R]{Page \\thepage\\ of 5}\n\
         \\fancyhead[L]{\\thepart}\\fancyhead[R]{\\arabic{part}}\n\
         \\begin{document}\n\\part{First}\nText one.\n\\newpage\n\\part{Second}\nText two.\n\\end{document}\n",
    );
    let errors: Vec<_> = rendered.v2.diagnostics.iter().filter(|d| format!("{d:?}").contains("Error")).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let words = words_of(&rendered);
    for (page, n, part) in [(1u32, "1", "I"), (2, "2", "II")] {
        let at = |x: f64, y: f64| {
            words
                .iter()
                .find(|w| w.page == page && (w.x - x).abs() < 0.02 && (w.baseline - y).abs() < 0.02)
                .map(|w| w.text.trim().to_string())
        };
        assert_eq!(at(133.77, 96.31).as_deref(), Some(part), "page {page}: {words:?}");
        assert_eq!(at(472.50, 96.31).as_deref(), Some(n), "page {page}");
        assert_eq!(at(133.77, 702.63).as_deref(), Some(n), "page {page}");
        assert_eq!(at(303.32, 702.63).as_deref(), Some(n), "page {page}: the \\small number, centred by its own width");
        assert!(at(428.64, 702.63).is_some_and(|t| t.starts_with("Page")), "page {page}");
        assert_eq!(at(472.50, 702.63).as_deref(), Some("5"), "page {page}");
    }
    // The styles themselves: 9pt roman for `\small`, bold sans for the left.
    let foot = rendered.v2.pages[0].resident_items();
    let sizes: Vec<f64> = foot
        .iter()
        .filter_map(|it| match it {
            Item::GlyphRun(run) if run.glyphs.first().is_some_and(|g| (g.baseline_y.to_bp() - 702.63).abs() < 0.02) => {
                Some(run.font_size.to_bp())
            }
            _ => None,
        })
        .collect();
    assert!(sizes.iter().any(|s| (s - 8.97).abs() < 0.02), "{sizes:?}");
}

/// A mark or counter placeholder that shares a word with other text of the
/// same style (`\leftmark:`, `(\rightmark)`, `Page~\thepage.`,
/// `\thesection--\thepage`) is replaced inside the word, and the word is
/// measured with the value in it. pdflatex (see the module oracle): page 1
/// has `1 INTRO:` left and `()` right (the section's `\markboth` leaves the
/// right mark empty); page 2 reads `Left Words:` and `(Right Words)` from a
/// `\markboth`; the foot is `Page 1.` / `1--1`, then `Page 2.` / `1--2`,
/// the right field flush with the right margin.
#[test]
fn placeholders_inside_a_word_are_replaced_and_measured() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let rendered = render_one(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\\fancyhf{}\n\
         \\fancyhead[L]{\\leftmark:}\n\\fancyhead[R]{(\\rightmark)}\n\
         \\fancyfoot[L]{Page~\\thepage.}\n\\fancyfoot[R]{\\thesection--\\thepage}\n\
         \\begin{document}\n\\section{Intro}\n\\subsection{Detail}\nText one.\n\\newpage\n\
         \\markboth{Left Words}{Right Words}\nText two.\n\\end{document}\n",
    );
    let words = words_of(&rendered);
    let sentinel = |c: char| ('\u{F8FC}'..='\u{F8FF}').contains(&c);
    assert!(!words.iter().any(|w| w.text.chars().any(sentinel)), "{words:?}");
    assert_words(
        &words,
        1,
        &[("1", 133.768, 96.309), ("INTRO:", 148.712, 96.309), ("()", 469.727, 96.309), ("Page", 133.768, 702.635), ("1.", 157.987, 702.635), ("1\u{2013}1", 462.532, 702.635)],
        0.02,
    );
    assert_words(
        &words,
        2,
        &[
            ("Left", 133.768, 96.309),
            ("Words:", 154.659, 96.309),
            ("(Right", 414.436, 96.309),
            ("Words)", 445.851, 96.309),
            ("Page", 133.768, 702.635),
            ("2.", 157.987, 702.635),
            ("1\u{2013}2", 462.532, 702.635),
        ],
        0.02,
    );
}
