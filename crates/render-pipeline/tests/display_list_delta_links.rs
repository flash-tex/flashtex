//! GH-1003: `display-list-v2-delta` next to `display-list-v2-links`. The Mac
//! app always requests links, so before this the producer declined every
//! delta for it and each keystroke shipped the whole document. A delta now
//! carries the new list's `navigation` verbatim, and the `dl2-canon-1` header
//! digest binds it (when negotiated and non-empty), so:
//!
//! - base + delta reconstructs to the byte-identical full line a fresh
//!   compile emits, `navigation` included;
//! - a change that moves or retargets only a link still yields a different
//!   `list_digest` than its base;
//! - a consumer that does not digest `navigation` can never acknowledge a
//!   linked base the producer recognises, so it keeps receiving full lines
//!   (the pre-GH-1003 behaviour) instead of rebuilding a frame with stale
//!   links.

mod common;

use std::time::Instant;

use common::lm_available;
use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::delta::{self, DeltaState};
use flashtex_render_pipeline::display::{self, Wire};
use flashtex_render_pipeline::protocol::{handle_line, handle_line_with};
use flashtex_render_pipeline::{FontSet, RenderCache, RenderOptions};

/// A multi-page article with an `\href` and a `\url` in every section.
fn document(sections: usize) -> String {
    let mut s = String::from("\\documentclass{article}\n\\usepackage{hyperref}\n\\begin{document}\n\\tableofcontents\n");
    for i in 1..=sections {
        s.push_str(&format!("\\section{{Section {i}}}\n"));
        for j in 1..=3 {
            s.push_str(&format!(
                "Paragraph {i}.{j} has a sentence with some words in it, then an inline formula $x_{{{i}}} + y^{j} = z$ and more words so that lines wrap and pages fill up over time.\n\n"
            ));
        }
        s.push_str(&format!("See \\href{{https://example.com/s{i}}}{{the notes for {i}}} or \\url{{https://example.org/{i}}}.\n\n"));
    }
    s.push_str("\\end{document}\n");
    s
}

/// Typing-like edits spread over the document.
fn edited(base: &str, i: usize) -> String {
    let mut s = base.to_string();
    let needle = format!("Paragraph {}.{} has", 1 + i % 12, 1 + i % 3);
    let k = s.find(&needle).unwrap_or(0);
    match i % 4 {
        0 => s.insert_str(k, "Inserted "),
        1 => s.insert_str(k, &"x".repeat(1 + i % 7)),
        2 => {
            if let Some(e) = s[k..].find(' ') {
                s.replace_range(k..k + e, "Replaced");
            }
        }
        _ => s.insert_str(k, &format!(" {}", "word ".repeat(1 + i % 4))),
    }
    s
}

fn request(id: &str, revision: u64, text: &str, caps: &[&str], base: Option<&delta::Base>) -> String {
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("delta-links"));
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

const LINKS: &str = "display-list-v2-links";
/// What the Mac app's live v2 pane requests (`setLiveV2` + the per-request
/// delta/only capabilities).
const APP: &[&str] = &["rules-v1", "font-hints-v1", "display-list-v2", "display-list-v2-images", LINKS, "display-list-v2-delta", "display-list-v2-only"];
const FULL: &[&str] = &["rules-v1", "font-hints-v1", "display-list-v2", "display-list-v2-images", LINKS];
const WIRE: Wire = Wire { images: true, device_color: false, diagnostics: false, links: true };

/// The consumer's exact-size formula (`fullLineBytes`) with `navigation`:
/// fixed framing + the delta's header parts (+ `,"navigation":` and its
/// object when present) + Σ page_bytes + separators.
fn full_line_bytes_from_delta(payload: &Value, id: &str, page_bytes: &[usize]) -> usize {
    let part = |k: &str| json::write(payload.get(k).unwrap()).len();
    let nav = payload.get("navigation").map_or(0, |n| ",\"navigation\":".len() + json::write(n).len());
    display::FULL_LINE_FRAME_BYTES
        + json::write(&json::str_(id)).len()
        + part("project_id")
        + part("revision")
        + part("required_features")
        + part("documents")
        + part("fonts")
        + part("diagnostics")
        + nav
        + page_bytes.iter().sum::<usize>()
        + page_bytes.len().saturating_sub(1)
}

#[test]
fn links_no_longer_decline_delta_and_deltas_reconstruct_with_navigation() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let cache = RenderCache::new();
    let state = DeltaState::new();
    let base_doc = document(24);
    let mut previous_pages: Vec<display::Page> = Vec::new();
    let mut text = base_doc.clone();
    let mut previous_text = String::new();
    let mut deltas = 0;
    for i in 0..16 {
        text = edited(&text, i);
        let id = format!("r{i}");
        let base = state.acknowledgement();
        let reply = handle_line_with(&request(&id, i as u64 + 1, &text, APP, base.as_ref()), &fonts, &options, Some(&cache), Some(&state));
        let fresh = handle_line(&request(&id, i as u64 + 1, &text, FULL, None), &fonts, &options, None);
        let fresh_line = &fresh.extra_lines[0];
        assert!(fresh_line.contains("\"navigation\":"), "edit {i}: the document has links");
        let sibling = &reply.extra_lines[0];
        let env = json::parse(sibling).unwrap();
        let echoed = accepted(&reply.line);
        assert!(echoed.iter().any(|c| c == LINKS), "edit {i}: links echoed");
        let rendered = reply.rendered.as_ref().unwrap();
        if env.get("type").unwrap().as_str() == Some("display_list_delta") {
            deltas += 1;
            assert!(echoed.iter().any(|c| c == delta::CAP), "edit {i}: delta emitted but not echoed");
            let payload = env.get("payload").unwrap();
            // `navigation` rides on the delta, exactly as the full line writes it.
            let fresh_payload = json::parse(fresh_line).unwrap();
            let fresh_nav = fresh_payload.get("payload").unwrap().get("navigation").unwrap();
            assert_eq!(json::write(payload.get("navigation").expect("delta carries navigation")), json::write(fresh_nav), "edit {i}: navigation");
            // Reconstruct base + delta and compare with the fresh full line.
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
            let changed: Vec<&Value> = payload.get("changed_pages").unwrap().as_arr().unwrap().iter().collect();
            let page_bytes: Vec<usize> = payload.get("page_bytes").unwrap().as_arr().unwrap().iter().map(|v| v.as_i64().unwrap() as usize).collect();
            let mut objects = Vec::new();
            for (pi, page) in rendered.v2.pages.iter().enumerate() {
                let number = pi as i64 + 1;
                let obj = match changed.iter().find(|c| c.get("number").and_then(Value::as_i64) == Some(number)) {
                    Some(c) => json::write(c),
                    None => {
                        let moved = delta::relocate_page(&previous_pages[pi], &relocs).unwrap_or_else(|| panic!("edit {i}: page {number} relocation"));
                        let mut o = String::new();
                        display::write_page(&mut o, &moved, WIRE);
                        o
                    }
                };
                assert_eq!(obj.len(), page_bytes[pi], "edit {i}: page {number} page_bytes");
                let _ = page;
                objects.push(obj);
            }
            let rebuilt = rendered.v2.write_json_wire_with_page_objects(&id, WIRE, &objects);
            assert_eq!(&rebuilt, fresh_line, "edit {i}: base + delta reconstructs the fresh full line");
            assert_eq!(full_line_bytes_from_delta(payload, &id, &page_bytes), fresh_line.len(), "edit {i}: exact size formula");
            // The list digest binds the navigation the delta carries.
            let digests: Vec<[u8; 32]> = rendered.v2.pages.iter().map(|p| delta::page_digest(p, WIRE)).collect();
            assert_eq!(
                payload.get("list_digest").unwrap().as_str().unwrap(),
                flashtex_font_engine::sha256::hex(&delta::list_digest(&rendered.v2, WIRE, &digests)),
                "edit {i}: list_digest"
            );
        } else {
            assert_eq!(sibling, fresh_line, "edit {i}: full line is the fresh one");
        }
        previous_pages = rendered.v2.pages.clone();
        previous_text = text.clone();
    }
    let _ = previous_text;
    assert!(deltas >= 10, "links must no longer force every reply full: {deltas} deltas in 16 edits");
}

#[test]
fn a_link_only_change_still_changes_the_list_digest() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let cache = RenderCache::new();
    let state = DeltaState::new();
    let text = document(12);
    let first = handle_line_with(&request("a", 1, &text, APP, None), &fonts, &options, Some(&cache), Some(&state));
    assert_eq!(json::parse(&first.extra_lines[0]).unwrap().get("type").unwrap().as_str(), Some("display_list"));
    let base = state.acknowledgement().expect("a linked full line is a delta base");
    // Retarget one `\href`: the glyphs do not change, only its URI.
    let retargeted = text.replacen("https://example.com/s3}", "https://example.net/s3}", 1);
    assert_ne!(retargeted, text);
    let reply = handle_line_with(&request("b", 2, &retargeted, APP, Some(&base)), &fonts, &options, Some(&cache), Some(&state));
    let env = json::parse(&reply.extra_lines[0]).unwrap();
    assert_eq!(env.get("type").unwrap().as_str(), Some("display_list_delta"));
    let payload = env.get("payload").unwrap();
    assert!(payload.get("changed_pages").unwrap().as_arr().unwrap().is_empty(), "no page changed");
    assert_ne!(payload.get("list_digest").unwrap().as_str().unwrap(), base.list_digest, "the digest binds the new URI");
    assert!(json::write(payload.get("navigation").unwrap()).contains("https://example.net/s3"));
}

#[test]
fn navigation_is_in_the_header_digest_only_when_negotiated() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let reply = handle_line(&request("a", 1, &document(2), FULL, None), &fonts, &options, None);
    let list = &reply.rendered.as_ref().unwrap().v2;
    assert!(list.navigation.as_ref().is_some_and(|n| !n.is_empty()));
    let off = Wire { links: false, ..WIRE };
    let mut stripped = list.clone();
    stripped.navigation = None;
    // Off: the old canonical form, whatever the model holds.
    assert_eq!(delta::header_digest(list, off), delta::header_digest(&stripped, off));
    // On: navigation is hashed.
    assert_ne!(delta::header_digest(list, WIRE), delta::header_digest(&stripped, WIRE));
    // On with nothing to send: the old canonical form too.
    assert_eq!(delta::header_digest(&stripped, WIRE), delta::header_digest(&stripped, off));
}

/// GH-1003 measurement: keystroke replies on a real multi-page linked
/// document with the Mac app's capabilities, against the same edits without
/// `-delta` -- exactly the path every app request took while `-links`
/// declined it. The two workers alternate on each edit so machine load hits
/// both alike. `cargo test --release --test display_list_delta_links --
/// --ignored --nocapture measure`.
#[test]
#[ignore]
fn measure_app_capabilities_over_an_edit_sequence() {
    if !lm_available() {
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let no_delta: Vec<&str> = APP.iter().copied().filter(|c| *c != delta::CAP).collect();
    struct Worker<'a> {
        caps: &'a [&'a str],
        cache: RenderCache,
        state: DeltaState,
        ms: Vec<f64>,
        bytes: usize,
        deltas: usize,
    }
    let mut workers = [
        Worker { caps: &no_delta, cache: RenderCache::new(), state: DeltaState::new(), ms: Vec::new(), bytes: 0, deltas: 0 },
        Worker { caps: APP, cache: RenderCache::new(), state: DeltaState::new(), ms: Vec::new(), bytes: 0, deltas: 0 },
    ];
    let mut text = document(40);
    let mut pages = 0;
    for w in &mut workers {
        let warm = handle_line_with(&request("w", 1, &text, w.caps, None), &fonts, &options, Some(&w.cache), Some(&w.state));
        pages = warm.rendered.as_ref().unwrap().v2.pages.len();
    }
    let n = 40;
    for i in 0..n {
        text = edited(&text, i);
        for k in 0..2 {
            let w = &mut workers[(i + k) % 2];
            let base = w.state.acknowledgement();
            let req = request(&format!("r{i}"), i as u64 + 2, &text, w.caps, base.as_ref());
            let t = Instant::now();
            let reply = handle_line_with(&req, &fonts, &options, Some(&w.cache), Some(&w.state));
            w.ms.push(t.elapsed().as_secs_f64() * 1e3);
            w.bytes += reply.extra_lines[0].len() + reply.line.len();
            w.deltas += usize::from(reply.extra_lines[0].contains("\"type\":\"display_list_delta\""));
        }
    }
    for (name, w) in ["links, no delta (before)", "links + delta (after)"].iter().zip(&mut workers) {
        w.ms.sort_by(f64::total_cmp);
        eprintln!(
            "GH-1003 measure [{name}]: {pages} pages, {n} edits, {} deltas; mean reply {} B/edit; producer ms/edit min {:.1} p25 {:.1} median {:.1} p90 {:.1}",
            w.deltas,
            w.bytes / n,
            w.ms[0],
            w.ms[n / 4],
            w.ms[n / 2],
            w.ms[n * 9 / 10]
        );
    }
}
