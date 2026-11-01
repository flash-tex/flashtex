//! FT-064: line breaks with Liang hyphenation compared against pdflatex.
//!
//! Oracle data: `tests/oracle/hyphenation_corpus.txt`, produced by
//! `tests/oracle/gen_hyphenation_oracle.py` from pdfTeX 3.141592653-2.6-1.40.29
//! (TeX Live 2026): `[12pt]{article}`, T1 + `times` (ptmr8t), `geometry
//! margin=1in` (`\hsize` 469.75499pt), `\parindent 0pt`, justified, LaTeX's
//! default hyphenation (`english` = Knuth's hyphen.tex, hyphenmins 2/3,
//! `\hyphenpenalty`=`\exhyphenpenalty`=50, `\pretolerance` 100, `\tolerance`
//! 200). Each paragraph was typeset in a `\vbox` and its lines read back from
//! `\showbox`. TeX is only the oracle; nothing here runs it.

use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::*;

struct Para {
    text: String,
    lines: Vec<String>,
    /// pdflatex reported an Overfull \hbox for this paragraph.
    overfull: bool,
}

fn corpus() -> Vec<Para> {
    let raw = include_str!("oracle/hyphenation_corpus.txt");
    let mut out: Vec<Para> = Vec::new();
    for l in raw.lines() {
        if let Some(t) = l.strip_prefix("P\t") {
            out.push(Para {
                text: t.to_string(),
                lines: Vec::new(),
                overfull: false,
            });
        } else if let Some(t) = l.strip_prefix("L\t") {
            out.last_mut().expect("L after P").lines.push(t.to_string());
        } else if l.starts_with("O\t") {
            out.last_mut().expect("O after P").overfull = true;
        }
    }
    out
}

/// The text of each set line: source bytes of its glyph runs (explicit `\-`
/// markers removed, whitespace collapsed) plus a trailing `-` for a hyphen.
fn line_texts(text: &str, lines: &Lines) -> Vec<String> {
    lines
        .lines
        .iter()
        .map(|line| {
            let body: Vec<_> = line.runs.iter().filter(|r| !r.is_hyphen).collect();
            let mut s = match (body.first(), body.last()) {
                (Some(a), Some(b)) => text[a.source.start..b.source.end].replace("\\-", ""),
                _ => String::new(),
            };
            s = s.split_whitespace().collect::<Vec<_>>().join(" ");
            if line.runs.iter().any(|r| r.is_hyphen) {
                s.push('-');
            }
            s
        })
        .collect()
}

fn layout(text: &str, hyph: &dyn Hyphenator) -> Lines {
    let mut b = ParagraphBuilder::new(hyph);
    b.text(&Core14Times::ROMAN, 12.0, text, 0).unwrap();
    let items = b.finish(Glue::fil());
    layout_paragraph(&items, &LineBreakParams::article_12pt_letter_1in()).unwrap()
}

#[test]
fn thirty_paragraphs_break_like_pdflatex() {
    let corpus = corpus();
    assert_eq!(corpus.len(), 30);
    let hyph = LiangHyphenator::english();
    let mut mismatches = Vec::new();
    let (mut lines_total, mut hyphenated) = (0, 0);
    for (i, p) in corpus.iter().enumerate() {
        let lines = layout(&p.text, &hyph);
        let got = line_texts(&p.text, &lines);
        lines_total += p.lines.len();
        hyphenated += p.lines.iter().filter(|l| l.ends_with('-')).count();
        if got != p.lines || lines.stats.overfull.is_empty() == p.overfull {
            mismatches.push(format!(
                "paragraph {}:\n  pdflatex: {:#?}\n  flashtex: {:#?}\n  overfull: {:?}",
                i + 1,
                p.lines,
                got,
                lines.stats.overfull
            ));
        }
    }
    assert!(
        hyphenated >= 15,
        "corpus must exercise hyphenation ({hyphenated})"
    );
    assert!(
        mismatches.is_empty(),
        "{} of 30 paragraphs differ ({lines_total} oracle lines):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Without patterns the same corpus is set differently (proves the oracle
/// actually depends on hyphenation, not just on the breaker).
#[test]
fn corpus_needs_hyphenation() {
    let differ = corpus()
        .iter()
        .filter(|p| line_texts(&p.text, &layout(&p.text, &ExplicitDiscretionary)) != p.lines)
        .count();
    assert!(
        differ >= 10,
        "only {differ} paragraphs depend on hyphenation"
    );
}

/// TeX never hyphenates the first word of a paragraph (it does not follow glue).
#[test]
fn first_word_of_paragraph_is_not_hyphenated() {
    let hyph = LiangHyphenator::english();
    let mut b = ParagraphBuilder::new(&hyph);
    b.text(
        &Core14Times::ROMAN,
        12.0,
        "counterexample counterexample",
        0,
    )
    .unwrap();
    let pens: Vec<usize> = b
        .items()
        .iter()
        .enumerate()
        .filter(|(_, it)| matches!(it, Item::Penalty(p) if p.automatic))
        .map(|(i, _)| i)
        .collect();
    let glue = b
        .items()
        .iter()
        .position(|it| matches!(it, Item::Glue(_)))
        .unwrap();
    assert_eq!(pens.len(), 3, "only the second word gets coun-terex-am-ple");
    assert!(pens.iter().all(|&i| i > glue));
}

/// An explicit hyphen is an empty discretionary with `\exhyphenpenalty`,
/// and it suppresses pattern hyphenation of the whole word.
#[test]
fn explicit_hyphen_is_an_empty_discretionary() {
    let hyph = LiangHyphenator::english();
    let mut b = ParagraphBuilder::new(&hyph);
    b.ex_hyphen_penalty = 77;
    b.text(
        &Core14Times::ROMAN,
        12.0,
        "a counterexample-generating tool",
        0,
    )
    .unwrap();
    let pens: Vec<&Penalty> = b
        .items()
        .iter()
        .filter_map(|it| match it {
            Item::Penalty(p) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(pens.len(), 1);
    assert_eq!(pens[0].value, 77);
    assert!(pens[0].flagged && pens[0].pre_break.is_none() && !pens[0].automatic);
}

/// `\discretionary{pre}{post}{nobreak}`: when the line breaks there, `pre`
/// ends it, `post` starts the next one and `nobreak` disappears; otherwise
/// only `nobreak` is set.
#[test]
fn discretionary_pre_post_and_no_break() {
    let font = &Core14Times::ROMAN;
    let build = |b: &mut ParagraphBuilder| {
        b.text(font, 12.0, "xxxxbac", 0).unwrap();
        b.discretionary(font, 12.0, "k-", "k", "ck", 8..30);
        b.text(font, 12.0, "eryyyy", 30).unwrap();
    };
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    build(&mut b);
    let items = b.finish(Glue::fil());

    // Wide measure: one line, no-break text present, no pre/post.
    let mut params = LineBreakParams::article_12pt_letter_1in();
    let wide = layout_paragraph(&items, &params).unwrap();
    assert_eq!(wide.lines.len(), 1);
    let widths: f64 = wide.lines[0].runs.iter().map(|r| r.width).sum();
    let ck = shape_run(font, 12.0, "ck", 0).width;
    assert!(
        wide.lines[0]
            .runs
            .iter()
            .any(|r| (r.width - ck).abs() < 1e-9)
    );

    // No spaces: the discretionary is the only interior breakpoint. The
    // measure fits "xxxxbak-" but not the whole word; ragged-right keeps the
    // short lines' badness finite.
    params.mode = BreakMode::RaggedRight;
    params.line_width =
        shape_run(font, 12.0, "xxxxbac", 0).width + shape_run(font, 12.0, "k-", 0).width + 0.5;
    let narrow = layout_paragraph(&items, &params).unwrap();
    let hyph_line = narrow
        .lines
        .iter()
        .position(|l| l.hyphenated)
        .unwrap_or_else(|| {
            panic!(
                "breaks at the discretionary; width {} lines {:#?}",
                params.line_width,
                narrow
                    .lines
                    .iter()
                    .map(|l| (l.items.clone(), l.natural_width, l.runs.len()))
                    .collect::<Vec<_>>()
            )
        });
    let first = &narrow.lines[hyph_line];
    let next = &narrow.lines[hyph_line + 1];
    let k_dash = shape_run(font, 12.0, "k-", 0).width;
    let k = shape_run(font, 12.0, "k", 0).width;
    assert!((first.runs.last().unwrap().width - k_dash).abs() < 1e-9);
    assert!(first.runs.last().unwrap().is_hyphen);
    assert!(
        (next.runs[0].width - k).abs() < 1e-9,
        "post-break text starts the next line"
    );
    assert!(
        !narrow
            .lines
            .iter()
            .flat_map(|l| &l.runs)
            .any(|r| (r.width - ck).abs() < 1e-9),
        "no-break text is dropped at the break"
    );
    assert!(widths > 0.0);
}
