//! Pinned non-regression gate over the project's own compatibility corpus.
//!
//! `tests/tex-corpus` (FT-011) is the project's statement of what the compiler
//! must eventually handle. This gate pins what it does TODAY: every case's exact
//! status and whether it produced positioned output. A change that improves a
//! case fails this test, which is intended — the expectation is updated
//! deliberately, with the improvement visible in the diff, instead of a number
//! quietly drifting between commits.
//!
//! It also enforces the rule that matters most for a live preview: no corpus
//! case may fail silently, hang, or return no reply.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;
use std::path::{Path, PathBuf};

/// (case id, expected status, must produce positioned output)
///
/// Pinned 2026-09-12. `recovered` entries are features that are honestly
/// unimplemented, each with its diagnostic asserted below, not silent failures.
const PINNED: &[(&str, &str, bool)] = &[
    ("plain-paragraphs", "ok", true),
    ("macro-arguments", "ok", true),
    ("macro-scope", "ok", true),
    // Times-Roman has no Japanese glyphs; shaping now reports that explicitly.
    ("unicode-literals", "recovered", true),
    ("math-inline-display", "ok", true),
    ("included-file", "ok", true),
    ("include-scope", "ok", true),
    ("comments-escapes", "ok", true),
    // The literal CJK glyph is absent from Times-Roman and is now diagnosed.
    ("literal-source-map", "recovered", true),
    ("unknown-command", "recovered", true),
    ("unclosed-group", "recovered", true),
    ("extra-closing-group", "recovered", true),
    ("missing-include", "recovered", true),
    ("tikz-required", "recovered", true),
];

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/tex-corpus")
}

fn manifest() -> Value {
    let raw = std::fs::read_to_string(corpus_root().join("manifest.json"))
        .expect("corpus manifest must exist");
    json::parse(&raw).expect("corpus manifest must be valid JSON")
}

fn cases() -> Vec<Value> {
    match manifest().get("cases") {
        Some(Value::Arr(a)) => a.clone(),
        Some(Value::Obj(m)) => m.values().cloned().collect(),
        _ => panic!("corpus manifest has no cases"),
    }
}

fn compile_case(case: &Value) -> Value {
    let id = case.get("id").and_then(|v| v.as_str()).unwrap().to_string();
    let entry = case.get("entry_path").and_then(|v| v.as_str()).unwrap();
    let dir = corpus_root().join("cases").join(&id);

    let docs: Vec<Value> = case
        .get("documents")
        .and_then(|v| v.as_arr())
        .expect("case documents")
        .iter()
        .map(|d| {
            let rel = d.as_str().expect("document path");
            // Never default to empty: a gate that silently compiles an empty
            // document passes every assertion while testing nothing. This bit
            // me — two cases were reported ok while their real text was never
            // read, because the entry path and the document path differ.
            let full = dir.join(rel);
            let text = std::fs::read_to_string(&full).unwrap_or_else(|e| {
                panic!("corpus case {id}: cannot read {}: {e}", full.display())
            });
            let mut doc = Value::obj();
            doc.set("path", json::str_(rel));
            doc.set("text", json::str_(text));
            doc
        })
        .collect();

    let mut payload = Value::obj();
    payload.set("project_id", json::str_(id.clone()));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(entry));
    payload.set("documents", Value::Arr(docs));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_(id));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);

    json::parse(&handle_line(&json::write(&env))).expect("reply must be valid JSON")
}

#[test]
fn every_corpus_case_matches_its_pinned_result() {
    let cases = cases();
    assert_eq!(
        cases.len(),
        PINNED.len(),
        "the corpus gained or lost cases; update PINNED deliberately"
    );

    for (id, expected_status, expect_output) in PINNED {
        let case = cases
            .iter()
            .find(|c| c.get("id").and_then(|v| v.as_str()) == Some(*id))
            .unwrap_or_else(|| panic!("corpus case {id} is missing"));

        let reply = compile_case(case);
        assert_eq!(
            reply.get("type").and_then(|v| v.as_str()),
            Some("compile_result"),
            "{id}: must produce a compile_result, never an error envelope or silence"
        );

        let payload = reply.get("payload").unwrap();
        let status = payload.get("status").and_then(|v| v.as_str()).unwrap();
        assert_eq!(
            status, *expected_status,
            "{id}: status changed. If this is an improvement, update PINNED in the same commit"
        );

        let items: usize = payload
            .get("pages")
            .and_then(|v| v.as_arr())
            .map(|pages| {
                pages
                    .iter()
                    .map(|p| {
                        p.get("items")
                            .and_then(|i| i.as_arr())
                            .map_or(0, |i| i.len())
                    })
                    .sum()
            })
            .unwrap_or(0);
        assert_eq!(
            items > 0,
            *expect_output,
            "{id}: positioned output presence changed (got {items} items)"
        );

        // Recovery must always be explained, never silent.
        if status == "recovered" {
            let diags = payload
                .get("diagnostics")
                .and_then(|v| v.as_arr())
                .cloned()
                .unwrap_or_default();
            assert!(
                !diags.is_empty(),
                "{id}: recovered without saying why — that is a silent failure"
            );
        }
    }
}

#[test]
fn every_corpus_case_span_slices_its_own_document() {
    for case in cases() {
        let id = case.get("id").and_then(|v| v.as_str()).unwrap().to_string();
        let dir = corpus_root().join("cases").join(&id);
        let reply = compile_case(&case);
        let payload = reply.get("payload").unwrap();

        for page in payload
            .get("pages")
            .and_then(|v| v.as_arr())
            .cloned()
            .unwrap_or_default()
        {
            for item in page
                .get("items")
                .and_then(|v| v.as_arr())
                .cloned()
                .unwrap_or_default()
            {
                let src = item.get("source").unwrap();
                let path = src.get("path").and_then(|v| v.as_str()).unwrap();
                let a = src.get("start_byte").unwrap().as_i64().unwrap() as usize;
                let b = src.get("end_byte").unwrap().as_i64().unwrap() as usize;
                let text = std::fs::read_to_string(dir.join(path))
                    .unwrap_or_else(|e| panic!("{id}: cannot read {path} to verify a span: {e}"));
                assert!(
                    text.get(a..b).is_some(),
                    "{id}: span {a}..{b} in {path} does not slice that document"
                );
            }
        }
    }
}
