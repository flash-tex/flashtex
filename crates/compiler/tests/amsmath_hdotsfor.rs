//! amsmath `\hdotsfor[spacing]{n}` in grid environments (issue: dotted row
//! missing with "\hdotsfor is not supported in math mode").
//!
//! amsmath.sty 1106-1115 defines `\hdotsfor` as `\multicolumn{n}{c}`
//! filled with dot leaders. Measured with TeX Live 2026 pdflatex,
//! `pdflatex -interaction=nonstopmode hdots.tex` where hdots.tex is
//! `\documentclass{article}\usepackage{amsmath}\begin{document}`
//! `\[\begin{pmatrix} a & b & c\\ \hdotsfor{3} \\ d&e&f\end{pmatrix}\]`
//! `\end{document}`: 0 errors, 3 rows, the middle row 8 leader dots
//! spanning the full 3-column width (dots at y 149.17-158.02pt between
//! the `abc` row at y 137.22pt and the `def` row at y 161.13pt).

use flashtex_compiler::math::{layout, MathList, Nucleus};
use flashtex_compiler::parser::{parse, Block, Inline};

fn amsmath_matrix(body: &str) -> Vec<Vec<MathList>> {
    let source = format!(
        "\\documentclass{{article}}\\usepackage{{amsmath}}\\begin{{document}}{body}\\end{{document}}"
    );
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{body}: {:?}",
        parsed.diagnostics
    );
    for block in &parsed.blocks {
        let Block::Paragraph(content) = block else {
            continue;
        };
        for inline in content {
            if let Inline::Math { list, .. } = inline {
                for atom in &list.atoms {
                    if let Nucleus::Matrix { rows, .. } = &atom.nucleus {
                        return rows.clone();
                    }
                }
            }
        }
    }
    panic!("no matrix in {body}");
}

fn cell_text(cell: &MathList) -> String {
    cell.atoms
        .iter()
        .map(|a| match &a.nucleus {
            Nucleus::Symbol(s) | Nucleus::Text(s) => s.clone(),
            _ => format!("{:?}", a.nucleus),
        })
        .collect()
}

#[test]
fn hdotsfor_spans_three_columns_with_dots() {
    let rows =
        amsmath_matrix("$\\begin{pmatrix} a & b & c\\\\ \\hdotsfor{3} \\\\ d&e&f\\end{pmatrix}$");
    // pdflatex sets 3 rows (bbox rows at y 137.22, 149.17 and 161.13pt).
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[0].len(), 3);
    assert_eq!(rows[2].len(), 3);
    // The dotted row spans all 3 columns with dots.
    assert_eq!(rows[1].len(), 3, "{rows:?}");
    for cell in &rows[1] {
        assert_eq!(cell_text(cell), "...", "{cell:?}");
    }
    assert_eq!(cell_text(&rows[0][0]), "a");
    assert_eq!(cell_text(&rows[2][2]), "f");
}

#[test]
fn hdotsfor_optional_spacing_spans_with_dots() {
    let rows = amsmath_matrix(
        "$\\begin{pmatrix} a & b & c\\\\ \\hdotsfor[2]{3} \\\\ d&e&f\\end{pmatrix}$",
    );
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[1].len(), 3, "{rows:?}");
    for cell in &rows[1] {
        assert_eq!(cell_text(cell), "...", "{cell:?}");
    }
}

#[test]
fn hdotsfor_mid_row_span_starts_at_its_column() {
    let rows = amsmath_matrix(
        "$\\begin{pmatrix} a & b & c\\\\ x & \\hdotsfor{2} \\\\ d&e&f\\end{pmatrix}$",
    );
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[1].len(), 3, "{rows:?}");
    assert_eq!(cell_text(&rows[1][0]), "x");
    assert_eq!(cell_text(&rows[1][1]), "...");
    assert_eq!(cell_text(&rows[1][2]), "...");
}

#[test]
fn hdotsfor_overspan_is_tolerated_like_pdflatex() {
    // pdflatex sets `\hdotsfor{5}` in a 2-column matrix with no error.
    let rows = amsmath_matrix("$\\begin{pmatrix} a & b \\\\ \\hdotsfor{5} \\\\ c&d\\end{pmatrix}$");
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[1].len(), 5, "{rows:?}");
    for cell in &rows[1] {
        assert_eq!(cell_text(cell), "...", "{cell:?}");
    }
}

#[test]
fn hdotsfor_trailing_content_sets_after_the_dots() {
    // pdflatex sets `\hdotsfor{2}x` (no error) as dots followed by `x`.
    let rows =
        amsmath_matrix("$\\begin{pmatrix} a & b & c \\\\ \\hdotsfor{2}x \\\\ d&e&f\\end{pmatrix}$");
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[1].len(), 2, "{rows:?}");
    assert_eq!(cell_text(&rows[1][0]), "...");
    assert_eq!(cell_text(&rows[1][1]), "...x");
}

#[test]
fn hdotsfor_matrix_lays_out_with_dots_present() {
    // The reported bug: the dotted row was missing entirely (FlashTeX
    // errored "\hdotsfor is not supported in math mode"). pdflatex sets one
    // `\multicolumn{3}{c}` row of dot leaders — measured with TeX Live 2026
    // pdflatex, `pdflatex -interaction=nonstopmode hdots.tex` on the file
    // quoted at the top of this module, then `pdftotext -layout hdots.pdf`:
    // the middle text line holds 8 `.` chars. This compiler approximates
    // that spanning leader row as one `...` cell per spanned column, so the
    // exact painted dot count is deliberately NOT pinned here (it is 9, not
    // pdflatex's 8): only that the grid lays out with no error and paints
    // dots. Row shape is pinned by `hdotsfor_spans_three_columns_with_dots`
    // and dots-row height by `hdotsfor_dots_row_is_as_tall_as_typed_dots`.
    let source = "\\documentclass{article}\\usepackage{amsmath}\\begin{document}$\\begin{pmatrix} a & b & c\\\\ \\hdotsfor{3} \\\\ d&e&f\\end{pmatrix}$\\end{document}";
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    for block in &parsed.blocks {
        let Block::Paragraph(content) = block else {
            continue;
        };
        for inline in content {
            if let Inline::Math { list, .. } = inline {
                let mut diagnostics = Vec::new();
                let laid = layout(list, 10.0, &mut diagnostics);
                assert!(diagnostics.is_empty(), "{diagnostics:?}");
                assert!(laid.width > 0.0);
                let dots = laid.items.iter().filter(|item| item.text == ".").count();
                assert!(dots > 0, "dotted row painted no dots: {laid:?}");
                return;
            }
        }
    }
    panic!("no formula");
}

#[test]
fn hdotsfor_dots_row_is_as_tall_as_typed_dots() {
    // The spanned cells are the same `...` atoms a typed row would parse
    // to, so the dots row is exactly as tall as pdflatex's dots content.
    let rows =
        amsmath_matrix("$\\begin{pmatrix} a & b & c\\\\ \\hdotsfor{3} \\\\ d&e&f\\end{pmatrix}$");
    let mut diagnostics = Vec::new();
    let dotted = layout(&rows[1][0], 10.0, &mut diagnostics);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let literal = amsmath_matrix("$\\begin{pmatrix}...\\end{pmatrix}$");
    let mut diagnostics = Vec::new();
    let typed = layout(&literal[0][0], 10.0, &mut diagnostics);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(dotted.ascent, typed.ascent);
    assert_eq!(dotted.descent, typed.descent);
    assert_eq!(dotted.width, typed.width);
}

fn messages_of(body: &str, preamble: &str) -> Vec<String> {
    let source =
        format!("\\documentclass{{article}}{preamble}\\begin{{document}}{body}\\end{{document}}");
    parse(&source)
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn hdotsfor_without_amsmath_is_rejected() {
    // pdflatex under plain article: `! Undefined control sequence`.
    let found = messages_of("$\\begin{pmatrix} a \\\\ \\hdotsfor{1} \\end{pmatrix}$", "");
    assert!(found.iter().any(|m| m.contains("\\hdotsfor")), "{found:?}");
}

#[test]
fn hdotsfor_after_cell_content_still_errors() {
    // pdflatex: `! Misplaced \omit` for `x\hdotsfor{2}`.
    let found = messages_of(
        "$\\begin{pmatrix} a & b \\\\ x\\hdotsfor{2} \\\\ c&d\\end{pmatrix}$",
        "\\usepackage{amsmath}",
    );
    assert!(found.iter().any(|m| m.contains("\\hdotsfor")), "{found:?}");
}

#[test]
fn hdotsfor_outside_a_grid_still_errors() {
    // pdflatex: `\hdotsfor` outside an alignment is misplaced (`\omit`).
    let found = messages_of("$a \\hdotsfor{2} b$", "\\usepackage{amsmath}");
    assert!(found.iter().any(|m| m.contains("\\hdotsfor")), "{found:?}");
}

#[test]
fn hdotsfor_bad_count_errors_and_keeps_trailing_content() {
    // pdflatex: `! Missing number, treated as zero` for `\hdotsfor{abc}`.
    let source = "\\documentclass{article}\\usepackage{amsmath}\\begin{document}$\\begin{pmatrix} a \\\\ \\hdotsfor{abc}x \\end{pmatrix}$\\end{document}";
    let parsed = parse(source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\hdotsfor")),
        "{:?}",
        parsed.diagnostics
    );
    for block in &parsed.blocks {
        let Block::Paragraph(content) = block else {
            continue;
        };
        for inline in content {
            if let Inline::Math { list, .. } = inline {
                for atom in &list.atoms {
                    if let Nucleus::Matrix { rows, .. } = &atom.nucleus {
                        assert_eq!(rows.len(), 2, "{rows:?}");
                        assert_eq!(cell_text(&rows[1][0]), "x");
                        return;
                    }
                }
            }
        }
    }
    panic!("no matrix");
}
