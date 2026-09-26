//! exam.cls question lists: `questions`/`parts`/`subparts` with `\question`,
//! `\part` and `\subpart`.
//!
//! Oracle: pdflatex (TeX Live 2026) on the repro below, word boxes from
//! `pdftotext -bbox` (full command and output in the check-in). Page text
//! block starts at x = 72bp; all words CMR10:
//!
//! ```text
//! '1.'      xMin=76.981000                      (question label)
//! '(10'     xMin=89.711210  'First' xMin=142.344622
//! '(a)'     xMin=93.032000  '(2' xMin=110.743510  'Part' xMin=158.395622
//! '(b)'     xMin=92.479000  '(3' xMin=110.744431
//! 'i.'      xMin=118.769000 'Sub' xMin=129.285521
//! 'ii.'     xMin=116.002000 'Sub' xMin=129.286131
//! '2.'      xMin=76.981000  'Second' xMin=89.711210
//! ```
//!
//! Read off directly:
//!
//! * questions is a list labelled `\thequestion.` (`1.`, `2.`), right-aligned;
//!   its body starts at 89.711210bp, i.e. leftmargin 17.711210bp.
//! * `\question[10]` prints `(10 points)` plus an interword space at the
//!   start of the item body (no `\pointsinmargin`); `(1 point)` is singular.
//! * parts is a nested list labelled `(\alph{partno})` (`(a)`, `(b)`),
//!   right-aligned (`\hss\llap`); its body starts at 110.743510bp, i.e. an
//!   own leftmargin share of 21.032300bp (`(m)` + `\labelsep`).
//! * `\part[2]`/`\part[3]` print `(2 points)`/`(3 points)` the same way.
//! * subparts is labelled `\roman{subpart}.` (`i.`, `ii.`); its body starts
//!   at 129.285521bp, i.e. an own share of 18.542011bp (`vii.` + `\labelsep`).
//! * the label right edge sits one `\labelsep` before the body:
//!   89.711210 - 84.729910 = 4.981300bp = 5pt (`\labelsep` is `.5em` at the
//!   class default 10pt; article.cls line 338).
//!
//! This compiler lays out in TeX points on a 72pt margin, so a measured
//! `m`bp compares as `m * 72.27 / 72`pt against `x_pt - 72.0`, with a 0.1pt
//! tolerance (0.1pt = 0.0996bp, inside the 0.1bp budget).

use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{self, Block, Inline, ListLeftMargin};

/// The supervisor's repro, verbatim.
const SOURCE: &str = "\\documentclass{exam}\\begin{document}\\begin{questions}\\question[10] First question text.\\begin{parts}\\part[2] Part a text.\\part[3] Part b text.\\begin{subparts}\\subpart Sub one.\\subpart Sub two.\\end{subparts}\\end{parts}\\question Second question.\\end{questions}\\end{document}";

/// bp to TeX points (`1bp = 72.27/72 pt`, the `scan_dimen` rule the parser
/// itself documents for `parse_dimen_pt`).
fn bp(bp: f64) -> f64 {
    bp * 72.27 / 72.0
}

fn body_text(content: &[Inline]) -> String {
    content
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn list_items(source: &str) -> Vec<(Option<String>, String, ListLeftMargin, Option<f64>)> {
    parser::parse(source)
        .blocks
        .into_iter()
        .filter_map(|block| match block {
            Block::ListItem {
                label,
                content,
                leftmargin,
                labelsep_pt,
                ..
            } => Some((
                label.map(|(text, _)| text),
                body_text(&content),
                leftmargin,
                labelsep_pt,
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn exam_lists_parse_with_pdflatex_labels_and_points() {
    let parsed = parser::parse(SOURCE);
    assert!(
        parsed.diagnostics.is_empty(),
        "the repro must compile cleanly, got: {:#?}",
        parsed.diagnostics
    );
    let items = list_items(SOURCE);
    let labels: Vec<&str> = items
        .iter()
        .map(|(label, _, _, _)| label.as_deref().unwrap_or("<none>"))
        .collect();
    assert_eq!(labels, ["1.", "(a)", "(b)", "i.", "ii.", "2."]);
    let bodies: Vec<&str> = items.iter().map(|(_, body, _, _)| body.as_str()).collect();
    assert_eq!(
        bodies,
        [
            "(10 points) First question text.",
            "(2 points) Part a text.",
            "(3 points) Part b text.",
            "Sub one.",
            "Sub two.",
            "Second question.",
        ]
    );
}

#[test]
fn exam_points_are_followed_by_an_interword_space() {
    let parsed = parser::parse(SOURCE);
    let first = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::ListItem { content, .. } => Some(content),
            _ => None,
        })
        .expect("a first exam item");
    let texts: Vec<(&str, bool)> = first
        .iter()
        .filter_map(|i| match i {
            Inline::Text {
                text, space_before, ..
            } => Some((text.as_str(), *space_before)),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts[0],
        ("(10 points)", false),
        "the points block starts the body: {texts:?}"
    );
    // The class puts `\enspace` after the points, unconditionally.
    assert!(
        texts[1].1,
        "the body word after the points is spaced: {texts:?}"
    );
}

#[test]
fn exam_list_body_x_matches_pdflatex() {
    let out = compile_full(SOURCE, LayoutConstraints::default());
    assert!(
        out.diagnostics.is_empty(),
        "the repro must compile cleanly, got: {:#?}",
        out.diagnostics
    );
    let words: Vec<(&str, f64)> = out
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| (item.text.as_str(), item.x_pt - 72.0))
        .collect();
    let first_x = |word: &str, occurrence: usize| {
        words
            .iter()
            .filter(|(text, _)| *text == word)
            .map(|(_, x)| *x)
            .nth(occurrence)
            .unwrap_or_else(|| {
                panic!("word {word:?} (occurrence {occurrence}) not laid out in {words:#?}")
            })
    };
    // Body starts, measured in bp from the 72bp text edge. (This layout
    // keeps one inline run together, so the points block is a single word.)
    let cases = [
        ("(10 points)", 0, 89.711210 - 72.0),
        ("(2 points)", 0, 110.743510 - 72.0),
        ("(3 points)", 0, 110.744431 - 72.0),
        ("Sub", 0, 129.285521 - 72.0),
        ("Sub", 1, 129.286131 - 72.0),
        ("Second", 0, 89.711210 - 72.0),
    ];
    for (word, occurrence, want_bp) in cases {
        let got = first_x(word, occurrence);
        let want = bp(want_bp);
        assert!(
            (got - want).abs() < 0.1,
            "{word:?} body x: got {got:.4}pt, want {want:.4}pt ({want_bp}bp from pdflatex)"
        );
    }
}

#[test]
fn exam_lists_carry_explicit_margins_and_exam_labelsep() {
    let items = list_items(SOURCE);
    assert_eq!(items.len(), 6, "the repro has six exam items");
    for (_, _, leftmargin, labelsep_pt) in items {
        assert!(
            matches!(leftmargin, ListLeftMargin::Explicit(_)),
            "exam lists pin their measured margins, got {leftmargin:?}"
        );
        // 5pt: `\labelsep` is `.5em` at the class default 10pt, exactly the
        // 4.9813bp the oracle puts between each label and its body.
        assert_eq!(labelsep_pt, Some(5.0), "exam labelsep is the class 5pt");
    }
}

#[test]
fn question_points_are_singular_for_one_point() {
    let source = "\\documentclass{exam}\\begin{document}\\begin{questions}\\question[1] Solo.\\end{questions}\\end{document}";
    let parsed = parser::parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "got: {:#?}",
        parsed.diagnostics
    );
    let items = list_items(source);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].0.as_deref(), Some("1."));
    assert_eq!(items[0].1, "(1 point) Solo.");
}

#[test]
fn part_in_article_keeps_its_sectioning_meaning_and_question_stays_undefined() {
    // `\part` is the kernel sectioning command this compiler does not
    // implement: in `article` it must keep reporting exactly that (and keep
    // the title on the page), never become an exam item.
    let parsed =
        parser::parse("\\documentclass{article}\\begin{document}\\part{Grand part}\\end{document}");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\part")),
        "expected a \\part diagnostic, got: {:#?}",
        parsed.diagnostics
    );
    let text: String = parsed
        .blocks
        .iter()
        .flat_map(|block| match block {
            Block::Paragraph(content) => content.clone(),
            _ => Vec::new(),
        })
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text.contains("Grand part"),
        "the part title must stay on the page, got blocks: {:#?}",
        parsed.blocks
    );
    // `\question` is defined by exam.cls alone: under `article` it stays
    // undefined, like pdflatex's "Undefined control sequence".
    let parsed = parser::parse(
        "\\documentclass{article}\\begin{document}\\begin{questions}\\question Q.\\end{questions}\\end{document}",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\question") || d.message.contains("questions")),
        "expected a \\question/questions diagnostic, got: {:#?}",
        parsed.diagnostics
    );
    assert!(
        !parsed.blocks.iter().any(|block| matches!(
            block,
            Block::ListItem { label: Some((label, _)), .. } if label == "1."
        )),
        "no exam item may be produced under article: {:#?}",
        parsed.blocks
    );
}
