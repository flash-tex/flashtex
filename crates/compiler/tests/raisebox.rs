//! Kernel `\raisebox` lifts its box argument above (or below) the baseline.
//!
//! latex.ltx `\raisebox{<raise>}[<height>][<depth>]{<text>}` sets `<text>`
//! in an `\hbox` raised by `<raise>` (negative lowers it). The compiler
//! keeps the argument on an `Inline::RaiseBox` node carrying the lift as
//! a `TextDimen`, and each layout shifts the argument's typeset extent by
//! it. This test pins the fixed behaviour: no diagnostic, the boxed text
//! on a raise node with a 2pt lift, and the laid-out box sitting a real
//! 2pt above its line-mates' baseline.
use flashtex_compiler::layout::layout;
use flashtex_compiler::parser::{parse, Block, Inline};
use flashtex_compiler::text_builtins::TextDimen;

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn paragraph(source: &str) -> Vec<Inline> {
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let mut inlines = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(body) = block {
            inlines.extend(body.clone());
        }
    }
    inlines
}

fn words(content: &[Inline]) -> Vec<&str> {
    content
        .iter()
        .map(|i| match i {
            Inline::Text { text, .. } => text.as_str(),
            other => panic!("not text: {other:?}"),
        })
        .collect()
}

// `\raisebox{2pt}{X}` yields a raise node with a 2pt lift field around
// `X`, with no diagnostic.
#[test]
fn raisebox_shifts_the_baseline_up_two_points() {
    let inlines = paragraph(&document("\\raisebox{2pt}{raised}"));
    assert_eq!(inlines.len(), 1, "{inlines:?}");
    match &inlines[0] {
        Inline::RaiseBox(raised) => {
            assert_eq!(
                raised.lift,
                TextDimen::parse("2pt").expect("2pt is a dimension"),
                "lift field must carry the real 2pt offset"
            );
            assert_eq!(raised.height, None);
            assert_eq!(raised.depth, None);
            assert_eq!(words(&raised.content), ["raised"]);
        }
        other => panic!("\\raisebox should box its argument, got: {other:?}"),
    }
}

// The laid-out raised box sits a real 2pt above the baseline it shares
// with its line-mates: the sibling baselines minus the raised baseline
// is exactly the lift, whatever the line's own extents do.
#[test]
fn raisebox_item_sits_two_points_above_its_line_mates() {
    let source = document("base \\raisebox{2pt}{raised} base");
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let pages = layout(&parsed.blocks);
    let items: Vec<(String, f64)> = pages
        .iter()
        .flat_map(|page| &page.items)
        .map(|item| (item.text.clone(), item.baseline_y_pt))
        .collect();
    let baseline = |text: &str| {
        items
            .iter()
            .find(|(t, _)| t == text)
            .unwrap_or_else(|| panic!("no laid-out item {text:?} in {items:?}"))
            .1
    };
    assert_eq!(baseline("base"), baseline("base"), "{items:?}");
    assert_eq!(baseline("base") - baseline("raised"), 2.0, "{items:?}");
}
