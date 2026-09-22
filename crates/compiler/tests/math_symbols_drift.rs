//! The generated declaration table (`math_symbols`, from `fontmath.ltx` and
//! the symbol packages via `scripts/gen_math_symbols.py`) against the tables
//! the engine still writes by hand: `math::COMMAND_GLYPHS` (command -> text),
//! `math::symbol_class` (text -> class) and the generated `amssymb` table.
//! TeX never runs here; the generated file is committed.
//!
//! Every disagreement is either a latent bug or a deliberate engine choice,
//! and each one is listed here by name so that it is reviewed, not hidden:
//! a listed item that stops disagreeing fails the test too.

use flashtex_compiler::amssymb;
use flashtex_compiler::math::{symbol_class_of, AtomClass, COMMAND_GLYPHS};
use flashtex_compiler::math_symbols::{
    self, declarations, Kind, MathSymbol, Provider, SymbolClass, SymbolFont, SYMBOLS,
};
use flashtex_compiler::supported::{self, Mode};
use std::collections::{BTreeMap, BTreeSet};

/// Hand rows whose text differs from the declaration's, with the reason.
const KNOWN_TEXT: &[(&str, &str)] = &[];

/// Hand rows whose class (`symbol_class` of their text) differs from the
/// declared class, with the reason.
const KNOWN_CLASS: &[(&str, &str)] = &[];

/// Declared kernel commands the compiler's inventory does not list: the
/// pieces with no character of their own (`fontmath.ltx` 340, 374-375, 462:
/// `\mapstochar` is zero-width cmsy "37, `\lhook`/`\rhook` cmmi "2C/"2D,
/// `\Arrowvert` cmsy "6B with cmex "3D) and the radical sign itself.
/// cmex "7A-"7D (`\lmoustache`, `\rmoustache`, the brace tips) are
/// excluded by `math::declared_kernel_symbol` because no bundled face paints
/// them at the TFM's advance (tools/kernel-math-gap).
const KNOWN_MISSING_KERNEL: &[&str] = &[
    "Arrowvert", "braceld", "bracelu", "bracerd", "braceru", "lhook", "lmoustache", "mapstochar",
    "rhook", "rmoustache", "sqrtsign",
];

fn hand_glyph(name: &str) -> Option<&'static str> {
    COMMAND_GLYPHS.iter().find(|(n, _)| *n == name).map(|(_, g)| *g)
}

fn kernel_symbols() -> impl Iterator<Item = &'static MathSymbol> {
    SYMBOLS
        .iter()
        .filter(|s| s.provider == Provider::Kernel && !s.character && !s.name.contains('@'))
}

fn expect_listed(actual: BTreeMap<String, String>, known: &[(&str, &str)], what: &str) {
    let known_names: BTreeSet<&str> = known.iter().map(|(n, _)| *n).collect();
    let unexpected: Vec<_> = actual
        .iter()
        .filter(|(n, _)| !known_names.contains(n.as_str()))
        .map(|(n, d)| format!("  {n}: {d}"))
        .collect();
    let resolved: Vec<_> = known_names
        .iter()
        .filter(|n| !actual.contains_key(**n))
        .collect();
    assert!(
        unexpected.is_empty(),
        "{what}: {} unlisted disagreement(s) between the declarations and the hand rows:\n{}",
        unexpected.len(),
        unexpected.join("\n")
    );
    assert!(
        resolved.is_empty(),
        "{what}: listed disagreements no longer disagree; remove them: {resolved:?}"
    );
}

#[test]
fn hand_glyph_rows_agree_with_the_kernel_declarations() {
    let mut text = BTreeMap::new();
    let mut class = BTreeMap::new();
    for s in kernel_symbols() {
        let Some(glyph) = hand_glyph(s.name) else { continue };
        if glyph != s.text {
            text.insert(s.name.to_string(), format!("hand {glyph:?}, declared {:?} ({})", s.text, s.source));
        }
        if s.kind != Kind::Symbol {
            continue;
        }
        let declared = s.class.atom_class();
        let hand = symbol_class_of(glyph);
        if hand != declared {
            class.insert(s.name.to_string(), format!("hand {hand:?}, declared {declared:?} ({})", s.source));
        }
    }
    expect_listed(text, KNOWN_TEXT, "text");
    expect_listed(class, KNOWN_CLASS, "class");
}

#[test]
fn every_kernel_declaration_is_in_the_inventory() {
    let inventory = supported::inventory();
    let listed: BTreeSet<&str> = inventory
        .commands
        .iter()
        .filter(|c| c.mode == Mode::Math)
        .map(|c| c.name)
        .collect();
    let missing: Vec<&str> = kernel_symbols()
        .map(|s| s.name)
        .filter(|n| !listed.contains(n))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let known: BTreeSet<&str> = KNOWN_MISSING_KERNEL.iter().copied().collect();
    let unexpected: Vec<_> = missing.iter().filter(|n| !known.contains(**n)).collect();
    let resolved: Vec<_> = known.iter().filter(|n| !missing.contains(n)).collect();
    assert!(unexpected.is_empty(), "kernel commands the inventory lacks: {unexpected:?}");
    assert!(resolved.is_empty(), "now in the inventory; remove from KNOWN_MISSING_KERNEL: {resolved:?}");
}

#[test]
fn the_amssymb_table_is_a_view_of_the_declarations() {
    for a in amssymb::SYMBOLS {
        if a.name.contains('@') {
            continue;
        }
        let font = match a.font {
            amssymb::SymbolFont::Msam => SymbolFont::AMSa,
            amssymb::SymbolFont::Msbm => SymbolFont::AMSb,
        };
        let class = match a.class {
            amssymb::SymbolClass::Ord => SymbolClass::Ord,
            amssymb::SymbolClass::Bin => SymbolClass::Bin,
            amssymb::SymbolClass::Rel => SymbolClass::Rel,
            amssymb::SymbolClass::Open => SymbolClass::Open,
            amssymb::SymbolClass::Close => SymbolClass::Close,
        };
        let rows: Vec<_> = declarations(a.name)
            .filter(|s| matches!(s.provider, Provider::Amsfonts | Provider::Amssymb))
            .collect();
        assert!(!rows.is_empty(), "amssymb row \\{} has no AMS declaration", a.name);
        let hit = rows.iter().find(|s| s.font == font && s.slot == a.slot);
        assert!(
            hit.is_some(),
            "\\{}: amssymb table {:?} {:#04x}, declarations {:?}",
            a.name,
            a.font,
            a.slot,
            rows.iter().map(|s| (s.font, s.slot, s.source)).collect::<Vec<_>>()
        );
        let s = hit.unwrap();
        assert_eq!(s.class, class, "\\{} class", a.name);
        assert_eq!(s.text, a.text, "\\{} text", a.name);
        assert!((s.width_em - a.width_em).abs() < 1e-6, "\\{} width", a.name);
    }
    for (alias, target) in amssymb::ALIASES {
        assert!(
            math_symbols::aliases_of(alias).any(|(t, _)| t == *target),
            "amssymb alias \\{alias} -> \\{target} missing from ALIASES"
        );
    }
}

#[test]
fn declared_classes_cover_every_atom_class_the_engine_spaces() {
    // Sanity: the mapping is total and the kernel uses every class but Punct
    // only through `\ldotp`/`\cdotp`/`\colon`/`,`/`;`.
    let used: Vec<AtomClass> = SYMBOLS.iter().map(|s| s.class.atom_class()).collect();
    for c in [AtomClass::Ord, AtomClass::Op, AtomClass::Bin, AtomClass::Rel, AtomClass::Open, AtomClass::Close, AtomClass::Punct] {
        assert!(used.contains(&c), "no declaration of class {c:?}");
    }
}
