use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::math::{MathList, Nucleus, TextPiece, TextStyle};
use flashtex_compiler::parser::{parse, Block, Inline};

fn first_math(parsed: &flashtex_compiler::parser::Parsed) -> &MathList {
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::Math { list, .. } => Some(list),
                _ => None,
            }),
            _ => None,
        })
        .expect("math inline")
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

#[test]
fn equation_tag_accepts_nested_math_and_preserves_the_run() {
    let parsed = parse(r"\begin{equation}a=b\tag{hi $x^2$}\end{equation}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let pieces = text_run(first_math(&parsed));
    assert!(matches!(
        pieces.as_slice(),
        [
            TextPiece::Text { text, style: TextStyle::NORMAL },
            TextPiece::Math(math),
            TextPiece::Text { text: tail, style: TextStyle::NORMAL },
        ] if text == "(hi " && tail == ")" && math.atoms.len() == 1
    ));
    let math = match &pieces[1] {
        TextPiece::Math(math) => math,
        _ => unreachable!(),
    };
    assert_eq!(math.atoms[0].nucleus, Nucleus::Symbol("x".into()));
    assert_eq!(math.atoms[0].superscript.as_ref().unwrap().atoms[0].nucleus, Nucleus::Symbol("2".into()));
}

#[test]
fn tag_math_command_is_valid_only_inside_nested_math() {
    let tagged = parse(r"\begin{equation}a=b\tag{$\ast$}\end{equation}");
    assert!(tagged.diagnostics.is_empty(), "{:?}", tagged.diagnostics);
    assert!(text_run(first_math(&tagged)).iter().any(|piece| matches!(piece, TextPiece::Math(math) if math.atoms.len() == 1)));

    let outside = parse(r"\begin{equation}a=b\tag{\ast}\end{equation}");
    assert!(
        outside
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("not supported inside \\tag")),
        "{:?}",
        outside.diagnostics
    );
}

#[test]
fn text_accepts_multiple_nested_math_spans_and_styles() {
    let parsed = parse(
        r"\begin{equation}\text{for all $x$ in $S$ and \(T\) and \textbf{bold} \emph{italic}}\end{equation}",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let pieces = text_run(first_math(&parsed));
    assert_eq!(
        pieces
            .iter()
            .filter(|piece| matches!(piece, TextPiece::Math(_)))
            .count(),
        3
    );
    assert!(pieces.iter().any(|piece| matches!(
        piece,
        TextPiece::Text { text, style: TextStyle::BOLD } if text == "bold"
    )));
    assert!(pieces.iter().any(|piece| matches!(
        piece,
        TextPiece::Text { text, style: TextStyle::ITALIC } if text == "italic"
    )));
}

#[test]
fn tag_does_not_insert_the_old_two_quad_glue() {
    let parsed = parse(r"\begin{equation}a=b\tag{hi}\end{equation}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let list = first_math(&parsed);
    assert!(list
        .atoms
        .iter()
        .any(|atom| matches!(&atom.nucleus, Nucleus::Text(text) if text == "(hi)")));
    assert!(!list.atoms.iter().any(|atom| matches!(
        atom.nucleus,
        Nucleus::Space { em, font_em: true } if (em - 2.0).abs() < f64::EPSILON
    )));
}

#[test]
fn starred_tag_omits_parentheses() {
    let parsed = parse(r"\begin{equation}a=b\tag*{custom}\end{equation}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let list = first_math(&parsed);
    assert!(list
        .atoms
        .iter()
        .any(|atom| matches!(&atom.nucleus, Nucleus::Text(text) if text == "custom")));
    assert!(!list
        .atoms
        .iter()
        .any(|atom| matches!(&atom.nucleus, Nucleus::Text(text) if text == "(custom)")));
}

#[test]
fn eqref_uses_a_custom_tag_as_the_label_value() {
    let source = r"See \eqref{e}.\begin{equation}a=b\tag{custom}\label{e}\end{equation}";
    let output = compile_full(source, LayoutConstraints::default());
    let eqref = output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .find(|item| item.span.start < source.len() && &source[item.span.start..item.span.end] == r"\eqref{e}")
        .expect("eqref item");
    assert_eq!(eqref.text, "(custom)");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}
