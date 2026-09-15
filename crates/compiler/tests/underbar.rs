//! GH-UNDERBAR: kernel `\underbar{text}` must parse and render exactly like
//! kernel text-mode `\underline{text}`.
//!
//! Real LaTeX2e (`latex.ltx`) defines `\underbar` as a text-mode-only
//! `$\@@underline{\hbox{#1}}$` construction (TeXbook Rule 10, like
//! `\underline`): one unbreakable hbox with the math rule under it. The only
//! practical difference from `\underline` is that `\underbar` never breaks
//! across lines — which is already how this compiler lays out
//! `Inline::Underline` (one fragment, no line break inside). So `\underbar`
//! reuses the `\underline` geometry (`UnderlineGeom::MathUnderline`) with no
//! package required.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{
    parse, Block, Inline, SourceDocument, UnderlineGeom, MATH_RULE_THETA_PT,
};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass[10pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}"
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
            Inline::Underline(u) => out.push_str(&text_of(&u.content)),
            _ => {}
        }
    }
    out
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

/// `\underbar` is a kernel command: no package, no "not supported" error.
#[test]
fn underbar_needs_no_package_and_is_supported() {
    let source = doc("Text \\underbar{under} here.");
    let messages = messages(&source);
    assert!(
        !messages
            .iter()
            .any(|m| m.contains("not supported")),
        "\\underbar should be implemented (kernel, no package): {messages:?}"
    );
    let inlines = paragraph_inlines(&source);
    assert!(
        text_of(&inlines).contains("under"),
        "argument must remain visible: {inlines:?}"
    );
}

/// Parse level: `\underbar{under}` wraps its argument in the same
/// `MathUnderline` geometry, thickness and content as `\underline{under}`.
#[test]
fn underbar_parses_like_underline() {
    let underbar = paragraph_inlines(&doc("Text \\underbar{under} here."));
    let underline = paragraph_inlines(&doc("Text \\underline{under} here."));
    let shape = |inlines: &[Inline]| {
        inlines
            .iter()
            .find_map(|inline| match inline {
                Inline::Underline(u) => {
                    Some((u.geom, u.thickness_pt, text_of(&u.content)))
                }
                _ => None,
            })
            .expect("expected an underline wrapper")
    };
    let (geom, thickness, text) = shape(&underbar);
    assert_eq!(geom, UnderlineGeom::MathUnderline);
    assert!((thickness - MATH_RULE_THETA_PT).abs() < 1e-9);
    assert_eq!(text, "under");
    assert_eq!(shape(&underline), (geom, thickness, text));
}

/// Layout level: both commands paint the identical rule (Rule 10: θ thick,
/// top 3θ below the hbox depth) under identical glyphs.
#[test]
fn underbar_renders_the_same_rule_as_underline() {
    let rules_of = |source: &str| {
        let out = compiled(source);
        let items = &out.pages[0].items;
        let baseline = items
            .iter()
            .find(|item| item.text.contains("under"))
            .expect("argument glyphs missing")
            .baseline_y_pt;
        let rule = items
            .iter()
            .find(|item| item.rule.is_some())
            .expect("expected a rule")
            .rule
            .expect("filtered");
        (rule.height_pt, rule.y_pt - baseline)
    };
    let from_underbar = rules_of(&doc("Text \\underbar{under} here."));
    let from_underline = rules_of(&doc("Text \\underline{under} here."));
    assert!(
        (from_underbar.0 - MATH_RULE_THETA_PT).abs() < 0.01,
        "rule height should be θ, got {}",
        from_underbar.0
    );
    assert!(
        (from_underbar.1 - 3.0 * MATH_RULE_THETA_PT).abs() < 0.01,
        "rule top should sit 3θ below the baseline, got {}",
        from_underbar.1
    );
    assert_eq!(
        from_underbar, from_underline,
        "\\underbar must render identically to \\underline"
    );
}
