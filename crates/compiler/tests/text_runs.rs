use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};
use flashtex_compiler::math::{self, MathList, Nucleus, TextPiece, TextStyle, MAX_MATH_DEPTH};
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

fn eqref_text(source: &str) -> String {
    let output = compile_full(source, LayoutConstraints::default());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    eqref_item_text(source, &output)
}

fn eqref_text_allowing_text_font_warnings(source: &str) -> String {
    let output = compile_full(source, LayoutConstraints::default());
    assert!(
        output
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == Severity::Warning),
        "unexpected error diagnostics: {:?}",
        output.diagnostics
    );
    eqref_item_text(source, &output)
}

fn eqref_item_text(source: &str, output: &CompileOutput) -> String {
    output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .find_map(|item| {
            source
                .get(item.span.start..item.span.end)
                .filter(|text| *text == r"\eqref{l}")
                .map(|_| item.text.clone())
        })
        .expect("eqref item")
}

fn parsed_text_run_box(source: &str) -> flashtex_compiler::math::MathBox {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let atom = first_math(&parsed)
        .atoms
        .iter()
        .find(|atom| matches!(atom.nucleus, Nucleus::TextRun(_)))
        .expect("text run atom")
        .clone();
    let list = MathList { atoms: vec![atom] };
    let mut diagnostics = Vec::new();
    let result = math::layout(&list, 10.0, &mut diagnostics);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    result
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
fn tag_keeps_the_interim_two_quad_gap_until_margin_placement() {
    let parsed = parse(r"\begin{equation}a=b\tag{hi}\end{equation}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let list = first_math(&parsed);
    assert!(list
        .atoms
        .iter()
        .any(|atom| matches!(&atom.nucleus, Nucleus::Text(text) if text == "(hi)")));
    assert!(list.atoms.iter().any(|atom| matches!(
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
fn starred_tag_keeps_literal_parentheses_for_the_equation_and_eqref_adds_another_pair() {
    let starred = parse(r"\begin{equation}a=b\tag*{(custom)}\end{equation}");
    assert!(starred.diagnostics.is_empty(), "{:?}", starred.diagnostics);
    assert!(starred.blocks.iter().any(|block| matches!(
        block,
        Block::Paragraph(inlines) if inlines.iter().any(|inline| matches!(
            inline,
            Inline::Math { list, .. } if list.atoms.iter().any(|atom| matches!(
                &atom.nucleus,
                Nucleus::Text(text) if text == "(custom)"
            ))
        ))
    )));

    let unstarred = parse(r"\begin{equation}a=b\tag{(custom)}\end{equation}");
    assert!(unstarred.diagnostics.is_empty(), "{:?}", unstarred.diagnostics);
    assert!(unstarred.blocks.iter().any(|block| matches!(
        block,
        Block::Paragraph(inlines) if inlines.iter().any(|inline| matches!(
            inline,
            Inline::Math { list, .. } if list.atoms.iter().any(|atom| matches!(
                &atom.nucleus,
                Nucleus::Text(text) if text == "((custom))"
            ))
        ))
    )));
}

#[test]
fn eqref_uses_a_custom_tag_as_the_label_value() {
    let source = r"See \eqref{e}.\begin{equation}a=b\tag{custom}\label{e}\end{equation}";
    assert_eq!(eqref_text(&source.replace("{e}", "{l}")), "(custom)");
}

#[test]
fn eqref_uses_rich_starred_and_plain_tag_values() {
    let rich = r"See \eqref{l}.\begin{equation}a=b\tag{hi $x^2$}\label{l}\end{equation}";
    assert_eq!(eqref_text(rich), "(hi x²)");

    let starred = r"See \eqref{l}.\begin{equation}a=b\tag*{custom}\label{l}\end{equation}";
    assert_eq!(eqref_text(starred), "(custom)");

    let starred_literal_parens =
        r"See \eqref{l}.\begin{equation}a=b\tag*{(custom)}\label{l}\end{equation}";
    assert_eq!(eqref_text(starred_literal_parens), "((custom))");

    let unstarred_literal_parens =
        r"See \eqref{l}.\begin{equation}a=b\tag{(custom)}\label{l}\end{equation}";
    assert_eq!(eqref_text(unstarred_literal_parens), "((custom))");

    let plain = r"See \eqref{l}.\begin{equation}a=b\tag{1}\label{l}\end{equation}";
    assert_eq!(eqref_text(plain), "(1)");
}

#[test]
fn eqref_keeps_rendered_and_source_fallback_math_in_rich_tags() {
    let radical = r"See \eqref{l}.\begin{equation}a=b\tag{$\sqrt{x}$}\label{l}\end{equation}";
    assert_eq!(eqref_text(radical), r"(\sqrt{x})");

    let fraction = r"See \eqref{l}.\begin{equation}a=b\tag{$\frac{a}{b}$}\label{l}\end{equation}";
    assert_eq!(eqref_text(fraction), r"(\frac{a}{b})");

    let greek_subscript = r"See \eqref{l}.\begin{equation}a=b\tag{$\alpha_1$}\label{l}\end{equation}";
    // References are string-only in the compiler route. The flattened Greek
    // and subscript glyphs therefore go through the legacy text face, which
    // may report a text-font fidelity warning; the reference value is intact.
    assert_eq!(eqref_text_allowing_text_font_warnings(greek_subscript), "(α₁)");
}

#[test]
fn compiler_layouts_parser_text_runs_at_the_legacy_core14_pin() {
    let source = r"\begin{equation}a=b\tag{hi $x^2$}\end{equation}";
    let output = compile_full(source, LayoutConstraints::default());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    // Measured with:
    // /Library/TeX/texbin/pdflatex -interaction=nonstopmode -halt-on-error
    //   -output-directory=/private/tmp/flashtex-tag-pdflatex-measurement flashtex_tag_box.tex
    // flashtex_tag_box.tex contains:
    //   \setbox\flashbox=\hbox{$\text{(hi $x^2$)}$}
    // This is not a pdflatex oracle: TeX Live 2026 reported 29.64589pt for
    // the Computer Modern hbox, while this compiler route intentionally keeps
    // its existing Core14 text/NewCM metrics. The 24.88pt value pins that
    // legacy compiler route; PR 2 owns the real pdflatex comparison.
    let width = parsed_text_run_box(source).width;
    assert!((width - 24.88).abs() <= 0.01, "{width}pt vs 24.88pt");
}

#[test]
fn compiler_layouts_parser_text_sentence_and_bold_math() {
    for source in [
        r"\begin{equation}\text{for all $x$ in $S$}\end{equation}",
        r"\begin{equation}\textbf{$x$}\end{equation}",
    ] {
        let output = compile_full(source, LayoutConstraints::default());
        assert!(output.diagnostics.is_empty(), "{source:?}: {:?}", output.diagnostics);
        assert!(parsed_text_run_box(source).width > 0.0, "{source:?}");
    }
}

#[test]
fn unbalanced_math_inside_text_is_diagnosed_through_compile() {
    let source = r"\begin{equation}\text{broken $x}\end{equation}";
    let output = compile_full(source, LayoutConstraints::default());
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("missing its closing delimiter")), "{:?}", output.diagnostics);
}

#[test]
fn unsupported_math_inside_text_is_diagnosed_through_compile() {
    let source = r"\begin{equation}\text{bad $\foo$}\end{equation}";
    let output = compile_full(source, LayoutConstraints::default());
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains(r"\foo is not supported in math mode")), "{:?}", output.diagnostics);
}

#[test]
fn nested_text_runs_share_the_math_depth_limit() {
    let depth = MAX_MATH_DEPTH + 8;
    let source = format!(
        r"\begin{{equation}}{}x{}\end{{equation}}",
        r"\text{".repeat(depth),
        "}".repeat(depth)
    );
    let parsed = parse(&source);
    let expected = format!("math nesting deeper than {MAX_MATH_DEPTH} levels is not supported");
    assert_eq!(
        parsed
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message == expected)
            .count(),
        1,
        "{:?}",
        parsed.diagnostics
    );
}
