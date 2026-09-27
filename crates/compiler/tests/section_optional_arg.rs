//! `\section[short]{title}` and siblings: latex.ltx's `\@dblarg` reads an
//! optional `[short]` contents-line title before the `{title}`, and a
//! braced group inside the brackets (e.g. `\emph{t}`) does not end the
//! scan. The heading typesets the full title; the `.toc` line reads the
//! short one, defaulting to the full title.
//!
//! Oracle: pdflatex (TeX Live 2026), each file compiled twice with
//! `pdflatex -interaction=nonstopmode <file>.tex`, 0 `^! ` error lines:
//! - article `\section[Short \emph{t}]{Long title}` +
//!   `\subsection[Sub \emph{s}]{Long sub}` writes
//!   `\contentsline {section}{\numberline {1}Short \emph {t}}{1}{}%` and
//!   `\contentsline {subsection}{\numberline {1.1}Sub \emph {s}}{1}{}%`.
//! - report `\chapter[Short \emph{t}]{Long title}` + `\section[A]{B}`
//!   writes `\contentsline {chapter}{\numberline {1}Short \emph {t}}{2}{}%`
//!   and `\contentsline {section}{\numberline {1.1}A}{2}{}%`.
//! - `\section*[s]{Starred}` typesets `[s]` as body text before the head
//!   and writes no `.toc` line; `\section[]{Empty short}` writes a
//!   number-only line (`\numberline {1}` with an empty title).
//! - `\chapter{Nope}` under article fails with
//!   `! Undefined control sequence.`
use flashtex_compiler::parser::{parse, Block, Inline, Parsed};

fn errors(parsed: &Parsed) -> Vec<String> {
    parsed
        .diagnostics
        .iter()
        .filter(|d| d.severity.as_str() == "error")
        .map(|d| d.message.clone())
        .collect()
}

/// The plain text of text runs, in order: a run preceded by source
/// whitespace (`space_before`) or carrying interword glue (`glue_before`,
/// how a space before a style command like `\emph` is kept) is separated
/// from the previous one.
fn text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        if let Inline::Text { text, space_before, glue_before, .. } = inline {
            if (*space_before || glue_before.is_some()) && !out.is_empty() {
                out.push(' ');
            }
            out.push_str(text);
        }
    }
    out
}

fn headings(parsed: &Parsed) -> Vec<(u8, String, String)> {
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Heading { level, number, content, .. } => {
                Some((*level, number.clone(), text(content)))
            }
            _ => None,
        })
        .collect()
}

fn paragraphs(parsed: &Parsed) -> Vec<String> {
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(text(inlines)),
            _ => None,
        })
        .collect()
}

#[test]
fn section_short_title_with_braced_group_matches_pdflatex() {
    let parsed = parse(
        "\\documentclass{article}\\begin{document}\\section[Short \\emph{t}]{Long title}Text.\\end{document}",
    );
    assert_eq!(errors(&parsed), Vec::<String>::new());
    assert_eq!(headings(&parsed), vec![(1, "1".to_string(), "Long title".to_string())]);
    assert_eq!(parsed.toc_entries.len(), 1);
    let entry = &parsed.toc_entries[0];
    assert_eq!(entry.level, 1);
    assert_eq!(entry.number, "1");
    assert_eq!(text(&entry.title), "Long title");
    // The TOC title reads "Short t" with the emphasis intact, as in
    // pdflatex's `\numberline {1}Short \emph {t}`.
    assert_eq!(text(&entry.short_title), "Short t");
    assert!(
        entry.short_title.iter().any(|inline| matches!(
            inline,
            Inline::Text { text, style, .. } if text == "t" && style.italic
        )),
        "short title keeps \\emph{{t}} italic: {:?}",
        entry.short_title
    );
}

#[test]
fn subsection_short_title_with_braced_group_matches_pdflatex() {
    let parsed = parse(
        "\\documentclass{article}\\begin{document}\\section{T}\\subsection[Sub \\emph{s}]{Long sub}More.\\end{document}",
    );
    assert_eq!(errors(&parsed), Vec::<String>::new());
    assert_eq!(
        headings(&parsed),
        vec![
            (1, "1".to_string(), "T".to_string()),
            (2, "1.1".to_string(), "Long sub".to_string()),
        ]
    );
    assert_eq!(parsed.toc_entries.len(), 2);
    let entry = &parsed.toc_entries[1];
    assert_eq!(entry.level, 2);
    assert_eq!(entry.number, "1.1");
    assert_eq!(text(&entry.short_title), "Sub s");
    assert!(
        entry.short_title.iter().any(|inline| matches!(
            inline,
            Inline::Text { text, style, .. } if text == "s" && style.italic
        )),
        "short title keeps \\emph{{s}} italic: {:?}",
        entry.short_title
    );
}

#[test]
fn chapter_short_title_in_report_matches_pdflatex() {
    let parsed = parse(
        "\\documentclass{report}\\begin{document}\\tableofcontents\\chapter[Short \\emph{t}]{Long title}Text.\\section[A]{B}Body.\\end{document}",
    );
    assert_eq!(errors(&parsed), Vec::<String>::new());
    // `\chapter` lays its head out as a bold paragraph (unchanged); the
    // head still reads the full title.
    let body = paragraphs(&parsed);
    assert!(body.iter().any(|p| p == "Long title"), "{body:?}");
    assert_eq!(parsed.toc_entries.len(), 2);
    let chapter = &parsed.toc_entries[0];
    assert_eq!(chapter.level, 0);
    assert_eq!(chapter.number, "1");
    assert_eq!(text(&chapter.title), "Long title");
    assert_eq!(text(&chapter.short_title), "Short t");
    assert!(
        chapter.short_title.iter().any(|inline| matches!(
            inline,
            Inline::Text { text, style, .. } if text == "t" && style.italic
        )),
        "short title keeps \\emph{{t}} italic: {:?}",
        chapter.short_title
    );
    // The plain `[A]` short form is unchanged: head "B", TOC title "A",
    // as in pdflatex's `\numberline {1.1}A`.
    let section = &parsed.toc_entries[1];
    assert_eq!(section.level, 1);
    assert_eq!(section.number, "1.1");
    assert_eq!(text(&section.short_title), "A");
    assert_eq!(
        headings(&parsed),
        vec![(1, "1.1".to_string(), "B".to_string())]
    );
}

#[test]
fn section_without_short_title_defaults_to_the_full_title() {
    let parsed =
        parse("\\documentclass{article}\\begin{document}\\section{Tight}End.\\end{document}");
    assert_eq!(errors(&parsed), Vec::<String>::new());
    assert_eq!(headings(&parsed), vec![(1, "1".to_string(), "Tight".to_string())]);
    assert_eq!(parsed.toc_entries.len(), 1);
    assert_eq!(text(&parsed.toc_entries[0].short_title), "Tight");
}

#[test]
fn empty_short_title_stays_empty_like_pdflatex() {
    let parsed = parse(
        "\\documentclass{article}\\begin{document}\\section[]{Empty short}Body.\\end{document}",
    );
    assert_eq!(errors(&parsed), Vec::<String>::new());
    assert_eq!(
        headings(&parsed),
        vec![(1, "1".to_string(), "Empty short".to_string())]
    );
    // pdflatex writes a number-only `.toc` line for `\section[]`.
    assert_eq!(parsed.toc_entries.len(), 1);
    assert!(parsed.toc_entries[0].short_title.is_empty());
}

#[test]
fn starred_section_writes_no_toc_line_like_pdflatex() {
    let parsed = parse(
        "\\documentclass{article}\\begin{document}\\section*{Starred}After.\\end{document}",
    );
    assert_eq!(errors(&parsed), Vec::<String>::new());
    assert_eq!(headings(&parsed).len(), 1);
    // The starred form writes no `.toc` line in LaTeX.
    assert!(parsed.toc_entries.is_empty());
}

#[test]
fn chapter_is_rejected_under_article_like_pdflatex() {
    // pdflatex: `! Undefined control sequence.` — `\chapter` only exists
    // under report/book, so it must stay an error here too.
    let parsed =
        parse("\\documentclass{article}\\begin{document}\\chapter{Nope}Text.\\end{document}");
    let problems = errors(&parsed);
    assert!(!problems.is_empty(), "expected an error for \\chapter");
    assert!(problems.iter().any(|m| m.contains("\\chapter")), "{problems:?}");
    assert!(parsed.toc_entries.is_empty());
}
