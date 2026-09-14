//! Producer gate for `display-list-v2-compact`
//! (`protocol/proposals/display-list-v2-compact.md`):
//!
//! 1. Round trip: every glyph run of every real-world fixture (and of a
//!    synthetic list built from the writer's worst cases) written in the
//!    compact encoding reads back to the identical model — `carets`,
//!    `hit_rects`, provenance and text ranges included — so a consumer that
//!    derives them sees exactly what the full line carries.
//! 2. Byte identity: a request that does not list the capability gets the
//!    byte-identical sibling of a plain `display-list-v2` request, and the
//!    capability is never accepted without `display-list-v2` or without a
//!    sibling line.
//! 3. The `display-list-v2-delta` producer gate holds under the compact
//!    encoding: every delta over the edit sequence reconstructs to the
//!    byte-identical compact full line, with exact `page_bytes` for the
//!    relocated (unchanged) pages, i.e. the run-level width rule of
//!    `display_list_compact::source_width_delta` is the serialisation's.

mod common;

use common::lm_available;
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::delta::{self, DeltaState};
use flashtex_render_pipeline::display::{self, Caret, Carets, Cluster, DisplayList, Glyph, GlyphRun, Item, Paint, Provenance, Rect, SourceRange, Tick, Wire};
use flashtex_render_pipeline::display_list_compact as compact;
use flashtex_render_pipeline::protocol::{handle_line, handle_line_with};
use flashtex_render_pipeline::{render, FontSet, RenderCache, RenderOptions};

fn fixture_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/real-world")
}

/// `(entry, documents)` of every real-world fixture, with `\input` parts.
fn corpus() -> Vec<(String, Vec<(String, String)>)> {
    let root = fixture_root();
    let mut out = Vec::new();
    let mut dirs: Vec<_> = std::fs::read_dir(&root).unwrap().filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for dir in dirs {
        let mut docs = Vec::new();
        let mut entry = None;
        for name in ["main.tex", "HW1.tex", "HW2.tex"] {
            let p = dir.join(name);
            if let Ok(text) = std::fs::read_to_string(&p) {
                docs.push((name.to_string(), text));
                entry = Some(name.to_string());
            }
        }
        let Some(entry) = entry else { continue };
        if let Ok(sections) = std::fs::read_dir(dir.join("sections")) {
            let mut parts: Vec<_> = sections.filter_map(Result::ok).map(|e| e.path()).collect();
            parts.sort();
            for p in parts {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    docs.push((format!("sections/{}", p.file_name().unwrap().to_string_lossy()), text));
                }
            }
        }
        out.push((format!("{}/{entry}", dir.file_name().unwrap().to_string_lossy()), docs));
    }
    // A TikZ picture: node text runs clamp the hit width but not the end
    // caret (the one non-derivable end caret PR #232 names).
    out.push((
        "tikz".into(),
        vec![("main.tex".into(), "Before text.\n\n\\begin{tikzpicture}\n\\draw[->] (0,0) -- (2,1) node[midway,above] {hi};\n\\node[draw] at (0,0) {box};\n\\end{tikzpicture}\n\nAfter text.\n".into())],
    ));
    out
}

fn render_docs(fonts: &FontSet, docs: &[(String, String)], entry: &str) -> DisplayList {
    let sources: Vec<SourceDocument<'_>> = docs.iter().map(|(p, t)| SourceDocument { path: p, text: t }).collect();
    let entry = entry.rsplit('/').next().unwrap();
    render(&sources, entry, 7, "compact-gate", fonts, &RenderOptions::default()).v2
}

/// Per-fixture override statistics of the compact clusters (for the evidence).
#[derive(Default, Debug)]
struct Stats {
    runs: usize,
    clusters: usize,
    c: usize,
    e: usize,
    h: usize,
    hv: usize,
    l: usize,
    s: usize,
    explicit: usize,
    ts: usize,
    end_carets: usize,
}

fn stats(line: &str) -> Stats {
    let v = json::parse(line).unwrap();
    let mut st = Stats::default();
    for p in v.get("payload").unwrap().get("pages").unwrap().as_arr().unwrap() {
        for it in p.get("items").unwrap().as_arr().unwrap() {
            if it.get("kind").and_then(Value::as_str) != Some("glyph_run") {
                continue;
            }
            st.runs += 1;
            st.end_carets += usize::from(it.get("end_caret").is_some());
            for c in it.get("clusters").unwrap().as_arr().unwrap() {
                st.clusters += 1;
                st.c += usize::from(c.get("c").is_some());
                st.e += usize::from(c.get("e").is_some());
                st.h += usize::from(c.get("h").is_some());
                st.hv += usize::from(c.get("hv").is_some());
                st.l += usize::from(c.get("l").is_some());
                st.s += usize::from(c.get("s").is_some());
                st.ts += usize::from(c.get("ts").is_some());
                st.explicit += usize::from(c.get("sources").is_some() || c.get("synthetic_reason").is_some());
            }
        }
    }
    st
}

#[test]
fn compact_lines_read_back_to_the_identical_model_on_the_corpus() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let full_wire = Wire::default();
    let compact_wire = Wire { compact: true, ..Wire::default() };
    let mut total_clusters = 0;
    for (entry, docs) in corpus() {
        let list = render_docs(&fonts, &docs, &entry);
        let full = list.write_json_wire("id", full_wire);
        let line = list.write_json_wire("id", compact_wire);
        assert!(line.contains("\"cluster_encoding\":\"compact-1\""), "{entry}: the payload announces the encoding");
        assert!(!full.contains("cluster_encoding"), "{entry}: the full line is unchanged");
        let model = compact::model_runs(&list);
        let read = compact::read_runs(&line).unwrap_or_else(|e| panic!("{entry}: {e:?}"));
        assert_eq!(read.len(), model.len(), "{entry}: pages");
        for ((pn, mruns), (rn, rruns)) in model.iter().zip(&read) {
            assert_eq!(pn, rn, "{entry}: page numbers");
            assert_eq!(mruns.len(), rruns.len(), "{entry} page {pn}: runs");
            for (i, (m, r)) in mruns.iter().zip(rruns).enumerate() {
                assert!(m == r, "{entry} page {pn} run {i} ({:?}) differs after the compact round trip:\n model {m:#?}\n read  {r:#?}", m.text);
            }
        }
        // The full (today's) encoding reads back to the same model too, so the
        // reader is not tautological with the writer.
        let read_full = compact::read_runs(&full).unwrap();
        assert!(read_full == model, "{entry}: the full line reads back to the model");
        let st = stats(&line);
        total_clusters += st.clusters;
        eprintln!(
            "{entry}: full {} B, compact {} B ({:.1}%); runs {} clusters {} | overrides c {} e {} h {} hv {} l {} s {} ts {} explicit {} end_carets {}",
            full.len(),
            line.len(),
            100.0 * line.len() as f64 / full.len() as f64,
            st.runs,
            st.clusters,
            st.c,
            st.e,
            st.h,
            st.hv,
            st.l,
            st.s,
            st.ts,
            st.explicit,
            st.end_carets
        );
        assert!(line.len() * 2 < full.len(), "{entry}: the compact line is under half the full line ({} vs {})", line.len(), full.len());
    }
    assert!(total_clusters > 10_000, "the corpus exercised {total_clusters} clusters");
}

/// A run that defeats every default at once: a caret pair not on the hit
/// rect, a cluster without a glyph, multi-range and synthetic provenance,
/// another document's path, a non-contiguous text range, a second caret on a
/// non-last cluster, a negative source delta and a zero-advance glyph.
fn adversarial_list() -> DisplayList {
    let src = |p: &str, a, b| SourceRange { path: std::rc::Rc::from(p), start_byte: a, end_byte: b };
    let rect = |x, top, w, h| Rect { x: Tick(x), top: Tick(top), width: Tick(w), height: Tick(h) };
    let caret = |tb, x, top, h| Caret { text_byte: tb, x: Tick(x), top: Tick(top), height: Tick(h) };
    let glyph = |gid, ox, adv, cluster| Glyph { gid, origin_x: Tick(ox), baseline_y: Tick(700), advance_x: Tick(adv), advance_y: Tick(0), cluster };
    let run = |text: &str, glyphs: Vec<Glyph>, clusters: Vec<Cluster>| {
        Item::GlyphRun(GlyphRun { font_id: std::rc::Rc::from("f"), font_size: Tick(12 << 20), text: text.into(), glyphs, clusters, paint: Paint::BLACK, role: display::RunRole::Text })
    };
    let plain = |tb_start, tb_end, x, w, prov| Cluster { text_start_byte: tb_start, text_end_byte: tb_end, hit_rect: rect(x, -10, w, 20), carets: Carets { first: caret(tb_start, x, -10, 20), last: None }, provenance: prov };
    let items = vec![
        // Every default holds: `{}` clusters.
        run("ab", vec![glyph(1, 0, 5, 0), glyph(2, 5, 6, 1)], vec![plain(0, 1, 0, 5, Provenance::Source(src("m", 10, 11))), {
            let mut c = plain(1, 2, 5, 6, Provenance::Source(src("m", 11, 12)));
            c.carets.last = Some(caret(2, 11, -10, 20));
            c
        }]),
        // Ligature (3-byte scalar, 2 source bytes), accent (2 scalars in one
        // cluster), a gap in the source chain, a backwards source range.
        run(
            "ﬁe\u{301}x",
            vec![glyph(3, 0, 7, 0), glyph(4, 7, 5, 1), glyph(5, 7, 0, 1), glyph(6, 12, 4, 2)],
            vec![
                plain(0, 3, 0, 7, Provenance::Source(src("m", 20, 22))),
                plain(3, 6, 7, 5, Provenance::Source(src("m", 30, 33))),
                plain(6, 7, 12, 4, Provenance::Source(src("m", 5, 6))),
            ],
        ),
        // Overrides: caret pair off the rect, no glyph, multi-range, synthetic,
        // other path, non-contiguous text, second caret on a non-last cluster,
        // hit rect x/width and top/height off the derivation.
        run(
            "pqrstu",
            vec![glyph(7, 0, 5, 0), glyph(8, 5, 5, 2), glyph(9, 10, 5, 3), glyph(10, 15, 5, 4), glyph(11, 20, 5, 5)],
            vec![
                Cluster { text_start_byte: 0, text_end_byte: 1, hit_rect: rect(0, -10, 5, 20), carets: Carets { first: caret(0, 1, -11, 21), last: Some(caret(1, 4, -10, 20)) }, provenance: Provenance::Source(src("m", 40, 41)) },
                plain(1, 2, 0, 0, Provenance::Sources(vec![src("m", 41, 42), src("m", 50, 51)])),
                plain(2, 3, 5, 5, Provenance::Synthetic("heading number".into())),
                plain(3, 4, 10, 5, Provenance::Source(src("other.tex", 0, 1))),
                Cluster { text_start_byte: 4, text_end_byte: 5, hit_rect: rect(16, -12, 3, 24), carets: Carets { first: caret(4, 16, -12, 24), last: None }, provenance: Provenance::Source(src("m", 44, 45)) },
                Cluster { text_start_byte: 5, text_end_byte: 6, hit_rect: rect(20, -30, 5, 40), carets: Carets { first: caret(5, 20, -30, 40), last: Some(caret(6, 25, -30, 40)) }, provenance: Provenance::Source(src("m", 45, 46)) },
            ],
        ),
        // No sources at all (all synthetic), and a run whose text range does
        // not start at 0.
        run("§", vec![glyph(12, 0, 3, 0)], vec![plain(0, 2, 0, 3, Provenance::Synthetic("x".into()))]),
        run("ab", vec![glyph(13, 0, 3, 0)], vec![plain(1, 2, 0, 3, Provenance::Source(src("m", 60, 61)))]),
    ];
    DisplayList {
        project_id: "adv".into(),
        revision: 1,
        documents: vec![display::DocumentResource { path: "m".into(), revision: 1, sha256: "00".into(), byte_length: 100 }],
        fonts: vec![display::FontResource { font_id: std::rc::Rc::from("f"), sha256: "ff".into(), byte_length: 10, format: "opentype-cff".into(), face_index: 0, units_per_em: 1000, glyph_count: 100, postscript_name: "F".into(), path: None }],
        pages: vec![display::Page { number: 1, width: Tick(612 << 20), height: Tick(792 << 20), items }],
        diagnostics: Vec::new(),
    }
}

#[test]
fn every_override_round_trips_on_an_adversarial_run() {
    let list = adversarial_list();
    let line = list.write_json_wire("adv", Wire { compact: true, ..Wire::default() });
    let read = compact::read_runs(&line).unwrap();
    assert!(read == compact::model_runs(&list), "{line}\n{read:#?}");
    let st = stats(&line);
    assert!(st.c == 1 && st.h == 1 && st.hv == 1 && st.l == 1 && st.s == 4 && st.ts == 1 && st.e == 1 && st.explicit == 4 && st.end_carets == 2, "{st:?}\n{line}");
    // The first run is the override-free shape.
    assert!(line.contains("\"clusters\":[{},{}],\"end_caret\":{\"text_byte\":2,\"x\":11}"), "{line}");
    // Today's encoding of the same list is unchanged by the module (value tree parity still holds).
    let full = list.write_json_wire("adv", Wire::default());
    assert_eq!(full, json::write(&list.to_json_wire("adv", Wire::default())));
    assert!(compact::read_runs(&full).unwrap() == compact::model_runs(&list));
}

// ---------------------------------------------------------------- negotiation

fn request(id: &str, revision: u64, text: &str, caps: &[&str], base: Option<&delta::Base>) -> String {
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("compact"));
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

const FULL: &[&str] = &["rules-v1", "font-hints-v1", "display-list-v2"];
const COMPACT: &[&str] = &["rules-v1", "font-hints-v1", "display-list-v2", "display-list-v2-compact"];

#[test]
fn compact_is_negotiated_only_next_to_a_sibling_and_leaves_other_replies_byte_identical() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let text = document(2);
    let plain = handle_line(&request("a", 1, &text, FULL, None), &fonts, &options, None);
    let with = handle_line(&request("a", 1, &text, COMPACT, None), &fonts, &options, None);
    assert!(accepted(&with.line).iter().any(|c| c == compact::CAP), "{:?}", accepted(&with.line));
    assert!(!accepted(&plain.line).iter().any(|c| c == compact::CAP));
    // The v1 line differs only in the echoed capability list.
    let strip = |l: &str| l.replace(",\"display-list-v2-compact\"", "");
    assert_eq!(strip(&with.line), plain.line);
    // The plain sibling is exactly the writer's default; the compact one is the compact writer's.
    let rendered = with.rendered.as_ref().unwrap();
    assert_eq!(plain.extra_lines[0], rendered.v2.write_json_wire("a", Wire::default()));
    assert_eq!(with.extra_lines[0], rendered.v2.write_json_wire("a", Wire { compact: true, ..Wire::default() }));
    assert!(with.extra_lines[0].len() < plain.extra_lines[0].len() / 2);
    // Never without display-list-v2.
    let r = handle_line(&request("b", 2, &text, &["rules-v1", "display-list-v2-compact"], None), &fonts, &options, None);
    assert!(!accepted(&r.line).iter().any(|c| c == compact::CAP));
    assert!(r.extra_lines.is_empty());
    // Never echoed when the sibling is declined for size (the echo is the
    // consumer's only signal that the frame is compact).
    std::env::set_var("FLASHTEX_MAX_REPLY_BYTES", "4096");
    let declined = handle_line(&request("c", 3, &text, COMPACT, None), &fonts, &options, None);
    std::env::remove_var("FLASHTEX_MAX_REPLY_BYTES");
    let acc = accepted(&declined.line);
    assert!(declined.extra_lines.is_empty() || declined.line.contains("display_list_declined"), "{}", declined.line);
    if declined.extra_lines.is_empty() {
        assert!(!acc.iter().any(|c| c == compact::CAP), "{acc:?}");
        assert!(!acc.iter().any(|c| c == "display-list-v2"), "{acc:?}");
    }
    // A failed compile never carries the sibling or the echo.
    let failed = handle_line(&request("d", 4, "", COMPACT, None), &fonts, &options, None);
    if failed.line.contains("\"status\":\"failed\"") {
        assert!(!accepted(&failed.line).iter().any(|c| c == compact::CAP));
    }
}

// ---------------------------------------------------------------- delta under compact

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

const DELTA_COMPACT: &[&str] = &["rules-v1", "font-hints-v1", "display-list-v2", "display-list-v2-delta", "display-list-v2-compact"];

fn full_line_bytes_from_delta(payload: &Value, id: &str, page_bytes: &[usize], wire: Wire) -> usize {
    let part = |k: &str| json::write(payload.get(k).unwrap()).len();
    display::full_line_frame_bytes(wire)
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
fn deltas_reconstruct_to_the_fresh_compact_line_over_an_edit_sequence() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let cache = RenderCache::new();
    let state = DeltaState::new();
    let wire = Wire { compact: true, ..Wire::default() };
    let base_doc = document(40);
    let mut previous_pages: Vec<display::Page> = Vec::new();
    let (mut deltas, mut fulls, mut delta_bytes, mut full_bytes, mut unchanged_pages) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut text = base_doc.clone();
    for i in 0..60 {
        text = if i % 9 == 8 { edited(&base_doc, i) } else { edited(&text, i) };
        let id = format!("r{i}");
        let base = state.acknowledgement();
        let req = request(&id, i as u64 + 1, &text, DELTA_COMPACT, base.as_ref());
        let reply = handle_line_with(&req, &fonts, &options, Some(&cache), Some(&state));
        let fresh = handle_line(&request(&id, i as u64 + 1, &text, COMPACT, None), &fonts, &options, None);
        let fresh_line = &fresh.extra_lines[0];
        assert!(fresh_line.contains("\"cluster_encoding\":\"compact-1\""));
        assert_eq!(reply.extra_lines.len(), 1, "edit {i}: one sibling line");
        let sibling = &reply.extra_lines[0];
        let env = json::parse(sibling).unwrap();
        let rendered = reply.rendered.as_ref().unwrap();
        let echoed = accepted(&reply.line);
        assert!(echoed.iter().any(|c| c == compact::CAP), "edit {i}: compact echoed with a sibling");
        full_bytes += fresh_line.len();
        if env.get("type").unwrap().as_str() == Some("display_list_delta") {
            deltas += 1;
            delta_bytes += sibling.len();
            let payload = env.get("payload").unwrap();
            assert_eq!(payload.get("cluster_encoding").unwrap().as_str(), Some(compact::ENCODING), "edit {i}: the delta announces the encoding");
            let n = payload.get("page_count").unwrap().as_i64().unwrap() as usize;
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
            let page_bytes: Vec<usize> = payload.get("page_bytes").unwrap().as_arr().unwrap().iter().map(|v| v.as_i64().unwrap() as usize).collect();
            let page_digests: Vec<String> = payload.get("page_digests").unwrap().as_arr().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
            let changed: Vec<&Value> = payload.get("changed_pages").unwrap().as_arr().unwrap().iter().collect();
            let mut objects: Vec<String> = Vec::with_capacity(n);
            let mut ci = 0;
            for (pi, new_page) in rendered.v2.pages.iter().enumerate() {
                let number = pi as i64 + 1;
                let is_changed = changed.get(ci).and_then(|c| c.get("number")).and_then(Value::as_i64) == Some(number);
                let page_model = if is_changed {
                    let obj = json::write(changed[ci]);
                    ci += 1;
                    let mut expect = String::new();
                    display::write_page(&mut expect, new_page, wire);
                    assert_eq!(json::parse(&expect).unwrap(), json::parse(&obj).unwrap(), "edit {i}: changed page {number} value");
                    assert_eq!(page_bytes[pi], expect.len(), "edit {i}: page_bytes of changed page {number}");
                    objects.push(expect);
                    new_page.clone()
                } else {
                    unchanged_pages += 1;
                    let moved = delta::relocate_page(&previous_pages[pi], &relocs).unwrap_or_else(|| panic!("edit {i}: page {number} relocation invalid"));
                    let mut o = String::new();
                    display::write_page(&mut o, &moved, wire);
                    // The consumer's rule: cached length + the run-level width
                    // delta (`source_width_delta`) must equal the serialisation.
                    assert_eq!(page_bytes[pi], o.len(), "edit {i}: page_bytes of unchanged (relocated) page {number} under the compact width rule");
                    objects.push(o);
                    moved
                };
                assert_eq!(flashtex_font_engine::sha256::hex(&delta::page_digest(&page_model, wire)), page_digests[pi], "edit {i}: page {number} digest");
            }
            assert_eq!(ci, changed.len());
            let reconstructed = rendered.v2.write_json_wire_with_page_objects(&id, wire, &objects);
            assert!(reconstructed == *fresh_line, "edit {i}: reconstructed compact line differs from the fresh one ({} vs {} bytes)", reconstructed.len(), fresh_line.len());
            assert_eq!(full_line_bytes_from_delta(payload, &id, &page_bytes, wire), fresh_line.len(), "edit {i}: exact size formula with the compact frame");
        } else {
            assert_eq!(env.get("type").unwrap().as_str(), Some("display_list"));
            assert!(sibling == fresh_line, "edit {i}: full compact line differs from the fresh line");
            fulls += 1;
        }
        previous_pages = rendered.v2.pages.clone();
    }
    eprintln!("compact: {deltas} deltas ({delta_bytes} B) and {fulls} full lines ({full_bytes} B fresh total) over 60 edits; {unchanged_pages} relocated pages verified");
    assert!(deltas >= 30, "most edits should be answered with a delta: {deltas} deltas, {fulls} full");
    assert!(unchanged_pages > 100, "the width rule was exercised on {unchanged_pages} relocated pages");
}
