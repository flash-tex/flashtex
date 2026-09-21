//! Beamer Tier 4, the compiler's share (issue #944): the `\frame{...}`
//! command form, `\note` in its argument forms, the `[short]` title-block
//! forms and `\usetheme` for the theme's footline.
//!
//! Oracle: pdflatex (TeX Live 2026) on `fixtures/real-world/beamer-fragile`
//! (8 pages; the strings "presenter" and "hidden" of its two `\note`s appear
//! on no page) and `fixtures/real-world/beamer-madrid` (the footline sets
//! `\insertshortauthor` "Whitfield", `\insertshortinstitute` "FlashTeX",
//! `\insertshorttitle` "Incremental" and `\insertshortdate` "March 2026" —
//! `\date{March 2026}` has no short form, so `\@dblarg` uses the whole
//! argument). The geometry is the render pipeline's
//! (`crates/render-pipeline/tests/beamer_t4.rs`).

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline};

fn deck(body: &str) -> String {
    format!("\\documentclass{{beamer}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
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

/// `\frame{...}` is one frame: a `BeamerFrameBegin` with the `\frametitle`
/// patched in, the body's paragraph, and the `BeamerFrameEnd` at the `}`.
#[test]
fn frame_command_form_is_a_frame() {
    let src = deck("\\frame{\n  \\frametitle{The command form}\n  Written with the command.\n}\n\\begin{frame}\nNext.\n\\end{frame}");
    let parsed = parse(&src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let kinds: Vec<&str> = parsed
        .blocks
        .iter()
        .map(|b| match b {
            Block::BeamerFrameBegin { .. } => "begin",
            Block::BeamerFrameEnd { .. } => "end",
            Block::Paragraph { .. } => "para",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["begin", "para", "end", "begin", "para", "end"], "{kinds:?}");
    let Block::BeamerFrameBegin { title, .. } = &parsed.blocks[0] else { unreachable!() };
    assert_eq!(text_of(title), "The command form");
    let out = compiled(&src);
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 2, "{:?}", page_texts(&out));
    let texts = page_texts(&out);
    assert!(texts[0].contains("Written with the command."), "{texts:?}");
    assert!(texts[1].contains("Next."), "{texts:?}");
}

/// `\frame<spec>[options]{...}`: the head is read like `\begin{frame}`'s.
#[test]
fn frame_command_form_reads_the_head() {
    let src = deck("\\frame<1>[plain,t]{Body.}");
    let parsed = parse(&src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let Block::BeamerFrameBegin { options, title, .. } = &parsed.blocks[0] else { panic!("{:?}", parsed.blocks) };
    assert!(options.plain);
    assert_eq!(options.align, flashtex_compiler::parser::BeamerFrameAlign::Top);
    assert!(title.is_empty(), "the body group is not a title: {title:?}");
    assert!(matches!(parsed.blocks[1], Block::Paragraph { .. }));
    assert!(matches!(parsed.blocks[2], Block::BeamerFrameEnd { .. }));
}

/// The corpus deck's notes: `\note{...}` inside a frame, after
/// `\end{frame}`, and `\note[item]{...}` all typeset nothing.
#[test]
fn notes_in_every_form_are_silent() {
    let src = deck(
        "\\begin{frame}\nA.\n\\note{This note is for the presenter and must not be printed.}\n\\end{frame}\n\\note{A second note, also hidden.}\n\\begin{frame}\nB.\n\\note[item]{An item note}\n\\note<2>[item]{Overlay note}\n\\end{frame}",
    );
    let out = compiled(&src);
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    assert_eq!(out.pages.len(), 2, "{:?}", page_texts(&out));
    let all = page_texts(&out).join(" ");
    for word in ["presenter", "hidden", "item note", "Overlay"] {
        assert!(!all.contains(word), "{word:?} leaked: {all}");
    }
}

/// `Parsed::beamer` carries the theme and the short forms the Madrid
/// footline sets (the full argument when no `[short]` is given).
#[test]
fn deck_info_has_the_theme_and_the_short_forms() {
    let src = "\\documentclass{beamer}\n\\usetheme{Madrid}\n\\title[Incremental]{Incremental Typesetting}\n\\subtitle{Why}\n\\author[Whitfield]{J. Whitfield}\n\\institute[FlashTeX]{FlashTeX corpus}\n\\date{March 2026}\n\\begin{document}\n\\begin{frame}\n\\titlepage\n\\end{frame}\n\\end{document}\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let deck = parsed.beamer.expect("a beamer deck");
    assert_eq!(deck.theme.as_deref(), Some("Madrid"));
    assert_eq!(text_of(&deck.short_title), "Incremental");
    assert_eq!(text_of(&deck.short_author), "Whitfield");
    assert_eq!(text_of(&deck.short_institute), "FlashTeX");
    assert_eq!(text_of(&deck.short_date), "March 2026");
    // The title page still gets the full forms.
    let tp = parsed.blocks.iter().find_map(|b| match b {
        Block::BeamerTitlePage { title, institute, .. } => Some((text_of(title), text_of(institute))),
        _ => None,
    });
    assert_eq!(tp, Some(("Incremental Typesetting".to_string(), "FlashTeX corpus".to_string())));
}

/// No theme, no short forms: the deck record is still there (the default
/// theme has no footline to read it) and an article has none at all.
#[test]
fn default_theme_deck_and_articles() {
    let parsed = parse(&deck("\\begin{frame}\nA.\n\\end{frame}"));
    let d = parsed.beamer.expect("beamer");
    assert_eq!(d.theme, None);
    assert!(d.short_title.is_empty() && d.short_author.is_empty());
    let article = parse("\\documentclass{article}\n\\title[x]{T}\n\\begin{document}\nA.\n\\end{document}\n");
    assert!(article.beamer.is_none());
}

/// A `[short]` form with blanks spans several word tokens.
#[test]
fn short_forms_with_blanks() {
    let src = "\\documentclass{beamer}\n\\usetheme{Madrid}\n\\author[A. Person and B. Other]{Anne Person}\n\\begin{document}\n\\begin{frame}\nA.\n\\end{frame}\n\\end{document}\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(text_of(&parsed.beamer.unwrap().short_author), "A. Person and B. Other");
}

/// Inside a frame beamer restores the kernel's `\frame{<text>}` (an
/// `\fbox` with `\fboxsep` 0): a boxed inline, no new page, no diagnostic.
#[test]
fn frame_inside_a_frame_is_the_kernel_box() {
    let src = deck("\\begin{frame}\nA \\frame{boxed} word.\n\\end{frame}");
    let parsed = parse(&src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let frames = parsed.blocks.iter().filter(|b| matches!(b, Block::BeamerFrameBegin { .. })).count();
    assert_eq!(frames, 1);
    let boxed = parsed.blocks.iter().any(|b| match b {
        Block::Paragraph(inlines) => inlines.iter().any(|i| matches!(i, Inline::ColorBox(cb) if cb.fboxsep_pt == 0.0 && text_of(&cb.content) == "boxed")),
        _ => false,
    });
    assert!(boxed, "{:?}", parsed.blocks);
}

/// `\logo{..}` (`beamerbaseframecomponents.sty`: `\def\logo{\def
/// \insertlogo}`) sets nothing where it is written: the deck carries it
/// for every frame's `sidebar right` template, the last one winning, in
/// the preamble or the body. Outside beamer it is an unknown command.
#[test]
fn logo_is_carried_by_the_deck() {
    let src = "\\documentclass{beamer}\n\\logo{First}\n\\logo{\\textbf{FT}}\n\\begin{document}\n\\begin{frame}\nBody.\n\\end{frame}\n\\end{document}\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let logo = &parsed.beamer.as_ref().expect("a beamer deck").logo;
    assert_eq!(text_of(logo), "FT", "{logo:?}");
    assert!(logo.iter().any(|i| matches!(i, Inline::Text { style, .. } if style.bold)), "{logo:?}");
    let out = compiled(src);
    assert!(out.diagnostics.is_empty(), "{:?}", messages(&out));
    let texts = page_texts(&out);
    assert!(!texts[0].contains("First") && !texts[0].contains("FT"), "{texts:?}");

    let none = parse(&deck("\\begin{frame}\nBody.\n\\end{frame}"));
    assert!(none.beamer.as_ref().expect("a beamer deck").logo.is_empty());

    let article = parse("\\documentclass{article}\n\\begin{document}\n\\logo{X}\n\\end{document}\n");
    assert!(!article.diagnostics.is_empty(), "\\logo outside beamer is unknown");
}
