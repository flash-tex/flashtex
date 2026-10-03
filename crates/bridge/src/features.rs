//! The honest list of math constructs Grok is told it may use.
//!
//! Named symbols come from `data/math-symbol-features.txt`, an MIT table that
//! snapshots the command names of the original compiler's
//! `math::COMMAND_GLYPHS` (old-engine retirement S1, #1236), so the bridge no
//! longer links that crate. A few structural constructs (fractions, radicals,
//! super/subscripts) were handled by the compiler's math parser directly rather
//! than through that glyph table, so they are a small constant here. While the
//! compiler exists, `crates/compiler/tests/bridge_math_features.rs` re-derives
//! both from the compiler's real tables, lexer and math parser and fails the
//! day either drifts (see issues #51/#23).

/// The named-symbol table, one `\command` per line (see the file's header).
const MATH_SYMBOL_FEATURES: &str = include_str!("../data/math-symbol-features.txt");

/// Constructs the compiler's math parser special-cased ahead of the named
/// glyph table (`crates/compiler/src/math.rs`, `command_atom`).
pub const STRUCTURAL_MATH_FEATURES: &[&str] = &["\\frac{}{}", "\\sqrt{}", "^{}", "_{}"];

/// The named-symbol commands of `data/math-symbol-features.txt`, in file order.
fn math_symbol_features() -> impl Iterator<Item = &'static str> {
    MATH_SYMBOL_FEATURES
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// The honest list of math features to hand to Grok as `supported_features`.
/// Deterministic and independent of anything a caller supplies, so it cannot
/// be weakened by a stale or optimistic client value.
pub fn supported_features() -> Vec<String> {
    let mut features: Vec<String> = math_symbol_features().map(str::to_string).collect();
    features.extend(STRUCTURAL_MATH_FEATURES.iter().map(|s| s.to_string()));
    features.sort();
    features.dedup();
    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_list_includes_named_symbols_and_structural_constructs() {
        let derived = supported_features();
        assert!(derived.contains(&"\\pi".to_string()));
        assert!(derived.contains(&"\\sqrt{}".to_string()));
        assert!(derived.contains(&"\\frac{}{}".to_string()));
        assert!(derived.contains(&"^{}".to_string()));
        assert!(derived.contains(&"_{}".to_string()));
        assert_eq!(
            derived.len(),
            math_symbol_features().count() + STRUCTURAL_MATH_FEATURES.len()
        );
    }

    /// The table is a sorted, duplicate-free list of `\` + ASCII letters, so
    /// a hand edit cannot smuggle in a malformed or repeated entry.
    #[test]
    fn symbol_table_is_sorted_unique_control_words() {
        let names: Vec<&str> = math_symbol_features().collect();
        assert!(
            names.len() > 100,
            "table unexpectedly short: {}",
            names.len()
        );
        assert!(
            names.windows(2).all(|w| w[0] < w[1]),
            "not sorted and unique"
        );
        for name in names {
            let word = name.strip_prefix('\\').unwrap_or("");
            assert!(
                !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphabetic()),
                "malformed entry {name:?}"
            );
        }
    }

    /// Regression: the context limits once allowed only 64 features, so the
    /// derived list made every real `capture_convert` fail with
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
            .expect("the derived feature list must fit the conversion context");
    }
}
