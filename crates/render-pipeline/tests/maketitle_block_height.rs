//! The `\maketitle` title block's height, measured against pdflatex at 10, 11
//! and 12 pt — and the page-glue mechanism that made it *look* wrong.
//!
//! ## Why this file exists
//!
//! The 2026-09-16 corpus sweep (`docs/evidence/corpus-fidelity-2026-09-16T1130Z`,
//! PR #750, corrected by #753) reported a cumulative finding it labelled
//! "`\maketitle` title-block height": `math-sheet` −0.513 bp carried down 63
//! lines, `lecture-notes` +0.176 bp carried down 27, 90 displaced reference
//! lines between them. The two signs disagreed, so the report concluded it
//! could not be one constant.
//!
//! It is not a title-block defect at all. Measured directly, the block is
//! exact:
//!
//! | base | title → author | author → date | date → first body line |
//! |---|---|---|---|
//! | 10 pt | 28.8880 bp | 23.3740 bp | 36.8591 bp |
//! | 11 pt | 30.2190 bp | 24.3960 bp | 41.7750 bp |
//! | 12 pt | 35.4880 bp | 27.9590 bp | 44.9520 bp |
//!
//! and the pipeline reproduces every one of those to better than 0.01 bp (see
//! [`BOTH`] / [`NONE`] below, which pin the absolute baselines).
//!
//! What actually moved the 90 lines is the **page builder's glue set ratio**.
//! `\@maketitle`'s only stretchable/shrinkable glue is `center`'s closing
//! `\@endparenv` skip, `\addvspace{\@topsepadd}` — `12.0 plus 4.0 minus 6.0`
//! at an 11 pt base (`\topsep` + `\partopsep`). Everything else in the block
//! (`\vskip 2em`, `\vskip 1.5em`, `\vskip 1em`, every `\baselineskip`) is
//! rigid, and `\@topsepadd` is `center`'s *closing* skip: it sits *below*
//! the title, author and date lines, not above them. So when page 1 is
//! *shrunk*, the title block itself does not move at all — its three
//! baselines are anchored — and everything *below* the block (the first
//! body line and every line after it) rides up by `ratio × 6 pt`.
//!
//! Both offending corpus pages are shrunk pages. pdfTeX's own `\tracingoutput`
//! says so:
//!
//! ```text
//! math-sheet   p1  ..\vbox(650.43001+0.0)x469.75502, glue set - 0.74042
//! lecture-notes p1 ..\vbox(650.43001+0.0)x469.75502, glue set - 0.30122
//! ```
//!
//! Shrinkable glue above the first body line is `\@topsepadd`'s 6 pt plus
//! `\section*`'s `\@minus.2ex` (0.94266 pt at 11 pt) = 6.94266 pt, so pdfTeX
//! lifts that line by 0.74042 × 6.94266 = 5.1405 pt = **5.1213 bp** on
//! `math-sheet` and by 0.30122 × 6.94266 = 2.0912 pt = **2.0834 bp** on
//! `lecture-notes`. Truncating each fixture so page 1 no longer overflows
//! removes exactly those amounts from the reference, and the reported step
//! collapses from −0.5132 bp to **+0.0037 bp** (`math-sheet`) and from
//! +0.1755 bp to **+0.0041 bp** (`lecture-notes`).
//!
//! In other words the two steps are the *difference of two shrink ratios*, and
//! the ratio differs only because the two sides' page-1 natural heights differ
//! — from the display-math box heights on `math-sheet` (the sweep's own F2)
//! and from the amsthm closing skips on `lecture-notes` (its F5). On a shrunk
//! page a divergence anywhere moves every line **above** it too, so a
//! cumulative step does not localise its cause to the line it appears on.
//! That is the one thing worth remembering from this file.
//!
//! The ratio arithmetic itself is right, which is what
//! [`a_shrunk_page_keeps_every_baseline`] pins: a plain-text page whose
//! content is identical on both sides and whose glue pdfTeX sets to
//! `- 0.23944` matches on all 29 baselines.
//!
//! ## Oracle
//!
//! pdflatex (TeX Live, `/Library/TeX/texbin/pdflatex`) on each probe below,
//! glyph origins read out of the shipped PDF with PyMuPDF and grouped into
//! baselines at a 0.05 bp tolerance. The document font is the T1 EC family
//! pdfLaTeX picks for `\usepackage[T1]{fontenc}` without `lmodern`, so every
//! `em` in `\@maketitle` is that font's `\fontdimen6`, which is *not* its
//! design size: `ecrm1000` 9.99756 pt, `ecrm1095` 10.88788 pt, `ecrm1200`
//! 11.74713 pt. Using the nominal size instead puts the title baseline
//! 0.124 bp out at 11 pt, so this is checked, not assumed — and the pipeline's
//! title/author/date **x** positions agree to 0.000 bp at all three sizes, so
//! there is no design-size substitution in the block either.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// Well under the 0.5 bp glyph gate and the 0.1 bp rule gate; the worst
/// measured deviation over every case here is 0.016 bp.
const TOL: f64 = 0.05;

/// Two baselines this far apart in bp are the same line.
const LINE_TOL: f64 = 0.05;

fn preamble(size: u32, author: &str, date: &str) -> String {
    format!(
        "\\documentclass[{size}pt]{{article}}\n\
         \\usepackage[T1]{{fontenc}}\n\
         \\usepackage[margin=1in]{{geometry}}\n\
         \\title{{Formula Sheet: Calculus and Linear Algebra}}\n\
         \\author{{{author}}}\n\
         \\date{{{date}}}\n\
         \\begin{{document}}\n\
         \\maketitle\n"
    )
}

const BODY: &str = "Alpha one. Alpha two. Alpha three.\n\n\
                    Beta one. Beta two. Beta three.\n\n\
                    Gamma one. Gamma two. Gamma three.\n\
                    \\end{document}\n";

/// Every distinct glyph baseline on `page`, top to bottom, in bp.
fn page_baselines(text: &str, page: usize) -> Vec<f64> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "maketitle-block-height", &fonts, &RenderOptions::default());
    assert!(r.v2.pages.len() > page, "expected more than {page} page(s), got {}", r.v2.pages.len());
    let mut ys: Vec<f64> = Vec::new();
    for item in r.v2.pages[page].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        for g in &run.glyphs {
            ys.push(g.baseline_y.to_bp());
        }
    }
    ys.sort_by(f64::total_cmp);
    let mut out: Vec<f64> = Vec::new();
    for y in ys {
        if out.last().is_none_or(|last| (y - last).abs() >= LINE_TOL) {
            out.push(y);
        }
    }
    out
}

fn assert_baselines(what: &str, got: &[f64], want: &[f64]) {
    assert_eq!(
        got.len(),
        want.len(),
        "{what}: {} baselines on page 1, pdflatex has {}\n  got  {got:?}\n  want {want:?}",
        got.len(),
        want.len()
    );
    for (i, (&g, &w)) in got.iter().zip(want).enumerate() {
        assert!(
            (g - w).abs() <= TOL,
            "{what}: baseline {i} at {g:.4} bp, pdflatex {w:.4} bp (off by {:+.4} bp, gate {TOL} bp)",
            g - w
        );
    }
}

/// `\author{}` `\date{}` — `math-sheet`'s own shape. The empty author
/// `tabular` still sets a line (see `maketitle_empty_author.rs`); the date
/// contributes nothing but its `\vskip 1em`, which `\@endparenv`'s
/// `\addvspace{\@topsepadd}` then replaces because 12 pt > 1 em.
const NONE: &[(u32, &[f64])] = &[
    (10, &[123.8010, 189.5470, 201.5020, 213.4570, 749.8881]),
    (11, &[126.5710, 198.5650, 212.1140, 225.6630, 749.8880]),
    (12, &[132.2680, 212.7080, 227.1540, 241.6000, 749.8879]),
];

/// `\author{A. Reyes \and B. Okafor}` `\date{August 2025}` —
/// `lecture-notes`' shape plus a second `\and` author, so the author row is
/// two `tabular`s sharing one line.
const BOTH: &[(u32, &[f64])] = &[
    (10, &[123.8010, 152.6890, 176.0630, 212.9221, 224.8771, 236.8321, 749.8881]),
    (11, &[126.5710, 156.7900, 181.1860, 222.9610, 236.5100, 250.0600, 749.8880]),
    (12, &[132.2680, 167.7560, 195.7150, 240.6670, 255.1130, 269.5590, 749.8879]),
];

#[test]
fn the_title_block_is_pdflatexs_at_ten_eleven_and_twelve_point() {
    if !common::lm_available() {
        return;
    }
    for &(size, want) in NONE {
        let tex = preamble(size, "", "") + BODY;
        assert_baselines(&format!("{size}pt, no author, no date"), &page_baselines(&tex, 0), want);
    }
    for &(size, want) in BOTH {
        let tex = preamble(size, "A. Reyes \\and B. Okafor", "August 2025") + BODY;
        assert_baselines(&format!("{size}pt, two authors + date"), &page_baselines(&tex, 0), want);
    }
}

/// The cumulative half: a whole page's baseline sequence below the title, on a
/// page pdfTeX *shrinks* (`\tracingoutput`: `glue set - 0.23944`), which is the
/// regime both corpus fixtures are in. `\@topsepadd` is the block's *closing*
/// glue, so the title, author and date baselines are anchored and do not move
/// under shrink — but a wrong `\@topsepadd`, or a shrink ratio computed off
/// the wrong natural height, moves every line *below* the block, which is all
/// but the first 3 of these 29.
#[test]
fn a_shrunk_page_keeps_every_baseline() {
    if !common::lm_available() {
        return;
    }
    let para = "Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron \
                pi rho sigma tau upsilon phi chi psi omega and some more words to fill the line.\n\n";
    let mut tex = preamble(11, "A. Reyes", "August 2025");
    tex.push_str("\\tableofcontents\n\n");
    for k in 1..=4 {
        tex.push_str(&format!("\\section{{Section number {k}}}\n"));
        tex.push_str(para);
        tex.push_str(para);
    }
    tex.push_str("\\end{document}\n");

    // pdflatex, page 1: the title block, the table of contents, then four
    // `\section`s of two paragraphs each, all lifted by the page's shrink.
    const WANT: &[f64] = &[
        126.5710, 156.7900, 181.1860, // title, author, date
        225.8530, 250.2499, 274.6459, 299.0419, 323.4389, // "Contents" + 4 entries
        357.5819, 381.9309, 395.4809, 409.0299, 422.5789, // section 1
        456.7220, 481.0720, 494.6210, 508.1700, 521.7190, // section 2
        555.8630, 580.2120, 593.7610, 607.3100, 620.8600, // section 3
        655.0030, 679.3520, 692.9020, 706.4510, 720.0000, // section 4
        749.8880, // the page number
    ];
    assert_baselines("11pt, shrunk page 1", &page_baselines(&tex, 0), WANT);
}
