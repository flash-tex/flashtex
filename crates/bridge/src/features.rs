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

/// The engine that typesets the destination document: transfer-v1
/// `capture_convert.engine` (additive; absent means `previous`). The caller
/// names the engine, never the list: the bridge picks its own table for it,
/// so a client still cannot widen what the provider is told (issues #51/#23).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TypesettingEngine {
    /// The old engine (its compiler's table below), until it retires.
    #[default]
    Previous,
    /// The pdfLaTeX-compatible engine (`flashtex-host`).
    New,
}

/// The new engine typesets what pdfLaTeX does with the document's own
/// packages, so its list is a policy, not an enumeration: each entry starts
/// with `POLICY:` (the system prompt says how to read those), and the
/// package question is answered from the context's `\usepackage` lines.
/// Old-engine retirement plan #1236, stage S3r.
pub const NEW_ENGINE_FEATURES: &[&str] = &[
    "POLICY: pdfLaTeX-compatible destination: any construct pdfLaTeX typesets with the packages this document already loads",
    "POLICY: math in $...$, \\[ ... \\] or equation; amsmath environments (align, gather, cases …) only if the document loads amsmath",
    "POLICY: \\mathbb, \\mathcal and the like only if the document loads their package (amssymb, amsfonts)",
    "POLICY: never add \\usepackage lines or new macros; a construct needing a package the document lacks is UNSUPPORTED",
];

/// The list the provider receives for a document `engine` typesets.
pub fn supported_features_for(engine: TypesettingEngine) -> Vec<String> {
    match engine {
        TypesettingEngine::Previous => supported_features(),
        TypesettingEngine::New => NEW_ENGINE_FEATURES.iter().map(|s| s.to_string()).collect(),
    }
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

    /// The engine picks the list: the old one is this table, unchanged; the
    /// new one is the policy list, within the conversion context's limits.
    #[test]
    fn the_engine_picks_the_list() {
        assert_eq!(supported_features_for(TypesettingEngine::Previous), supported_features());
        assert_eq!(TypesettingEngine::default(), TypesettingEngine::Previous);
        let new = supported_features_for(TypesettingEngine::New);
        assert!(new.iter().all(|f| f.starts_with("POLICY: ") && f.len() <= 128), "{new:?}");
        assert!(!new.contains(&"\\pi".to_string()));
        let doc = crate::Document {
            project_id: "p".into(),
            path: "main.tex".into(),
            revision: 1,
            text: "x".into(),
        };
        crate::context::build(&doc, 0, 1, std::iter::once(&doc), new).expect("fits the context");
        for (wire, engine) in [("\"new\"", TypesettingEngine::New), ("\"previous\"", TypesettingEngine::Previous)] {
            assert_eq!(serde_json::from_str::<TypesettingEngine>(wire).unwrap(), engine);
        }
        assert!(serde_json::from_str::<TypesettingEngine>("\"pdftex\"").is_err());
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
