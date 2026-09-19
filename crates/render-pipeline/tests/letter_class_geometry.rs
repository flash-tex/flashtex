//! `letter.cls`'s vertical structure, against the pinned pdfLaTeX reference
//! `fixtures/real-world/letter/reference.pdf` (TeX Live 2025; pdflatex is an
//! oracle only and never runs here).
//!
//! Four facts this file pins, each of which the pipeline got wrong before and
//! each of which fails independently:
//!
//! 1. **`\parskip` is a class length.** letter.cls line 91 sets it to `0.7em`
//!    (7.66498pt at 11pt), where article and friends have `0pt plus 1pt`.
//!    Reading it from `flashtex-class-geometry` rather than from
//!    `Stylesheet::article`'s default is what makes a letter's paragraph gaps
//!    21.19 bp instead of 13.55.
//! 2. **`\opening`'s blocks are one paragraph each**, their lines joined by
//!    `\\`, so consecutive lines are exactly `\baselineskip` apart. One
//!    paragraph per line adds a `\parskip` per line that pdfLaTeX has not.
//! 3. **`{\raggedleft ...\par}` is a declaration, not an environment**, so it
//!    closes no `\trivlist` and adds none of `\@endparenv`'s `\@topsepadd`
//!    (9pt at 11pt) after itself.
//! 4. **letter.cls's page-1 `\@texttop`** (line 405,
//!    `\ifnum\c@page=1\vskip \z@ plus.00006fil\relax\fi`) shares the page's
//!    leftover space with line 404's unguarded `\raggedbottom`, so the first
//!    baseline sits below `\topskip`, not at it -- by 4/11 of the slack, not
//!    3/8: TeX keeps fil stretch in scaled points, so `.00006fil` is 4 sp and
//!    `.0001fil` 7 sp (measured 43.586 pt of 119.859; 3/8 is 1.36 bp low).
//!    The slack is taken to the last *baseline*, `\@makecol`'s `\vskip
//!    -\dimen@` having removed the last depth (0.56 bp with it counted).
//! 5. **The class's boxes are boxes.** `\opening`'s return address is a
//!    `tabular{l@{}}` in `\raggedleft` -- `\vcenter`ed on the math axis,
//!    `\@arstrut`-tall rows sharing one left edge `\textwidth` less the
//!    widest row in -- so the recipient below it takes `\lineskip`, not
//!    `\baselineskip` (0.98 bp), and its rows start at 437.195 bp, not at
//!    the right edge of each. `\closing` is a `\parbox` (`$\vcenter{...}$`)
//!    `\longindentation` (180 pt) in, its signature line `\strut`-tall, so
//!    "Sincerely," sits 25.903 bp under the body, not 30.19 (4.29 bp). `\cc`
//!    and `\encl` are `\parbox[t]`s ending in `\strut` (line height 9.52 +
//!    4.08, not the glyphs' 7.54 + 2.13: 3.29 bp on everything below) whose
//!    `\@hangfrom` label is an `\hbox` with its `\sfcode`-2000 space inside
//!    (4.822 bp between `encl:` and the text, which was missing).
//!
//! Every line is within 0.02 bp of the reference in both axes; the
//! tolerances below are that gate, not measured residuals.

mod common;

use common::*;

const SOURCE: &str = include_str!("../../../fixtures/real-world/letter/main.tex");

/// The bottom of the text area: 1in margin + letter's `\textheight`
/// (650.43pt), in bp. Anything below it is page chrome, not body text.
const TEXT_BOTTOM: f64 = 72.0 + 650.43 / 1.00375;

/// Body-text baselines, deduplicated into lines, in page order.
///
/// The folio is excluded deliberately, and it is a real difference: this page
/// carries one and the reference does not, because `\opening` runs
/// `\thispagestyle{empty}` whenever `\fromaddress` is non-empty (letter.cls
/// lines 170-176). That is a page-style event the compiler does not emit yet,
/// and it is chrome rather than geometry, so it does not belong in the
/// measurements below.
fn baselines() -> Vec<f64> {
    let r = render_one(SOURCE);
    let mut ys: Vec<f64> = words_of(&r).iter().map(|w| w.baseline).filter(|y| *y < TEXT_BOTTOM).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() <= 0.05);
    ys
}

#[track_caller]
fn close(got: f64, want: f64, tol: f64, what: &str) {
    assert!(
        (got - want).abs() <= tol,
        "{what}: got {got:.3} bp, pdflatex {want:.3} bp ({:+.3} off, tolerance {tol})",
        got - want
    );
}

/// The reference's line baselines in bp from the top of the page, read out of
/// `fixtures/real-world/letter/reference.pdf` with
/// `tools/visual-oracle/pdftext.py`, and the residual each is still allowed
/// (see the module note). Index into [`baselines`].
const REFERENCE: &[(usize, &str, f64, f64)] = &[
    (0, "\\address line 1", 124.907, 0.02),
    (1, "\\address line 2", 138.456, 0.02),
    (2, "\\date", 167.278, 0.02),
    (3, "recipient line 1", 202.761, 0.02),
    (6, "recipient line 4", 243.409, 0.02),
    (7, "\\opening's salutation", 279.867, 0.02),
    (8, "first body line", 301.052, 0.02),
    (20, "last body line", 494.188, 0.02),
    (21, "\\closing", 520.091, 0.02),
    (22, "\\signature", 579.458, 0.02),
    (23, "\\encl", 601.640, 0.02),
    (24, "\\cc", 622.826, 0.02),
    (25, "\\ps", 644.011, 0.02),
];

/// Left edges the class's boxes give, in bp from the page's left edge, with
/// the baseline each word sits on (`pdftext.py` on the reference).
const REFERENCE_X: &[(&str, f64, f64)] = &[
    // The `tabular{l@{}}` rows share one left edge: the right margin
    // (72 + 469.75502 pt) less the widest row, "Pittsburgh, PA 15213".
    ("123", 437.195, 124.907),
    ("Pittsburgh,", 437.195, 138.456),
    ("September", 437.195, 167.278),
    // `\hspace*{\longindentation}`: 72 + 180 pt.
    ("Sincerely,", 251.328, 520.091),
    ("Jordan", 251.328, 579.458),
    // `\@hangfrom{\normalfont\enclname: }`: the label's `\hbox` is 26.61899
    // pt wide (21.7795 for `encl:` plus the 3.63054 + 1.20892 `\sfcode`
    // 2000 space), so the text starts 26.52 bp in.
    ("encl:", 72.0, 601.640),
    ("Curriculum", 98.522, 601.640),
    ("cc:", 72.0, 622.826),
    ("Professor", 89.481, 622.826),
];

#[test]
fn letter_paragraph_gaps_are_the_class_parskip() {
    if !lm_available() {
        return;
    }
    let ys = baselines();
    assert_eq!(ys.len(), 26, "expected the letter's 26 lines on one page, got {}", ys.len());
    // Body paragraph to body paragraph: `\baselineskip` + letter's `\parskip`
    // = 13.549 + 7.636 bp. Article's `0pt plus 1pt` gave 13.55.
    close(ys[10] - ys[9], 21.185, 0.02, "body paragraph to body paragraph");
    // Within one paragraph it is `\baselineskip` alone, unchanged.
    close(ys[12] - ys[11], 13.549, 0.02, "body line to body line");
}

#[test]
fn opening_lines_are_one_paragraph_not_one_each() {
    if !lm_available() {
        return;
    }
    let ys = baselines();
    // `\address{...\\...}` is a `\\` inside one paragraph, so exactly
    // `\baselineskip`. One paragraph per line would add a `\parskip`.
    close(ys[1] - ys[0], 13.549, 0.02, "address line 1 to line 2");
    close(ys[4] - ys[3], 13.549, 0.02, "recipient line 1 to line 2");
    close(ys[6] - ys[5], 13.549, 0.02, "recipient line 3 to line 4");
    // `\\*[2\parskip]` between `\fromaddress` and `\@date`.
    close(ys[2] - ys[1], 28.821, 0.02, "address to date");
    // `\closing`'s `\\[6\medskipamount]`, where letter.cls line 236 makes
    // `\medskipamount` the class `\parskip`: 13.549 + 6 * 7.636.
    close(ys[22] - ys[21], 59.370, 0.02, "closing to signature");
}

#[test]
fn the_raggedleft_group_closes_no_trivlist() {
    if !lm_available() {
        return;
    }
    let ys = baselines();
    // date -> recipient: the tabular's last row keeps `\@arstrut`'s depth
    // (4.08003 pt), then `\vspace{2\parskip}`, `\parskip`, `\lineskip`
    // (the box is deeper than `\baselineskip` allows) and the line's height:
    // 4.08003 + 15.32996 + 7.66498 + 1.0 + 7.54149 = 35.61646 pt = 35.483 bp.
    // A `flushright` *environment* here would add `\@endparenv`'s
    // `\@topsepadd` (`\topsep` 9pt at 11pt) on top.
    close(ys[3] - ys[2], 35.483, 0.02, "date to recipient");
    // recipient -> salutation is the same three skips with no tabular in the
    // way, and is exact.
    close(ys[7] - ys[6], 36.457, 0.02, "recipient to salutation");
}

#[test]
fn page_one_texttop_fil_pushes_the_first_baseline_down() {
    if !lm_available() {
        return;
    }
    let ys = baselines();
    // Without `\@texttop` the first baseline is `\topskip` (11pt = 10.96 bp)
    // below the 1in text top: 82.96 bp. pdfLaTeX puts it at 124.907: the
    // tabular's first row, 9.51996 pt under the box's top, which is 4/11 of
    // the 119.859 pt slack down (3/8 would give 126.26, and counting the
    // last line's depth in the slack 125.47).
    assert!(
        ys[0] > 110.0,
        "first baseline {:.3} bp: letter.cls's page-1 `\\@texttop` fil glue is not being applied \
         (without it the first baseline is 82.96 bp)",
        ys[0]
    );
    close(ys[0], 124.907, 0.02, "first baseline");
}

/// `\opening` with no `\address`: letter.cls lines 266-268 set the date as
/// `{\raggedleft\@date\par}`, a plain line, and `\closing` (line 291) adds
/// no `\hspace*{\longindentation}` when `\fromaddress` is empty. Measured
/// with pdflatex (TeX Live 2026, `pdftext.py`) on the fixture's letter with
/// its `\address` removed and the body cut to one paragraph.
const NO_ADDRESS: &str = "\\documentclass[11pt]{letter}\n\\usepackage[T1]{fontenc}\n\\usepackage[utf8]{inputenc}\n\
\\usepackage[margin=1in]{geometry}\n\n\\signature{Jordan Example}\n\\date{September 12, 2026}\n\n\
\\begin{document}\n\n\\begin{letter}{Admissions Committee\\\\Example University}\n\n\
\\opening{Dear Members of the Committee,}\n\nThank you for your consideration.\n\n\\closing{Sincerely,}\n\n\
\\end{letter}\n\n\\end{document}\n";

#[test]
fn opening_without_an_address_sets_the_date_as_a_plain_line() {
    if !lm_available() {
        return;
    }
    let r = render_one(NO_ADDRESS);
    let words = words_of(&r);
    // (word, x, baseline): the date at the right margin with its glyphs' own
    // height (no `\@arstrut`, no `\vcenter`), the recipient `\vspace
    // {2\parskip}` + `\parskip` + `\baselineskip` below it (36.459 bp; a
    // `tabular` here would leave `\@arstrut`'s depth and take `\lineskip`),
    // and the closing at the left margin.
    for (text, x, y) in [
        ("September", 447.193, 244.971),
        ("2026", 518.309, 244.971),
        ("Admissions", 72.0, 281.430),
        ("Dear", 72.0, 331.437),
        ("Sincerely,", 72.0, 378.526),
        ("Jordan", 72.0, 437.893),
    ] {
        let Some(w) = words.iter().find(|w| w.text == text) else {
            panic!("{text:?} not set (words: {:?})", words.iter().map(|w| &w.text).collect::<Vec<_>>());
        };
        close(w.x, x, 0.02, &format!("left edge of {text:?}"));
        close(w.baseline, y, 0.02, &format!("baseline of {text:?}"));
    }
}

#[test]
fn the_class_boxes_start_where_pdflatex_puts_them() {
    if !lm_available() {
        return;
    }
    let r = render_one(SOURCE);
    let words = words_of(&r);
    for (text, x, y) in REFERENCE_X {
        let hit = words.iter().find(|w| w.text == *text && (w.baseline - y).abs() <= 0.02);
        let Some(w) = hit else {
            panic!("{text:?} on the baseline at {y:.3} bp: not set (words: {:?})", words.iter().map(|w| (&w.text, w.baseline)).collect::<Vec<_>>());
        };
        close(w.x, *x, 0.02, &format!("left edge of {text:?}"));
    }
}

#[test]
fn every_line_is_where_pdflatex_put_it() {
    if !lm_available() {
        return;
    }
    let ys = baselines();
    for (i, what, want, tol) in REFERENCE {
        close(ys[*i], *want, *tol, what);
    }
}
