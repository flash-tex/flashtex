use flashtex_math_layout::{
    layout, Atom, AtomClass, CmMathMetrics, MathList, Nucleus, Style, TextPiece, TextStyle,
};

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

    // Oracle command:
    // /Library/TeX/texbin/pdflatex -interaction=nonstopmode -halt-on-error
    //   -output-directory=/private/tmp/flashtex-tag-oracle oracle_tag.tex
    // oracle_tag.tex contains:
    //   \setbox\flashbox=\hbox{$\text{hi $x^2$}$}
    //   \setbox\flashbox=\hbox{$\text{$\ast$}$}
    //   \setbox\flashbox=\hbox{$\text{for all $x$ in $S$}$}
    // TeX Live 2026 reported 21.86809pt, 5.00002pt, and 56.61810pt.
    let widths = [
        layout(&tag, Style::TEXT, &metrics).width,
        layout(&star, Style::TEXT, &metrics).width,
        layout(&sentence, Style::TEXT, &metrics).width,
    ];
    let oracles = [21.86809, 5.00002, 56.61810];
    for (&width, oracle) in widths.iter().zip(oracles) {
        assert!((width - oracle).abs() <= 0.5, "{width}pt vs {oracle}pt");
    }
    assert_eq!(f5(widths[0]), "21.86809");
    assert_eq!(f5(widths[1]), "5.00002");
    assert_eq!(f5(widths[2]), "56.61810");
}

#[test]
fn nested_math_keeps_the_surrounding_text_size_in_scripts() {
    let metrics = CmMathMetrics::latex_10pt();
    let list = run(vec![TextPiece::Math(MathList::symbols("x"))]);
    let display = layout(&list, Style::DISPLAY, &metrics);
    let script = layout(&list, Style::SCRIPT, &metrics);
    assert!(display.width > script.width);
}
