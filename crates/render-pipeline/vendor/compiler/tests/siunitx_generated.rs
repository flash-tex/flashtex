//! The generated siunitx tables (`src/siunitx_generated.rs`, extracted from the
//! real `siunitx.sty` by `tools/extract_siunitx.py`) must cover at least what
//! the hand-maintained tables in `src/siunitx.rs` already know, so replacing
//! hand-maintenance never silently loses a unit or prefix. Needs no TeX Live:
//! it only compares the two committed files.

use flashtex_compiler::siunitx::UNITS;
use flashtex_compiler::siunitx_generated::{
    GENERATED_BINARY_PREFIXES, GENERATED_POWERS, GENERATED_PREFIXES, GENERATED_UNITS, SIUNITX_DATE,
    SIUNITX_VERSION,
};

/// Names from the hand-maintained `PREFIXES` table, read live from
/// `src/siunitx.rs` (`PREFIXES` itself is private, so the drift check parses
/// its `("name", "symbol"),` rows instead of freezing a copy here).
fn hand_prefix_names() -> Vec<String> {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/siunitx.rs"),
    )
    .expect("src/siunitx.rs is readable");
    let block = source
        .split("const PREFIXES")
        .nth(1)
        .expect("const PREFIXES exists")
        .split("];")
        .next()
        .expect("PREFIXES block ends");
    block
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("(\"")?
                .split("\",")
                .next()
                .map(str::to_string)
        })
        .collect()
}

#[test]
fn generated_units_cover_hand_maintained_units() {
    assert!(
        !UNITS.is_empty(),
        "hand-maintained UNITS is the oracle here"
    );
    for (name, _) in UNITS {
        assert!(
            GENERATED_UNITS.iter().any(|(n, _, _)| n == name),
            "generated table lost hand-maintained unit `{name}`"
        );
    }
}

#[test]
fn generated_prefixes_cover_hand_maintained_prefixes() {
    let hand = hand_prefix_names();
    assert!(!hand.is_empty(), "parsed no PREFIXES rows");
    for name in &hand {
        assert!(
            GENERATED_PREFIXES.iter().any(|(n, _, _)| n == name),
            "generated table lost hand-maintained prefix `{name}`"
        );
    }
}

#[test]
fn generated_tables_are_well_formed() {
    assert!(!SIUNITX_VERSION.is_empty() && !SIUNITX_DATE.is_empty());
    assert!(
        GENERATED_UNITS.len() >= UNITS.len(),
        "generated units shrank below the hand-maintained set"
    );
    assert!(GENERATED_PREFIXES.len() >= hand_prefix_names().len());
    for (name, definition, _) in GENERATED_UNITS {
        assert!(!name.is_empty() && !definition.is_empty(), "empty unit row");
    }
    let mut names: Vec<&str> = GENERATED_UNITS.iter().map(|(n, _, _)| *n).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(
        before,
        names.len(),
        "duplicate unit names in generated table"
    );
    // Spot checks against siunitx.sty 3.4.14: an alias, a composed
    // unit, the option-carrying `\kWh`, and the binary/powers tables.
    let def = |n: &str| {
        GENERATED_UNITS
            .iter()
            .find(|(m, _, _)| *m == n)
            .map(|(_, d, _)| *d)
    };
    assert_eq!(def("meter"), Some("\\metre"));
    assert_eq!(def("kilogram"), Some("\\kilo \\gram"));
    assert_eq!(def("celsius"), Some("\\degreeCelsius"));
    let kwh = GENERATED_UNITS.iter().find(|(n, _, _)| *n == "kWh");
    assert_eq!(kwh.map(|(_, _, o)| *o), Some("inter-unit-product ="));
    assert!(GENERATED_PREFIXES.contains(&("micro", "\\__siunitx_unit_non_latin:n { \"03BC }", -6)));
    assert!(GENERATED_BINARY_PREFIXES.contains(&("kibi", "Ki")));
    assert!(GENERATED_POWERS.contains(&("square", "squared", "2")));
}
