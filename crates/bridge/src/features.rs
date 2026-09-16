//! The honest list of math constructs Grok is told it may use.
//!
//! This is derived from the original FlashTeX compiler's own tables rather than
//! a hand-maintained guess, so it cannot silently drift from what the compiler
//! actually renders (see issues #51/#23). Named symbols come straight from
//! `flashtex_compiler::math::COMMAND_GLYPHS`. A few structural constructs
//! (fractions, radicals, super/subscripts) are handled by the compiler's math
//! parser directly rather than through that glyph table, so they are listed as
//! a small checked-in constant here; `structural_constructs_are_still_supported`
//! below re-parses each one with the compiler's real lexer and math parser and
//! fails the build the day any of them stops being accepted.
use flashtex_compiler::math::COMMAND_GLYPHS;

/// Constructs the compiler's math parser special-cases ahead of the named
/// glyph table (`crates/compiler/src/math.rs`, `command_atom`). Not derivable
/// from a data table today; kept honest by the drift test in this module.
pub const STRUCTURAL_MATH_FEATURES: &[&str] = &["\\frac{}{}", "\\sqrt{}", "^{}", "_{}"];

/// The honest, compiler-derived list of math features to hand to Grok as
/// `supported_features`. Deterministic and independent of anything a caller
/// supplies, so it cannot be weakened by a stale or optimistic client value.
pub fn supported_features() -> Vec<String> {
    let mut features: Vec<String> = COMMAND_GLYPHS
        .iter()
        .map(|(command, _)| format!("\\{command}"))
        .collect();
    features.extend(STRUCTURAL_MATH_FEATURES.iter().map(|s| s.to_string()));
    features.sort();
    features.dedup();
    features
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashtex_compiler::{
        diagnostics::Diagnostic,
        lexer::tokenize,
        math::{parse_tokens, MathList, MathPackages},
    };

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

    #[test]
    fn derived_list_includes_named_symbols_and_structural_constructs() {
        let derived = supported_features();
        assert!(derived.contains(&"\\pi".to_string()));
        assert!(derived.contains(&"\\sqrt{}".to_string()));
        assert!(derived.contains(&"\\frac{}{}".to_string()));
        assert!(derived.contains(&"^{}".to_string()));
        assert!(derived.contains(&"_{}".to_string()));
        assert_eq!(derived.len(), {
            let mut all: Vec<String> = COMMAND_GLYPHS
                .iter()
                .map(|(c, _)| format!("\\{c}"))
                .chain(STRUCTURAL_MATH_FEATURES.iter().map(|s| s.to_string()))
                .collect();
            all.sort();
            all.dedup();
            all.len()
        });
    }

    /// Regression: the context limits once allowed only 64 features, so the
    /// compiler-derived list made every real `capture_convert` fail with
    /// `context_too_large` before any provider was contacted.
    #[test]
    fn derived_list_fits_the_conversion_context_limits() {
        let derived = supported_features();
        assert!(derived.len() > 64, "list is larger than the old limit");
        let doc = crate::Document {
            project_id: "p".into(),
            path: "main.tex".into(),
            revision: 1,
            text: "x".into(),
        };
        crate::context::build(&doc, 0, 1, std::iter::once(&doc), derived)
            .expect("the compiler-derived feature list must fit the conversion context");
    }

    /// Drift detector: if the compiler ever stops special-casing `\frac`,
    /// `\sqrt`, `^` or `_`, this fails instead of `supported_features()`
    /// silently continuing to claim support Grok can no longer rely on.
    #[test]
    fn structural_constructs_are_still_supported_by_the_compiler_parser() {
        let (_, diagnostics) = parse("\\frac{1}{2}");
        assert!(
            !unsupported(&diagnostics),
            "\\frac regressed: {diagnostics:?}"
        );
        let (_, diagnostics) = parse("\\sqrt{2}");
        assert!(
            !unsupported(&diagnostics),
            "\\sqrt regressed: {diagnostics:?}"
        );
        let (_, diagnostics) = parse("x^{2}");
        assert!(
            !unsupported(&diagnostics),
            "superscript regressed: {diagnostics:?}"
        );
        let (_, diagnostics) = parse("x_{2}");
        assert!(
            !unsupported(&diagnostics),
            "subscript regressed: {diagnostics:?}"
        );
    }

    /// Every named symbol in COMMAND_GLYPHS must round-trip through the parser
    /// without an "is not supported" diagnostic, or the derived list would be
    /// claiming support the compiler does not actually have.
    #[test]
    fn every_command_glyph_parses_without_an_unsupported_diagnostic() {
        for (command, _) in COMMAND_GLYPHS {
            let (_, diagnostics) = parse(&format!("\\{command}"));
            assert!(
                !unsupported(&diagnostics),
                "\\{command} is listed in COMMAND_GLYPHS but the parser rejected it: {diagnostics:?}"
            );
        }
    }
}
