//! Section numbering, `\ref`/`\eqref`/`\pageref` resolution and
//! `\tableofcontents` (article.cls geometry).

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints, Session};
use flashtex_compiler::layout::{text_width, Font, TextItem, MARGIN_PT};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

fn items(output: &CompileOutput) -> Vec<&TextItem> {
    output.pages.iter().flat_map(|page| &page.items).collect()
}

fn slice<'a>(source: &'a str, item: &TextItem) -> &'a str {
    &source[item.span.start..item.span.end]
}

const BODY: f64 = 12.0;
const RIGHT: f64 = MARGIN_PT + 468.0;

#[test]
fn subsubsection_numbers_reset_and_starred_headings_stay_unnumbered() {
    let source = r"\section{A}\subsection{B}\subsubsection{C}\label{c}\subsection*{Star}\label{s}\subsubsection{D}\section{E}\subsubsection{F} See \ref{c} and \ref{s}.";
    let output = compile(source);
    let texts: Vec<&str> = items(&output)
        .iter()
        .map(|item| item.text.as_str())
        .collect();
    for expected in ["1", "1.1", "1.1.1", "1.1.2", "2", "2.0.1"] {
        assert!(texts.contains(&expected), "missing {expected}: {texts:?}");
    }
    // The starred heading does not step \@currentlabel: \ref{s} is still 1.1.1.
    assert_eq!(texts.iter().filter(|text| **text == "1.1.1").count(), 3);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn section_number_is_followed_by_a_quad() {
    let source = r"\section{Title}";
    let output = compile(source);
    let all = items(&output);
    let number = all.iter().find(|item| item.text == "1").unwrap();
    let title = all.iter().find(|item| item.text == "Title").unwrap();
    let size = number.font_size_pt;
    let expected = number.x_pt + text_width("1", size, Font::TimesBold) + size;
    assert!(
        (title.x_pt - expected).abs() < 0.02,
        "{} vs {expected}",
        title.x_pt
    );
}

#[test]
fn eqref_is_parenthesized_and_glued_like_ref() {
    let source = r"By \eqref{e} and (\ref{e}).\begin{equation}x\label{e}\end{equation}";
    let output = compile(source);
    let all = items(&output);
    let eqref = all
        .iter()
        .find(|item| slice(source, item) == r"\eqref{e}")
        .expect("eqref item");
    assert_eq!(eqref.text, "(1)");
    let open = all.iter().find(|item| item.text == "(").unwrap();
    let reference = all
        .iter()
        .find(|item| slice(source, item) == r"\ref{e}")
        .unwrap();
    assert_eq!(reference.text, "1");
    let glued = open.x_pt + text_width("(", BODY, Font::TimesRoman);
    assert!((reference.x_pt - glued).abs() < 0.02, "no gap after '('");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn undefined_references_render_bold_question_marks_and_warn_like_latex() {
    let source = r"See \ref{nope}, \eqref{gone} and \pageref{nope}.";
    let output = compile(source);
    let all = items(&output);
    let marks: Vec<_> = all.iter().filter(|item| item.text == "??").collect();
    assert_eq!(marks.len(), 3);
    assert!(marks.iter().all(|item| item.font == Font::TimesBold));
    let eqref: Vec<&str> = all
        .iter()
        .filter(|item| slice(source, item) == r"\eqref{gone}")
        .map(|item| item.text.as_str())
        .collect();
    assert_eq!(eqref, ["(", "??", ")"]);
    let warnings: Vec<_> = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .collect();
    assert_eq!(warnings.len(), 3);
    assert_eq!(warnings[0].message, "Reference `nope' on page 1 undefined");
    let span = warnings[1].span.expect("span");
    assert_eq!(&source[span.start..span.end], r"\eqref{gone}");
}

#[test]
fn table_of_contents_follows_article_cls_geometry() {
    let source = r"\tableofcontents \section{Intro}\subsection{Detail}\section*{Unnumbered}\subsubsection{Deep}\section{End}";
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let all = items(&output);
    let toc_span = |item: &&&TextItem| slice(source, item) == r"\tableofcontents";

    let contents = all.iter().find(|item| item.text == "Contents").unwrap();
    assert!(toc_span(&contents));
    assert_eq!(contents.font, Font::TimesBold);
    // Starred headings never reach the contents.
    assert_eq!(
        all.iter().filter(|item| item.text == "Unnumbered").count(),
        1
    );

    // First occurrence of each title is its contents line.
    let line_of = |title: &str| {
        let title = all.iter().find(|item| item.text == title).unwrap();
        let line: Vec<&TextItem> = all
            .iter()
            .copied()
            .filter(|item| item.baseline_y_pt == title.baseline_y_pt)
            .collect();
        (*title, line)
    };

    let (intro, line) = line_of("Intro");
    assert_eq!(intro.font, Font::TimesBold);
    assert_eq!(intro.x_pt, MARGIN_PT + 1.5 * BODY);
    assert_eq!(slice(source, intro), "Intro");
    assert_eq!(line[0].text, "1");
    assert_eq!(slice(source, line[0]), r"\section");
    assert!(line.iter().all(|item| item.text != "."), "no leaders");
    let page = line.last().unwrap();
    assert_eq!(page.text, "1");
    assert_eq!(page.font, Font::TimesBold);
    let page_right = page.x_pt + text_width("1", BODY, Font::TimesBold);
    assert!((page_right - RIGHT).abs() < 0.02);

    let (detail, line) = line_of("Detail");
    assert_eq!(detail.font, Font::TimesRoman);
    assert_eq!(line[0].text, "1.1");
    assert_eq!(line[0].x_pt, MARGIN_PT + 1.5 * BODY);
    assert_eq!(detail.x_pt, MARGIN_PT + 3.8 * BODY);
    let dots: Vec<_> = line.iter().filter(|item| item.text == ".").collect();
    assert!(dots.len() > 30, "dotted leaders: {}", dots.len());
    assert!(dots.iter().all(toc_span));
    // Aligned \leaders: every box starts at a multiple of its width.
    let dot_box = 9.0 * BODY / 18.0 + text_width(".", BODY, Font::TimesRoman);
    for dot in &dots {
        let slot = (dot.x_pt - 4.5 * BODY / 18.0 - MARGIN_PT) / dot_box;
        assert!(
            (slot - slot.round()).abs() < 0.01,
            "unaligned dot at {}",
            dot.x_pt
        );
    }
    let last_dot = dots.last().unwrap();
    assert!(last_dot.x_pt + dot_box <= RIGHT - 1.55 * BODY + 4.5 * BODY / 18.0 + 0.01);
    let first_dot_box = dots[0].x_pt - 4.5 * BODY / 18.0;
    let title_end = detail.x_pt + text_width("Detail", BODY, Font::TimesRoman);
    assert!(first_dot_box >= title_end - 0.01 && first_dot_box < title_end + dot_box);

    let (deep, line) = line_of("Deep");
    assert_eq!(line[0].text, "1.1.1");
    assert_eq!(line[0].x_pt, MARGIN_PT + 3.8 * BODY);
    assert_eq!(deep.x_pt, MARGIN_PT + 7.0 * BODY);

    // Section entries after the first get \addvspace{1em} on top of a line.
    let (_, end_line) = line_of("End");
    let end = end_line[0];
    assert_eq!(end.text, "2");
    assert!((end.baseline_y_pt - deep.baseline_y_pt - (BODY * 1.2 + BODY)).abs() < 0.02);
}

#[test]
fn contents_page_numbers_converge_after_the_contents_move_headings() {
    let filler = "word ".repeat(700);
    let source = format!(
        r"\tableofcontents {filler}\section{{Late}}\label{{late}} {filler}\subsection{{Later}} On page \pageref{{late}}."
    );
    let output = compile(&source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let all = items(&output);
    let page_of = |text: &str, nth: usize| {
        output
            .pages
            .iter()
            .flat_map(|page| page.items.iter().map(move |item| (page.number, item)))
            .filter(|(_, item)| item.text == text)
            .nth(nth)
            .map(|(number, _)| number)
            .unwrap()
    };
    let late_page = page_of("Late", 1);
    let later_page = page_of("Later", 1);
    assert!(later_page > late_page);
    let line_page = |title: &str| {
        let entry = all.iter().find(|item| item.text == title).unwrap();
        all.iter()
            .rfind(|item| item.baseline_y_pt == entry.baseline_y_pt && item.text != ".")
            .unwrap()
            .text
            .clone()
    };
    assert_eq!(line_page("Late"), late_page.to_string());
    assert_eq!(line_page("Later"), later_page.to_string());
    let pageref = all
        .iter()
        .find(|item| slice(&source, item) == r"\pageref{late}")
        .unwrap();
    assert_eq!(pageref.text, late_page.to_string());
}

#[test]
fn incremental_edits_to_headings_match_a_fresh_compile() {
    let constraints = LayoutConstraints::default();
    let revisions = [
        r"\tableofcontents \section{One}\label{one} Text \ref{two}.\section{Two}\label{two}",
        r"\tableofcontents \section{One renamed}\label{one} Text \ref{two}.\section{Two}\label{two}",
        r"\tableofcontents \section{Zero}\section{One renamed}\label{one} Text \ref{two}.\subsection{Two}\label{two}",
        r"\section{Zero}\section{One renamed}\label{one} Text \eqref{two}.\subsection{Two}\label{two}",
    ];
    let mut session = Session::new();
    for revision in revisions {
        let incremental = session.compile(revision, constraints);
        let fresh = compile_full(revision, constraints);
        assert_eq!(
            format!("{:?}", incremental.output),
            format!("{fresh:?}"),
            "revision {revision}"
        );
    }
    let last = compile_full(revisions[2], constraints);
    let texts: Vec<&str> = items(&last).iter().map(|item| item.text.as_str()).collect();
    assert!(texts.contains(&"2.1"));
    assert_eq!(texts.iter().filter(|text| **text == "renamed").count(), 2);
}

#[test]
fn refstepcounter_on_a_newcounter_feeds_the_following_label() {
    let source =
        r"\newcounter{myc}\refstepcounter{myc}\label{mylab}Value \arabic{myc}, ref \ref{mylab}.";
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let all = items(&output);
    let reference = all
        .iter()
        .find(|item| slice(source, item) == r"\ref{mylab}")
        .expect("ref item");
    assert_eq!(reference.text, "1");
    let rendered: Vec<&str> = all.iter().map(|item| item.text.as_str()).collect();
    assert!(rendered.contains(&"Value"), "{rendered:?}");
    assert!(rendered.contains(&"1"), "{rendered:?}");
}

#[test]
fn refstepcounter_label_uses_the_counter_representation() {
    let source = r"\newcounter{myc}\renewcommand{\themyc}{M-\arabic{myc}}\refstepcounter{myc}\refstepcounter{myc}\label{ml}See \ref{ml}.";
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let all = items(&output);
    let reference = all
        .iter()
        .find(|item| slice(source, item) == r"\ref{ml}")
        .expect("ref item");
    assert_eq!(reference.text, "M-2");
}

#[test]
fn section_labels_still_win_over_an_earlier_bare_refstepcounter() {
    let source =
        r"\newcounter{myc}\refstepcounter{myc}\refstepcounter{myc}\section{Title}\label{s}See \ref{s}.";
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let all = items(&output);
    let reference = all
        .iter()
        .find(|item| slice(source, item) == r"\ref{s}")
        .expect("ref item");
    assert_eq!(reference.text, "1");
}

#[test]
fn each_label_reads_the_most_recent_refstepcounter_in_source_order() {
    let source = r"\newcounter{ca}\newcounter{cb}\refstepcounter{ca}\label{la}\refstepcounter{cb}\refstepcounter{cb}\label{lb}A \ref{la} B \ref{lb}.";
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let all = items(&output);
    let first = all
        .iter()
        .find(|item| slice(source, item) == r"\ref{la}")
        .expect("first ref item");
    let second = all
        .iter()
        .find(|item| slice(source, item) == r"\ref{lb}")
        .expect("second ref item");
    assert_eq!(first.text, "1");
    assert_eq!(second.text, "2");
}
