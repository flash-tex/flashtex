//! GitHub issue #520: `eqnarray`/`eqnarray*` typeset as real three-column
//! math displays, not plain text.
//!
//! ## Oracle
//!
//! latex.ltx (`ltmath.dtx`, quoted verbatim by several sources since the
//! sandbox has no TeX Live to run; pdfLaTeX is an oracle only and never
//! runs in the product path):
//!
//! ```text
//! \halign to\displaywidth\bgroup
//! \hskip\@centering$\displaystyle\tabskip\z@skip{##}$\@eqnsel
//! &\global\@eqcnt\@ne\hskip \tw@\arraycolsep \hfil${##}$\hfil
//! &\global\@eqcnt\tw@ \hskip \tw@\arraycolsep $\displaystyle{##}$\hfil\tabskip\@centering
//! &\global\@eqcnt\thr@@ \hb@xt@\z@\bgroup\hss##\egroup \tabskip\z@skip
//! ```
//!
//! So the three columns are right/centred/left, the block is centred on the
//! line (`\@centering` tabskip), the equation number sits flush right in a
//! zero-width fourth column, and the quirk the issue calls out: the gaps
//! are a fixed `\tw@\arraycolsep` = 2 x 5 pt = 10 pt on each side of the
//! relation column — wider than, and unrelated to, `align`'s spread. The
//! middle column is `${##}$`, so the relation takes no `\thickmuskip` at
//! either edge: the whole gap is the 10 pt of column separation.
//!
//! On US Letter with 1 in margins the text runs from 72 bp to 540 bp, so a
//! centred block is centred on 306 bp and 10 pt = 9.9626 bp.

mod common;

use common::*;

/// 1 bp = 1.00375 TeX pt.
const BP: f64 = 1.00375;
/// The documented inter-column gap: 2 x `\arraycolsep` (5 pt), in bp.
const SEP_BP: f64 = 10.0 / BP;
/// Page centre of the text block on US Letter with 1 in margins, in bp.
const CENTRE_BP: f64 = 306.0;
/// Right text edge, in bp.
const RIGHT_BP: f64 = 540.0;
const TOL: f64 = 0.5;

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage[margin=1in]{{geometry}}\n\\pagestyle{{empty}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn no_unsupported_feature(r: &flashtex_render_pipeline::Rendered) {
    assert!(
        !r.v2
            .diagnostics
            .iter()
            .any(|d| d.code == "unsupported_feature"),
        "unexpected unsupported_feature in {:?}",
        r.v2.diagnostics
    );
}

/// One row of single-glyph runs on the same baseline: (left, relation,
/// right) runs plus the row's equation-number run, if any.
fn row_on<'a>(words: &'a [Word], baseline: f64) -> (Vec<&'a Word>, Option<&'a Word>) {
    let mut cells: Vec<&Word> = words
        .iter()
        .filter(|w| (w.baseline - baseline).abs() < 0.01 && !w.text.starts_with('('))
        .collect();
    cells.sort_by(|a, b| a.x.total_cmp(&b.x));
    let number = words
        .iter()
        .find(|w| (w.baseline - baseline).abs() < 0.01 && w.text.starts_with('('));
    (cells, number)
}

/// Needs a `vendor/compiler` re-pin past the `eqnarray` compiler change
/// (issue #520): until then the vendored compiler lowers `eqnarray` bodies
/// to plain text, so this can only fail here. Un-ignore at re-pin time.
#[test]
#[ignore = "needs vendor/compiler re-pin carrying the eqnarray compiler change"]
fn eqnarray_sets_three_columns_with_kernel_spacing_and_numbers() {
    if !lm_available() {
        eprintln!("SKIP eqnarray_sets_three_columns_with_kernel_spacing_and_numbers: Latin Modern not installed");
        return;
    }
    let r = render_one(&doc(
        "\\begin{eqnarray}\nx &=& x \\\\\nlongvar &=& y\n\\end{eqnarray}",
    ));
    no_unsupported_feature(&r);
    let words = words_of(&r);
    let baselines: Vec<f64> = {
        let mut ys: Vec<f64> = words
            .iter()
            .filter(|w| w.text == "=")
            .map(|w| w.baseline)
            .collect();
        ys.sort_by(f64::total_cmp);
        ys
    };
    assert_eq!(
        baselines.len(),
        2,
        "two relation signs expected, got {words:?}"
    );
    let mut rows = Vec::new();
    for y in &baselines {
        let (cells, number) = row_on(&words, *y);
        assert_eq!(cells.len(), 3, "three columns expected, got {cells:?}");
        rows.push((cells, number));
    }
    // The relation column shares one tab stop: both `=` centres coincide.
    let centre = |eq: &Word| eq.x + eq.width / 2.0;
    assert!(
        (centre(rows[0].0[1]) - centre(rows[1].0[1])).abs() < 0.05,
        "relation column not aligned: {rows:?}"
    );
    // The documented quirk: exactly 2 x `\arraycolsep` of separation on
    // each side of the relation, with no `\thickmuskip` either side.
    for (cells, _) in &rows {
        let left_gap = cells[1].x - (cells[0].x + cells[0].width);
        let right_gap = cells[2].x - (cells[1].x + cells[1].width);
        for (side, gap) in [("left", left_gap), ("right", right_gap)] {
            assert!(
                (gap - SEP_BP).abs() <= TOL,
                "{side} gap {gap:.4} bp, pdflatex {SEP_BP:.4} bp: {cells:?}"
            );
        }
    }
    // The whole block is centred on the text width ...
    let left = rows
        .iter()
        .map(|(cells, _)| cells[0].x)
        .fold(f64::INFINITY, f64::min);
    let right = rows
        .iter()
        .map(|(cells, _)| cells[2].x + cells[2].width)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        ((left + right) / 2.0 - CENTRE_BP).abs() <= TOL,
        "block centre {:.4} bp, pdflatex {CENTRE_BP:.4} bp",
        (left + right) / 2.0
    );
    // ... and every row is numbered flush right against the equation counter.
    let numbers: Vec<String> = rows
        .iter()
        .filter_map(|(_, n)| n.map(|w| w.text.clone()))
        .collect();
    assert_eq!(numbers, ["(1)", "(2)"]);
    for (_, number) in &rows {
        let tag = number.expect("every eqnarray row is numbered");
        assert!(
            (tag.x + tag.width - RIGHT_BP).abs() <= 1.0,
            "number not flush right: {tag:?}"
        );
    }
}

/// Needs a `vendor/compiler` re-pin past the `eqnarray` compiler change
/// (issue #520); see above. Un-ignore at re-pin time.
#[test]
#[ignore = "needs vendor/compiler re-pin carrying the eqnarray compiler change"]
fn eqnarray_star_and_nonumber_suppress_numbering() {
    if !lm_available() {
        eprintln!("SKIP eqnarray_star_and_nonumber_suppress_numbering: Latin Modern not installed");
        return;
    }
    let r = render_one(&doc(
        "\\begin{eqnarray}\na &=& b \\nonumber \\\\\nc &=& d\n\\end{eqnarray}\n\\begin{eqnarray*}\ne &=& f\n\\end{eqnarray*}",
    ));
    no_unsupported_feature(&r);
    let words = words_of(&r);
    let numbers: Vec<&str> = words
        .iter()
        .filter(|w| w.text.starts_with('('))
        .map(|w| w.text.as_str())
        .collect();
    // Only the numbered row of the unstarred environment prints; the `*`
    // form prints none at all.
    assert_eq!(numbers, ["(1)"], "{words:?}");
    // Both environments still set real columns: four `=` signs on four
    // distinct rows.
    let mut ys: Vec<f64> = words
        .iter()
        .filter(|w| w.text == "=")
        .map(|w| w.baseline)
        .collect();
    ys.sort_by(f64::total_cmp);
    assert_eq!(ys.len(), 4, "{words:?}");
}
