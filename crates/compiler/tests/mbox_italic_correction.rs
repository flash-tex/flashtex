//! `\/` (italic correction) inside math-mode `\mbox`/`\text`.
//!
//! A math text group used to typeset `\/` as a literal `/`. pdflatex instead
//! inserts the last character's TFM italic correction as a kern inside the
//! box, exactly as in text mode.
//!
//! Oracle: TeX Live 2026 `pdflatex -interaction=nonstopmode measure.tex`
//! with `\setbox0=\hbox{...}\showthe\wd0` gives `{\itshape f\/}` 5.1861pt
//! against `{\itshape f}` 3.06665pt (kern 2.11945pt), `{f\/}` 3.83336pt
//! against `{f}` 3.05557pt (kern 0.77779pt), `{\bfseries f\/}` 4.60413pt
//! against `{\bfseries f}` 3.51387pt (kern 1.09026pt), and `{\itshape\/f}`
//! 3.06665pt (nothing without a preceding character). `pdflatex
//! -interaction=nonstopmode mathshow.tex` with `\showoutput` on
//! `$a \mbox{\itshape f\/} x$` shows the kern inside the math hbox:
//! `\hbox(...)x5.1861` holding `\OT1/cmr/m/it/10 f` then `\kern 2.11945`.
//! The position tests below assert that kern gap within 0.1, which also
//! absorbs the pt/bp unit slop (0.4%).
use flashtex_compiler::math::{self, MathList, Nucleus, TextPiece, TextStyle};
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage{{amsmath}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn first_math(source: &str) -> (MathList, Vec<flashtex_compiler::diagnostics::Diagnostic>) {
    let parsed = parse(&doc(source));
    let list = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::Math { list, .. } => Some(list.clone()),
                _ => None,
            }),
            _ => None,
        })
        .expect("math inline");
    (list, parsed.diagnostics)
}

fn text_run(list: &MathList) -> &Vec<TextPiece> {
    list.atoms
        .iter()
        .find_map(|atom| match &atom.nucleus {
            Nucleus::TextRun(pieces) => Some(pieces),
            _ => None,
        })
        .expect("text run")
}

/// The kern `\/` leaves in `pieces`: `(em, font_em)` of its single space.
fn correction(pieces: &[TextPiece]) -> Option<(f64, bool)> {
    pieces.iter().find_map(|piece| match piece {
        TextPiece::Math(list) => match list.atoms.as_slice() {
            [atom] => match atom.nucleus {
                Nucleus::Space { em, font_em } => Some((em, font_em)),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    })
}

/// The laid-out x of the `x` following the box in `$a <body> x$` at 10pt.
fn x_after(body: &str) -> f64 {
    let (list, diagnostics) = first_math(body);
    assert!(diagnostics.is_empty(), "{body}: {diagnostics:?}");
    let laid = math::layout(&list, 10.0, &mut Vec::new());
    laid.items
        .iter()
        .find(|item| item.text == "x")
        .map(|item| item.x)
        .expect("x item")
}

#[test]
fn slash_in_math_mbox_is_a_kern_not_a_slash() {
    for (body, style, em) in [
        (r"$a \mbox{\itshape f\/} x$", TextStyle::Italic, 0.21194),
        (r"$a \mbox{f\/} x$", TextStyle::Normal, 0.07778),
        (r"$a \mbox{\bfseries f\/} x$", TextStyle::Bold, 0.10903),
        (r"$a \text{\itshape f\/} x$", TextStyle::Italic, 0.21194),
    ] {
        let (list, diagnostics) = first_math(body);
        assert!(diagnostics.is_empty(), "{body}: {diagnostics:?}");
        let pieces = text_run(&list);
        assert!(
            pieces.iter().any(|piece| matches!(
                piece,
                TextPiece::Text { text, style: piece_style }
                if text == "f" && *piece_style == style
            )),
            "{body}: {pieces:?}"
        );
        // No literal slash anywhere in the run.
        assert!(
            !pieces.iter().any(|piece| matches!(
                piece,
                TextPiece::Text { text, .. } if text.contains('/')
            )),
            "{body}: slash leaked into text: {pieces:?}"
        );
        let (kern_em, font_em) = correction(pieces).expect("{body}: no kern piece");
        assert!(font_em, "{body}: kern must be a text-font em space");
        assert!(
            (kern_em - em).abs() < 1e-4,
            "{body}: kern {kern_em}em, want {em}em"
        );
    }
}

#[test]
fn correction_matches_pdflatex_after_italic_f() {
    let gap = x_after(r"$a \mbox{\itshape f\/} x$") - x_after(r"$a \mbox{\itshape f} x$");
    assert!(
        (gap - 2.11945).abs() < 0.1,
        "kern gap {gap}, pdflatex measures 2.11945pt"
    );
}

#[test]
fn correction_matches_pdflatex_after_upright_f() {
    let gap = x_after(r"$a \mbox{f\/} x$") - x_after(r"$a \mbox{f} x$");
    assert!(
        (gap - 0.77779).abs() < 0.1,
        "kern gap {gap}, pdflatex measures 0.77779pt"
    );
}

#[test]
fn correction_is_silent_without_a_preceding_character() {
    // Leading `\/`, `\/` after a space, and `\/` after nested math all
    // correct nothing (tex.web 1113): no kern, no diagnostic, no slash.
    // Without a kern the all-upright runs below collapse to `Nucleus::Text`.
    for (body, text) in [
        (r"$\mbox{\/f}$", "f"),
        (r"$\mbox{a \/f}$", "a f"),
        (r"$\mbox{a$b$\/}$", "a"),
    ] {
        let (list, diagnostics) = first_math(body);
        assert!(diagnostics.is_empty(), "{body}: {diagnostics:?}");
        let [atom] = list.atoms.as_slice() else {
            panic!("{body}: {:?}", list.atoms);
        };
        match &atom.nucleus {
            Nucleus::Text(got) => assert_eq!(got, text, "{body}"),
            Nucleus::TextRun(pieces) => {
                assert!(
                    correction(pieces).is_none(),
                    "{body}: unexpected kern: {pieces:?}"
                );
                let merged: String = pieces
                    .iter()
                    .filter_map(|piece| match piece {
                        TextPiece::Text { text, .. } => Some(text.as_str()),
                        TextPiece::Math(_) => None,
                    })
                    .collect();
                // The nested formula contributes no literal text.
                assert_eq!(merged, text, "{body}: {pieces:?}");
            }
            nucleus => panic!("{body}: unexpected nucleus {nucleus:?}"),
        }
    }
}

#[test]
fn typed_slash_is_untouched() {
    // A typed `/` is not a control symbol: it stays literal text.
    let (list, diagnostics) = first_math(r"$\mbox{a/b}$");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let [atom] = list.atoms.as_slice() else {
        panic!("{:?}", list.atoms);
    };
    assert!(
        matches!(&atom.nucleus, Nucleus::Text(text) if text == "a/b"),
        "{atom:?}"
    );
}

#[test]
fn correction_in_string_group_is_glue_not_math() {
    // `\/` in a string-only group (`\mathcal` here) is a glue kern, not math
    // syntax: no "math syntax" diagnostic, and the literal text drops the
    // correction (a string cannot carry a kern).
    let (_, diagnostics) = first_math(r"$a \mathcal{f\/} b$");
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.message.contains("math syntax is not supported")),
        "{diagnostics:?}"
    );
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(
        diagnostics[0]
            .message
            .contains("\\mathcal supports only capital letters A-Z, not \"f\""),
        "{diagnostics:?}"
    );
}
