//! Beamer `frame` environments: one slide per page (issue #841).
//!
//! Oracle: pdflatex (TeX Live 2026) with the stated preamble
//! (`\documentclass{beamer}`, no packages) sets the 12-line repro below on
//! 3 pages, one per frame. Before this fix FlashTeX set all three frames on
//! 1 page with zero diagnostics: `frame` was always read as the
//! rule-bordered box environment, even under `\documentclass{beamer}`.
//!
//! The bordered-box reading is kept for every other class (see
//! `frame_env.rs`); only `beamer` takes the slide path.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline};

/// The issue's 12-line minimal repro, byte for byte (no packages).
const REPRO: &str = "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}{First}\nAlpha content.\n\\end{frame}\n\\begin{frame}{Second}\nBeta content.\n\\end{frame}\n\\begin{frame}{Third}\nGamma content.\n\\end{frame}\n\\end{document}\n";

fn compiled(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn page_texts(output: &CompileOutput) -> Vec<String> {
    output
        .pages
        .iter()
        .map(|page| {
            page.items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn messages(output: &CompileOutput) -> Vec<String> {
    output
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn three_frames_make_three_pages() {
    let out = compiled(REPRO);
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 3, "{:?}", page_texts(&out));
    let texts = page_texts(&out);
    assert!(
        texts[0].contains("First") && texts[0].contains("Alpha content."),
        "{texts:?}"
    );
    assert!(
        texts[1].contains("Second") && texts[1].contains("Beta content."),
        "{texts:?}"
    );
    assert!(
        texts[2].contains("Third") && texts[2].contains("Gamma content."),
        "{texts:?}"
    );
}

#[test]
fn frame_title_argument_form_renders_the_title() {
    let out = compiled(
        "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}{Hello}\nBody text.\n\\end{frame}\n\\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 1, "{:?}", page_texts(&out));
    let texts = page_texts(&out);
    assert!(texts[0].contains("Hello"), "{texts:?}");
    assert!(texts[0].contains("Body text."), "{texts:?}");
}

#[test]
fn frametitle_in_the_body_renders_the_title() {
    let out = compiled(
        "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}\n\\frametitle{World}\nBody text.\n\\end{frame}\n\\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 1, "{:?}", page_texts(&out));
    let texts = page_texts(&out);
    assert!(texts[0].contains("World"), "{texts:?}");
    assert!(texts[0].contains("Body text."), "{texts:?}");
}

#[test]
fn frames_do_not_warn_or_error() {
    for body in [
        "\\begin{frame}{A}x\\end{frame}",
        "\\begin{frame}[fragile]{A}x\\end{frame}",
        "\\begin{frame}\\frametitle{A}x\\end{frame}",
        "\\begin{frame}\\framesubtitle{S}x\\end{frame}",
        "\\begin{frame}{T}{S}x\\end{frame}",
        "\\begin{frame}{}x\\end{frame}",
        "\\begin{frame}\\frametitle{}x\\end{frame}",
    ] {
        let out = compiled(&format!(
            "\\documentclass{{beamer}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
        ));
        let m = messages(&out);
        assert!(
            m.iter().all(|m| !m.contains("not implemented")
                && !m.contains("not supported")
                && !m.contains("is defined by")),
            "{body:?}: {m:?}"
        );
    }
}

#[test]
fn beamer_frame_options_and_overlay_specs_leave_no_trace() {
    // `[fragile]`-style options and `<1->` overlay specs are read as frame
    // parameters, not typeset: ignoring overlays means one page per frame
    // with no leaked markup on it.
    let out = compiled(
        "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}[t]{A}\nx\n\\end{frame}\n\\begin{frame}<1>{B}\ny\n\\end{frame}\n\\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 2, "{:?}", page_texts(&out));
    let texts = page_texts(&out);
    assert!(!texts[0].contains("[t]"), "{texts:?}");
    assert!(!texts[1].contains("<1>"), "{texts:?}");
}

#[test]
fn alert_sets_red_text_under_beamer() {
    let parsed = parse(
        "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}{A}\n\\alert{Warning words} plain.\n\\end{frame}\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut reds = Vec::new();
    let mut plains = Vec::new();
    for block in &parsed.blocks {
        let content = match block {
            Block::Paragraph(content) | Block::Heading { content, .. } => content,
            _ => continue,
        };
        for inline in content {
            if let Inline::Text { text, style, .. } = inline {
                if style.color.is_some() {
                    reds.push(text.clone());
                } else {
                    plains.push(text.clone());
                }
            }
        }
    }
    assert!(
        reds.iter().any(|t| t.contains("Warning")),
        "reds: {reds:?}, plains: {plains:?}"
    );
    assert!(plains.iter().any(|t| t.contains("plain")), "{plains:?}");
    // Beamer's alert colour is red: xcolor `red` is `1 0 0 rg`.
    let parsed2 =
        parse("\\documentclass{beamer}\n\\begin{document}\n\\alert{x}\n\\end{document}\n");
    let mut ops = Vec::new();
    for block in &parsed2.blocks {
        let content = match block {
            Block::Paragraph(content) | Block::Heading { content, .. } => content,
            _ => continue,
        };
        for inline in content {
            if let Inline::Text { style, .. } = inline {
                if let Some(color) = style.color {
                    ops.push(color.fill_operator());
                }
            }
        }
    }
    assert_eq!(ops, vec!["1 0 0 rg".to_string()], "{ops:?}");
}

#[test]
fn beamer_commands_outside_beamer_name_the_class() {
    // Like `letter.cls` commands outside `letter` (see `letter_class.rs`):
    // in an `article` these are undefined, and the diagnostic says so.
    for command in ["\\frametitle{X}", "\\framesubtitle{X}", "\\alert{X}"] {
        let parsed = parse(&format!(
            "\\documentclass{{article}}\n\\begin{{document}}\n{command}\n\\end{{document}}\n"
        ));
        let m: Vec<String> = parsed
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect();
        assert!(m.iter().any(|m| m.contains("beamer")), "{command}: {m:?}");
    }
}

#[test]
fn fifty_frames_make_at_least_fifty_pages() {
    // The issue's scale check: a 50-frame deck must paginate per frame.
    let mut source = "\\documentclass{beamer}\n\\begin{document}\n".to_string();
    for n in 1..=50 {
        source.push_str(&format!(
            "\\begin{{frame}}{{Slide {n}}}\nContent {n}.\n\\end{{frame}}\n"
        ));
    }
    source.push_str("\\end{document}\n");
    let out = compiled(&source);
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert!(out.pages.len() >= 50, "got {} pages", out.pages.len());
}
