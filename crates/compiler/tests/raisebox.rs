//! Kernel `\raisebox` lifts its box argument above (or below) the baseline.
//!
//! latex.ltx `\raisebox{<raise>}[<height>][<depth>]{<text>}` sets `<text>`
//! in an `\hbox` raised by `<raise>` (negative lowers it). The compiler
//! currently rejects it with `\raisebox is not supported by this compiler
//! version` and typesets the argument as ordinary text at the baseline.
//! This test pins the fixed behaviour: no diagnostic, and the boxed text
//! carried on an inline item with a 2pt vertical offset.
//!
//! IGNORED until the fix lands: implementing `\raisebox` needs a new
//! inline node, and a new node does not fit in `parser.rs` alone (see the
//! module docs below and slice-1 check-in). Run with
//! `cargo test --test raisebox -- --ignored` to watch it fail on the base.
use flashtex_compiler::parser::{parse, Block, Inline};

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

// Pending: `\raisebox{2pt}{X}` must yield an inline item with a 2pt
// vertical offset field (a raised box around `X`), with no diagnostic.
// On the base this fails at the `paragraph` helper: the only diagnostic
// is `error: \raisebox is not supported by this compiler version`.
// The offset-field assertion is added with the fix, once the node type
// exists; the assertions below already pin the surrounding contract.
#[test]
#[ignore = "needs a scope decision: a new inline node touches files outside slice-1 scope"]
fn raisebox_shifts_the_baseline_up_two_points() {
    let inlines = paragraph(&document("\\raisebox{2pt}{raised}"));
    assert_eq!(inlines.len(), 1, "{inlines:?}");
    match &inlines[0] {
        Inline::HBox(raised) => {
            assert_eq!(words(&raised.content), ["raised"]);
        }
        other => panic!("\\raisebox should box its argument, got: {other:?}"),
    }
}
