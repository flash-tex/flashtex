//! The generated kernel slot table (`cm_slots`, from `fontmath.ltx` via
//! `crates/compiler/scripts/gen_math_symbols.py`) against the hand-written
//! `cm::symbol_slot` / `cm::delimiter_slot` matches. TeX never runs here.
//!
//! A character the hand table maps to a different (family, slot) than the
//! declaration is a latent bug and is listed by name below so that it is
//! reviewed rather than hidden; a listed item that stops disagreeing fails
//! the test too. Characters the hand table does not map at all are reported
//! as a count (they are what the switch-over supplies).

use flashtex_math_layout::cm::{delimiter_slot, symbol_slot, Family};
use flashtex_math_layout::cm_slots::{DECLARED_DELIMITERS, DECLARED_SLOTS, SHARED_TEXT};
use std::collections::BTreeSet;

/// (command, reason) for every declared character the hand table maps elsewhere.
const KNOWN_SLOT: &[(&str, &str)] = &[];

/// Declared delimiters the hand `delimiter_slot` maps elsewhere or lacks.
const KNOWN_DELIMITER: &[(&str, &str)] = &[];

fn family_name(f: Family) -> &'static str {
    match f {
        Family::Roman => "cmr",
        Family::Italic => "cmmi",
        Family::Symbol => "cmsy",
        Family::Extension => "cmex",
    }
}

#[test]
fn hand_symbol_slots_agree_with_the_declarations() {
    let known: BTreeSet<&str> = KNOWN_SLOT.iter().map(|(n, _)| *n).collect();
    let mut wrong = Vec::new();
    let mut unmapped = Vec::new();
    let mut agreeing = 0;
    for &(ch, family, slot, name) in DECLARED_SLOTS {
        match symbol_slot(ch) {
            Some((f, s)) if (f, s) == (family, slot) => agreeing += 1,
            Some((f, s)) => wrong.push((
                name,
                format!(
                    "{ch:?} (\\{name}): hand {} {s:#04x}, declared {} {slot:#04x}",
                    family_name(f),
                    family_name(family)
                ),
            )),
            None => unmapped.push(name),
        }
    }
    let unexpected: Vec<_> = wrong.iter().filter(|(n, _)| !known.contains(n)).map(|(_, d)| d).collect();
    let resolved: Vec<_> = known.iter().filter(|n| !wrong.iter().any(|(w, _)| w == *n)).collect();
    eprintln!(
        "cm_slots: {agreeing} agree, {} disagree, {} unmapped by the hand table: {unmapped:?}",
        wrong.len(),
        unmapped.len()
    );
    assert!(unexpected.is_empty(), "unlisted slot disagreements:\n{}", unexpected.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    assert!(resolved.is_empty(), "listed disagreements no longer disagree: {resolved:?}");
    assert!(agreeing > 150, "the hand table should agree on most characters ({agreeing})");
}

#[test]
fn shared_text_rows_name_a_different_slot_than_the_first_declaration() {
    for &(ch, family, slot, name) in SHARED_TEXT {
        let first = DECLARED_SLOTS.iter().find(|(c, ..)| *c == ch);
        assert!(first.is_some(), "{ch:?} (\\{name}) has no first declaration");
        let (_, f, s, _) = first.unwrap();
        assert_ne!((*f, *s), (family, slot), "{ch:?} (\\{name}) is not actually shared");
    }
}

#[test]
fn hand_delimiter_slots_agree_with_the_declarations() {
    let known: BTreeSet<&str> = KNOWN_DELIMITER.iter().map(|(n, _)| *n).collect();
    let mut wrong = Vec::new();
    let mut unmapped = Vec::new();
    for &(ch, family, slot, large, name) in DECLARED_DELIMITERS {
        match delimiter_slot(ch) {
            Some(((f, s), l)) if (f, s, l) == (family, slot, large) => {}
            Some(((f, s), l)) => wrong.push((
                name,
                format!(
                    "{ch:?} (\\{name}): hand {} {s:#04x} large {l:#04x}, declared {} {slot:#04x} large {large:#04x}",
                    family_name(f),
                    family_name(family)
                ),
            )),
            None => unmapped.push(name),
        }
    }
    eprintln!("cm_slots delimiters: {} disagree, {} unmapped by the hand table: {unmapped:?}", wrong.len(), unmapped.len());
    let unexpected: Vec<_> = wrong.iter().filter(|(n, _)| !known.contains(n)).map(|(_, d)| d.as_str()).collect();
    let resolved: Vec<_> = known.iter().filter(|n| !wrong.iter().any(|(w, _)| w == *n)).collect();
    assert!(unexpected.is_empty(), "unlisted delimiter disagreements:\n{}", unexpected.join("\n"));
    assert!(resolved.is_empty(), "listed disagreements no longer disagree: {resolved:?}");
}
