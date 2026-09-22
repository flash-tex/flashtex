//! Pins the engine assumptions `examples/real_sty_harness.rs` relies on to
//! measure real `.sty` files (that example needs a TeX Live installation,
//! so it cannot run here; these tests use synthetic in-memory packages and
//! the same two-run `\ifcsname` probe technique on them).
//!
//! - An unknown command produces NO diagnostic: it passes through to the
//!   output untouched, so the probe must read the output, not diagnostics.
//! - One `\ifcsname <name>\endcsname\else <marker>\fi` per candidate (a
//!   space separates `\ifcsname` from the name) classifies exactly the
//!   names still undefined after the load: engine-known tokens emitted by
//!   design and package-defined names stay unflagged, genuinely unknown
//!   names (letters, `@`-names and symbolic names alike) get flagged.

use std::collections::BTreeSet;
use std::rc::Rc;

use flashtex_tex_expansion::{tokens_to_display_string, Engine, TokenKind};

const BEGIN_MARK: &str = "HARNESS-UNMODELLED-BEGIN";
const END_MARK: &str = "HARNESS-UNMODELLED-END";
const DONE_MARK: &str = "HARNESS-PROBE-DONE";

fn engine_with(src: &str, files: &[(&str, &str)]) -> Engine {
    let owned: Vec<(String, String)> = files.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect();
    let mut engine = Engine::new(src);
    engine.set_package_reader(Rc::new(move |name, ext| {
        owned.iter().find(|(n, _)| *n == format!("{name}.{ext}")).map(|(_, t)| t.clone())
    }));
    engine
}

/// Distinct control sequences emitted from package-file sources.
fn candidates(tokens: &[flashtex_tex_expansion::Token], engine: &Engine) -> BTreeSet<String> {
    let sids: BTreeSet<u32> = engine.opened_package_files().iter().map(|f| f.source_id).collect();
    tokens
        .iter()
        .filter(|t| matches!(t.kind, TokenKind::ControlSequence(_)) && sids.contains(&t.span.source_id))
        .filter_map(|t| match &t.kind {
            TokenKind::ControlSequence(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

/// The harness's run-2 probe: tail + marker parsing. Returns the flagged
/// names and whether the sentinel was present.
fn probe(src: &str, files: &[(&str, &str)], names: &BTreeSet<String>) -> (BTreeSet<String>, bool) {
    let mut tail = String::from("\\relax ");
    let mut ordered: Vec<String> = names.iter().cloned().collect();
    ordered.sort();
    for name in &ordered {
        tail.push_str(&format!("\\ifcsname {name}\\endcsname\\else {BEGIN_MARK}{name}{END_MARK}\\fi "));
    }
    tail.push_str(DONE_MARK);
    let full = format!("{src}\n{tail}");
    let mut engine = engine_with(&full, files);
    let out = tokens_to_display_string(&engine.run());
    let mut flagged = BTreeSet::new();
    for part in out.split(BEGIN_MARK).skip(1) {
        if let Some(end) = part.find(END_MARK) {
            flagged.insert(part[..end].to_string());
        }
    }
    (flagged, out.contains(DONE_MARK))
}

#[test]
fn unknown_commands_pass_through_silently() {
    let mut engine = Engine::new("\\nosuchcmdZZZ");
    let tokens = engine.run();
    let diags = engine.take_diagnostics();
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(tokens_to_display_string(&tokens), "\\nosuchcmdZZZ ");
}

#[test]
fn ifcsname_tail_flags_only_names_undefined_after_the_load() {
    let sty = "\\ProvidesPackage{syn}\n\\newcommand{\\synmacro}{X}\n\\synmacro\\par\\nosuchcmdZZZ\n";
    let src = "\\documentclass{article}\n\\usepackage{syn}\n";
    let mut engine = engine_with(src, &[("syn.sty", sty)]);
    let tokens = engine.run();
    assert!(engine.take_diagnostics().is_empty());
    let cands = candidates(&tokens, &engine);
    assert_eq!(cands, BTreeSet::from(["par".to_string(), "nosuchcmdZZZ".to_string()]));
    // `\synmacro` was consumed by expansion (never emitted) but is defined
    // by the package; probing it as well must still leave it unflagged,
    // while engine-known `\par` and the controls stay unflagged too.
    let mut probed = cands.clone();
    probed.insert("synmacro".to_string());
    for c in ["par", "relax", "usepackage", "@empty"] {
        probed.insert(c.to_string());
    }
    let (flagged, done) = probe(src, &[("syn.sty", sty)], &probed);
    assert!(done);
    assert_eq!(flagged, BTreeSet::from(["nosuchcmdZZZ".to_string()]));
}

#[test]
fn symbolic_names_survive_the_csname_probe() {
    let sty = "\\ProvidesPackage{sym}\n\\{\n";
    let src = "\\usepackage{sym}\n";
    let mut engine = engine_with(src, &[("sym.sty", sty)]);
    let tokens = engine.run();
    assert!(engine.take_diagnostics().is_empty());
    let cands = candidates(&tokens, &engine);
    assert_eq!(cands, BTreeSet::from(["{".to_string()]));
    let (flagged, done) = probe(src, &[("sym.sty", sty)], &cands);
    assert!(done);
    assert_eq!(flagged, BTreeSet::from(["{".to_string()]));
}
