//! Text commands inside math inherit the ambient text shape: in an italic
//! `amsthm` plain-style theorem body `\text{...}` (and `\mbox`, `\hbox`,
//! `\textrm`, which change family only) stay italic, `\textup`/`\textnormal`
//! reset to upright, and `\textbf` adds bold to the italic. Outside the
//! theorem everything stays upright.
//!
//! Oracle: TeX Live 2026 pdflatex `\showoutput` on the minimal document in
//! `/tmp/textshape-oracle/oracle.tex` traces `\text{ whenever }` in the
//! theorem as `\OT1/cmr/m/it/10` (italic), `\textup{upright}` as
//! `\OT1/cmr/m/n/10`, `\textrm{roman}` as `\OT1/cmr/m/it/10`,
//! `\textbf{bold}` as `\OT1/cmr/bx/it/10`, display `\text{for all }` as
//! italic, and the same `\text` outside the theorem as `\OT1/cmr/m/n/10`;
//! `pdffonts` lists CMTI10 and CMBXTI10.

use flashtex_compiler::math::{MathList, Nucleus, TextPiece, TextStyle as MathFace};
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage{{amsmath,amsthm}}\n\\newtheorem{{theorem}}{{Theorem}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

/// The literal-text pieces of every formula in the document, in order: one
/// entry per formula, each a list of `(text, face)` pairs. `Nucleus::Text`
/// (the all-upright fast path) reports as `Normal`.
fn math_texts(body: &str) -> Vec<Vec<(String, MathFace)>> {
    let parsed = parse(&doc(body));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let (Block::Paragraph(content) | Block::Styled { content, .. }) = block else {
            continue;
        };
        for inline in content {
            if let Inline::Math { list, .. } = inline {
                let mut pieces = Vec::new();
                for atom in &list.atoms {
                    match &atom.nucleus {
                        Nucleus::Text(text) => pieces.push((text.clone(), MathFace::Normal)),
                        // `\textbf`'s all-upright fast path (and `\mathbf`).
                        Nucleus::Bold(text) => pieces.push((text.clone(), MathFace::Bold)),
                        Nucleus::TextRun(run) => {
                            for piece in run {
                                if let TextPiece::Text { text, style } = piece {
                                    pieces.push((text.clone(), *style));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                out.push(pieces);
            }
        }
    }
    out
}

fn pieces_of_list(list: &MathList) -> Vec<(String, MathFace)> {
    let mut pieces = Vec::new();
    for atom in &list.atoms {
        match &atom.nucleus {
            Nucleus::Text(text) => pieces.push((text.clone(), MathFace::Normal)),
            // `\textbf`'s all-upright fast path (and `\mathbf`).
            Nucleus::Bold(text) => pieces.push((text.clone(), MathFace::Bold)),
            Nucleus::TextRun(run) => {
                for piece in run {
                    if let TextPiece::Text { text, style } = piece {
                        pieces.push((text.clone(), *style));
                    }
                }
            }
            _ => {}
        }
    }
    pieces
}

/// The literal-text pieces of every `align`-style display cell, in order:
/// one entry per cell, each a list of `(text, face)` pairs.
fn display_cell_texts(source: &str) -> Vec<Vec<(String, MathFace)>> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let content = match block {
            Block::Paragraph(content) | Block::Styled { content, .. } => content,
            _ => continue,
        };
        for inline in content {
            if let Inline::MathRows { rows, .. } = inline {
                for row in rows {
                    for cell in &row.cells {
                        out.push(pieces_of_list(cell));
                    }
                }
            }
        }
    }
    out
}

fn face_of(body: &str, needle: &str) -> MathFace {
    let formulas = math_texts(body);
    for pieces in &formulas {
        let merged: String = pieces.iter().map(|(text, _)| text.as_str()).collect();
        if merged.contains(needle) {
            let face = pieces
                .iter()
                .find(|(text, _)| text.contains(needle))
                .map(|(_, face)| *face);
            return face.unwrap_or_else(|| panic!("{needle:?} split across pieces: {pieces:?}"));
        }
    }
    panic!("{needle:?} not typeset as math text in {formulas:?}")
}

#[test]
fn text_in_theorem_body_stays_italic() {
    assert_eq!(
        face_of(
            "\\begin{theorem}$a = b \\text{ whenever } c$\\end{theorem}",
            "whenever"
        ),
        MathFace::Italic
    );
}

#[test]
fn display_text_in_theorem_body_stays_italic() {
    assert_eq!(
        face_of(
            "\\begin{theorem}\\[ x = y \\quad\\text{for all } z. \\]\\end{theorem}",
            "for all"
        ),
        MathFace::Italic
    );
}

#[test]
fn textup_in_theorem_body_resets_to_upright() {
    assert_eq!(
        face_of(
            "\\begin{theorem}$d \\textup{upright} e$\\end{theorem}",
            "upright"
        ),
        MathFace::Normal
    );
}

#[test]
fn textrm_in_theorem_body_keeps_italic() {
    assert_eq!(
        face_of(
            "\\begin{theorem}$e \\textrm{roman} f$\\end{theorem}",
            "roman"
        ),
        MathFace::Italic
    );
}

#[test]
fn textbf_in_theorem_body_adds_bold_to_italic() {
    assert_eq!(
        face_of("\\begin{theorem}$f \\textbf{bold} g$\\end{theorem}", "bold"),
        MathFace::BoldItalic
    );
}

#[test]
fn mathbf_in_theorem_body_stays_upright_bold() {
    // `\mathbf` is a math alphabet, not a text command: the ambient italic
    // does not touch it (the oracle traces it as `\OT1/cmr/bx/n/10`,
    // upright bold, inside the theorem).
    assert_eq!(
        face_of("\\begin{theorem}$f \\mathbf{bold} g$\\end{theorem}", "bold"),
        MathFace::Bold
    );
}

#[test]
fn normalfont_in_align_text_resets_without_diagnostics() {
    // Exact repro from testmath.tex line 1439: pdflatex typesets upright
    // "if" with 0 errors; FlashTeX used to report "\normalfont is not
    // supported inside \text" and typeset the command name literally.
    let source = "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\n\\begin{align}\\text{\\normalfont if } x &= 1\\end{align}\n\\end{document}";
    let cells = display_cell_texts(source);
    let merged: Vec<String> = cells
        .iter()
        .map(|pieces| pieces.iter().map(|(text, _)| text.as_str()).collect())
        .collect();
    assert!(
        merged.iter().any(|cell| cell.contains("if ")),
        "\"if \" not typeset in any align cell: {merged:?}"
    );
    assert!(
        !merged.iter().any(|cell| cell.contains('\\')),
        "command name leaked into the output: {merged:?}"
    );
    let face = cells
        .iter()
        .flat_map(|pieces| pieces.iter())
        .find(|(text, _)| text.contains("if "))
        .map(|(_, face)| *face);
    assert_eq!(face, Some(MathFace::Normal));
}

// Oracle for the tests below: TeX Live 2026 pdflatex `\typeout` of
// `\f@family/\f@series/\f@shape` inside `\text` in `/tmp/textdecl-oracle/`.
// `oracle.tex` (0 errors): normal context every declaration gives `cmr/m/n`;
// theorem (italic) context plain `\text` gives `cmr/m/it`, `\normalfont`
// gives `cmr/m/n`, `\rmfamily` keeps `cmr/m/it`, `\upshape` gives `cmr/m/n`,
// `\mdseries` keeps `cmr/m/it`. `combo.tex` (0 errors): after
// `\bfseries\itshape`, `\normalfont` gives `cmr/m/n`, `\upshape` gives
// `cmr/bx/n`, `\mdseries` gives `cmr/m/it`, `\rmfamily` keeps `cmr/bx/it`.

#[test]
fn font_declarations_in_text_reset_without_diagnostics() {
    for (declaration, needle) in [
        ("\\normalfont", "nrm"),
        ("\\rmfamily", "rmf"),
        ("\\upshape", "ups"),
        ("\\mdseries", "mds"),
    ] {
        assert_eq!(
            face_of(&format!("$a \\text{{{declaration} {needle}}} b$"), needle),
            MathFace::Normal,
            "{declaration} in normal-context \\text"
        );
    }
}

#[test]
fn font_declarations_in_theorem_text_match_pdflatex() {
    assert_eq!(
        face_of(
            "\\begin{theorem}$a \\text{\\normalfont if } b$\\end{theorem}",
            "if "
        ),
        MathFace::Normal
    );
    assert_eq!(
        face_of(
            "\\begin{theorem}$a \\text{\\rmfamily if } b$\\end{theorem}",
            "if "
        ),
        MathFace::Italic
    );
    assert_eq!(
        face_of(
            "\\begin{theorem}$a \\text{\\upshape if } b$\\end{theorem}",
            "if "
        ),
        MathFace::Normal
    );
    assert_eq!(
        face_of(
            "\\begin{theorem}$a \\text{\\mdseries if } b$\\end{theorem}",
            "if "
        ),
        MathFace::Italic
    );
}

#[test]
fn font_declarations_after_bold_italic_in_text_match_pdflatex() {
    assert_eq!(
        face_of("$a \\text{\\bfseries\\itshape\\normalfont x} b$", "x"),
        MathFace::Normal
    );
    assert_eq!(
        face_of("$a \\text{\\bfseries\\itshape\\rmfamily x} b$", "x"),
        MathFace::BoldItalic
    );
    assert_eq!(
        face_of("$a \\text{\\bfseries\\itshape\\upshape x} b$", "x"),
        MathFace::Bold
    );
    assert_eq!(
        face_of("$a \\text{\\bfseries\\itshape\\mdseries x} b$", "x"),
        MathFace::Italic
    );
}

#[test]
fn text_outside_theorem_stays_upright() {
    assert_eq!(
        face_of("$p \\text{ whenever } q$", "whenever"),
        MathFace::Normal
    );
    assert_eq!(
        face_of("\\[ x \\text{for all } z \\]", "for all"),
        MathFace::Normal
    );
    assert_eq!(face_of("$e \\textrm{roman} f$", "roman"), MathFace::Normal);
    assert_eq!(face_of("$f \\textbf{bold} g$", "bold"), MathFace::Bold);
}
