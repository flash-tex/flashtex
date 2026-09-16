//! GH-312: ulem `\uline{...}` must typeset an underline rule, not an
//! `unsupported` error with the argument as plain text.
//!
//! Geometry follows ulem.sty `\uline` / `\ULset`: `\ULthickness=.4pt`;
//! `\UL@setULdepth` takes `\dp` of `\hbox{{(j}}` (max of `(` and `j`;
//! 0.25em for cmr/lmr) plus 0.4pt. The painted rule is
//! `\hrule height -0.25em depth (0.25em+0.4pt)` — pdflatex 10pt
//! `rule(-2.5+2.9)`, 12pt `rule(-3.0+3.4)`. A single-line underline is
//! the first step: the argument is one unbreakable fragment, unlike
//! ulem's line-breaking leaders.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline, SourceDocument};

fn uline_doc(class_opt: &str) -> String {
    format!(
        r"\documentclass{class_opt}{{article}}
\usepackage[normalem]{{ulem}}
\begin{{document}}
Text \uline{{underlined words}} here.
\end{{document}}"
    )
}

fn compiled(source: &str) -> flashtex_compiler::incremental::CompileOutput {
    compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: source,
        }],
        "main.tex",
        LayoutConstraints::default(),
    )
}

fn messages(source: &str) -> Vec<String> {
    parse(source)
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

fn paragraph_inlines(source: &str) -> Vec<Inline> {
    parse(source)
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .unwrap_or_default()
}

fn text_of(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, .. } => out.push_str(text),
            Inline::ColorBox(b) => out.push_str(&text_of(&b.content)),
            Inline::Underline(u) => out.push_str(&text_of(&u.content)),
            _ => {}
        }
    }
    out
}

/// The Commander's leftover-error report: `\uline` is `error[compiler]
/// unsupported` and the argument is typeset as plain text.
#[test]
fn uline_with_ulem_is_not_an_unsupported_error() {
    let messages = messages(&uline_doc("[10pt]"));
    assert!(
        !messages.iter().any(|m| m.contains("\\uline is not supported")),
        "\\uline should be implemented when ulem is loaded: {messages:?}"
    );
}

/// The argument stays visible, but it is not a bare `Inline::Text` run with
/// no underline wrapper: that is the current recovery, which loses the rule.
#[test]
fn uline_argument_is_not_recovered_as_plain_text() {
    let inlines = paragraph_inlines(&uline_doc("[10pt]"));
    let joined = text_of(&inlines);
    assert!(
        joined.contains("underlined") && joined.contains("words"),
        "argument must remain visible: {inlines:?}"
    );
    let only_plain_text = inlines.iter().all(|inline| {
        matches!(
            inline,
            Inline::Text { .. } | Inline::LineBreak { .. } | Inline::TextGlue { .. }
        )
    });
    assert!(
        !only_plain_text,
        "\\uline must emit an underline wrapper, not only plain text: {inlines:?}"
    );
}

fn assert_ulem_rule(class_opt: &str, size_pt: f64) {
    let out = compiled(&uline_doc(class_opt));
    let items = &out.pages[0].items;
    let words: Vec<_> = items
        .iter()
        .filter(|item| item.text.contains("underlined") || item.text.contains("words"))
        .collect();
    assert!(!words.is_empty(), "argument glyphs missing: {items:?}");
    let rules: Vec<_> = items.iter().filter(|item| item.rule.is_some()).collect();
    assert!(
        !rules.is_empty(),
        "expected an underline rule under the argument: {items:?}"
    );
    let rule = rules[0].rule.expect("filtered");
    assert!(
        (rule.height_pt - 0.4).abs() < 0.01,
        "ulem \\ULthickness is 0.4pt, got {}",
        rule.height_pt
    );
    let baseline = words[0].baseline_y_pt;
    let want_top = 0.25 * size_pt;
    assert!(
        (rule.y_pt - baseline - want_top).abs() < 0.01,
        "ulem rule top should sit {want_top}pt below baseline {baseline} at {size_pt}pt, got y={}",
        rule.y_pt
    );
}

/// Core 14 layout: 0.4pt rule whose top is 0.25em below the baseline.
#[test]
fn uline_lays_out_a_rule_under_the_argument() {
    assert_ulem_rule("[10pt]", 10.0);
    assert_ulem_rule("[12pt]", 12.0);
}
