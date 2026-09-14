//! `array`/`cases`/matrix grids laid out on the pipeline side (`mathgrid`):
//! rows stacked on `\@arstrut` with `\\[<dimen>]` extra depth, columns at
//! `\arraycolsep` with `l`/`c`/`r` alignment, `\,`/`\;` inside cells kept as
//! kerns, the grid `\vcenter`ed and (for `cases`) fenced.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// Every glyph of the document as `(char, x, baseline)` in bp (math and
/// the `\text` runs inside it alike).
fn math_glyphs(text: &str) -> (Vec<(char, f64, f64)>, Vec<flashtex_render_pipeline::display::Diagnostic>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for it in &page.to_items() {
            if let Item::GlyphRun(run) = it {
                for g in &run.glyphs {
                    let c = &run.clusters[g.cluster as usize];
                    let ch = run.text[c.text_start_byte as usize..c.text_end_byte as usize].chars().next().unwrap_or('?');
                    out.push((ch, g.origin_x.to_bp(), g.baseline_y.to_bp()));
                }
            }
        }
    }
    (out, r.v2.diagnostics.clone())
}

fn bp(pt: f64) -> f64 {
    pt * 72.0 / 72.27
}

#[test]
fn array_rows_stack_on_the_strut_with_row_skips_and_columns_at_arraycolsep() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // HW1's grid: two `l` columns, `\\[2pt]` after the first three rows.
    let src = "\\documentclass[11pt]{article}\\begin{document}\n\\[\n\\begin{array}{ll}\n\\text{(a)} & \\forall m\\,\\exists n\\; D(m,n),\\\\[2pt]\n\\text{(b)} & \\exists n\\,\\forall m\\; D(m,n),\\\\[2pt]\n\\text{(c)} & \\forall n\\,\\exists m\\; D(m,n),\\\\[2pt]\n\\text{(d)} & \\exists m\\,\\forall n\\; D(m,n).\n\\end{array}\n\\]\n\\end{document}";
    let (glyphs, diags) = math_glyphs(src);
    assert!(!diags.iter().any(|d| d.code == "math_limitation"), "no limitation left: {diags:?}");
    // Four rows: the `D` of each.
    let mut ds: Vec<(f64, f64)> = glyphs.iter().filter(|(c, ..)| *c == 'D').map(|(_, x, y)| (*x, *y)).collect();
    ds.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    assert_eq!(ds.len(), 4, "{ds:?}");
    // `\@arstrut` 0.7/0.3 x 13.6pt = 9.52/4.08 with `\baselineskip\z@`:
    // rows are 13.6pt apart plus the 2pt of `\\[2pt]`.
    for w in ds.windows(2) {
        assert!((w[1].1 - w[0].1 - bp(15.6)).abs() < 0.02, "row pitch: {}", w[1].1 - w[0].1);
    }
    // The second column is left-aligned: every `D`'s cell starts at the
    // same x (the quantifier is the first glyph of the cell).
    let mut quantifiers: Vec<(char, f64, f64)> = glyphs.iter().filter(|(c, ..)| matches!(c, '\u{2200}' | '\u{2203}')).cloned().collect();
    quantifiers.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
    let firsts: Vec<f64> = quantifiers.chunks(2).map(|row| row[0].1).collect();
    assert_eq!(firsts.len(), 4);
    assert!(firsts.iter().all(|x| (x - firsts[0]).abs() < 1e-6), "left-aligned column: {firsts:?}");
    // `\,` (3/18 em) and `\;` (5/18 em) inside a cell are kerns: the gap
    // between `m` and the second quantifier of row (a) holds the 3mu.
    let row_a: Vec<&(char, f64, f64)> = glyphs.iter().filter(|(_, _, y)| (*y - ds[0].1).abs() < 1e-6).collect();
    let m = row_a.iter().find(|(c, ..)| *c == 'm').expect("m");
    let exists = row_a.iter().find(|(c, ..)| *c == '\u{2203}').expect("exists");
    // `m` advance ~9.6pt at 10.95pt plus the thin space (1.8pt) plus the
    // Ord-Ord spacing (none): clearly more than the bare advance.
    assert!(exists.1 - m.1 > bp(9.6 + 1.5), "thin space kept: {}", exists.1 - m.1);
    // Second column starts 2\arraycolsep = 10pt after the first column's
    // widest cell, `(d)` (`d` is 0.6pt wider than `a`): the `)` of `(a)` to
    // `\forall` is that plus the difference.
    let close = row_a.iter().filter(|(c, ..)| *c == ')').map(|g| g.1).fold(f64::MAX, f64::min);
    let forall = row_a.iter().find(|(c, ..)| *c == '\u{2200}').expect("forall");
    let paren_w = 4.26; // ec-lmr10 `)` advance at 10.95pt
    let d_minus_a = 0.61; // ec-lmr10 `d` (0.5556em) - `a` (0.5em) at 10.95pt
    assert!((forall.1 - close - bp(paren_w + d_minus_a + 10.0)).abs() < 0.1, "2\\arraycolsep between columns: {}", forall.1 - close);
}

#[test]
fn cases_is_fenced_with_a_brace_sized_to_the_rows_and_quad_between_columns() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[11pt]{article}\\begin{document}\n\\[ f(x) = \\begin{cases} x & x \\geq 0 \\\\ -x & x < 0 \\end{cases} \\]\n\\end{document}";
    let (glyphs, diags) = math_glyphs(src);
    assert!(!diags.iter().any(|d| d.code == "math_limitation"), "{diags:?}");
    let braces: Vec<&(char, f64, f64)> = glyphs.iter().filter(|(c, ..)| *c == '{').collect();
    assert!(!braces.is_empty(), "a left brace is drawn: {glyphs:?}");
    // Two rows, `\arraystretch 1.2`: 1.2 x 13.6 = 16.32pt apart.
    let mut xs: Vec<(f64, f64)> = glyphs.iter().filter(|(c, ..)| *c == 'x').map(|(_, x, y)| (*x, *y)).collect();
    xs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.partial_cmp(&b.0).unwrap()));
    let rows: Vec<f64> = {
        let mut r: Vec<f64> = xs.iter().map(|g| g.1).collect();
        r.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        r
    };
    // The grid is `\vcenter`ed: the first row above the `f(x)` baseline,
    // the second below it.
    assert_eq!(rows.len(), 3, "the two case rows around the `f(x)` line: {rows:?}");
    assert!((rows[2] - rows[0] - bp(16.32)).abs() < 0.02, "row pitch: {}", rows[2] - rows[0]);
    // The brace is centred on the axis, between the rows.
    let brace_y = braces[0].2;
    assert!(brace_y > rows[0] - 1.0 && brace_y < rows[2] + 1.0, "brace between the rows: {brace_y} vs {rows:?}");
}

#[test]
fn grids_nested_in_sub_formulas_keep_their_rows() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // A pmatrix in a fraction numerator, a matrix inside `\left(...\right)^T`
    // and a matrix inside another matrix's cell: each is a box of rows, not
    // one flattened row, and the fences are sized to the rows.
    for body in [
        "\\frac{\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}}{2}",
        "\\left( \\begin{matrix} a & b \\\\ c & d \\end{matrix} \\right)^{T}",
        "\\begin{pmatrix} \\begin{matrix} a & b \\\\ c & d \\end{matrix} & 0 \\\\ 0 & 1 \\end{pmatrix}",
    ] {
        let src = format!("\\documentclass[12pt]{{article}}\\usepackage{{amsmath}}\\begin{{document}}\n\\[ {body} \\]\n\\end{{document}}");
        let (glyphs, diags) = math_glyphs(&src);
        assert!(!diags.iter().any(|d| d.code == "math_limitation"), "{body}: {diags:?}");
        let y = |ch: char| glyphs.iter().find(|g| g.0 == ch).map(|g| (g.1, g.2)).unwrap_or_else(|| panic!("{body}: no {ch}"));
        let (a, b, c, d) = (y('a'), y('b'), y('c'), y('d'));
        assert!((a.1 - b.1).abs() < 1e-6 && (c.1 - d.1).abs() < 1e-6, "{body}: a/b and c/d share rows");
        assert!(c.1 - a.1 > 10.0, "{body}: the second row sits below the first: {a:?} {c:?}");
        // Centred columns: `a` and `c` (different widths) share a column.
        assert!((a.0 - c.0).abs() < 2.0 && b.0 > a.0 + 5.0, "{body}: columns line up: {a:?} {b:?} {c:?}");
        let parens: Vec<f64> = glyphs.iter().filter(|g| g.0 == '(').map(|g| g.2).collect();
        assert!(!parens.is_empty(), "{body}: an opening fence is drawn");
    }
}
