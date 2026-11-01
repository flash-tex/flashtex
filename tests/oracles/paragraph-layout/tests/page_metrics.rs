//! Golden page/baseline metrics with geometry from `flashtex-document-style`.

use flashtex_document_style::{BaseSize, ClassOptions, Geometry, Paper, Pt};
use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::runtime_v1::{compile_result_json, words};
use flashtex_paragraph_layout::style::{ArticleLayout, heading_block};
use flashtex_paragraph_layout::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn article() -> ArticleLayout {
    ArticleLayout::new(
        ClassOptions {
            paper: Paper::Letter,
            size: BaseSize::Pt12,
        },
        None,
    )
}

/// A paragraph of `n` one-word lines via forced breaks, Times 12pt.
fn lines(n: usize) -> Lines {
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    for i in 0..n {
        b.word(&Core14Times::ROMAN, 12.0, "line", i * 5).unwrap();
        if i + 1 < n {
            b.line_break();
        }
    }
    layout_paragraph(&b.finish(Glue::fil()), &article().line).unwrap()
}

/// `[12pt]{article}` on Letter as document-style computes it (article.cls
/// and size12.clo): text area origin (111.27, 126.27) pt, 390 x 548.5 pt,
/// `\topskip` 12, `\maxdepth` 6, `\parskip` 0pt plus 1pt, `\baselineskip` 14.5,
/// `\parindent` 1.5em = 17.62482pt, justified.
#[test]
fn article_12pt_letter_page_params_come_from_document_style() {
    let a = article();
    assert!(close(a.page.page_width, 614.295));
    assert!(close(a.page.page_height, 794.97));
    assert!(close(a.page.margin_left, 111.27));
    assert!(close(a.page.margin_top, 126.27));
    assert!(close(a.page.text_width(), 390.0));
    assert!(close(a.page.text_height(), 548.5));
    assert!(close(a.page.topskip, 12.0));
    assert!(close(a.page.max_depth, 6.0));
    assert_eq!(a.page.parskip, Glue::finite(0.0, 1.0, 0.0));
    assert!(close(a.page.baselineskip, 14.5));
    assert!(close(a.line.line_width, 390.0));
    assert!((a.line.parindent - 17.62482).abs() < 1e-4);
    assert_eq!(a.line.mode, BreakMode::Justified);
    // geometry margin=1in overrides the text area only.
    let g = ArticleLayout::new(
        ClassOptions {
            paper: Paper::Letter,
            size: BaseSize::Pt12,
        },
        Some(Geometry::margin(Pt::inches(1.0))),
    );
    assert!(close(g.page.margin_left, 72.27));
    assert!(close(g.page.margin_top, 72.27));
    assert!(close(g.page.text_width(), 469.755));
    assert!(close(g.page.text_height(), 650.43));
    assert!(close(g.line.line_width, 469.755));
}

/// Two pages of one-line paragraphs: 38 baselines fit (topskip line + 37 x
/// 14.5 = 548.5), so 40 lines break after line 38. First baseline 138.27pt,
/// last on page 1 674.77pt, page 2 restarts at 138.27pt.
#[test]
fn two_page_document_baselines_and_break_line() {
    let a = article();
    let blocks: Vec<ParagraphBlock> = (0..40).map(|_| ParagraphBlock::body(lines(1))).collect();
    let pages = layout_pages(&blocks, &a.page);
    assert_eq!(pages.pages.len(), 2);
    assert!(pages.overflow.is_empty());
    let p1 = &pages.pages[0];
    assert_eq!(p1.lines.len(), 38);
    assert!(close(p1.lines[0].baseline_y, 138.27));
    assert!(close(p1.lines[37].baseline_y, 138.27 + 37.0 * 14.5));
    assert_eq!(p1.lines[37].paragraph, 37);
    let p2 = &pages.pages[1];
    assert_eq!(p2.lines.len(), 2);
    assert!(close(p2.lines[0].baseline_y, 138.27));
    assert_eq!(p2.lines[0].paragraph, 38);
    // A 40-line single paragraph breaks at the same line (38 + 2 satisfies
    // the widow rule); 39 lines would leave a widow, so one line is pulled
    // back: 37 + 2.
    let pages = layout_pages(&[ParagraphBlock::body(lines(40))], &a.page);
    assert_eq!(
        (pages.pages[0].lines.len(), pages.pages[1].lines.len()),
        (38, 2)
    );
    let pages = layout_pages(&[ParagraphBlock::body(lines(39))], &a.page);
    assert_eq!(
        (pages.pages[0].lines.len(), pages.pages[1].lines.len()),
        (37, 2)
    );
}

/// `\section` spacing from document-style's `section_spec` evaluated in the
/// body font's ex (Times 12pt: 5.4pt): before 3.5ex = 18.9pt (discarded at
/// the page top), after 2.3ex = 12.42pt, heading kept with the next block.
#[test]
fn section_heading_block_uses_class_spacing() {
    let a = article();
    let heading = heading_block(1, 5.4, lines(1)).unwrap();
    assert!(close(heading.space_before.width, 18.9) && close(heading.space_before.stretch, 5.4));
    assert!(close(heading.space_before.shrink, 1.08));
    assert!(close(heading.space_after.width, 12.42) && close(heading.space_after.stretch, 1.08));
    assert!(heading.keep_with_next);
    let pages = layout_pages(&[heading, ParagraphBlock::body(lines(2))], &a.page);
    let ys: Vec<f64> = pages.pages[0].lines.iter().map(|l| l.baseline_y).collect();
    // Heading at topskip; body line = 138.27 + 12.42 (afterskip) + 14.5.
    assert!(close(ys[0], 138.27));
    assert!(close(ys[1], 138.27 + 12.42 + 14.5));
    assert!(close(ys[2], ys[1] + 14.5));
}

/// runtime-v1 emission: one item per word (source-contiguous fragments and a
/// discretionary hyphen merge), PDF-point coordinates, exact spans.
#[test]
fn runtime_v1_items_are_words_with_exact_spans() {
    let text = "AVAVAV re\\-pro";
    let h = ExplicitDiscretionary;
    let mut b = ParagraphBuilder::new(&h);
    b.text(&Core14Times::ROMAN, 12.0, text, 0).unwrap();
    let a = article();
    // No \parindent here: "AVAVAV re-" is 43.88 + 3 + 13.32 = 60.2pt, shrunk
    // 0.2pt over 0.72pt of shrink (badness 2) onto a 60pt measure; "pro" last.
    let params = LineBreakParams {
        parindent: 0.0,
        ..a.line.clone().with_width(60.0)
    };
    let lines = layout_paragraph(&b.finish(Glue::fil()), &params).unwrap();
    assert_eq!(
        lines.lines.len(),
        2,
        "re-|pro must hyphenate on a 60pt measure"
    );
    assert_eq!(lines.lines[0].badness, 2.0);
    let pages = layout_pages(&[ParagraphBlock::body(lines)], &a.page);
    let w = words(&pages.pages[0], text);
    let texts: Vec<&str> = w.iter().map(|w| w.text.as_str()).collect();
    assert_eq!(texts, vec!["AVAVAV", "re-", "pro"]);
    assert_eq!(w[0].source, 0..6);
    assert_eq!(w[1].source, 7..11, "\"re\" plus the `\\-` marker bytes");
    assert_eq!(w[2].source, 11..14);
    let json = compile_result_json(&pages, text, "doc.tex", "p", 7, BP_PER_TEX_PT);
    assert!(json.starts_with("{\"protocol_version\":1,\"id\":\"p-7\",\"type\":\"compile_result\""));
    assert!(json.contains("\"width_pt\":612,\"height_pt\":792"));
    // First word: x = 111.27pt * 72/72.27 = 110.8543 bp; baseline 138.27pt = 137.7534 bp.
    assert!(json.contains("\"text\":\"AVAVAV\",\"x_pt\":110.8543,\"baseline_y_pt\":137.7534,\"font_size_pt\":11.9552,\"source\":{\"path\":\"doc.tex\",\"start_byte\":0,\"end_byte\":6}"), "{json}");
}
