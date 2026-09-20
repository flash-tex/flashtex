//! siunitx commands with empty or unbraced arguments, as pdflatex (TeX Live
//! 2026, siunitx v3) sets them -- `\showboxdepth=100 \showbox` of an 11pt
//! `[T1]{fontenc}` article:
//!
//! * `\hbox{a\num{}b}` and `\hbox{a\unit{}b}` are `a` `b` (11.49232pt): an
//!   empty number or unit sets nothing and raises no error;
//! * `\hbox{a \unit{} b}` is `a` `\glue 3.63054` `\glue 3.63054` `b`
//!   (18.7534pt): both source spaces survive as interword glue;
//! * `\hbox{a\qty{}{m}b}` is `a` `\mathon` `\OT1/cmr/m/n/10.95 m` `\mathoff`
//!   `b` (20.61734pt): no number, no `\penalty10000` and no product kern;
//! * `\hbox{older \si{} and \SI{} macro}` is `older` glue glue `and` glue
//!   `\mathon m \mathoff` `acro`: `\SI{}`'s second argument is TeX's
//!   undelimited `#2`, the `m` of `macro`.
//!
//! Before this the compiler dropped `\si{}` outright (one glue, so `and`
//! sat 2.92bp left of pdflatex on fixtures/real-world/siunitx-tables page
//! 2), set an empty text atom for `\SI{}` and refused the unbraced `m`.
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{siunitx}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// The first paragraph's inlines as a compact spelling: `T:text`,
/// `M:<atom count>`, `G:<pt>/<space before>/<space after>` (an `HSpace`),
/// with `+` for a leading space (the first word has the newline after
/// `\begin{document}`).
fn shape(body: &str) -> (Vec<String>, Vec<String>) {
    let parsed = parse(&doc(body));
    let diagnostics = parsed.diagnostics.iter().map(|d| d.message.clone()).collect();
    let inlines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("a paragraph");
    let spelled = inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text { text, space_before, .. } => format!("{}T:{text}", if *space_before { "+" } else { "" }),
            Inline::Math { list, space_before, .. } => format!("{}M:{}", if *space_before { "+" } else { "" }, list.atoms.len()),
            Inline::HSpace { pt, space_before_pt, space_after_pt, .. } => {
                format!("G:{pt}/{}/{}", (*space_before_pt > 0.0) as u8, (*space_after_pt > 0.0) as u8)
            }
            other => format!("{other:?}"),
        })
        .collect();
    (spelled, diagnostics)
}

#[test]
fn an_empty_number_or_unit_sets_nothing_and_keeps_both_glues() {
    let (inlines, diagnostics) = shape("a \\unit{} b");
    assert_eq!(inlines, ["+T:a", "G:0/1/1", "+T:b"]);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (inlines, diagnostics) = shape("a\\num{}b");
    assert_eq!(inlines, ["+T:a", "G:0/0/0", "T:b"]);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn an_empty_number_leaves_the_unit_without_a_product() {
    let (inlines, diagnostics) = shape("a\\qty{}{m}b");
    // One atom: the upright `m`; no `\,` product, no empty number atom.
    assert_eq!(inlines, ["+T:a", "M:1", "T:b"]);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let (with_number, _) = shape("a\\qty{1}{m}b");
    // Number, product, unit.
    assert_eq!(with_number, ["+T:a", "M:3", "T:b"]);
}

/// siunitx.sty 8035-8040: `\degree`, `\arcminute` and `\arcsecond` declare
/// `quantity-product = { }` for themselves. pdflatex's
/// `\hbox{\SI{8}{\degree} and}` (11pt) is `\mathon 8 \mathoff`,
/// `\penalty 10000`, `\mathon` the `{}^{\circ}` box `\mathoff`, glue,
/// `and`: 31.39632pt, no `\,` kern between the number and the unit.
#[test]
fn the_angle_units_take_no_product() {
    let (degree, diagnostics) = shape("a\\SI{8}{\\degree}b");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    // The number and the unit only.
    assert_eq!(degree, ["+T:a", "M:2", "T:b"]);
    let (arcminute, _) = shape("a\\qty{8}{\\arcminute}b");
    assert_eq!(arcminute, ["+T:a", "M:2", "T:b"]);
    let (metre, _) = shape("a\\qty{8}{\\metre}b");
    assert_eq!(metre, ["+T:a", "M:3", "T:b"]);
}

#[test]
fn an_unbraced_argument_is_the_next_token_as_in_tex() {
    let (inlines, diagnostics) = shape("older \\si{} and \\SI{} macro forms.");
    assert_eq!(inlines, ["+T:older", "G:0/1/1", "+T:and", "+M:1", "T:acro", "+T:forms."]);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    // The command's span reaches the `m` it took, so the text after it
    // starts exactly where `acro` does.
    let parsed = parse(&doc("x \\SI{} macro"));
    let source = doc("x \\SI{} macro");
    let inlines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .unwrap();
    let math_end = inlines.iter().find_map(|i| match i {
        Inline::Math { span, .. } => Some(span.end),
        _ => None,
    });
    let acro = inlines.iter().find_map(|i| match i {
        Inline::Text { text, span, .. } if text == "acro" => Some(span.start),
        _ => None,
    });
    assert_eq!(math_end, acro);
    assert_eq!(&source[acro.unwrap()..acro.unwrap() + 4], "acro");
}
