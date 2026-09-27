//! `\vrule` and `\leaders\hrule\hskip` in running text.
//!
//! Measured against pdflatex (TeX Live 2026, 10pt article):
//! `\vrule width 2pt height 5pt depth 1pt` is `\rule(5.0+1.0)x2.0`, a bare
//! `\vrule` is `\rule(*+*)x0.4` (the enclosing line's height/depth), and
//! `\leaders\hrule\hskip 1cm` is `\leaders 28.45274 \rule(0.4+0.0)x*`.
//! This layout sets type in Core-14 metrics on its own page frame (not
//! Computer Modern on the article page), so absolute page positions cannot
//! match pdflatex: these tests pin the rule geometry absolutely and the
//! word positions against `\rule`-equivalent references through the same
//! layout, which fails if a rule advances the cursor by the wrong width.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::parse;

const SOURCE: &str = "\\documentclass[10pt]{article}\\begin{document}A \\vrule width 2pt height 5pt depth 1pt\\ B \\vrule\\ C \\leaders\\hrule\\hskip 1cm\\ D next.\\end{document}";

/// The same line with every rule spelled as the `\rule` it must equal:
/// `\vrule H D W` is `\rule[-D]{W}{H+D}`, a bare `\vrule` at 10pt is the
/// strut box (`\baselineskip` 12pt: `\rule[-3.6pt]{0.4pt}{12pt}`), and the
/// leaders rule is `\rule{1cm}{0.4pt}`.
const RULE_EQUIVALENT: &str = "\\documentclass[10pt]{article}\\begin{document}A \\rule[-1pt]{2pt}{6pt}\\ B \\rule[-3.6pt]{0.4pt}{12pt}\\ C \\rule{1cm}{0.4pt}\\ D next.\\end{document}";

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

#[test]
fn oracle_source_has_no_diagnostics() {
    let parsed = parse(SOURCE);
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn explicit_vrule_is_2pt_wide_by_6pt_total() {
    let out = compile(SOURCE);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let rules: Vec<_> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|i| i.rule)
        .collect();
    assert_eq!(rules.len(), 3, "{rules:?}");
    assert_eq!(rules[0].width_pt, 2.0, "width 2pt: {rules:?}");
    assert_eq!(rules[0].height_pt, 6.0, "height 5pt + depth 1pt: {rules:?}");
}

#[test]
fn bare_vrule_is_strut_sized() {
    let out = compile(SOURCE);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let rules: Vec<_> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|i| i.rule)
        .collect();
    assert_eq!(rules.len(), 3, "{rules:?}");
    // 10pt: `\baselineskip` is 12pt, so the strut box is 8.4pt over the
    // baseline and 3.6pt under it.
    assert_eq!(rules[1].width_pt, 0.4, "default width 0.4pt: {rules:?}");
    assert_eq!(rules[1].height_pt, 12.0, "strut height+depth: {rules:?}");
    let items: Vec<_> = out.pages.iter().flat_map(|p| p.items.iter()).collect();
    let baseline = items
        .iter()
        .find(|i| i.text == "A")
        .expect("an A item")
        .baseline_y_pt;
    let rule_item = items.iter().find(|i| i.rule == Some(rules[1])).unwrap();
    assert!(
        (rule_item.rule.unwrap().y_pt - (baseline - 8.4)).abs() < 0.02,
        "8.4pt above the baseline: {:?}",
        rule_item.rule
    );
}

#[test]
fn leaders_rule_is_1cm_wide_by_0pt4_tall() {
    let out = compile(SOURCE);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let rules: Vec<_> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|i| i.rule)
        .collect();
    assert_eq!(rules.len(), 3, "{rules:?}");
    assert!(
        (rules[2].width_pt - 28.45).abs() < 0.01,
        "1cm wide: {rules:?}"
    );
    assert_eq!(rules[2].height_pt, 0.4, "0.4pt tall: {rules:?}");
    let items: Vec<_> = out.pages.iter().flat_map(|p| p.items.iter()).collect();
    let baseline = items
        .iter()
        .find(|i| i.text == "A")
        .expect("an A item")
        .baseline_y_pt;
    let rule_item = items.iter().find(|i| i.rule == Some(rules[2])).unwrap();
    assert!(
        (rule_item.rule.unwrap().y_pt - (baseline - 0.4)).abs() < 0.02,
        "resting on the baseline like \\hrulefill: {:?}",
        rule_item.rule
    );
}

#[test]
fn rules_advance_words_like_their_rule_equivalents() {
    let out = compile(SOURCE);
    let reference = compile(RULE_EQUIVALENT);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert!(
        reference.diagnostics.is_empty(),
        "{:?}",
        reference.diagnostics
    );
    let words = |o: &CompileOutput| {
        o.pages
            .iter()
            .flat_map(|p| p.items.iter())
            .filter(|i| i.rule.is_none() && !i.text.trim().is_empty())
            .map(|i| (i.text.clone(), i.x_pt, i.baseline_y_pt))
            .collect::<Vec<_>>()
    };
    let (left, right) = (words(&out), words(&reference));
    assert_eq!(
        left.len(),
        right.len(),
        "same word items: {left:?} vs {right:?}"
    );
    for (a, b) in left.iter().zip(right.iter()) {
        assert_eq!(a.0, b.0, "same words in order");
        assert!(
            (a.1 - b.1).abs() < 0.02 && (a.2 - b.2).abs() < 0.02,
            "{} at {:?} vs {:?}",
            a.0,
            (a.1, a.2),
            (b.1, b.2)
        );
    }
    let rules = |o: &CompileOutput| {
        o.pages
            .iter()
            .flat_map(|p| p.items.iter())
            .filter_map(|i| i.rule.map(|r| (i.x_pt, r)))
            .collect::<Vec<_>>()
    };
    let (left, right) = (rules(&out), rules(&reference));
    assert_eq!(left.len(), 3, "{left:?}");
    assert_eq!(right.len(), 3, "{right:?}");
    for (a, b) in left.iter().zip(right.iter()) {
        assert!(
            (a.0 - b.0).abs() < 0.02
                && (a.1.width_pt - b.1.width_pt).abs() < 0.02
                && (a.1.height_pt - b.1.height_pt).abs() < 0.02
                && (a.1.y_pt - b.1.y_pt).abs() < 0.02,
            "{a:?} vs {b:?}"
        );
    }
}

#[test]
fn infinite_leaders_stretch_become_an_hrulefill_leader() {
    let parsed = parse("\\documentclass{article}\\begin{document}A\\leaders\\hrule\\hskip 0pt plus 1fill B\\end{document}");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    let out = compile("A\\leaders\\hrule\\hskip 0pt plus 1fill B\n");
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let rule = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .find_map(|i| i.rule)
        .expect("a rule item across the fill");
    assert_eq!(rule.height_pt, 0.4, "{rule:?}");
    assert!(rule.width_pt > 10.0, "fills the slack: {rule:?}");
}

#[test]
fn unsupported_leaders_shapes_stay_diagnosed() {
    for source in ["\\leaders X B", "\\leaders\\hrule B", "\\vrule width oops B"] {
        let parsed = parse(source);
        assert_eq!(
            parsed.diagnostics.len(),
            1,
            "{source}: {:?}",
            parsed.diagnostics
        );
    }
    // A non-rule leaders box names the gap; the box command itself keeps
    // its own pre-existing diagnostic.
    let parsed = parse("\\leaders\\hbox to 1cm{A} B");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\leaders is only supported")),
        "{:?}",
        parsed.diagnostics
    );
    // A malformed dimension keeps its default: the rule still reaches the page.
    let out = compile("A\\vrule width oops B\n");
    assert!(out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .any(|i| i.rule.map_or(false, |r| r.width_pt == 0.4)));
}
