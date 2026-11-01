//! Drift check for flashtex-bridge's MIT math-feature table
//! (`crates/bridge/data/math-symbol-features.txt`). Moved here from
//! `crates/bridge/src/features.rs` in old-engine retirement stage S1 (#1236),
//! so that the bridge no longer links this crate; it retires with it.
//!
//! The bridge tells the conversion provider which math constructs it may use.
//! These tests fail if that table stops matching `math::COMMAND_GLYPHS`, or if
//! the parser stops accepting any listed symbol or structural construct.

use flashtex_compiler::{
    diagnostics::Diagnostic,
    lexer::tokenize,
    math::{parse_tokens, MathList, MathPackages, COMMAND_GLYPHS},
};

/// The bridge's `STRUCTURAL_MATH_FEATURES`, and the probes that exercise them.
const STRUCTURAL: &[(&str, &str)] = &[
    ("\\frac{}{}", "\\frac{1}{2}"),
    ("\\sqrt{}", "\\sqrt{2}"),
    ("^{}", "x^{2}"),
    ("_{}", "x_{2}"),
];

fn bridge_table() -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../bridge/data/math-symbol-features.txt"
    );
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn parse(source: &str) -> (MathList, Vec<Diagnostic>) {
    let tokens = tokenize(source);
    let mut diagnostics = Vec::new();
    // These probes carry no document, so no package is loaded: the
    // kernel's own math is what a feature check should see.
    let list = parse_tokens(&tokens, MathPackages::default(), &mut diagnostics);
    (list, diagnostics)
}

fn unsupported(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|d| d.message.contains("is not supported"))
}

/// The table is exactly the sorted, de-duplicated `COMMAND_GLYPHS` names.
#[test]
fn bridge_table_matches_command_glyphs() {
    let mut expected: Vec<String> = COMMAND_GLYPHS
        .iter()
        .map(|(command, _)| format!("\\{command}"))
        .collect();
    expected.sort();
    expected.dedup();
    assert_eq!(bridge_table(), expected);
}

/// The bridge's structural constants still name the four constructs probed
/// here, so a change on either side is noticed.
#[test]
fn bridge_structural_constants_match() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../bridge/src/features.rs"
    ))
    .expect("read bridge features.rs");
    let listed = STRUCTURAL
        .iter()
        .map(|(feature, _)| format!("{feature:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    assert!(
        source.contains(&format!("STRUCTURAL_MATH_FEATURES: &[&str] = &[{listed}];")),
        "bridge STRUCTURAL_MATH_FEATURES no longer [{listed}]"
    );
}

/// Drift detector: if the compiler ever stops special-casing `\frac`,
/// `\sqrt`, `^` or `_`, this fails instead of the bridge silently continuing
/// to claim support Grok can no longer rely on.
#[test]
fn structural_constructs_are_still_supported_by_the_compiler_parser() {
    for (feature, probe) in STRUCTURAL {
        let (_, diagnostics) = parse(probe);
        assert!(
            !unsupported(&diagnostics),
            "{feature} regressed: {diagnostics:?}"
        );
    }
}

/// Every named symbol in the bridge table must round-trip through the parser
/// without an "is not supported" diagnostic, or the bridge would be claiming
/// support the compiler does not actually have.
#[test]
fn every_bridge_symbol_parses_without_an_unsupported_diagnostic() {
    for command in bridge_table() {
        let (_, diagnostics) = parse(&command);
        assert!(
            !unsupported(&diagnostics),
            "{command} is in the bridge table but the parser rejected it: {diagnostics:?}"
        );
    }
}
