//! `\title`/`\author`/`\date` and `\maketitle` — `article.cls`'s
//! `\@maketitle`: `\LARGE` title, `\vskip 1.5em`, `\large` author block,
//! `\vskip 1em`, `\large` date, all inside a leading `\vskip 2em` and a
//! trailing `\vskip 1.5em`. See `crates/title-layout/src/title.rs` for the
//! `article.cls` provenance these numbers transcribe, and
//! `layout::LayoutCursor`'s `Block::TitleBlock` arms for how this compiler
//! reproduces them without a dependency on that crate.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::{LayoutConstraints, LINE_SPACING, MARGIN_PT};
use flashtex_compiler::date::TodayDate;
use flashtex_compiler::parser::{parse, Block, Inline, SourceDocument};

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}{preamble}\\begin{{document}}{body}\\end{{document}}")
}

fn title_block(source: &str) -> Block {
    parse(source)
        .blocks
        .into_iter()
        .find(|b| matches!(b, Block::TitleBlock { .. }))
        .expect("expected a title block")
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

#[test]
fn maketitle_without_title_or_author_is_an_honest_error_not_a_placeholder() {
    let no_title = parse(&doc("\\author{A}", "\\maketitle"));
    assert!(
        no_title
            .diagnostics
            .iter()
            .any(|d| d.message.contains("requires \\title")),
        "{:?}",
        no_title.diagnostics
    );
    assert!(!no_title
        .blocks
        .iter()
        .any(|b| matches!(b, Block::TitleBlock { .. })));

    let no_author = parse(&doc("\\title{T}", "\\maketitle"));
    assert!(
        no_author
            .diagnostics
            .iter()
            .any(|d| d.message.contains("requires \\author")),
        "{:?}",
        no_author.diagnostics
    );
    assert!(!no_author
        .blocks
        .iter()
        .any(|b| matches!(b, Block::TitleBlock { .. })));
}

/// latex.ltx 17225 is `\gdef\@date{\today}`, so a document with no `\date` at
/// all typesets exactly what `\date{\today}` does. Verified against pdflatex
/// 3.141592653-2.6-1.40.27 (TeX Live 2025): `\the\pagetotal` immediately after
/// `\maketitle` is 116.86673pt for both, to the scaled point.
#[test]
fn an_absent_date_is_exactly_date_today() {
    let default_date = doc("\\title{T}\\author{A}", "\\maketitle");
    let explicit_today = doc("\\title{T}\\author{A}\\date{\\today}", "\\maketitle");
    for source in [default_date, explicit_today] {
        let Block::TitleBlock { date, .. } = title_block(&source) else {
            unreachable!()
        };
        let date = date.expect("date line present");
        assert!(
            format!("{date:?}").contains(&TodayDate::EPOCH.latex_today()),
            "expected the request date in {date:?}"
        );
    }
}

/// `\date{}` leaves `\@date` empty. article.cls still runs `\vskip 1em` and
/// opens `{\large \@date}`, but an empty group typesets no material, so no
/// line — and therefore no `\baselineskip` — is contributed. Verified against
/// pdflatex (TeX Live 2025): `\the\pagetotal` after `\maketitle` is 95.2001pt
/// with `\date{}` against 114.4001pt with `\date{Zz}`, a difference of exactly
/// one `\large` baselineskip (19.2pt) and *not* the additional 1em, which stays.
#[test]
fn empty_date_suppresses_the_date_line() {
    let source = doc("\\title{T}\\author{A}\\date{}", "\\maketitle");
    let Block::TitleBlock { date, .. } = title_block(&source) else {
        unreachable!()
    };
    assert!(date.is_none(), "{date:?}");
}

#[test]
fn custom_date_text_is_used_verbatim() {
    let source = doc("\\title{T}\\author{A}\\date{Spring 2026}", "\\maketitle");
    let Block::TitleBlock { date, .. } = title_block(&source) else {
        unreachable!()
    };
    let text = format!("{:?}", date.unwrap());
    assert!(text.contains("Spring") && text.contains("2026"), "{text}");
}

#[test]
fn thanks_is_a_symbol_footnote_not_title_text() {
    let source = doc(
        "\\title{Zzztitle\\thanks{Funded by a grant}}\\author{Zzzauthor}",
        "\\maketitle",
    );
    let parsed = parse(&source);
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\thanks")),
        "{:?}",
        parsed.diagnostics
    );
    let Block::TitleBlock { title, .. } = parsed
        .blocks
        .into_iter()
        .find(|b| matches!(b, Block::TitleBlock { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    let plain: Vec<&Inline> = title
        .iter()
        .filter(|i| !matches!(i, Inline::Footnote { .. }))
        .collect();
    let text = format!("{plain:?}");
    assert!(text.contains("Zzztitle"));
    assert!(
        !text.contains("grant"),
        "footnote text leaked into the title: {text}"
    );
    let note = title
        .iter()
        .find_map(|i| match i {
            Inline::Footnote {
                number, mark, text, ..
            } => Some((number.clone(), *mark, format!("{text:?}"))),
            _ => None,
        })
        .expect("the \\thanks footnote");
    assert_eq!(note.0, "\u{2217}");
    assert!(note.1);
    assert!(note.2.contains("grant"), "{}", note.2);
}

#[test]
fn and_separated_authors_are_stacked_vertically_with_a_warning() {
    let source = doc("\\title{T}\\author{Zzzone \\and Zzztwo}", "\\maketitle");
    let parsed = parse(&source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("side by side")),
        "{:?}",
        parsed.diagnostics
    );
    let Block::TitleBlock { authors, .. } = parsed
        .blocks
        .into_iter()
        .find(|b| matches!(b, Block::TitleBlock { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(
        authors
            .iter()
            .filter(|inline| matches!(inline, Inline::LineBreak { .. }))
            .count(),
        1,
        "{authors:?}"
    );
    let text = format!("{authors:?}");
    assert!(text.contains("Zzzone") && text.contains("Zzztwo"));
}

#[test]
fn titlepage_class_option_gets_an_honest_diagnostic_and_still_a_compact_block() {
    let source =
        "\\documentclass[titlepage]{article}\\title{T}\\author{A}\\begin{document}\\maketitle\\end{document}";
    let parsed = parse(source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("titlepage")),
        "{:?}",
        parsed.diagnostics
    );
    // Still produces the compact block rather than nothing.
    assert!(parsed
        .blocks
        .iter()
        .any(|b| matches!(b, Block::TitleBlock { .. })));
}

#[test]
fn without_the_titlepage_option_there_is_no_titlepage_diagnostic() {
    let source = doc("\\title{T}\\author{A}", "\\maketitle");
    let parsed = parse(&source);
    assert!(!parsed
        .diagnostics
        .iter()
        .any(|d| d.message.contains("titlepage")));
}

#[test]
fn single_author_gets_no_side_by_side_warning() {
    let source = doc("\\title{T}\\author{Solo Author}", "\\maketitle");
    let parsed = parse(&source);
    assert!(!parsed
        .diagnostics
        .iter()
        .any(|d| d.message.contains("side by side")));
}

#[test]
fn maketitle_exact_vskip_amounts_and_font_sizes() {
    let source = doc(
        "\\title{Zzztitle}\\author{Zzzauthor}\\date{Zzzdate}",
        "\\maketitle",
    );
    let out = compiled(&source);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let items = &out.pages[0].items;
    let title = items.iter().find(|i| i.text == "Zzztitle").unwrap();
    let author = items.iter().find(|i| i.text == "Zzzauthor").unwrap();
    let date = items.iter().find(|i| i.text == "Zzzdate").unwrap();

    // `\LARGE` title, `\large` author and date — strictly decreasing, and
    // author/date share the same size.
    assert!(title.font_size_pt > author.font_size_pt);
    assert_eq!(author.font_size_pt, date.font_size_pt);

    let body = LayoutConstraints::default().font_size_pt;
    // Leading `\null \vskip 2em`, then the title's own (larger) size — the
    // same "line sits `size` below whatever precedes it" convention this
    // cursor already uses for the very first line of a document, and for an
    // ordinary first-block `\section` (see `ensure_extents`).
    let expected_title_baseline = MARGIN_PT + 2.0 * body + title.font_size_pt;
    assert!(
        (title.baseline_y_pt - expected_title_baseline).abs() < 0.02,
        "title baseline {} vs expected {}",
        title.baseline_y_pt,
        expected_title_baseline
    );

    // `\vskip 1.5em` between title and author, plus the title line's own
    // depth and one ordinary interline advance to the author line's size —
    // this compiler's usual "close a line, open the next" idiom (the same
    // one `Block::Heading` and display math already use for an explicit gap
    // after a differently-sized line), not real TeX's `\baselineskip` math
    // and not `\parskip`.
    let title_descent = title.font_size_pt * (LINE_SPACING - 1.0);
    assert!(
        (author.baseline_y_pt
            - title.baseline_y_pt
            - title_descent
            - author.font_size_pt
            - 1.5 * body)
            .abs()
            < 0.02,
        "title-to-author gap {}",
        author.baseline_y_pt - title.baseline_y_pt
    );

    // `\vskip 1em` between author and date, plus the author line's depth and
    // one interline advance to the date line's size.
    let author_descent = author.font_size_pt * (LINE_SPACING - 1.0);
    assert!(
        (date.baseline_y_pt
            - author.baseline_y_pt
            - author_descent
            - date.font_size_pt
            - 1.0 * body)
            .abs()
            < 0.02,
        "author-to-date gap {}",
        date.baseline_y_pt - author.baseline_y_pt
    );
}

#[test]
fn maketitle_is_the_only_block_and_stays_on_page_one_when_first() {
    let source = doc("\\title{T}\\author{A}", "\\maketitle");
    let out = compiled(&source);
    assert_eq!(out.pages.len(), 1, "{:?}", out.pages);
}

#[test]
fn maketitle_forces_a_fresh_page_when_preceded_by_content() {
    let source = doc("\\title{T}\\author{A}", "Some intro text.\n\n\\maketitle");
    let out = compiled(&source);
    assert!(out.pages.len() >= 2, "{:?}", out.pages.len());
    let intro_page = out
        .pages
        .iter()
        .position(|p| p.items.iter().any(|i| i.text == "Some"))
        .unwrap();
    let title_page = out
        .pages
        .iter()
        .position(|p| p.items.iter().any(|i| i.text == "T"))
        .unwrap();
    assert!(title_page > intro_page);
}
