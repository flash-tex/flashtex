//! `\hrule` and `\vspace` add only their own vertical space, not text lines.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::{LayoutConstraints, LINE_SPACING, PARAGRAPH_GAP_PT};
use flashtex_compiler::parser::SourceDocument;

fn gap_after_rule(between: &str) -> f64 {
    let text = format!("Above\n\n\\hrule\n{between}\nBelow\n");
    let out = compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: &text,
        }],
        "main.tex",
        LayoutConstraints::default(),
    );
    let items = &out.pages[0].items;
    let rule = items.iter().find(|i| i.rule.is_some()).unwrap();
    let below = items.iter().find(|i| i.text == "Below").unwrap();
    below.baseline_y_pt - rule.baseline_y_pt
}

#[test]
fn vspace_after_a_rule_adds_only_its_length() {
    let size = LayoutConstraints::default().font_size_pt;
    let plain = gap_after_rule("");
    let spaced = gap_after_rule("\\vspace{10pt}");
    // Exactly the \vspace, with no extra line advance inserted for it.
    assert!((spaced - plain - 10.0).abs() < 0.02, "{plain} -> {spaced}");
    // After a rule the next baseline is one line (with depth) plus the paragraph gap away,
    // not two or three lines (HW1's title rule showed a ~54pt gap).
    assert!(
        plain <= size * LINE_SPACING + PARAGRAPH_GAP_PT + 0.02,
        "rule to next baseline {plain}"
    );
}
