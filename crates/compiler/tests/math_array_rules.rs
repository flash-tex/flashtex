//! `\hline` and `\cline` inside a math `array`.
//!
//! Oracle: pdflatex (TeX Live 2026, 10pt article), run as
//! `pdflatex -interaction=nonstopmode <file>.tex` with `\typeout` and
//! `\showbox` probes. Measurements cited below (all in TeX pt, where
//! 1pt = 72/72.27bp, so 0.1bp = 0.1004pt):
//!
//! * `m.tex`: `$\begin{array}{ccc}1&2&3\\4&5&6\end{array}$` gives
//!   `PLAIN WD=45.00005pt HT=14.5pt DP=9.5pt`; with `\\\hline` it gives
//!   `HLINE WD=45.00005pt HT=14.7pt DP=9.7pt`, i.e. one full-width
//!   `\rule(0.4+0.0)x45.00005` taking 0.4pt of vertical space, rows
//!   recentred ±0.2pt. With `\\\cline{1-1}` it gives
//!   `CLINE WD=30.00003pt HT=14.5pt DP=9.5pt`: an `\hbox(0.4+0.0)`
//!   leaders rule over column 1 only, wrapped in `\glue(\lineskip) 0.0`
//!   before and `\glue -0.4` after, i.e. zero net vertical space.
//! * `m2.tex`: `\\\hline\hline` gives `DOUBLE WD=30.00003pt HT=15.7pt
//!   DP=10.7pt`, laid out as rule, `\glue 2.0`, `\glue -0.4`, rule:
//!   consecutive rules are `\doublerulesep` (2pt) apart top-to-top.
//!   `\cline{1-5}` in a 2-column array errors with
//!   `! Extra alignment tab has been changed to \cr.`
//! * `m3.tex`: leading `\hline` gives `LEAD HT=14.7pt DP=9.7pt` and
//!   trailing `\hline` gives `TRAIL HT=14.7pt DP=9.7pt` (the same ±0.2pt
//!   recentring); `\cline{2-2}` leaves `HT=14.5pt DP=9.5pt` unchanged.
//! * `edge.tex`: `\hline` mid-row (`1\hline 2`) errors with
//!   `! Misplaced \noalign.` (`\hline ->\noalign`); `\hline` after `\\`
//!   in `pmatrix`/`cases` is accepted by pdflatex but stays out of scope
//!   here (only `array` is handled).
use flashtex_compiler::diagnostics::Diagnostic;
use flashtex_compiler::lexer::tokenize;
use flashtex_compiler::math::{
    layout, parse_tokens, MathBox, MathItem, MathList, MathPackages, Nucleus,
};

/// 0.1bp in TeX pt: every positional assertion below compares against a
/// measured pdflatex value within this tolerance.
const TOL_PT: f64 = 0.1 * 72.27 / 72.0;

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOL_PT,
        "{what}: {actual} != {expected} (tolerance 0.1bp)"
    );
}

fn parsed(source: &str) -> (MathList, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let list = parse_tokens(&tokenize(source), MathPackages::KERNEL, &mut diagnostics);
    (list, diagnostics)
}

fn lay_out(source: &str) -> (MathBox, Vec<Diagnostic>) {
    let (list, mut diagnostics) = parsed(source);
    let laid = layout(&list, 10.0, &mut diagnostics);
    (laid, diagnostics)
}

/// The rule-carrying items of a laid-out grid.
fn rule_items(laid: &MathBox) -> Vec<&MathItem> {
    laid.items
        .iter()
        .filter(|item| item.rule.is_some())
        .collect()
}

/// Everything but the rules: glyphs whose positions rules must not disturb.
fn cell_items(laid: &MathBox) -> Vec<&MathItem> {
    laid.items
        .iter()
        .filter(|item| item.rule.is_none())
        .collect()
}

/// Stretched `\left`/`\right` fences around a grid: not cell rows.
const FENCE_GLYPHS: [&str; 8] = ["(", ")", "[", "]", "{", "}", "|", "‖"];

/// Row baselines, top row first, from the glyph items of a two-row grid.
fn row_baselines(laid: &MathBox) -> (f64, f64) {
    let mut baselines: Vec<f64> = cell_items(laid)
        .iter()
        .filter(|item| !FENCE_GLYPHS.contains(&item.text.as_str()))
        .map(|item| item.baseline)
        .collect();
    baselines.sort_by(|a, b| a.partial_cmp(b).unwrap());
    baselines.dedup();
    assert_eq!(baselines.len(), 2, "two-row grid, got {baselines:?}");
    (baselines[0], baselines[1])
}

/// Glyph geometry without source spans: comparable across different
/// source strings that lay out identically.
fn cell_geometry(laid: &MathBox) -> Vec<(&str, f64, f64, f64)> {
    cell_items(laid)
        .iter()
        .map(|item| (item.text.as_str(), item.x, item.baseline, item.size))
        .collect()
}

#[test]
fn hline_draws_a_full_width_rule_and_shifts_later_rows_like_pdflatex() {
    let (laid, diagnostics) = lay_out(r"\begin{array}{c c c} 1&2&3\\\hline 4&5&6\end{array}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (plain, _) = lay_out(r"\begin{array}{c c c} 1&2&3\\4&5&6\end{array}");
    let rules = rule_items(&laid);
    assert_eq!(rules.len(), 1, "{laid:?}");
    let rule = rules[0].rule.expect("filtered for rules");
    // pdflatex `\rule(0.4+0.0)`: `\arrayrulewidth` thick, full grid width.
    close(rule.height, 0.4, "hline thickness");
    close(rules[0].x, 0.0, "hline starts at the grid edge");
    close(rule.width, laid.width, "hline spans the grid");
    // The rule takes 0.4pt of vertical space: the whole grid recentres,
    // exactly pdflatex's HT/DP +0.2pt each way.
    let (row1, row2) = row_baselines(&laid);
    let (plain1, plain2) = row_baselines(&plain);
    close(row1 - plain1, -0.2, "row 1 recentring");
    close(row2 - plain2, 0.2, "row 2 shift (the reported 0.398bp)");
    // pdflatex puts the rule's top on the row above's bottom edge; the
    // compiler's minimum row descent is 0.2 times the size (see
    // `layout_matrix`'s descent fold).
    close(rule.y, row1 + 0.2 * 10.0, "hline top");
    // Glyph columns do not move horizontally.
    assert_eq!(
        cell_items(&laid)
            .iter()
            .map(|item| item.x)
            .collect::<Vec<_>>(),
        cell_items(&plain)
            .iter()
            .map(|item| item.x)
            .collect::<Vec<_>>(),
    );
    // The alignment letters stay first in `columns` so downstream readers
    // that take one letter per column are unaffected; the rule rides in
    // the trailer.
    let (list, _) = parsed(r"\begin{array}{c c c} 1&2&3\\\hline 4&5&6\end{array}");
    let Nucleus::Matrix { columns, .. } = &list.atoms[0].nucleus else {
        panic!("not a grid: {:?}", list.atoms);
    };
    assert!(columns.starts_with("ccc"), "{columns:?}");
    assert!(columns.contains("hline@1"), "{columns:?}");
}

#[test]
fn hline_in_the_fenced_issue_repro() {
    // `$\left(\begin{array}{c c c} 1&2&3\\\hline 4&5&6\end{array}\right)$`:
    // zero diagnostics, one 0.4pt rule, rows shifted exactly as unfenced.
    let src = r"\left(\begin{array}{c c c} 1&2&3\\\hline 4&5&6\end{array}\right)";
    let (laid, diagnostics) = lay_out(src);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let rules = rule_items(&laid);
    assert_eq!(rules.len(), 1, "{laid:?}");
    close(
        rules[0].rule.expect("filtered for rules").height,
        0.4,
        "rule thickness",
    );
    let plain_src = r"\left(\begin{array}{c c c} 1&2&3\\4&5&6\end{array}\right)";
    let (plain, _) = lay_out(plain_src);
    let (row1, row2) = row_baselines(&laid);
    let (plain1, plain2) = row_baselines(&plain);
    close(row1 - plain1, -0.2, "row 1 recentring");
    close(row2 - plain2, 0.2, "row 2 shift");
    // The rule covers every digit column even with fences outside it.
    let (rule_x, rule_end) = (rules[0].x, rules[0].x + rules[0].rule.unwrap().width);
    for cell in cell_items(&laid) {
        if FENCE_GLYPHS.contains(&cell.text.as_str()) {
            continue;
        }
        assert!(
            rule_x <= cell.x && cell.x <= rule_end,
            "column {} outside rule [{rule_x}, {rule_end}]",
            cell.text
        );
    }
}

#[test]
fn double_hline_rules_are_doublerulesep_apart() {
    // pdflatex: rule, `\glue 2.0`, `\glue -0.4`, rule — 2.0pt top-to-top,
    // 2.4pt of vertical space, rows recentred ±1.2pt.
    let (laid, diagnostics) = lay_out(r"\begin{array}{cc} 1&2\\\hline\hline 3&4\end{array}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (plain, _) = lay_out(r"\begin{array}{cc} 1&2\\3&4\end{array}");
    let rules = rule_items(&laid);
    assert_eq!(rules.len(), 2, "{laid:?}");
    let first = rules[0].rule.expect("filtered for rules");
    let second = rules[1].rule.expect("filtered for rules");
    close(first.height, 0.4, "first rule thickness");
    close(second.height, 0.4, "second rule thickness");
    close(second.y - first.y, 2.0, "doublerulesep top-to-top gap");
    close(
        (laid.ascent + laid.descent) - (plain.ascent + plain.descent),
        2.4,
        "vertical space taken",
    );
    let (row1, row2) = row_baselines(&laid);
    let (plain1, plain2) = row_baselines(&plain);
    close(row1 - plain1, -1.2, "row 1 recentring");
    close(row2 - plain2, 1.2, "row 2 shift");
}

#[test]
fn leading_and_trailing_hlines_cap_the_grid() {
    // pdflatex LEAD/TRAIL: HT=14.7pt DP=9.7pt, the same ±0.2pt recentring.
    for (src, above) in [
        (r"\begin{array}{cc}\hline 1&2\\3&4\end{array}", true),
        (r"\begin{array}{cc} 1&2\\3&4\\\hline\end{array}", false),
    ] {
        let (laid, diagnostics) = lay_out(src);
        assert!(diagnostics.is_empty(), "{src}: {diagnostics:?}");
        let (plain, _) = lay_out(r"\begin{array}{cc} 1&2\\3&4\end{array}");
        let rules = rule_items(&laid);
        assert_eq!(rules.len(), 1, "{src}: {laid:?}");
        let rule = rules[0].rule.expect("filtered for rules");
        close(rule.height, 0.4, "{src}: rule thickness");
        let (row1, row2) = row_baselines(&laid);
        let (plain1, plain2) = row_baselines(&plain);
        if above {
            close(rule.y, -laid.ascent, "{src}: rule caps the grid");
            close(row1 - plain1, 0.2, "{src}: row 1 moves down");
            close(row2 - plain2, 0.2, "{src}: row 2 moves down");
        } else {
            close(
                rule.y + rule.height,
                laid.descent,
                "{src}: rule extends the grid",
            );
            close(row1 - plain1, -0.2, "{src}: row 1 moves up");
            close(row2 - plain2, -0.2, "{src}: row 2 moves up");
        }
    }
}

#[test]
fn cline_covers_its_columns_and_takes_no_space() {
    // pdflatex CLINE: same HT/DP as without, rule over column 1 only.
    let (laid, diagnostics) = lay_out(r"\begin{array}{cc} 1&2\\\cline{1-1} 3&4\end{array}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (plain, _) = lay_out(r"\begin{array}{cc} 1&2\\3&4\end{array}");
    // Zero net vertical space: every glyph sits exactly where it would
    // without the rule (spans differ — the sources differ — so geometry
    // is compared, not whole items).
    assert_eq!(cell_geometry(&laid), cell_geometry(&plain));
    assert!((laid.width - plain.width).abs() < 1e-12);
    assert!((laid.ascent - plain.ascent).abs() < 1e-12);
    assert!((laid.descent - plain.descent).abs() < 1e-12);
    let rules = rule_items(&laid);
    assert_eq!(rules.len(), 1, "{laid:?}");
    let rule = rules[0].rule.expect("filtered for rules");
    close(rule.height, 0.4, "cline thickness");
    close(rules[0].x, 0.0, "cline starts at column 1");
    assert!(rule.width > 0.0 && rule.width < laid.width, "{rule:?}");
    // The rule's top is on the row above's bottom edge, like `\hline`.
    let (row1, _) = row_baselines(&laid);
    close(rule.y, row1 + 0.2 * 10.0, "cline top");
    // A full-width `\cline{1-2}` spans the grid; `\cline{2-2}` starts
    // past column 1.
    let (full, diagnostics) = lay_out(r"\begin{array}{cc} 1&2\\\cline{1-2} 3&4\end{array}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let rules = rule_items(&full);
    assert_eq!(rules.len(), 1, "{full:?}");
    close(rules[0].x, 0.0, "full cline starts at the grid edge");
    close(
        rules[0].rule.unwrap().width,
        full.width,
        "full cline spans the grid",
    );
    let (second, diagnostics) = lay_out(r"\begin{array}{cc} 1&2\\\cline{2-2} 3&4\end{array}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let rules = rule_items(&second);
    assert_eq!(rules.len(), 1, "{second:?}");
    assert!(rules[0].x > 0.0, "{rules:?}");
    close(
        rules[0].x + rules[0].rule.unwrap().width,
        second.width,
        "second-column cline ends at the grid edge",
    );
}

#[test]
fn hline_with_a_vertical_bar_in_the_column_spec() {
    // The issue's second repro shape: `|` is not a rule this change draws,
    // but the `\hline` in it must still parse cleanly and shift row 2 by
    // the measured 0.4pt in total (±0.2pt recentring).
    let (laid, diagnostics) = lay_out(r"\begin{array}{c|cc} 1&2&3\\\hline 4&5&6\end{array}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (plain, _) = lay_out(r"\begin{array}{c|cc} 1&2&3\\4&5&6\end{array}");
    assert_eq!(rule_items(&laid).len(), 1, "{laid:?}");
    let (row1, row2) = row_baselines(&laid);
    let (plain1, plain2) = row_baselines(&plain);
    close(row1 - plain1, -0.2, "row 1 recentring");
    close(row2 - plain2, 0.2, "row 2 shift");
}

#[test]
fn misplaced_hline_is_rejected_like_pdflatex() {
    // pdflatex `edge.tex`: `1\hline 2` gives `! Misplaced \noalign.`
    let (laid, diagnostics) = lay_out(r"\begin{array}{cc} 1\hline 2\\3&4\end{array}");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(
        diagnostics[0].message.contains("Misplaced \\noalign."),
        "{diagnostics:?}"
    );
    assert!(rule_items(&laid).is_empty(), "{laid:?}");
}

#[test]
fn bad_cline_range_is_rejected_like_the_text_tables() {
    // pdflatex errors on `\cline{1-5}` in a 2-column array
    // (`! Extra alignment tab has been changed to \cr.`); the text tables
    // diagnose it as a bad range, so the math array does the same.
    for src in [
        r"\begin{array}{cc} 1&2\\\cline{1-5} 3&4\end{array}",
        r"\begin{array}{cc} 1&2\\\cline{2-1} 3&4\end{array}",
        r"\begin{array}{cc} 1&2\\\cline{xx} 3&4\end{array}",
    ] {
        let (laid, diagnostics) = lay_out(src);
        assert_eq!(diagnostics.len(), 1, "{src}: {diagnostics:?}");
        assert!(
            diagnostics[0]
                .message
                .contains("must name a column range within columns 1-2"),
            "{src}: {diagnostics:?}"
        );
        assert!(rule_items(&laid).is_empty(), "{src}: {laid:?}");
    }
    let (_, diagnostics) = lay_out(r"\begin{array}{cc} 1&2\\\cline 3&4\end{array}");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(
        diagnostics[0]
            .message
            .contains("\\cline requires an argument"),
        "{diagnostics:?}"
    );
}

#[test]
fn hline_and_cline_stay_rejected_outside_arrays() {
    // `\hline`/`\cline` are kernel commands that only work in a table
    // context; pdflatex rejects them in plain math, and so does the
    // compiler — they were not added to the global math commands.
    for src in [r"a \hline b", r"a \cline{1-1} b"] {
        let (_, diagnostics) = lay_out(src);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("is not supported in math mode")),
            "{src}: {diagnostics:?}"
        );
    }
}
