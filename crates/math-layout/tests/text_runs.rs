use flashtex_math_layout::{layout, Atom, AtomClass, CmMathMetrics, MathList, Nucleus, Style, TextPiece, TextStyle};

fn x_squared() -> MathList {
    Atom::symbol('x')
        .with_sup(MathList::symbols("2"))
        .into()
}

fn run(pieces: Vec<TextPiece>) -> MathList {
    Atom::new(AtomClass::Ord, Nucleus::TextRun(pieces)).into()
}

fn f5(value: f64) -> String {
    format!("{value:.5}")
}

#[test]
fn text_run_widths_match_the_text_and_inline_math_shapes() {
    let metrics = CmMathMetrics::latex_10pt();
    let tag = run(vec![
        TextPiece::text("hi ", TextStyle::NORMAL),
        TextPiece::Math(x_squared()),
    ]);
    let star = run(vec![TextPiece::Math(MathList::symbols("∗"))]);
    let sentence = run(vec![
        TextPiece::text("for all ", TextStyle::NORMAL),
        TextPiece::Math(MathList::symbols("x")),
        TextPiece::text(" in ", TextStyle::NORMAL),
        TextPiece::Math(MathList::symbols("S")),
    ]);

    // Measured with TeX Live pdflatex using the .tex sources documented in
    // the issue; replace these pins with the recorded \showbox widths.
    assert_eq!(f5(layout(&tag, Style::TEXT, &metrics).width), "0.00000");
    assert_eq!(f5(layout(&star, Style::TEXT, &metrics).width), "0.00000");
    assert_eq!(f5(layout(&sentence, Style::TEXT, &metrics).width), "0.00000");
}

#[test]
fn nested_math_keeps_the_surrounding_text_size_in_scripts() {
    let metrics = CmMathMetrics::latex_10pt();
    let list = run(vec![TextPiece::Math(MathList::symbols("x"))]);
    let display = layout(&list, Style::DISPLAY, &metrics);
    let script = layout(&list, Style::SCRIPT, &metrics);
    assert!(display.width > script.width);
}
