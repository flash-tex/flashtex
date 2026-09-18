//! amsmath `\text` and `\boxed` in TEXT mode (Ross corpus slice).
//!
//! Stated pdflatex preamble (oracle: TeX Live 2026,
//! `/Library/TeX/texbin/pdflatex`, exit code 0, zero `!` errors):
//! `\documentclass[11pt]{article}` with `\usepackage{amsmath,amssymb}`
//! around `\begin{document}...\end{document}`. pdflatex accepts
//! text-mode `\text{...}` as `\mbox{...}` and draws the `\boxed`
//! frame; these tests pin that this compiler does the same with no
//! diagnostic, and that math-mode `\text` keeps working.
use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::parser::{parse, Block, Inline};
use flashtex_compiler::vocabulary::command_help;

/// The stated pdflatex preamble, corpus-shaped (`article` + amsmath).
fn document(body: &str) -> String {
    format!(
        "\\documentclass[11pt]{{article}}\n\\usepackage{{amsmath,amssymb}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn words(source: &str) -> Vec<String> {
    let output = compile_full(source, LayoutConstraints::default());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| item.text.clone())
        .collect()
}

/// The single body paragraph's inlines, for `space_before` assertions.
fn paragraph(source: &str) -> Vec<Inline> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            out.extend(inlines.clone());
        }
    }
    assert!(!out.is_empty(), "no paragraph in {source:?}");
    out
}

fn inline_texts(inlines: &[Inline]) -> Vec<&str> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// The corpus case (`set1.tex` line 65): `\text{\Large A. Student}` in
/// a `center` heading. Before the fix this emitted
/// `error[unsupported_feature]`; the words below are the exact
/// before-fix layout, pinned so the fix cannot move a glyph.
#[test]
fn text_mode_text_renders_without_error() {
    let source = document(
        "\\begin{center}\n\\textbf{\\LARGE Set 1} \\\\ \n\\medskip\n\\text{\\Large A. Student}\n\\end{center}",
    );
    assert_eq!(words(&source), ["Set", "1", "A.", "Student"]);
}

/// `\text` inside math mode is untouched by the text-mode arm.
#[test]
fn math_mode_text_still_works() {
    let source = document("\\begin{align*}\na &= a &\\quad &\\text{Reflection}\\\\\n\\end{align*}");
    let laid_out = words(&source);
    assert!(laid_out.contains(&"Reflection".to_string()), "{laid_out:?}");
}

/// `\text{}` contributes nothing and reports nothing (pdflatex:
/// `Before\text{}After.` sets "BeforeAfter.").
#[test]
fn empty_text_is_silent() {
    let inlines = paragraph(&document("Before\\text{}After."));
    assert_eq!(inline_texts(&inlines), ["Before", "After."]);
}

/// A leading argument-edge space survives on the first inline
/// (pdflatex sets "Before afterAfter.").
#[test]
fn leading_edge_space_survives() {
    let inlines = paragraph(&document("Before\\text{ after}After."));
    assert_eq!(inline_texts(&inlines), ["Before", "after", "After."]);
    let flags: Vec<bool> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { space_before, .. } => Some(*space_before),
            _ => None,
        })
        .collect();
    assert_eq!(flags, [true, true, false], "{inlines:?}");
}

/// A trailing argument-edge space is dropped — exactly like the
/// engine's sibling box arguments (`\textbf{before }After.` and a
/// plain `{before }` group both lay out "beforeAfter."). pdflatex
/// keeps the gap ("Beforebefore After."), so this pins current
/// behaviour for a future general fix to update deliberately.
#[test]
fn trailing_edge_space_matches_sibling_commands() {
    let inlines = paragraph(&document("Before\\text{before }After."));
    assert_eq!(inline_texts(&inlines), ["Before", "before", "After."]);
    let flags: Vec<bool> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { space_before, .. } => Some(*space_before),
            _ => None,
        })
        .collect();
    assert_eq!(flags, [true, false, false], "{inlines:?}");
}

fn single_colorbox(inlines: &[Inline]) -> &flashtex_compiler::parser::ColorBox {
    let mut boxes = inlines.iter().filter_map(|inline| match inline {
        Inline::ColorBox(boxed) => Some(boxed.as_ref()),
        _ => None,
    });
    let found = boxes.next().expect("one ColorBox");
    assert!(boxes.next().is_none(), "{inlines:?}");
    found
}

/// The corpus case (`variations/notes.tex`): `\boxed{$f(x) = 2x$.}` in
/// text mode. Before the fix the formula reached the page but the
/// frame was silently dropped; now it is one bordered box.
#[test]
fn text_mode_boxed_draws_its_frame() {
    let inlines = paragraph(&document("The answer is \\boxed{$f(x) = 2x$.}"));
    let boxed = single_colorbox(&inlines);
    assert!(boxed.frame.is_some(), "frame must be drawn");
    assert!(
        boxed
            .content
            .iter()
            .any(|inline| matches!(inline, Inline::Math { .. })),
        "explicit $...$ keeps its formula: {inlines:?}"
    );
    let laid_out = words(&document("The answer is \\boxed{$f(x) = 2x$.}"));
    for word in ["The", "answer", "is", "f", "2"] {
        assert!(laid_out.contains(&word.to_string()), "{laid_out:?}");
    }
}

/// A bare `\boxed{42}.` boxes as text, like `\fbox` — with a frame.
#[test]
fn text_mode_boxed_bare_argument_has_a_frame() {
    let inlines = paragraph(&document("The answer is \\boxed{42}."));
    let boxed = single_colorbox(&inlines);
    assert!(boxed.frame.is_some(), "frame must be drawn");
    assert_eq!(inline_texts(&boxed.content), ["42"]);
}

/// The "wrap this in math mode" advice is gone: for a genuinely
/// math-only command the help now states the mode without telling the
/// author to wrap anything.
#[test]
fn math_command_help_no_longer_suggests_wrapping() {
    assert_eq!(
        command_help("alpha").as_deref(),
        Some("\\alpha is a math command; use it in math mode")
    );
    for name in ["alpha", "boxed", "frac"] {
        let help = command_help(name).expect("math commands keep a mode hint");
        assert!(!help.contains("wrap this in math mode"), "{name}: {help}");
    }
}
