//! GitHub issue #520: `eqnarray`/`eqnarray*` set as real three-column math
//! displays, not plain text.
//!
//! ## Oracle
//!
//! Measured, not derived. pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026) at
//! `/Library/TeX/texbin/pdflatex`, glyph origins read out with PyMuPDF
//! 1.28.2, on
//!
//! ```tex
//! \documentclass[<size>]{article}
//! \usepackage[margin=1in]{geometry}
//! \pagestyle{empty}
//! \begin{document}
//! \noindent Lead in text.
//! \begin{eqnarray}
//! a &=& b \\
//! cd &=& e
//! \end{eqnarray}
//! \noindent Trail text.
//! \end{document}
//! ```
//!
//! US Letter with 1 in margins, so the text block runs 72 bp .. 540 bp and
//! `\displaywidth` is 468 bp. Everything below is in bp.
//!
//! | | 10 pt | 11 pt | 12 pt |
//! | --- | --- | --- | --- |
//! | gap each side of the relation | 9.9626 | 9.9600 | 9.9586 / 9.9706 |
//! | block left edge | 285.0950 | 284.0560 | 283.2120 |
//! | relation column left edge | 304.5619 | 304.4233 | 304.2890 |
//! | third column left edge | 322.2754 | 322.8706 | 323.3695 |
//! | block right edge | 326.9180 | 327.9542 | 328.7972 |
//! | block centre | 306.0065 | 306.0051 | 306.0046 |
//! | number left / right edge | 527.2858 / 540.0180 | 526.0744 / 540.0163 | 525.0418 / 540.0097 |
//! | row baseline gap | 14.9440 | 16.5380 | 17.4350 |
//!
//! Two things to read out of that. First, the quirk issue #520 names: the
//! gap either side of the relation is a fixed `\tw@\arraycolsep` = 10 pt =
//! 9.9626 bp, because the middle column is `${##}$` -- a group, so an Ord,
//! taking no `\thickmuskip` at either edge. The same document written with
//! amsmath `align` measures 2.7696 bp there at 10 pt, 3.6x narrower. It is
//! not `align` with different spacing.
//!
//! Second, the vertical is *not* special: the row baseline gaps are
//! 15.000 / 16.600 / 17.500 pt, i.e. `\baselineskip` + `\jot`, exactly what
//! `align` measures, because latex.ltx's `\@@eqncr` puts an explicit
//! `\vskip\jot` between rows where amsmath's `\displ@y@` says `\openup\jot`.
//! The lead-in line to first row distance is likewise identical
//! (21.917 / 24.508 / 26.401 bp). So the pipeline's existing `\jot`
//! handling is right for `eqnarray` and is deliberately left alone.
//!
//! ## Why these are `#[ignore]`d
//!
//! The `eqnarray` compiler support is on `main`
//! (`crates/compiler/src/parser.rs`), but `crates/render-pipeline` builds
//! against the frozen `vendor/compiler` mirror (`vendor/VENDORING.md`),
//! pinned at `c95977d6`, which predates it: that compiler still reports
//! `environment 'eqnarray' is not implemented; its body is typeset as
//! plain text` and emits no `MathRows` at all, so these can only fail here.
//! Re-pinning `vendor/` is integrator-owned. Un-ignore both at re-pin time.
//! The layout arithmetic they exercise is covered *now*, against the same
//! measurements, by `typeset::eqnarray_tests` in the library.

mod common;

use common::*;

/// Gate on glyph positions.
const TOL: f64 = 0.5;
/// Right edge of the text block, in bp.
const RIGHT: f64 = 540.0;
/// Centre of the text block, in bp.
const CENTRE: f64 = 306.0;

fn doc(size: &str, body: &str) -> String {
    format!(
        "\\documentclass[{size}]{{article}}\n\
         \\usepackage[margin=1in]{{geometry}}\n\
         \\pagestyle{{empty}}\n\
         \\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

const TWO_ROW: &str = "\\begin{eqnarray}\na &=& b \\\\\ncd &=& e\n\\end{eqnarray}";

fn assert_supported(r: &flashtex_render_pipeline::Rendered) {
    assert!(
        !r.v2.diagnostics.iter().any(|d| d.code == "unsupported_feature"),
        "unexpected unsupported_feature: {:?}",
        r.v2.diagnostics
    );
}

/// The words on one baseline, left to right, split into the math cells and
/// the equation number (the only run starting with `(`).
fn row_on(words: &[Word], baseline: f64) -> (Vec<Word>, Option<Word>) {
    let mut on: Vec<Word> = words
        .iter()
        .filter(|w| (w.baseline - baseline).abs() < 0.01)
        .cloned()
        .collect();
    on.sort_by(|a, b| a.x.total_cmp(&b.x));
    let number = on.iter().find(|w| w.text.starts_with('(')).cloned();
    let cells = on.into_iter().filter(|w| !w.text.starts_with('(')).collect();
    (cells, number)
}

fn math_baselines(words: &[Word]) -> Vec<f64> {
    let mut ys: Vec<f64> = words.iter().filter(|w| w.text == "=").map(|w| w.baseline).collect();
    ys.sort_by(f64::total_cmp);
    ys
}

/// Per-size expectations, all measured (see the module docs):
/// (class option, gap, block left, relation left, col3 left, block right,
/// number left, row baseline gap).
const SIZES: &[(&str, f64, f64, f64, f64, f64, f64, f64)] = &[
    ("10pt", 9.9626, 285.0950, 304.5619, 322.2754, 326.9180, 527.2858, 14.9440),
    ("11pt", 9.9600, 284.0560, 304.4233, 322.8706, 327.9542, 526.0744, 16.5380),
    ("12pt", 9.9626, 283.2120, 304.2890, 323.3695, 328.7972, 525.0418, 17.4350),
];

#[test]
#[ignore = "needs a vendor/compiler re-pin carrying the #520 compiler change"]
fn eqnarray_columns_and_numbers_match_pdflatex_at_ten_eleven_twelve_point() {
    if !lm_available() {
        eprintln!("SKIP eqnarray columns: Latin Modern not installed");
        return;
    }
    for &(size, gap, block_l, rel_l, col3_l, block_r, num_l, rowgap) in SIZES {
        let r = render_one(&doc(size, TWO_ROW));
        assert_supported(&r);
        let words = words_of(&r);
        let ys = math_baselines(&words);
        assert_eq!(ys.len(), 2, "{size}: two relation signs expected, got {words:?}");

        let mut rows = Vec::new();
        for y in &ys {
            let (cells, number) = row_on(&words, *y);
            assert_eq!(cells.len(), 3, "{size}: three columns expected, got {cells:?}");
            rows.push((cells, number));
        }

        // The three columns share tab stops across the rows: the relation
        // centres coincide, and the third column starts at one x.
        let centre_of = |w: &Word| w.x + w.width / 2.0;
        assert!(
            (centre_of(&rows[0].0[1]) - centre_of(&rows[1].0[1])).abs() < 0.05,
            "{size}: relation column not shared: {rows:?}"
        );
        assert!(
            (rows[0].0[2].x - rows[1].0[2].x).abs() < 0.05,
            "{size}: third column not shared: {rows:?}"
        );

        // The quirk: `2\arraycolsep` each side of the relation, no muskip.
        for (cells, _) in &rows {
            let left = cells[1].x - (cells[0].x + cells[0].width);
            let right = cells[2].x - (cells[1].x + cells[1].width);
            for (side, got) in [("left", left), ("right", right)] {
                assert!(
                    (got - gap).abs() <= TOL,
                    "{size}: {side} gap {got:.4} bp, pdflatex {gap:.4} bp: {cells:?}"
                );
            }
        }

        // Absolute column positions, from the widest row (row 1 here).
        let (wide, _) = &rows[1];
        for (what, got, want) in [
            ("block left", wide[0].x, block_l),
            ("relation left", wide[1].x, rel_l),
            ("third column left", wide[2].x, col3_l),
            ("block right", wide[2].x + wide[2].width, block_r),
        ] {
            assert!(
                (got - want).abs() <= TOL,
                "{size}: {what} at {got:.4} bp, pdflatex {want:.4} bp"
            );
        }
        let centre = (wide[0].x + wide[2].x + wide[2].width) / 2.0;
        assert!(
            (centre - CENTRE).abs() <= TOL,
            "{size}: block centre {centre:.4} bp, pdflatex {CENTRE:.4} bp"
        );

        // Every row numbered, flush right against the text edge.
        let numbers: Vec<String> =
            rows.iter().filter_map(|(_, n)| n.as_ref().map(|w| w.text.clone())).collect();
        assert_eq!(numbers, ["(1)", "(2)"], "{size}");
        let tag = rows[1].1.as_ref().expect("row 2 numbered");
        assert!(
            (tag.x - num_l).abs() <= TOL,
            "{size}: number left at {:.4} bp, pdflatex {num_l:.4} bp",
            tag.x
        );
        assert!(
            (tag.x + tag.width - RIGHT).abs() <= TOL,
            "{size}: number right at {:.4} bp, text edge {RIGHT:.4} bp",
            tag.x + tag.width
        );

        // `\baselineskip` + `\jot`, the same as `align`.
        let got = ys[1] - ys[0];
        assert!(
            (got - rowgap).abs() <= TOL,
            "{size}: row baseline gap {got:.4} bp, pdflatex {rowgap:.4} bp"
        );
    }
}

/// `eqnarray*` prints no number at all, and `\nonumber` drops exactly one
/// row's number while the rows that keep theirs still count from the
/// `equation` counter. Measured at 10 pt: with `\nonumber` on the first row
/// the second row carries `(1)`, because latex.ltx's `\@@eqncr` prints the
/// number *before* it steps the counter, so a skipped row consumes nothing.
#[test]
#[ignore = "needs a vendor/compiler re-pin carrying the #520 compiler change"]
fn eqnarray_star_and_nonumber_suppress_numbers() {
    if !lm_available() {
        eprintln!("SKIP eqnarray numbering: Latin Modern not installed");
        return;
    }
    let numbers_of = |src: &str| -> Vec<String> {
        let r = render_one(&doc("10pt", src));
        assert_supported(&r);
        let mut ns: Vec<(f64, String)> = words_of(&r)
            .into_iter()
            .filter(|w| w.text.starts_with('('))
            .map(|w| (w.baseline, w.text))
            .collect();
        ns.sort_by(|a, b| a.0.total_cmp(&b.0));
        ns.into_iter().map(|(_, t)| t).collect()
    };

    assert_eq!(numbers_of(TWO_ROW), ["(1)", "(2)"]);
    assert_eq!(
        numbers_of("\\begin{eqnarray*}\na &=& b \\\\\ncd &=& e\n\\end{eqnarray*}"),
        Vec::<String>::new()
    );
    assert_eq!(
        numbers_of("\\begin{eqnarray}\na &=& b \\nonumber \\\\\ncd &=& e\n\\end{eqnarray}"),
        ["(1)"]
    );
    assert_eq!(
        numbers_of("\\begin{eqnarray}\na &=& b \\notag \\\\\ncd &=& e\n\\end{eqnarray}"),
        ["(1)"]
    );

    // `eqnarray*` is still unnumbered when it sets real columns.
    let r = render_one(&doc("10pt", "\\begin{eqnarray*}\na &=& b \\\\\ncd &=& e\n\\end{eqnarray*}"));
    assert_supported(&r);
    assert_eq!(math_baselines(&words_of(&r)).len(), 2);
}

/// A long right-hand side grows the third column; the block stays centred
/// on the text width rather than being pushed off it. Measured at 10 pt for
/// `cd &=& xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`: block 202.0060 .. 410.1546 bp,
/// centre 306.0803 bp, with the relation still at 221.4729 bp and the same
/// 9.9626 bp gaps.
#[test]
#[ignore = "needs a vendor/compiler re-pin carrying the #520 compiler change"]
fn long_right_hand_side_keeps_the_block_centred() {
    if !lm_available() {
        eprintln!("SKIP eqnarray long rhs: Latin Modern not installed");
        return;
    }
    let r = render_one(&doc(
        "10pt",
        "\\begin{eqnarray}\na &=& b \\\\\ncd &=& xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\n\\end{eqnarray}",
    ));
    assert_supported(&r);
    let words = words_of(&r);
    let ys = math_baselines(&words);
    assert_eq!(ys.len(), 2, "{words:?}");
    let (cells, number) = row_on(&words, ys[1]);
    assert_eq!(cells.len(), 3, "{cells:?}");
    for (what, got, want) in [
        ("block left", cells[0].x, 202.0060),
        ("relation left", cells[1].x, 221.4729),
        ("third column left", cells[2].x, 239.1864),
        ("block right", cells[2].x + cells[2].width, 410.1546),
    ] {
        assert!((got - want).abs() <= TOL, "{what} at {got:.4} bp, pdflatex {want:.4} bp");
    }
    let centre = (cells[0].x + cells[2].x + cells[2].width) / 2.0;
    assert!((centre - 306.0803).abs() <= TOL, "block centre {centre:.4} bp");
    let tag = number.expect("numbered");
    assert!((tag.x + tag.width - RIGHT).abs() <= TOL, "number right {:.4} bp", tag.x + tag.width);
}
