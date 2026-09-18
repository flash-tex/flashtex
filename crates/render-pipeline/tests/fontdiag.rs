//! The font-diagnostic classification is the render pipeline's truth.
//!
//! 1. `supported/font-diagnostics.json` equals what `fontdiag::render_json()`
//!    prints now, so the copy the shell and Python gates read cannot go stale.
//! 2. Every diagnostic code `src/` actually emits is classified — either as a
//!    font diagnostic or explicitly as not one. A new code that nobody has
//!    classified fails here instead of silently landing outside every gate.
//! 3. Nothing is classified that is not emitted, so the table cannot rot into
//!    a list of codes that no longer exist.
//!
//! (2) is the check that matters: the bug this file exists to prevent is a
//! code being emitted that a hand-maintained gate list has never heard of.

use flashtex_render_pipeline::fontdiag::{
    self, FONT_DIAGNOSTICS, NON_FONT_DIAGNOSTICS,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

const REGENERATE: &str =
    "regenerate with: cargo run --release --manifest-path crates/render-pipeline/Cargo.toml \
     --bin flashtex-render -- --font-diagnostics json > crates/render-pipeline/supported/font-diagnostics.json";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `Diagnostic::warning|error|note("code"` literal in `src/`, with where
/// it came from. Each file is truncated at its `#[cfg(test)]` module so that
/// codes fabricated in unit-test fixtures are not mistaken for emissions —
/// every file in this crate has at most one, always trailing.
fn emitted_codes() -> BTreeSet<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).expect("read src/").flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&crate_dir().join("src"), &mut files);
    files.sort();

    let mut found = BTreeSet::new();
    for f in files {
        let text = std::fs::read_to_string(&f).expect("read source");
        let body = match text.find("#[cfg(test)]") {
            Some(i) => &text[..i],
            None => &text[..],
        };
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        for ctor in ["Diagnostic::warning(", "Diagnostic::error(", "Diagnostic::note("] {
            let mut at = 0;
            while let Some(i) = body[at..].find(ctor) {
                let start = at + i + ctor.len();
                at = start;
                // The code is the first argument: the next string literal,
                // provided only whitespace separates it from the paren.
                let rest = body[start..].trim_start();
                let Some(lit) = rest.strip_prefix('"') else { continue };
                let Some(end) = lit.find('"') else { continue };
                let code = &lit[..end];
                if !code.is_empty()
                    && code.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                {
                    found.insert((code.to_string(), name.clone()));
                }
            }
        }
    }
    assert!(
        found.len() > 20,
        "the source scan found only {} codes; the extractor is broken, not the source",
        found.len()
    );
    found
}

fn classified() -> BTreeSet<String> {
    FONT_DIAGNOSTICS
        .iter()
        .map(|d| d.code.to_string())
        .chain(NON_FONT_DIAGNOSTICS.iter().map(|c| c.to_string()))
        .collect()
}

#[test]
fn generated_json_is_current() {
    let path = crate_dir().join("supported/font-diagnostics.json");
    let on_disk = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}; {REGENERATE}", path.display()));
    assert!(
        on_disk == fontdiag::render_json(),
        "{} is stale; {REGENERATE}",
        path.display()
    );
}

#[test]
fn every_emitted_code_is_classified() {
    let known = classified();
    let mut unclassified: Vec<String> = Vec::new();
    for (code, file) in emitted_codes() {
        if !known.contains(&code) {
            unclassified.push(format!("{code} (emitted in src/{file})"));
        }
    }
    assert!(
        unclassified.is_empty(),
        "these diagnostic codes are emitted but classified in neither \
         FONT_DIAGNOSTICS nor NON_FONT_DIAGNOSTICS, so no acceptance gate counts them:\n  {}\n\
         Add each to crates/render-pipeline/src/fontdiag.rs — as a font diagnostic with its \
         substitution/geometry_void flags, or to NON_FONT_DIAGNOSTICS — then {REGENERATE}",
        unclassified.join("\n  ")
    );
}

#[test]
fn nothing_is_classified_that_is_not_emitted() {
    let emitted: BTreeSet<String> = emitted_codes().into_iter().map(|(c, _)| c).collect();
    let stale: Vec<String> = classified()
        .into_iter()
        .filter(|c| !emitted.contains(c))
        .collect();
    assert!(
        stale.is_empty(),
        "classified but no longer emitted anywhere in src/: {stale:?}; \
         remove them from crates/render-pipeline/src/fontdiag.rs and {REGENERATE}"
    );
}

#[test]
fn the_two_views_are_what_the_gates_need() {
    let subst: BTreeSet<&str> = fontdiag::substitution_codes().collect();
    let geom: BTreeSet<&str> = fontdiag::geometry_void_codes().collect();

    // The gap that started this: texmf-acceptance.sh counted only three codes.
    for c in ["ec_metrics_unavailable", "font_outline_substituted", "math_font_unavailable"] {
        assert!(subst.contains(c), "{c} must count as a missing font resource");
    }
    // A voided-geometry code is always a substitution; the converse is not
    // true, and font_outline_substituted is exactly why.
    assert!(geom.is_subset(&subst), "geometry_void must imply substitution");
    assert!(
        subst.contains("font_outline_substituted") && !geom.contains("font_outline_substituted"),
        "a substituted outline keeps the reference metrics, so positions still compare"
    );
    assert!(
        !subst.contains("font_shape_substituted"),
        "NFSS shape substitution is what pdflatex does too; it is not a missing resource"
    );
}

#[test]
fn notes_need_no_escaping() {
    for d in &FONT_DIAGNOSTICS {
        assert!(
            !d.note.contains('"') && !d.note.contains('\\'),
            "{}: note must stay free of quotes and backslashes, render_json does not escape",
            d.code
        );
        assert!(!d.note.is_empty(), "{}: every classification needs a reason", d.code);
    }
}

#[test]
fn generated_json_parses() {
    flashtex_compiler::json::parse(&fontdiag::render_json()).expect("valid JSON");
}
