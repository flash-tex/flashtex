//! `@{...}` spaces are template text on every row.
//!
//! pdflatex (TeX Live 2026) over
//! `\begin{tabular}{@{}l@{ : }r@{}}a&b\\cc&dd\end{tabular}` sets each row as
//! the entry, interword glue, `:`, interword glue, the entry: the `\showbox`
//! log shows `\glue 3.33333` before the colon and `\glue 4.44444` after it
//! (cmr10, the colon's spacefactor 2000) identically in both rows, and
//! `pdftotext -bbox` reads the words `a`, `:`, `b` / `cc`, `:`, `dd`.
//!
//! FlashTeX models interword space uniformly (its own text font, no
//! spacefactor — running `a : b` gets the same gap on both sides), so this
//! test pins what the fix owns: the `@`-expression's text, spaces included,
//! is inserted identically on every row, each gap exactly one engine
//! interword space wide, and each row reads `a : b` / `cc : dd` like
//! pdflatex's. Before the fix the edge spaces never reached the template
//! box (the paragraph parse folds a leading space into `space_before`, a
//! no-op at the start of the detached box, and drops a trailing one), so
//! the second row laid out as `cc:dd`.

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};

const SOURCE: &str = r"\documentclass{article}\begin{document}\begin{tabular}{@{}l@{ : }r@{}}a&b\\cc&dd\end{tabular}\end{document}";

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

/// Laid-out `(x, text)` items of one baseline, left to right.
fn row(output: &CompileOutput, baseline: f64) -> Vec<(f64, String)> {
    let mut items: Vec<(f64, String)> = output.pages[0]
        .items
        .iter()
        .filter(|item| (item.baseline_y_pt - baseline).abs() < 0.01)
        .map(|item| (item.x_pt, item.text.clone()))
        .collect();
    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    items
}

fn rows(output: &CompileOutput) -> Vec<Vec<(f64, String)>> {
    let mut ys: Vec<f64> = output.pages[0]
        .items
        .iter()
        .map(|item| item.baseline_y_pt)
        .collect();
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys.dedup();
    ys.into_iter().map(|y| row(output, y)).collect()
}

fn texts(row: &[(f64, String)]) -> Vec<&str> {
    row.iter().map(|(_, text)| text.as_str()).collect()
}

#[test]
fn at_expression_spaces_inserted_identically_on_every_row() {
    let output = compile(SOURCE);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let rows = rows(&output);
    assert_eq!(rows.len(), 2);
    assert_eq!(texts(&rows[0]), ["a", " ", ":", " ", "b"]);
    assert_eq!(texts(&rows[1]), ["cc", " ", ":", " ", "dd"]);
    // Each row reads like pdflatex's words joined with single spaces.
    let joined: Vec<String> = rows
        .iter()
        .map(|row| row.iter().map(|(_, text)| text.clone()).collect())
        .collect();
    assert_eq!(joined, ["a : b", "cc : dd"]);
    // Identical insertion: the space and colon advances match across rows.
    let gap = |row: &[(f64, String)], before: usize| row[before + 1].0 - row[before].0;
    assert!((gap(&rows[0], 1) - gap(&rows[1], 1)).abs() < 0.011);
    assert!((gap(&rows[0], 2) - gap(&rows[1], 2)).abs() < 0.011);
}

#[test]
fn at_expression_space_is_one_interword_space() {
    // The engine's own interword space, with no glyph widths involved: the
    // colon's offset in running `a : b` minus the advance of `a`.
    let x = |output: &CompileOutput, text: &str| {
        output.pages[0]
            .items
            .iter()
            .find(|item| item.text == text)
            .map(|item| item.x_pt)
            .expect(text)
    };
    let spaced = compile(r"\documentclass{article}\begin{document}a : b\end{document}");
    // The advance of `a` from a two-column caliper, so no glyph width is
    // hardcoded: the running gap is the colon's offset minus that advance.
    let caliper = compile(
        r"\documentclass{article}\begin{document}\begin{tabular}{@{}l@{}l@{}}a&x\end{tabular}\end{document}",
    );
    let advance_a = x(&caliper, "x") - x(&caliper, "a");
    let running_gap = (x(&spaced, ":") - x(&spaced, "a")) - advance_a;
    assert!(running_gap > 1.0, "sanity: the control gap is a real space");
    // Row 2 of the table (the row the bug emptied): `cc`, ` `, `:`, ` `, `dd`.
    let output = compile(SOURCE);
    let rows = rows(&output);
    assert_eq!(texts(&rows[1]), ["cc", " ", ":", " ", "dd"]);
    let second = &rows[1];
    let leading = second[2].0 - second[1].0;
    let trailing = second[4].0 - second[3].0;
    assert!(
        (leading - running_gap).abs() < 0.02,
        "{leading} vs {running_gap}"
    );
    assert!(
        (trailing - running_gap).abs() < 0.02,
        "{trailing} vs {running_gap}"
    );
}

#[test]
fn at_expression_without_spaces_unchanged() {
    // No edge spaces: no space runs are synthesised, exactly as before.
    for (source, first, second) in [
        (
            r"\documentclass{article}\begin{document}\begin{tabular}{@{}l@{:}r@{}}a&b\\cc&dd\end{tabular}\end{document}",
            vec!["a", ":", "b"],
            vec!["cc", ":", "dd"],
        ),
        (
            r"\documentclass{article}\begin{document}\begin{tabular}{@{}l@{--}r@{}}a&b\\cc&dd\end{tabular}\end{document}",
            vec!["a", "\u{2013}", "b"],
            vec!["cc", "\u{2013}", "dd"],
        ),
    ] {
        let output = compile(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let rows = rows(&output);
        assert_eq!(rows.len(), 2);
        assert_eq!(texts(&rows[0]), first);
        assert_eq!(texts(&rows[1]), second);
    }
}
