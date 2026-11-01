//! Producer gate for `display-list-v2-delta` (proposal r5 §10.1): over an
//! edit sequence on a multi-page document, every `display_list_delta` line
//! the worker emits reconstructs — base pages relocated exactly as §5.2,
//! changed pages as sent — to the byte-identical full `display_list` line a
//! fresh compile emits; `page_bytes`, `page_digests` and `list_digest` are
//! verified as the consumer verifies them; and the opt-in capabilities are
//! echoed only on the replies that honour them.

mod common;

use common::lm_available;
use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::delta::{self, DeltaState};
use flashtex_render_pipeline::display::{self, Wire};
use flashtex_render_pipeline::protocol::{handle_line, handle_line_with};
use flashtex_render_pipeline::{FontSet, RenderCache, RenderOptions};

fn document(sections: usize) -> String {
    let mut s = String::from("\\documentclass{article}\n\\begin{document}\n\\tableofcontents\n");
    for i in 1..=sections {
        s.push_str(&format!("\\section{{Section {i}}}\n"));
        for j in 1..=3 {
            s.push_str(&format!(
                "Paragraph {i}.{j} has a sentence with some words in it, then an inline formula $x_{{{i}}} + y^{j} = z$ and more words so that lines wrap and pages fill up over time.\n\n"
            ));
        }
    }
    s.push_str("\\end{document}\n");
    s
}

fn edited(base: &str, i: usize) -> String {
    let mut s = base.to_string();
    let needle = format!("Paragraph {}.{} has", 1 + i % 12, 1 + i % 3);
    let k = s.find(&needle).unwrap_or(0);
    match i % 5 {
        0 => s.insert_str(k, "Inserted "),
        1 => s.insert_str(k, &"x".repeat(1 + i % 7)),
        2 => {
            if let Some(e) = s[k..].find(' ') {
                s.replace_range(k..k + e, "Replaced");
            }
        }
        3 => s.insert_str(k, "\\section{Extra}\n"),
        _ => s.insert_str(k, &format!(" {}", "word ".repeat(1 + i % 4))),
    }
    s
}

fn request(id: &str, revision: u64, text: &str, caps: &[&str], base: Option<&delta::Base>) -> String {
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("delta"));
    payload.set("revision", json::num(revision as f64));
    payload.set("entry_path", json::str_("main.tex"));
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    payload.set("documents", Value::Arr(vec![doc]));
    payload.set("layout_capabilities", Value::Arr(caps.iter().map(|c| json::str_(*c)).collect()));
    if let Some(b) = base {
        let mut o = Value::obj();
        o.set("request_id", json::str_(&b.request_id));
        o.set("project_id", json::str_(&b.project_id));
        o.set("revision", json::num(b.revision as f64));
        o.set("page_count", json::num(b.page_count as f64));
        o.set("list_digest", json::str_(&b.list_digest));
        payload.set("display_list_base", o);
    }
    let mut v = Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(id));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v)
}

fn accepted(line: &str) -> Vec<String> {
    json::parse(line)
        .unwrap()
        .get("payload")
        .and_then(|p| p.get("layout_capabilities"))
        .and_then(|a| a.as_arr().map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect()))
        .unwrap_or_default()
}

fn v1_pages(line: &str) -> usize {
    json::parse(line).unwrap().get("payload").unwrap().get("pages").unwrap().as_arr().unwrap().len()
}

const FULL: &[&str] = &["rules-v1", "font-hints-v1", "display-list-v2"];
const DELTA: &[&str] = &["rules-v1", "font-hints-v1", "display-list-v2", "display-list-v2-delta"];

/// The exact-size formula of the consumer (`fullLineBytes`): fixed framing +
/// the delta line's own header parts + Σ page_bytes + separators.
fn full_line_bytes_from_delta(payload: &Value, id: &str, page_bytes: &[usize]) -> usize {
    let part = |k: &str| json::write(payload.get(k).unwrap()).len();
    display::FULL_LINE_FRAME_BYTES
        + json::write(&json::str_(id)).len()
        + part("project_id")
        + part("revision")
        + part("required_features")
        + part("documents")
        + part("fonts")
        + part("diagnostics")
        + page_bytes.iter().sum::<usize>()
        + page_bytes.len().saturating_sub(1)
}

#[test]
fn deltas_reconstruct_to_the_fresh_full_line_over_an_edit_sequence() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let cache = RenderCache::new();
    let state = DeltaState::new();
    let wire = Wire::default();
    let base_doc = document(40);
    let mut previous_pages: Vec<display::Page> = Vec::new();
    let (mut deltas, mut fulls, mut delta_bytes, mut full_bytes) = (0usize, 0usize, 0usize, 0usize);
    let mut text = base_doc.clone();
    let mut previous_text = String::new();
    for i in 0..60 {
        // Cumulative edits (typing) with an occasional jump back to the base
        // (an undo/paste: two distant edits in one relocation window).
        text = if i % 9 == 8 { edited(&base_doc, i) } else { edited(&text, i) };
        let id = format!("r{i}");
        let base = state.acknowledgement();
        let req = request(&id, i as u64 + 1, &text, DELTA, base.as_ref());
        let reply = handle_line_with(&req, &fonts, &options, Some(&cache), Some(&state));
        let fresh = handle_line(&request(&id, i as u64 + 1, &text, FULL, None), &fonts, &options, None);
        let fresh_line = &fresh.extra_lines[0];
        assert_eq!(reply.extra_lines.len(), 1, "edit {i}: one sibling line");
        let sibling = &reply.extra_lines[0];
        let env = json::parse(sibling).unwrap();
        let rendered = reply.rendered.as_ref().unwrap();
        let echoed = accepted(&reply.line);
        full_bytes += fresh_line.len();
        if env.get("type").unwrap().as_str() == Some("display_list_delta") {
            assert!(i > 0, "the first request has no base");
            assert!(echoed.iter().any(|c| c == delta::CAP), "edit {i}: delta emitted but not echoed");
            deltas += 1;
            delta_bytes += sibling.len();
            let payload = env.get("payload").unwrap();
            assert_eq!(payload.get("digest_scheme").unwrap().as_str(), Some(delta::DIGEST_SCHEME));
            let b = payload.get("base").unwrap();
            let base = base.unwrap();
            assert_eq!(b.get("request_id").unwrap().as_str(), Some(base.request_id.as_str()));
            assert_eq!(b.get("list_digest").unwrap().as_str(), Some(base.list_digest.as_str()));
            assert_eq!(b.get("page_count").unwrap().as_i64(), Some(base.page_count as i64));
            let n = payload.get("page_count").unwrap().as_i64().unwrap() as usize;
            assert_eq!(n, rendered.v2.pages.len());
            let relocs: Vec<delta::Relocation> = payload
                .get("relocations")
                .unwrap()
                .as_arr()
                .unwrap()
                .iter()
                .map(|r| delta::Relocation {
                    path: r.get("path").unwrap().as_str().unwrap().to_string(),
                    edit_start: r.get("edit_start").unwrap().as_i64().unwrap() as usize,
                    edit_end: r.get("edit_end").unwrap().as_i64().unwrap() as usize,
                    delta: r.get("delta").unwrap().as_i64().unwrap() as isize,
                })
                .collect();
            assert!(relocs.len() <= 1);
            let page_bytes: Vec<usize> = payload.get("page_bytes").unwrap().as_arr().unwrap().iter().map(|v| v.as_i64().unwrap() as usize).collect();
            let page_digests: Vec<String> = payload.get("page_digests").unwrap().as_arr().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
            assert_eq!(page_bytes.len(), n);
            assert_eq!(page_digests.len(), n);
            let removed: Vec<i64> = payload.get("removed_pages").unwrap().as_arr().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
            let expected_removed: Vec<i64> = (n as i64 + 1..=previous_pages.len() as i64).collect();
            assert_eq!(removed, expected_removed, "edit {i}");
            // Changed page objects exactly as they sit on the wire: re-split the
            // `changed_pages` array text by re-serialising each parsed page is
            // not byte-exact, so take the objects from the producer's writer
            // and check the wire array parses to the same values.
            let changed: Vec<&Value> = payload.get("changed_pages").unwrap().as_arr().unwrap().iter().collect();
            let mut objects: Vec<String> = Vec::with_capacity(n);
            let mut ci = 0;
            let mut last_number = 0;
            for (pi, new_page) in rendered.v2.pages.iter().enumerate() {
                let number = pi as i64 + 1;
                let is_changed = changed.get(ci).and_then(|c| c.get("number")).and_then(Value::as_i64) == Some(number);
                let page_model = if is_changed {
                    assert!(number > last_number, "edit {i}: changed pages ascending");
                    last_number = number;
                    let obj = json::write(changed[ci]);
                    ci += 1;
                    // The wire object must be the writer's bytes for this page.
                    let mut expect = String::new();
                    display::write_page(&mut expect, new_page, wire);
                    assert_eq!(json::parse(&expect).unwrap(), json::parse(&obj).unwrap(), "edit {i}: changed page {number} value");
                    assert_eq!(page_bytes[pi], expect.len(), "edit {i}: page_bytes of changed page {number}");
                    objects.push(expect);
                    new_page.clone()
                } else {
                    assert!(pi < previous_pages.len(), "edit {i}: unchanged page {number} must be within the base");
                    let moved = delta::relocate_page(&previous_pages[pi], &relocs).unwrap_or_else(|| panic!("edit {i}: page {number} relocation invalid"));
                    let mut o = String::new();
                    display::write_page(&mut o, &moved, wire);
                    assert_eq!(page_bytes[pi], o.len(), "edit {i}: page_bytes of unchanged page {number}");
                    objects.push(o);
                    moved
                };
                assert_eq!(flashtex_font_engine::sha256::hex(&delta::page_digest(&page_model, wire)), page_digests[pi], "edit {i}: page {number} digest");
            }
            assert_eq!(ci, changed.len(), "edit {i}: every changed page consumed");
            let reconstructed = rendered.v2.write_json_wire_with_page_objects(&id, wire, &objects);
            assert!(reconstructed == *fresh_line, "edit {i}: reconstructed line differs from the fresh full line ({} vs {} bytes)", reconstructed.len(), fresh_line.len());
            assert_eq!(full_line_bytes_from_delta(payload, &id, &page_bytes), fresh_line.len(), "edit {i}: exact size formula");
            assert!(sibling.len() * 4 <= fresh_line.len() * 3, "edit {i}: policy");
            // The header parts on the delta are the full line's.
            let fresh_payload = json::parse(fresh_line).unwrap();
            let fresh_payload = fresh_payload.get("payload").unwrap();
            for k in ["documents", "fonts", "diagnostics", "required_features", "project_id", "revision"] {
                assert_eq!(payload.get(k), fresh_payload.get(k), "edit {i}: {k}");
            }
        } else {
            assert_eq!(env.get("type").unwrap().as_str(), Some("display_list"));
            assert!(!echoed.iter().any(|c| c == delta::CAP), "edit {i}: full line must not echo -delta");
            assert!(sibling == fresh_line, "edit {i}: full line differs from the fresh line");
            fulls += 1;
            if i > 0 {
                let relocs: Vec<delta::Relocation> = delta::Relocation::diff("main.tex", &previous_text, &text).into_iter().collect();
                let unchanged = rendered.v2.pages.iter().enumerate().filter(|(pi, p)| previous_pages.get(*pi).is_some_and(|b| delta::unchanged_after_relocation(b, p, &relocs, wire).is_some())).count();
                eprintln!("edit {i} ({}): full line; {unchanged} of {} pages unchanged after relocation {relocs:?}", i % 5, rendered.v2.pages.len());
            }
        }
        previous_pages = rendered.v2.pages.clone();
        previous_text = text.clone();
    }
    eprintln!("{deltas} deltas ({delta_bytes} B) and {fulls} full lines ({full_bytes} B fresh total) over 60 edits");
    // Section renumbering and page-break cascades legitimately change every
    // page of this dense document on some edits; the policy then answers full.
    assert!(deltas >= 30, "most edits should be answered with a delta: {deltas} deltas, {fulls} full");
}

#[test]
fn delta_is_never_accepted_without_a_matching_base() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let state = DeltaState::new();
    let text = document(3);
    // No base: full line, -delta not echoed, a snapshot is retained.
    let r = handle_line_with(&request("a", 1, &text, DELTA, None), &fonts, &options, None, Some(&state));
    assert!(!accepted(&r.line).iter().any(|c| c == delta::CAP));
    assert!(state.has_snapshot());
    let ack = state.acknowledgement().unwrap();
    // A stale/unknown base: full line, snapshot replaced.
    let wrong = delta::Base { request_id: "zzz".into(), ..ack.clone() };
    let r = handle_line_with(&request("b", 2, &text, DELTA, Some(&wrong)), &fonts, &options, None, Some(&state));
    assert!(!accepted(&r.line).iter().any(|c| c == delta::CAP));
    assert_eq!(json::parse(&r.extra_lines[0]).unwrap().get("type").unwrap().as_str(), Some("display_list"));
    assert_eq!(state.acknowledgement().unwrap().request_id, "b");
    // Without delta state (the stateless entry point): never accepted.
    let r = handle_line(&request("c", 3, &text, DELTA, Some(&ack)), &fonts, &options, None);
    assert!(!accepted(&r.line).iter().any(|c| c == delta::CAP));
    // A reply without a sibling (display-list-v2 not requested) clears the chain.
    let r = handle_line_with(&request("d", 4, &text, &["rules-v1"], None), &fonts, &options, None, Some(&state));
    assert!(r.extra_lines.is_empty());
    assert!(!state.has_snapshot());
    // -delta without display-list-v2 is never accepted.
    let r = handle_line_with(&request("e", 5, &text, &["rules-v1", "display-list-v2-delta"], None), &fonts, &options, None, Some(&state));
    assert!(!accepted(&r.line).iter().any(|c| c == delta::CAP));
}

#[test]
fn v2_only_elides_v1_pages_exactly_when_a_sibling_carries_the_frame() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let state = DeltaState::new();
    let text = document(2);
    let with = ["rules-v1", "font-hints-v1", "display-list-v2", "display-list-v2-only"];
    let r = handle_line_with(&request("a", 1, &text, &with, None), &fonts, &options, None, Some(&state));
    let acc = accepted(&r.line);
    assert!(acc.iter().any(|c| c == "display-list-v2-only"), "{acc:?}");
    assert_eq!(v1_pages(&r.line), 0, "pages elided");
    assert_eq!(r.extra_lines.len(), 1);
    let full = handle_line(&request("a", 1, &text, FULL, None), &fonts, &options, None);
    assert!(v1_pages(&full.line) > 0);
    assert!(full.extra_lines[0] == r.extra_lines[0], "the sibling is unchanged by -only");
    // Diagnostics/status/revision are kept.
    let p = json::parse(&r.line).unwrap();
    let p = p.get("payload").unwrap().clone();
    let fp = json::parse(&full.line).unwrap();
    assert_eq!(p.get("status"), fp.get("payload").unwrap().get("status"));
    assert_eq!(p.get("diagnostics"), fp.get("payload").unwrap().get("diagnostics"));
    assert_eq!(p.get("revision").unwrap().as_i64(), Some(1));
    // Without display-list-v2 the capability is not accepted and pages stay.
    let r = handle_line_with(&request("b", 2, &text, &["rules-v1", "display-list-v2-only"], None), &fonts, &options, None, Some(&state));
    assert!(!accepted(&r.line).iter().any(|c| c == "display-list-v2-only"));
    assert!(v1_pages(&r.line) > 0);
    // With both -delta and -only, a delta reply also elides the pages.
    let both = ["rules-v1", "font-hints-v1", "display-list-v2", "display-list-v2-delta", "display-list-v2-only"];
    let r = handle_line_with(&request("c", 3, &text, &both, None), &fonts, &options, None, Some(&state));
    let ack = state.acknowledgement().unwrap();
    let edited_text = text.replacen("Paragraph 2.2", "Paragraph 2.2 edited", 1);
    let r2 = handle_line_with(&request("d", 4, &edited_text, &both, Some(&ack)), &fonts, &options, None, Some(&state));
    let acc = accepted(&r2.line);
    assert!(acc.iter().any(|c| c == "display-list-v2-only"), "{acc:?}");
    assert_eq!(v1_pages(&r.line), 0);
    assert_eq!(v1_pages(&r2.line), 0);
}
