//! `\hrulefill` and `\dotfill` (GH#319): `\hfill` glue filled with a rule or
//! with dots. pdflatex 1.40.29 (TeX Live 2026), `\hbox to 100pt{A\hrulefill B}`
//! and `{A\dotfill B}` at 10pt:
//!
//! ```text
//! \leaders 0.0 plus 1.0fill            \cleaders 0.0 plus 1.0fill
//! ..\rule(0.4+0.0)x*                   ..\hbox(1.05554+0.0)x4.40002, glue set 0.81113fil
//!                                      ...\glue 0.0 plus 1.0fil minus 1.0fil
//!                                      ...\OT1/cmr/m/n/10 .
//!                                      ...\glue 0.0 plus 1.0fil minus 1.0fil
//! ```
//!
//! so the rule is 0.4pt high on the baseline, and the dots are whole
//! 0.44em boxes, each period centred in its box and the boxes centred in the
//! glue. The layout measures fonts differently from pdfLaTeX (Core 14 here),
//! so these tests pin the construction against the fill the layout actually
//! gave the glue, not pdfLaTeX's absolute positions.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::{text_width, LayoutConstraints, TextItem};
use flashtex_compiler::parser::{parse, Block, FillLeader, Inline};

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn items(out: &CompileOutput) -> Vec<&TextItem> {
    out.pages.iter().flat_map(|p| p.items.iter()).collect()
}

fn fill_leaders(text: &str) -> Vec<FillLeader> {
    parse(text)
        .blocks
        .iter()
        .flat_map(|b| match b {
            Block::Paragraph(content) => content.clone(),
            _ => Vec::new(),
        })
        .filter_map(|i| match i {
            Inline::HFill { leader, .. } => Some(leader),
            _ => None,
        })
        .collect()
}

#[test]
fn the_three_fills_parse_to_their_leaders() {
    assert_eq!(
        fill_leaders("a\\hfill b\\hrulefill c\\dotfill d\n"),
        [FillLeader::None, FillLeader::Rule, FillLeader::Dots]
    );
}

#[test]
fn hrulefill_draws_a_baseline_rule_across_the_fill() {
    let out = compile("Name:\\hrulefill end\n");
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let all = items(&out);
    let name = all.iter().find(|i| i.text == "Name:").unwrap();
    let end = all.iter().find(|i| i.text == "end").unwrap();
    let rule = all
        .iter()
        .find_map(|i| i.rule.map(|r| (i, r)))
        .expect("a rule item");
    let (item, geometry) = rule;
    assert_eq!(geometry.height_pt, 0.4, "\\hrule in leaders is 0.4pt");
    assert_eq!(
        geometry.y_pt,
        ((item.baseline_y_pt - 0.4) * 100.0).round() / 100.0,
        "resting on the baseline"
    );
    assert_eq!(item.baseline_y_pt, name.baseline_y_pt);
    let name_end = name.x_pt + text_width("Name:", name.font_size_pt, name.font);
    assert!(
        (item.x_pt - name_end).abs() < 0.02,
        "starts where the text before it ends: {} vs {name_end}",
        item.x_pt
    );
    assert!(
        (item.x_pt + geometry.width_pt - end.x_pt).abs() < 0.02,
        "ends where the text after it starts: {} vs {}",
        item.x_pt + geometry.width_pt,
        end.x_pt
    );
}

#[test]
fn a_plain_hfill_draws_nothing_and_places_text_like_hrulefill() {
    let plain = compile("Name:\\hfill end\n");
    let rule = compile("Name:\\hrulefill end\n");
    let text = |o: &CompileOutput| {
        items(o)
            .iter()
            .filter(|i| i.rule.is_none())
            .map(|i| (i.text.clone(), (i.x_pt * 100.0).round()))
            .collect::<Vec<_>>()
    };
    assert!(items(&plain).iter().all(|i| i.rule.is_none()));
    assert_eq!(
        text(&plain),
        text(&rule),
        "the leader changes what fills the glue, not where text goes"
    );
}

#[test]
fn dotfill_sets_whole_044em_boxes_centred_in_the_fill() {
    let out = compile("Chapter\\dotfill 7\n");
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let all = items(&out);
    let chapter = all.iter().find(|i| i.text == "Chapter").unwrap();
    let seven = all.iter().find(|i| i.text == "7").unwrap();
    let dots: Vec<_> = all.iter().filter(|i| i.text == ".").collect();
    let size = chapter.font_size_pt;
    let start = chapter.x_pt + text_width("Chapter", size, chapter.font);
    let width = seven.x_pt - start;
    let pitch = 0.44 * size;
    assert_eq!(
        dots.len(),
        (width / pitch).floor() as usize,
        "whole boxes only"
    );
    for pair in dots.windows(2) {
        assert!(
            (pair[1].x_pt - pair[0].x_pt - pitch).abs() < 0.02,
            "one box apart"
        );
    }
    let dot = text_width(".", size, dots[0].font);
    let left = dots[0].x_pt - (pitch - dot) / 2.0 - start;
    let right = seven.x_pt - (dots.last().unwrap().x_pt + dot + (pitch - dot) / 2.0);
    assert!(
        (left - right).abs() < 0.03,
        "\\cleaders centres the boxes: {left} vs {right}"
    );
    assert!(dots
        .iter()
        .all(|d| d.baseline_y_pt == chapter.baseline_y_pt));
}

#[test]
fn two_rule_fills_on_a_line_share_the_slack() {
    let out = compile("Name: \\hrulefill\\quad Section: \\hrulefill\n");
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let rules: Vec<_> = items(&out).into_iter().filter_map(|i| i.rule).collect();
    assert_eq!(rules.len(), 2);
    assert!(
        (rules[0].width_pt - rules[1].width_pt).abs() < 0.02,
        "{rules:?}"
    );
}
