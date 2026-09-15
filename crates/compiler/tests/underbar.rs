//! GH-UNDERBAR: kernel `\underbar{text}`.
//!
//! Real LaTeX2e (`latex.ltx`, line 619) defines
//! `\def\underbar#1{\underline{\sbox\tw@{#1}\dp\tw@\z@\box\tw@}}`: the
//! same TeXbook Rule 10 construction as text-mode `\underline`
//! (`$\@@underline{\hbox{#1}}$`), but the boxed argument's depth is zeroed
//! (`\dp\tw@\z@`) before the rule is drawn. So `\underbar` draws its rule
//! at a FIXED 3\theta below the baseline (total depth 5\theta) while
//! `\underline` preserves the hbox depth (rule at depth + 3\theta, total
//! depth + 5\theta). pdflatex (cmr10, 10pt, \theta = 0.39998pt) confirms:
//! `\underline{y}` reports `\dp` 3.94434pt = 1.94444pt + 5\theta, while
//! `\underbar{y}` reports `\dp` 1.9999pt = 5\theta; descender-free content
//! (`a`) reports 1.9999pt under both.
//!
//! In math mode `\underbar` is NOT an error: it expands to `\underline`,
//! whose `\ifmmode` branch calls the primitive directly (verified: no
//! diagnostic from pdflatex for `$\underbar{x}$`). `\@badmath` ("Bad math
//! environment delimiter") belongs to `\(`, `\)`, `\[`, `\]` only and is
//! never involved. So math-mode `\underbar` parses like `\underline`
//! (`Frame::Under`).
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{
    parse, Block, Inline, SourceDocument, UnderlineGeom, MATH_RULE_THETA_PT,
    TEXT_DESCENDER_DEPTH_EM,
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

/// Rule height and top-from-baseline for the single underline rule in `source`.
fn rule_shape(source: &str, needle: &str) -> (f64, f64) {
    let out = compiled(source);
    let items = &out.pages[0].items;
    let baseline = items
        .iter()
        .find(|item| item.text.contains(needle))
        .expect("argument glyphs missing")
        .baseline_y_pt;
    let rule = items
        .iter()
        .find(|item| item.rule.is_some())
        .expect("expected a rule")
        .rule
        .expect("filtered");
    (rule.height_pt, rule.y_pt - baseline)
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

/// Parse level: `\underbar` gets the depth-zeroed geometry
/// (`UnderlineGeom::Underbar`), not the `\underline` one, with the same
/// Rule 10 thickness and visible content, and no ulem requirement.
#[test]
fn underbar_parses_to_depth_zeroed_geometry() {
    let inlines = paragraph_inlines(&doc("Text \\underbar{y} here."));
    let (geom, thickness, text) = inlines
        .iter()
        .find_map(|inline| match inline {
            Inline::Underline(u) => Some((u.geom, u.thickness_pt, text_of(&u.content))),
            _ => None,
        })
        .expect("expected an underline wrapper");
    assert_eq!(geom, UnderlineGeom::Underbar);
    assert!((thickness - MATH_RULE_THETA_PT).abs() < 1e-9);
    assert_eq!(text, "y");
    assert!(
        messages(&doc("\\underbar{y}"))
            .iter()
            .all(|m| !m.contains("ulem")),
        "\\underbar is kernel and must not require ulem"
    );
}

/// Layout level, pdflatex-calibrated (10pt, \theta = 0.39998pt):
/// `\underbar{y}` fixes its rule top at 3\theta below the baseline while
/// `\underline{y}` preserves the 1.94444pt descender depth, so the two rule
/// tops differ by exactly that depth.
#[test]
fn underbar_zeroes_descender_depth_while_underline_preserves_it() {
    let descender = TEXT_DESCENDER_DEPTH_EM * 10.0;
    assert!(
        (descender - 1.94444).abs() < 1e-4,
        "calibration: cmr10 descender depth is 1.94444pt at 10pt, got {descender}"
    );
    let (height, top_underbar) = rule_shape(&doc("Text \\underbar{y} here."), "y");
    let (_, top_underline) = rule_shape(&doc("Text \\underline{y} here."), "y");
    assert!(
        (height - MATH_RULE_THETA_PT).abs() < 0.01,
        "rule height should be θ, got {height}"
    );
    assert!(
        (top_underbar - 3.0 * MATH_RULE_THETA_PT).abs() < 0.01,
        "\\underbar{{y}} rule top should sit at a fixed 3θ below the baseline, got {top_underbar}"
    );
    assert!(
        (top_underline - (descender + 3.0 * MATH_RULE_THETA_PT)).abs() < 0.01,
        "\\underline{{y}} rule top should sit depth + 3θ below the baseline, got {top_underline}"
    );
    assert!(
        ((top_underline - top_underbar) - descender).abs() < 0.02,
        "the rules must differ by the zeroed descender depth {descender}, got {}",
        top_underline - top_underbar
    );
}

/// Descender-free content has hbox depth 0 either way, so both commands
/// paint the identical rule (Rule 10: θ thick, top 3θ below the baseline).
#[test]
fn no_descender_content_renders_identically() {
    let from_underbar = rule_shape(&doc("Text \\underbar{under} here."), "under");
    let from_underline = rule_shape(&doc("Text \\underline{under} here."), "under");
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
        "without descenders zeroing is a no-op: identical rules"
    );
}

/// Math mode: real `\underbar` works there (it expands to `\underline`,
/// whose `\ifmmode` branch calls the primitive — no `\@badmath`, which
/// belongs to `\(`, `\)`, `\[`, `\]` only). So `$\underbar{x}$` must parse
/// with no math-mode diagnostic and draw the same rule as `$\underline{x}$`.
#[test]
fn math_mode_underbar_typesets_like_underline() {
    for (label, body) in [
        ("underbar", "$\\underbar{x}$"),
        ("underline", "$\\underline{x}$"),
    ] {
        let out = compiled(&doc(body));
        let diags: Vec<_> = out
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert!(
            diags
                .iter()
                .all(|m| !m.contains("not supported in math mode")),
            "{label} must work in math mode: {diags:?}"
        );
        assert!(
            out.pages[0].items.iter().any(|item| item.rule.is_some()),
            "{label} must draw a rule in math mode"
        );
    }
    let shape = |body: &str| {
        let out = compiled(&doc(body));
        let items = &out.pages[0].items;
        let baseline = items
            .iter()
            .find(|item| item.text.contains('x'))
            .expect("body glyph missing")
            .baseline_y_pt;
        let rule = items
            .iter()
            .find(|item| item.rule.is_some())
            .expect("expected a rule")
            .rule
            .expect("filtered");
        (rule.height_pt, rule.y_pt - baseline)
    };
    assert_eq!(
        shape("$\\underbar{x}$"),
        shape("$\\underline{x}$"),
        "math-mode \\underbar must match \\underline"
    );
}
