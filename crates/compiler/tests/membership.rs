//! Issue #40: `\in`, `\notin`, `\ni` set-membership commands in math mode.
//!
//! The dispatch already exists (`command_glyph` in `src/math.rs` maps
//! `"in"` to U+2208, `"notin"` to U+2209, `"ni"` to U+220B); the gap was
//! that no test covered it. These tests pin each rendered glyph through the
//! real `compile_full_project` path with zero diagnostics, plus one combined
//! case so a future dispatch regression cannot break one name while the
//! others stay green.
//!
//! Convention follows `kernel_untested_b.rs`: compile, require no
//! "not supported" diagnostic and empty diagnostics, then assert on the
//! laid-out item texts (inter-word spaces are layout gaps, not text items).

use flashtex_compiler::incremental::{compile_full_project, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::SourceDocument;

fn compile(source: &str) -> CompileOutput {
    compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: source,
        }],
        "main.tex",
        LayoutConstraints::default(),
    )
}

fn joined(output: &CompileOutput) -> String {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .map(|item| item.text.as_str())
        .collect()
}

fn assert_clean(output: &CompileOutput) {
    assert!(
        !output
            .diagnostics
            .iter()
            .any(|d| d.message.contains("not supported")),
        "unexpected unsupported diagnostic: {:?}",
        output.diagnostics
    );
    assert!(
        output.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        output.diagnostics
    );
}

#[test]
fn in_renders_element_of() {
    let output = compile("$a \\in B$");
    assert_clean(&output);
    assert_eq!(joined(&output), "a∈B", "{:?}", output.pages);
}

#[test]
fn notin_renders_not_an_element_of() {
    let output = compile("$c \\notin D$");
    assert_clean(&output);
    assert_eq!(joined(&output), "c∉D", "{:?}", output.pages);
}

#[test]
fn ni_renders_contains_as_member() {
    let output = compile("$E \\ni f$");
    assert_clean(&output);
    assert_eq!(joined(&output), "E∋f", "{:?}", output.pages);
}

#[test]
fn combined_membership_case_renders_all_three_glyphs() {
    let output = compile("$a \\in B, c \\notin D, E \\ni f$");
    assert_clean(&output);
    assert_eq!(joined(&output), "a∈B,c∉D,E∋f", "{:?}", output.pages);
}
