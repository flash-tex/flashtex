//! GH-330: text-mode `\underline{under}` (kernel) and ulem `\sout{struck}`
//! currently drop their argument via unsupported-command recovery (the words
//! look like parameters). The text must survive and get a rule.
//!
//! Geometry (asserted after pdflatex measurement):
//! - kernel `\underline` is latex.ltx `$\@@underline{\hbox{#1}}$` (TeXbook
//!   Rule 10 / tex.web §735).
//! - ulem `\sout` is `\bgroup \ULdepth=-.55ex \ULset`.
//! `\uline` geometry is unchanged (see `uline.rs`).
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{
    parse, Block, Inline, SourceDocument, CMR_EX_PER_EM, MATH_RULE_THETA_PT, SOUT_RAISE_EX,
    UL_THICKNESS_PT,
};

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

fn has_underline(inlines: &[Inline]) -> bool {
    inlines
        .iter()
        .any(|inline| matches!(inline, Inline::Underline(_)))
}

fn underline_doc(class_opt: &str) -> String {
    format!(
        r"\documentclass{class_opt}{{article}}
\begin{{document}}
Text \underline{{under}} here.
\end{{document}}"
    )
}

fn sout_doc(class_opt: &str) -> String {
    format!(
        r"\documentclass{class_opt}{{article}}
\usepackage[normalem]{{ulem}}
\begin{{document}}
Text \sout{{struck}} here.
\end{{document}}"
    )
}

/// Issue #330: `\underline{under}` is `error[compiler] \underline is not
/// supported` and recovery skips the argument because it looks like a
/// parameter. The word must stay visible.
#[test]
fn text_mode_underline_does_not_drop_its_argument() {
    let source = underline_doc("[10pt]");
    let messages = messages(&source);
    assert!(
        !messages
            .iter()
            .any(|m| m.contains("\\underline is not supported")),
        "text-mode \\underline should be implemented (kernel, no package): {messages:?}"
    );
    let inlines = paragraph_inlines(&source);
    let joined = text_of(&inlines);
    assert!(
        joined.contains("under"),
        "argument must remain visible, not dropped as a parameter: {inlines:?}"
    );
    assert!(
        has_underline(&inlines),
        "\\underline must emit an underline wrapper: {inlines:?}"
    );
}

/// Issue #330: `\sout{struck}` is the same drop. Needs ulem, like `\uline`.
#[test]
fn sout_with_ulem_does_not_drop_its_argument() {
    let source = sout_doc("[10pt]");
    let messages = messages(&source);
    assert!(
        !messages
            .iter()
            .any(|m| m.contains("\\sout is not supported")),
        "\\sout should be implemented when ulem is loaded: {messages:?}"
    );
    let inlines = paragraph_inlines(&source);
    let joined = text_of(&inlines);
    assert!(
        joined.contains("struck"),
        "argument must remain visible, not dropped as a parameter: {inlines:?}"
    );
    assert!(
        has_underline(&inlines),
        "\\sout must emit an underline wrapper: {inlines:?}"
    );
}

fn assert_rule(source: &str, needle: &str, want_height: f64, want_top_from_baseline: f64) {
    let out = compiled(source);
    let items = &out.pages[0].items;
    let words: Vec<_> = items
        .iter()
        .filter(|item| item.text.contains(needle))
        .collect();
    assert!(
        !words.is_empty(),
        "argument glyphs missing ({needle}): {items:?}"
    );
    let rules: Vec<_> = items.iter().filter(|item| item.rule.is_some()).collect();
    assert!(!rules.is_empty(), "expected a rule on {needle}: {items:?}");
    let rule = rules[0].rule.expect("filtered");
    let baseline = words[0].baseline_y_pt;
    assert!(
        (rule.height_pt - want_height).abs() < 0.01,
        "rule height {want_height}pt, got {}",
        rule.height_pt
    );
    assert!(
        (rule.y_pt - baseline - want_top_from_baseline).abs() < 0.01,
        "rule top should sit {want_top_from_baseline}pt from baseline {baseline}, got y={}",
        rule.y_pt
    );
}

/// pdflatex 10pt/12pt: kern 1.19994, rule(0.39998+0.0); outer dp 1.9999.
/// Rule top is 3θ below the hbox depth (0 for "under").
#[test]
fn text_mode_underline_lays_out_a_rule() {
    let top = 3.0 * MATH_RULE_THETA_PT;
    assert_rule(&underline_doc("[10pt]"), "under", MATH_RULE_THETA_PT, top);
    assert_rule(&underline_doc("[12pt]"), "under", MATH_RULE_THETA_PT, top);
}

/// pdflatex 10pt rule(2.76805+-2.36806); 12pt rule(3.24167+-2.84167).
/// Rule top is 0.55ex+0.4pt above the baseline (negative y from baseline).
#[test]
fn sout_lays_out_a_rule() {
    let top_at = |size: f64| -(SOUT_RAISE_EX * CMR_EX_PER_EM * size + UL_THICKNESS_PT);
    assert_rule(&sout_doc("[10pt]"), "struck", UL_THICKNESS_PT, top_at(10.0));
    assert_rule(&sout_doc("[12pt]"), "struck", UL_THICKNESS_PT, top_at(12.0));
}
