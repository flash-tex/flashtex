//! Issue #843: a body whose only content is non-glyph horizontal material
//! must still set a line. pdflatex (TeX Live 2026, `article`) ships a
//! one-page PDF for `\hspace{1cm}`, `\hfill` and `\strut` alone, while a
//! genuinely empty or comment-only body ships nothing. FlashTeX dropped
//! these paragraphs outright (no box, no line, no page: `display list has
//! no pages`). The parser now starts such paragraphs with a zero-size rule
//! anchor, and `\strut` is implemented as a zero-width strut rule, so both
//! reach the line builder as a box that paints nothing.
//!
//! Oracle (preamble `\documentclass{article}`, `pdfinfo` page counts):
//! `\hspace{1cm}`, `\hfill`, `\strut`, `\hspace{0pt}`, `\mbox{}`,
//! `\hrule`, `\quad`, `\enskip`, `\hfil`, `\hskip1cm`, `\,`, `~`,
//! `\ref{foo}`, `\thepage`, `\footnotemark`, `\footnotetext{hi}` and two
//! `\hspace{1cm}` across a blank line all ship exactly 1 page; `\vspace`,
//! `\label{foo}` and `\/` (an error) ship none, as do the empty and
//! comment-only bodies.

use flashtex_compiler::incremental::{compile_full, LayoutConstraints};
use flashtex_compiler::parser::{parse, Block, Inline};
use flashtex_compiler::text_builtins::DimenContext;

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn paragraphs_of(body: &str) -> Vec<Vec<Inline>> {
    let parsed = parse(&document(body));
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics for {body:?}: {:?}",
        parsed.diagnostics
    );
    parsed
        .blocks
        .into_iter()
        .map(|block| match block {
            Block::Paragraph(inlines) => inlines,
            other => panic!("{body:?} laid out {other:?}, expected a paragraph"),
        })
        .collect()
}

fn rule_of(inline: &Inline) -> flashtex_compiler::text_builtins::TextRule {
    match inline {
        Inline::Rule { rule, .. } => rule.clone(),
        other => panic!("expected a rule, found {other:?}"),
    }
}

fn resolved_pt(body: &str, index: usize) -> (f64, f64, f64) {
    let paras = paragraphs_of(body);
    let rule = rule_of(&paras[0][index]);
    let b = rule.resolve(&DimenContext::default());
    (
        flashtex_compiler::text_builtins::sp_to_pt(b.width),
        flashtex_compiler::text_builtins::sp_to_pt(b.height),
        flashtex_compiler::text_builtins::sp_to_pt(b.depth),
    )
}

#[test]
fn hspace_alone_starts_a_line_with_an_empty_anchor() {
    let paras = paragraphs_of("\\hspace{1cm}");
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0].len(), 2);
    // The anchor paints nothing and measures nothing (the oracle ships a
    // `(0.0+0.0)` line), but it is a box, so the line is built.
    let (w, h, d) = resolved_pt("\\hspace{1cm}", 0);
    assert_eq!((w, h, d), (0.0, 0.0, 0.0));
    assert!(!rule_of(&paras[0][0]).resolve(&DimenContext::default()).painted());
    match &paras[0][1] {
        Inline::HSpace { pt, .. } => assert!((pt - 28.4527).abs() < 0.01, "{pt}"),
        other => panic!("expected the hspace, found {other:?}"),
    }
    // The anchor carries the paragraph's own source range.
    match (&paras[0][0], &paras[0][1]) {
        (Inline::Rule { span: anchor, .. }, Inline::HSpace { span: glue, .. }) => {
            assert_eq!(anchor, glue);
        }
        _ => unreachable!(),
    }
}

#[test]
fn zero_hspace_still_starts_a_line() {
    // The oracle ships a page even for `\hspace{0pt}`: zero width is not
    // nothing once the paragraph exists.
    let paras = paragraphs_of("\\hspace{0pt}");
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0].len(), 2);
    match &paras[0][1] {
        Inline::HSpace { pt, .. } => assert_eq!(*pt, 0.0),
        other => panic!("expected the hspace, found {other:?}"),
    }
}

#[test]
fn hfill_alone_starts_a_line() {
    let paras = paragraphs_of("\\hfill");
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0].len(), 2);
    let (w, h, d) = resolved_pt("\\hfill", 0);
    assert_eq!((w, h, d), (0.0, 0.0, 0.0));
    assert!(matches!(
        &paras[0][1],
        Inline::HFill {
            leader: flashtex_compiler::parser::FillLeader::None,
            ..
        }
    ));
}

#[test]
fn hrulefill_alone_keeps_its_shape_without_an_anchor() {
    // A leader fill already carries its own `\leavevmode` box downstream,
    // so the anchor must not fire beside it.
    let paras = paragraphs_of("\\hrulefill");
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0].len(), 1);
    assert!(matches!(
        &paras[0][0],
        Inline::HFill {
            leader: flashtex_compiler::parser::FillLeader::Rule,
            ..
        }
    ));
}

#[test]
fn strut_is_a_zero_width_rule_with_strut_height() {
    // latex.ltx `\strutbox`: height .7 and depth .3 of the baselineskip,
    // width zero. The compiler's body default is 12pt, so the baselineskip
    // is 14.4pt here (10.08/4.32; a 10pt class option would give 8.4/3.6).
    // Above all, `\strut` is no longer an unsupported-command error.
    let paras = paragraphs_of("\\strut");
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0].len(), 1);
    let (w, h, d) = resolved_pt("\\strut", 0);
    assert_eq!(w, 0.0);
    assert!((h - 10.08).abs() < 0.01, "{h}");
    assert!((d - 4.32).abs() < 0.01, "{d}");
    // With an explicit 10pt class option the oracle's own 8.4/3.6 come out.
    let src = "\\documentclass[10pt]{article}\n\\begin{document}\n\\strut\n\\end{document}\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let Block::Paragraph(inlines) = &parsed.blocks[0] else {
        panic!("expected a paragraph");
    };
    let rule = rule_of(&inlines[0]);
    let b = rule.resolve(&DimenContext::default());
    let h = flashtex_compiler::text_builtins::sp_to_pt(b.height);
    let d = flashtex_compiler::text_builtins::sp_to_pt(b.depth);
    assert!((h - 8.4).abs() < 0.01, "{h}");
    assert!((d - 3.6).abs() < 0.01, "{d}");
}

#[test]
fn strut_follows_the_size_in_force() {
    let paras = paragraphs_of("{\\Large \\strut}");
    assert_eq!(paras.len(), 1);
    let (w, h, d) = resolved_pt("{\\Large \\strut}", 0);
    assert_eq!(w, 0.0);
    assert!(h > 8.5, "{h}");
    assert!(d > 3.6, "{d}");
}

#[test]
fn strut_then_blank_line_is_one_boxed_paragraph() {
    // The oracle ships one page; the strut itself is the box, so no anchor
    // is added beside it.
    let paras = paragraphs_of("\\strut\n\n");
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0].len(), 1);
    assert!(matches!(&paras[0][0], Inline::Rule { .. }));
}

#[test]
fn two_hspaces_make_two_anchored_paragraphs() {
    // The oracle ships one page (two paragraphs, one page): each paragraph
    // carries its own anchor.
    let paras = paragraphs_of("\\hspace{1cm}\n\n\\hspace{1cm}");
    assert_eq!(paras.len(), 2);
    for para in &paras {
        assert_eq!(para.len(), 2);
        assert!(matches!(&para[0], Inline::Rule { .. }));
        assert!(matches!(&para[1], Inline::HSpace { .. }));
    }
}

#[test]
fn glue_beside_text_gets_no_anchor() {
    let paras = paragraphs_of("a\\hspace{1cm}b");
    assert_eq!(paras.len(), 1);
    assert!(matches!(&paras[0][0], Inline::Text { .. }));
    assert!(!paras[0].iter().any(|inline| matches!(
        inline,
        Inline::Rule { rule, .. } if {
            let b = rule.resolve(&DimenContext::default());
            b.width == 0
                && flashtex_compiler::text_builtins::sp_to_pt(b.height) == 0.0
                && flashtex_compiler::text_builtins::sp_to_pt(b.depth) == 0.0
        }
    )));
}

#[test]
fn label_only_paragraph_gets_no_anchor() {
    // The oracle ships nothing for `\label{foo}` alone: a whatsit is not
    // horizontal material.
    let parsed = parse(&document("\\label{foo}"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.blocks.len(), 1);
    match &parsed.blocks[0] {
        Block::Paragraph(inlines) => {
            assert_eq!(inlines.len(), 1);
            assert!(matches!(inlines[0], Inline::Label { .. }));
        }
        other => panic!("expected a paragraph, found {other:?}"),
    }
}

#[test]
fn empty_and_comment_only_bodies_set_no_blocks() {
    // The control rows of issue #843: genuinely empty documents correctly
    // produce nothing in both engines, so the anchor must not invent a
    // paragraph where there is no material at all.
    for body in ["", "% only a comment\n"] {
        let parsed = parse(&document(body));
        assert!(
            parsed.diagnostics.is_empty(),
            "{body:?}: {:?}",
            parsed.diagnostics
        );
        assert!(
            parsed.blocks.is_empty(),
            "{body:?} laid out {:?}",
            parsed.blocks
        );
        let output = compile_full(&document(body), LayoutConstraints::default());
        assert!(output.pages.iter().all(|page| page.items.is_empty()));
    }
}

#[test]
fn vspace_alone_stays_vertical() {
    // The oracle ships nothing for `\vspace{1cm}` alone: vertical material
    // never starts a paragraph.
    let parsed = parse(&document("\\vspace{1cm}"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.blocks.len(), 1);
    assert!(matches!(parsed.blocks[0], Block::VSpace { .. }));
}

#[test]
fn hrule_alone_is_untouched() {
    let parsed = parse(&document("\\hrule"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.blocks.len(), 1);
    assert!(matches!(parsed.blocks[0], Block::Rule { .. }));
}
