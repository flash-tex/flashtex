//! Kernel `\phantom`, `\hphantom` and `\vphantom` in TEXT mode (GH-494).
//!
//! Math mode already implements all three (`Nucleus::Phantom` in
//! `src/math.rs`); text mode rejected them with
//! `error[unsupported_feature]` and typeset the argument as plain visible
//! text. These tests pin the text-mode fix: no diagnostic, nothing painted,
//! and the reserved advance and line extents exactly equal to the
//! argument's visible typeset extent (the same measurement code path lays
//! out both, so equality is structural, not coincidental).
//!
//! Geometry semantics (latex.ltx `\ph@nt`, matching math's
//! `Nucleus::Phantom` flags): `\phantom` keeps width, height and depth;
//! `\hphantom` keeps only the width; `\vphantom` keeps only height/depth.
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::layout::TextItem;
use flashtex_compiler::parser::{parse, Block, Inline, Phantom};

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn laid_out(body: &str) -> Vec<flashtex_compiler::layout::Page> {
    let output = compile_full(&document(body), LayoutConstraints::default());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    output.pages
}

fn items(pages: &[flashtex_compiler::layout::Page]) -> Vec<&TextItem> {
    pages.iter().flat_map(|page| page.items.iter()).collect()
}

fn texts(pages: &[flashtex_compiler::layout::Page]) -> Vec<&str> {
    items(pages)
        .iter()
        .map(|item| item.text.as_str())
        .collect()
}

fn item<'a>(pages: &'a [flashtex_compiler::layout::Page], text: &str) -> &'a TextItem {
    items(pages)
        .into_iter()
        .find(|item| item.text == text)
        .unwrap_or_else(|| panic!("no {text:?} in {:?}", texts(pages)))
}

/// The exact GH-494 reproduction: all three commands in running text.
#[test]
fn issue_repro_reports_no_diagnostics() {
    let output = compile_full(
        &document("Text \\phantom{X} more \\hphantom{Y} end \\vphantom{Z} done"),
        LayoutConstraints::default(),
    );
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    // The arguments paint nothing: only the surrounding words reach the page.
    assert_eq!(
        texts(&output.pages),
        ["Text", "more", "end", "done"],
        "{:?}",
        texts(&output.pages)
    );
}

/// The parser keeps the argument as an hbox with math's flag convention
/// (`horizontal` false only for `\vphantom`, `vertical` false only for
/// `\hphantom`).
#[test]
fn phantom_node_shape_and_flags() {
    for (command, horizontal, vertical) in [
        ("phantom", true, true),
        ("hphantom", true, false),
        ("vphantom", false, true),
    ] {
        let parsed = parse(&document(&format!("A\\{command}{{X}}B")));
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let mut inlines = Vec::new();
        for block in &parsed.blocks {
            if let Block::Paragraph(body) = block {
                inlines.extend(body.clone());
            }
        }
        assert_eq!(inlines.len(), 3, "{command}: {inlines:?}");
        assert!(matches!(inlines[0], Inline::Text { .. }), "{inlines:?}");
        let Inline::Phantom(p) = &inlines[1] else {
            panic!("{command} did not produce a Phantom node: {inlines:?}");
        };
        let expected = Phantom {
            content: p.content.clone(),
            horizontal,
            vertical,
            span: p.span,
            space_before: p.space_before,
        };
        assert_eq!(**p, expected);
        assert!(
            matches!(&p.content[..], [Inline::Text { text, .. }] if text == "X"),
            "{command} content: {:?}",
            p.content
        );
        assert!(matches!(inlines[2], Inline::Text { .. }), "{inlines:?}");
    }
}

/// `\phantom{X}` advances exactly like the visible `X` (glued and spaced),
/// and paints nothing.
#[test]
fn phantom_advance_matches_visible_text_and_paints_nothing() {
    // Glued `AXB` lexes as one word, so the glued reference splices the
    // argument through `\text` (text-mode `\mbox`, same words, visible).
    for (hidden, visible) in [
        ("A\\phantom{X}B", "A\\text{X}B"),
        ("A \\phantom{X} B", "A X B"),
    ] {
        let hidden = laid_out(hidden);
        let visible = laid_out(visible);
        assert_eq!(texts(&hidden), ["A", "B"], "{hidden:?}");
        assert_eq!(texts(&visible), ["A", "X", "B"], "{visible:?}");
        assert!(
            !items(&hidden).iter().any(|item| item.rule.is_some()),
            "{hidden:?}"
        );
        assert_eq!(
            item(&hidden, "B").x_pt,
            item(&visible, "B").x_pt,
            "hidden {hidden:?} vs visible {visible:?}"
        );
    }
}

/// A `\rule` argument pins height as well as width: the phantom lays out
/// exactly like the visible rule (same following advance, same line
/// extents), minus the paint. A descender argument pins the same for text.
#[test]
fn phantom_geometry_matches_visible_rule_and_descender_text() {
    let hidden = laid_out("top \\phantom{\\rule{10pt}{30pt}}\\\\ bottom");
    let visible = laid_out("top \\rule{10pt}{30pt}\\\\ bottom");
    assert!(
        !items(&hidden).iter().any(|item| item.rule.is_some()),
        "{hidden:?}"
    );
    assert_eq!(
        item(&hidden, "bottom").baseline_y_pt,
        item(&visible, "bottom").baseline_y_pt,
        "hidden {hidden:?} vs visible {visible:?}"
    );
    let hidden = laid_out("top \\phantom{g}\\\\ bottom");
    let visible = laid_out("top g\\\\ bottom");
    assert_eq!(texts(&hidden), ["top", "bottom"]);
    assert_eq!(
        item(&hidden, "bottom").baseline_y_pt,
        item(&visible, "bottom").baseline_y_pt,
        "hidden {hidden:?} vs visible {visible:?}"
    );
}

/// `\hphantom` keeps the width (same advance as `\phantom`) but drops the
/// height and depth (same line gap as no box at all).
#[test]
fn hphantom_keeps_width_drops_height() {
    let full = laid_out("A \\phantom{\\rule{10pt}{20pt}} B\\\\ bottom");
    let half = laid_out("A \\hphantom{\\rule{10pt}{20pt}} B\\\\ bottom");
    let plain = laid_out("A B\\\\ bottom");
    assert_eq!(item(&half, "B").x_pt, item(&full, "B").x_pt);
    assert_ne!(item(&half, "B").x_pt, item(&plain, "B").x_pt);
    assert_eq!(
        item(&half, "bottom").baseline_y_pt,
        item(&plain, "bottom").baseline_y_pt,
        "half {half:?} vs plain {plain:?}"
    );
    assert!(
        item(&full, "bottom").baseline_y_pt > item(&plain, "bottom").baseline_y_pt,
        "full {full:?} vs plain {plain:?}"
    );
    assert!(
        !items(&half).iter().any(|item| item.rule.is_some()),
        "{half:?}"
    );
}

/// `\vphantom` drops the width but keeps the height and depth: the line
/// lays out exactly like a zero-width visible strut of the same height
/// (a taller line than no box at all, and no paint).
#[test]
fn vphantom_drops_width_keeps_height() {
    let tall = laid_out("A \\vphantom{\\rule{10pt}{20pt}} B\\\\ bottom");
    let strut = laid_out("A \\rule{0pt}{20pt} B\\\\ bottom");
    let plain = laid_out("A B\\\\ bottom");
    assert_eq!(texts(&tall), ["A", "B", "bottom"]);
    assert_eq!(
        item(&tall, "B").x_pt,
        item(&strut, "B").x_pt,
        "tall {tall:?} vs strut {strut:?}"
    );
    assert_eq!(
        item(&tall, "bottom").baseline_y_pt,
        item(&strut, "bottom").baseline_y_pt,
        "tall {tall:?} vs strut {strut:?}"
    );
    assert!(
        item(&tall, "bottom").baseline_y_pt > item(&plain, "bottom").baseline_y_pt,
        "tall {tall:?} vs plain {plain:?}"
    );
    assert!(
        !items(&tall).iter().any(|item| item.rule.is_some()),
        "{tall:?}"
    );
}

/// Styled, sized and math arguments measure through the same path as their
/// visible counterparts.
#[test]
fn phantom_measures_styled_sized_and_math_content() {
    let hidden = laid_out("A\\phantom{\\Large X}B");
    let visible = laid_out("A\\text{\\Large X}B");
    assert_eq!(texts(&hidden), ["A", "B"]);
    assert_eq!(
        item(&hidden, "B").x_pt,
        item(&visible, "B").x_pt,
        "hidden {hidden:?} vs visible {visible:?}"
    );
    let hidden = laid_out("A\\phantom{$x$}B");
    let visible = laid_out("A\\text{$x$}B");
    assert_eq!(
        item(&hidden, "B").x_pt,
        item(&visible, "B").x_pt,
        "hidden {hidden:?} vs visible {visible:?}"
    );
    // A nested phantom composes: the inner width still advances the line.
    let nested = laid_out("A\\phantom{\\phantom{X}}B");
    let plain = laid_out("A\\text{X}B");
    assert_eq!(
        item(&nested, "B").x_pt,
        item(&plain, "B").x_pt,
        "nested {nested:?} vs plain {plain:?}"
    );
}

/// An empty argument is silent and takes no space; a missing argument is a
/// plain braced-argument error, never an unsupported-command error.
#[test]
fn phantom_argument_edge_cases() {
    let pages = laid_out("A \\phantom{} B");
    assert_eq!(texts(&pages), ["A", "B"]);
    let parsed = parse(&document("A\\phantom B"));
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        parsed.diagnostics[0]
            .message
            .contains("\\phantom requires a braced argument"),
        "{:?}",
        parsed.diagnostics
    );
    assert!(
        !parsed.diagnostics[0].message.contains("not supported"),
        "{:?}",
        parsed.diagnostics
    );
}

/// A body holding only a phantom still produces its page in this layout
/// (one page, like visible content alone, with nothing painted).
#[test]
fn lone_phantom_body_produces_a_page() {
    let hidden = laid_out("\\phantom{X}");
    let visible = laid_out("X");
    assert_eq!(hidden.len(), 1, "{hidden:?}");
    assert_eq!(hidden.len(), visible.len());
    assert!(items(&hidden).is_empty(), "{hidden:?}");
}

/// Text-mode `\phantom` does not touch math mode: `$...$` formulae using
/// any of the three still lay out with no diagnostic.
#[test]
fn math_mode_phantom_unchanged() {
    for body in ["$a\\phantom{x}b$", "$\\hphantom{x}$", "$\\vphantom{x}$"] {
        let output = compile_full(&document(body), LayoutConstraints::default());
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let parsed = parse(&document(body));
        assert!(
            parsed
                .blocks
                .iter()
                .any(|block| matches!(block, Block::Paragraph(inlines) if inlines.iter().any(|inline| matches!(inline, Inline::Math { .. })))),
            "{body}: {parsed:?}"
        );
    }
}
