//! Beamer Tier 0, the page model (issue #944; corpus PR #943): what the
//! parser hands the render pipeline for a slide deck.
//!
//! Oracle: pdflatex (TeX Live 2026), `\documentclass{beamer}`, no packages.
//! Measured on `fixtures/real-world/beamer-default`: 7 pages for 7 frames
//! *including* the frame that holds only `\titlepage`; `\section{Motivation}`
//! between frames prints nothing; `\title[short]{...}` etc. take an optional
//! short form; `\note{...}` prints nothing. The geometry itself (frametitle
//! box, `[c]` body glue, title page pitches) is the render pipeline's, tested
//! in `crates/render-pipeline/tests/beamer_default.rs`.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, BeamerFrameAlign, Block, Inline};

fn deck(body: &str) -> String {
    format!("\\documentclass{{beamer}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn blocks(text: &str) -> Vec<Block> {
    parse(text).blocks
}

fn compiled(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn messages(output: &CompileOutput) -> Vec<String> {
    output.diagnostics.iter().map(|d| d.message.clone()).collect()
}

fn text_of(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn page_texts(output: &CompileOutput) -> Vec<String> {
    output
        .pages
        .iter()
        .map(|page| page.items.iter().map(|item| item.text.as_str()).collect::<Vec<_>>().join(" "))
        .collect()
}

#[test]
fn every_frame_is_bracketed_even_when_empty() {
    let b = blocks(&deck("\\begin{frame}\n\\end{frame}\n\\begin{frame}\nText.\n\\end{frame}"));
    let edges: Vec<&str> = b
        .iter()
        .map(|b| match b {
            Block::BeamerFrameBegin { .. } => "begin",
            Block::BeamerFrameEnd { .. } => "end",
            Block::Paragraph(_) => "para",
            _ => "other",
        })
        .collect();
    assert_eq!(edges, ["begin", "end", "begin", "para", "end"], "{b:?}");
}

#[test]
fn an_empty_frame_is_a_page_in_the_compiler_layout_too() {
    // pdflatex: 2 pages (the first frame is empty).
    let out = compiled(&deck("\\begin{frame}\n\\end{frame}\n\\begin{frame}\nText.\n\\end{frame}"));
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 2, "{:?}", page_texts(&out));
}

#[test]
fn the_head_arguments_and_frametitle_fill_the_begin_block() {
    let b = blocks(&deck(
        "\\begin{frame}{Head}{Sub}\nA.\n\\end{frame}\n\\begin{frame}[t]\n\\frametitle<1->[short]{Body form}\n\\framesubtitle{Under}\nB.\n\\end{frame}",
    ));
    let heads: Vec<(BeamerFrameAlign, String, String)> = b
        .iter()
        .filter_map(|b| match b {
            Block::BeamerFrameBegin { options, title, subtitle, .. } => Some((options.align, text_of(title), text_of(subtitle))),
            _ => None,
        })
        .collect();
    assert_eq!(
        heads,
        [
            (BeamerFrameAlign::Center, "Head".to_string(), "Sub".to_string()),
            (BeamerFrameAlign::Top, "Body form".to_string(), "Under".to_string()),
        ]
    );
    // The body form never becomes a heading block of its own.
    assert!(!b.iter().any(|b| matches!(b, Block::Heading { .. })), "{b:?}");
}

#[test]
fn frame_options_are_read() {
    use flashtex_compiler::parser::beamer_frame_options;
    let o = beamer_frame_options("t,fragile=singleslide, allowframebreaks");
    assert_eq!(o.align, BeamerFrameAlign::Top);
    assert!(o.fragile && o.allowframebreaks && !o.plain);
    let o = beamer_frame_options("plain,b,label=intro");
    assert_eq!(o.align, BeamerFrameAlign::Bottom);
    assert!(o.plain && !o.fragile);
}

#[test]
fn sections_between_frames_typeset_nothing() {
    let out = compiled(&deck(
        "\\begin{frame}\nA.\n\\end{frame}\n\\section{Motivation}\n\\subsection*{Detail}\n\\section<presentation>[Short]{Design}\n\\begin{frame}\nB.\n\\end{frame}",
    ));
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    let texts = page_texts(&out);
    assert_eq!(texts.len(), 2, "{texts:?}");
    for word in ["Motivation", "Detail", "Design", "Short", "1"] {
        assert!(!texts.iter().any(|t| t.split(' ').any(|w| w == word)), "{word:?} printed: {texts:?}");
    }
    let b = blocks(&deck("\\begin{frame}\nA.\n\\end{frame}\n\\section{Motivation}\n\\begin{frame}\nB.\n\\end{frame}"));
    assert!(!b.iter().any(|b| matches!(b, Block::Heading { .. })), "{b:?}");
}

#[test]
fn titlepage_carries_every_preamble_field_with_short_forms_dropped() {
    let source = "\\documentclass{beamer}\n\\title[Short]{Incremental Typesetting}\n\\subtitle{Why a from-scratch engine can be fast}\n\\author[JW]{J. Whitfield}\n\\institute[FT]{FlashTeX corpus}\n\\date[2026]{March 2026}\n\\begin{document}\n\\begin{frame}\n\\titlepage\n\\end{frame}\n\\end{document}\n";
    let b = blocks(source);
    let Some(Block::BeamerTitlePage { title, subtitle, authors, institute, date, .. }) =
        b.iter().find(|b| matches!(b, Block::BeamerTitlePage { .. }))
    else {
        panic!("no title page block: {b:?}");
    };
    assert_eq!(text_of(title), "Incremental Typesetting");
    assert_eq!(text_of(subtitle), "Why a from-scratch engine can be fast");
    assert_eq!(text_of(authors), "J. Whitfield");
    assert_eq!(text_of(institute), "FlashTeX corpus");
    assert_eq!(text_of(date), "March 2026");
    for word in ["Short", "JW", "FT", "2026"] {
        assert!(!b.iter().any(|b| matches!(b, Block::Paragraph(p) if text_of(p).contains(word))), "{word} leaked: {b:?}");
    }
    let out = compiled(source);
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 1);
}

#[test]
fn titlepage_with_nothing_declared_is_still_a_block_and_a_page() {
    let out = compiled(&deck("\\begin{frame}\n\\titlepage\n\\end{frame}\n\\begin{frame}\nB.\n\\end{frame}"));
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 2, "{:?}", page_texts(&out));
    let b = blocks(&deck("\\begin{frame}\n\\titlepage\n\\end{frame}"));
    assert!(b.iter().any(|b| matches!(b, Block::BeamerTitlePage { title, .. } if title.is_empty())), "{b:?}");
}

#[test]
fn notes_alerts_with_overlays_and_theme_declarations_leave_no_trace() {
    let source = "\\documentclass{beamer}\n\\usetheme{Madrid}\n\\usecolortheme[named=blue]{structure}\n\\setbeamertemplate{navigation symbols}{}\n\\setbeamercovered{transparent}\n\\setbeamercolor{title}{fg=red}\n\\setbeamerfont{title}{series=\\bfseries}\n\\beamertemplatenavigationsymbolsempty\n\\begin{document}\n\\begin{frame}\nShown \\alert<2>{now}.\n\\note{hidden presenter text}\n\\note[item]{also hidden}\n\\end{frame}\n\\end{document}\n";
    let out = compiled(source);
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    let texts = page_texts(&out);
    assert_eq!(texts.len(), 1, "{texts:?}");
    assert!(texts[0].contains("Shown") && texts[0].contains("now"), "{texts:?}");
    for word in ["hidden", "presenter", "Madrid", "transparent", "navigation"] {
        assert!(!texts[0].contains(word), "{word} printed: {texts:?}");
    }
}

#[test]
fn an_articles_own_note_or_alert_macro_wins() {
    // Not kernel commands: a user definition in another class is ordinary
    // LaTeX (the `macro_argument_fonts` pipeline oracle defines `\note`).
    let out = compiled(concat!(
        "\\documentclass{article}\n",
        "\\newcommand{\\note}[1]{{\\bfseries #1}}\n",
        "\\newcommand{\\alert}[1]{[#1]}\n",
        "\\begin{document}\n",
        "\\note{Bold} and \\alert{boxed}.\n",
        "\\end{document}\n",
    ));
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    let texts = page_texts(&out);
    assert!(texts[0].contains("Bold") && texts[0].contains("boxed"), "{texts:?}");
}

#[test]
fn beamer_only_commands_are_refused_elsewhere() {
    let out = compiled("\\documentclass{article}\n\\begin{document}\n\\titlepage x\n\\end{document}\n");
    let m = messages(&out);
    assert!(m.iter().any(|m| m.contains("\\titlepage is defined by the beamer document class")), "{m:?}");
}
