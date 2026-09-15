//! Footnote numbering beyond the running `footnote` counter: `\thanks`
//! (`\@fnsymbol` marks, `\setcounter{footnote}{0}` after `\maketitle`),
//! report/book `\chapter` (`\@addtoreset{footnote}{chapter}`, and figure,
//! equation and section numbers within the chapter) and `minipage`
//! (`mpfootnote`, `\alph`).

use flashtex_compiler::parser::{parse, Block, Inline};

fn notes(inlines: &[Inline], out: &mut Vec<(String, bool, bool)>) {
    for inline in inlines {
        if let Inline::Footnote {
            number, mark, text, ..
        } = inline
        {
            out.push((number.clone(), *mark, text.is_some()));
        }
    }
}

fn all_notes(source: &str) -> Vec<(String, bool, bool)> {
    let parsed = parse(source);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        match block {
            Block::Paragraph(i)
            | Block::Styled { content: i, .. }
            | Block::ListItem { content: i, .. } => notes(i, &mut out),
            Block::TitleBlock {
                title,
                authors,
                date,
            } => {
                notes(title, &mut out);
                notes(authors, &mut out);
                notes(date.as_deref().unwrap_or(&[]), &mut out);
            }
            _ => {}
        }
    }
    out
}

fn numbers(source: &str) -> Vec<String> {
    all_notes(source).into_iter().map(|n| n.0).collect()
}

#[test]
fn thanks_marks_follow_fnsymbol_in_title_author_date_order_then_the_counter_restarts() {
    let source = "\\documentclass{article}\\title{T\\thanks{a}}\\author{A\\thanks{b} \\and B\\thanks{c}}\\date{D\\thanks{d}}\\begin{document}\\maketitle Body\\footnote{e} more.\\end{document}";
    let parsed = parse(source);
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("thanks")),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(
        numbers(source),
        ["\u{2217}", "\u{2020}", "\u{2021}", "\u{a7}", "1"]
    );
    assert!(all_notes(source).iter().all(|n| n.1 && n.2));
}

#[test]
fn a_tenth_thanks_is_diagnosed_and_printed_in_arabic() {
    let thanks: String = (0..10).map(|i| format!("\\thanks{{n{i}}}")).collect();
    let source = format!("\\documentclass{{article}}\\title{{T{thanks}}}\\author{{A}}\\begin{{document}}\\maketitle\\end{{document}}");
    let parsed = parse(&source);
    assert_eq!(numbers(&source).last().map(String::as_str), Some("10"));
    assert_eq!(numbers(&source)[6], "\u{2217}\u{2217}");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("@fnsymbol")),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn report_chapter_resets_footnote_figure_and_equation_counters() {
    let source = "\\documentclass{report}\\begin{document}\\chapter{One}a\\footnote{x} b\\footnote{y}\\begin{equation}x\\label{e1}\\end{equation}\\section{S}\\label{s1}\\chapter*{Pre}c\\footnote{z}\\chapter[Short]{Two}d\\footnote{w}\\begin{equation}y\\label{e2}\\end{equation} \\ref{e1} \\ref{e2} \\ref{s1}\\end{document}";
    let parsed = parse(source);
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\chapter")),
        "{:?}",
        parsed.diagnostics
    );
    // `\chapter*` does not step `chapter`, so it resets nothing.
    assert_eq!(numbers(source), ["1", "2", "3", "1"]);
    let text = format!("{:?}", parsed.blocks);
    for value in ["\"1.1\"", "\"2.1\""] {
        assert!(text.contains(value), "{value} missing: {text}");
    }
    // The chapter title stays in the text as a paragraph (the head's layout
    // belongs to the renderer), never its `[short]` argument.
    assert!(text.contains("\"Two\""), "{text}");
    assert!(!text.contains("Short"), "{text}");
}

#[test]
fn book_numbers_figures_within_the_chapter() {
    let source = "\\documentclass{book}\\begin{document}\\chapter{A}\\begin{figure}\\caption{c}\\end{figure}\\chapter{B}\\begin{figure}\\caption{d}\\end{figure}\\end{document}";
    let text = format!("{:?}", parse(source).blocks);
    assert!(
        text.contains("Figure 1.1:") && text.contains("Figure 2.1:"),
        "{text}"
    );
}

#[test]
fn article_has_no_chapter_and_never_resets() {
    let source = "\\documentclass{article}\\begin{document}a\\footnote{x}\\chapter{C}b\\footnote{y}\\end{document}";
    let parsed = parse(source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\chapter")),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(numbers(source), ["1", "2"]);
}

#[test]
fn minipage_footnotes_use_alph_mpfootnote_and_leave_the_footnote_counter() {
    let source = "\\documentclass{article}\\begin{document}a\\footnote{x}\\begin{minipage}{3cm}b\\footnote{y} c\\footnote{z} d\\footnotemark{} e\\footnotetext{t}\\end{minipage}\\begin{minipage}{3cm}f\\footnote[3]{u}g\\footnote{v}\\end{minipage} h\\footnote{w}\\end{document}";
    assert_eq!(numbers(source), ["1", "a", "b", "2", "b", "c", "a", "3"]);
}

fn note_bodies(source: &str) -> Vec<String> {
    let parsed = parse(source);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let inlines: &[Inline] = match block {
            Block::Paragraph(i) | Block::Styled { content: i, .. } | Block::ListItem { content: i, .. } => i,
            _ => continue,
        };
        for inline in inlines {
            if let Inline::Footnote { text: Some(body), .. } = inline {
                let mut text = String::new();
                for inner in body {
                    if let Inline::Text { text: piece, .. } = inner {
                        text.push_str(piece);
                    }
                }
                out.push(text);
            }
        }
    }
    out
}

#[test]
fn fnsymbol_of_footnote_renders_each_notes_own_mark() {
    let source = "\\documentclass{article}\\begin{document}a\\footnote{first \\fnsymbol{footnote} mark}b\\footnote{second \\fnsymbol{footnote} mark}\\end{document}";
    let parsed = parse(source);
    assert!(
        !parsed.diagnostics.iter().any(|d| d.message.contains("No counter")
            || d.message.contains("too large")
            || d.message.contains("nine symbols")),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(numbers(source), ["1", "2"]);
    assert_eq!(note_bodies(source), ["first∗mark", "second†mark"]);
}

#[test]
fn fnsymbol_reports_unknown_counters_and_out_of_range_values() {
    let source = "\\documentclass{article}\\begin{document}\\fnsymbol{nosuchcounter} \\fnsymbol{footnote}\\end{document}";
    let parsed = parse(source);
    let messages: Vec<&str> = parsed.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages.iter().any(|m| m.contains("No counter 'nosuchcounter' defined")),
        "{messages:?}"
    );
    // No footnote has stepped the counter yet, so its value is 0: an
    // honest out-of-range error, not a "not defined" misdiagnosis.
    assert!(
        messages.iter().any(|m| m.contains("nine symbols")),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("No counter 'footnote'")),
        "{messages:?}"
    );
}
