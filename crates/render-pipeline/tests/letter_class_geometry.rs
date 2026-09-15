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
//!    baseline sits 3/8 of the slack below `\topskip`, not at it.
//!
//! ## What is still not modelled, and why the tolerances differ
//!
//! Two TeX box-model facts remain, both measured and neither hidden:
//!
//! * `\opening` sets the return address in a `tabular{l@{}}`, whose last row
//!   carries `\@arstrut`'s `0.3\baselineskip` depth where a text line carries
//!   its glyphs'. The boundary *out of* the address block is therefore
//!   **0.98 bp** tight. It is one boundary, once per letter.
//! * `\closing` sets its argument in a `\parbox`, whose first line is a
//!   `\strut` and whose height drives the interline glue TeX chooses. The
//!   boundary *into* the closing is **2.92 bp** loose.
//!
//! Everything else is within 0.01 bp of the reference's own spacing. The
//! tolerances below are the measured residuals, not a margin chosen to pass:
//! tightening them is the next lane's gate.

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
    (0, "\\address line 1", 124.910, 0.5),
    (1, "\\address line 2", 138.460, 0.5),
    (2, "\\date", 167.280, 0.5),
    (3, "recipient line 1", 202.760, 1.5),
    (6, "recipient line 4", 243.410, 1.5),
    (7, "\\opening's salutation", 279.870, 1.5),
    (8, "first body line", 301.050, 1.5),
    (20, "last body line", 494.190, 1.5),
    (21, "\\closing", 520.090, 4.5),
    (22, "\\signature", 579.460, 4.5),
    (23, "\\encl", 601.640, 3.5),
    (24, "\\cc", 622.830, 3.5),
    (25, "\\ps", 644.010, 3.5),
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
    // date -> recipient is `\baselineskip` + `\parskip` + `\vspace{2\parskip}`
    // = 36.457 bp. A `flushright` *environment* here would add
    // `\@endparenv`'s `\@topsepadd` (`\topsep` 9pt at 11pt) on top.
    close(ys[3] - ys[2], 36.457, 1.0, "date to recipient (tabular depth: 0.98 bp tight)");
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
    // below the 1in text top: 82.96 bp. pdfLaTeX puts it at 124.910.
    assert!(
        ys[0] > 110.0,
        "first baseline {:.3} bp: letter.cls's page-1 `\\@texttop` fil glue is not being applied \
         (without it the first baseline is 82.96 bp)",
        ys[0]
    );
    close(ys[0], 124.910, 0.5, "first baseline");
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
