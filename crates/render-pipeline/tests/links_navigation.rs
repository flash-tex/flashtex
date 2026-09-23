//! GH-323 / `protocol/proposals/display-list-v2-links.md`: the producer half
//! of `display-list-v2-links`. The capability is negotiated only next to
//! `display-list-v2`, echoed when accepted, and the accepted reply's
//! `display_list` line gains one top-level `navigation` object with the
//! document's `\url`/`\href` rectangles.
//!
//! The geometry is checked against pdflatex + hyperref, which is the only
//! authority on where a link rectangle goes.

mod common;

use std::sync::Mutex;

use common::{lm_available, render_one};
use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::display::Wire;
use flashtex_render_pipeline::protocol::handle_line;
use flashtex_render_pipeline::{FontSet, RenderOptions};

/// `FLASHTEX_MAX_REPLY_BYTES` is process-global; `handle_line` tests must not
/// run over a sibling that lowers it.
static REPLY_LIMIT: Mutex<()> = Mutex::new(());

const LINKS: &str = "display-list-v2-links";
const V2: &str = "display-list-v2";

/// Ticks per PDF point, the envelope's `bp_2pow20`.
const TICKS_PER_BP: f64 = 1_048_576.0;

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{hyperref}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn compile_line(id: &str, body: &str, caps: &[&str]) -> String {
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("links"));
    payload.set("revision", json::num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    let mut d = Value::obj();
    d.set("path", json::str_("main.tex"));
    d.set("text", json::str_(body));
    payload.set("documents", Value::Arr(vec![d]));
    payload.set("layout_capabilities", Value::Arr(caps.iter().map(|c| json::str_(*c)).collect()));
    let mut v = Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(id));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v)
}

fn echoed_caps(line: &str) -> Vec<String> {
    json::parse(line)
        .unwrap()
        .get("payload")
        .and_then(|p| p.get("layout_capabilities"))
        .and_then(|a| a.as_arr().map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect()))
        .unwrap_or_default()
}

fn sibling_navigation(extra: &[String]) -> Option<Value> {
    let parsed = json::parse(extra.first().expect("display_list sibling")).expect("v2 JSON");
    parsed.get("payload").and_then(|p| p.get("navigation")).cloned()
}

fn links_of(nav: &Value) -> Vec<Value> {
    nav.get("links").and_then(Value::as_arr).cloned().unwrap_or_default()
}

fn uri(link: &Value) -> &str {
    link.get("target").and_then(|t| t.get("uri")).and_then(Value::as_str).expect("target.uri")
}

/// The one `rect`, in PDF points, y down from the page's top-left.
fn rect_bp(link: &Value) -> [f64; 4] {
    let r = link.get("rect").and_then(Value::as_arr).expect("single-piece link writes `rect`");
    let mut out = [0.0; 4];
    for (i, v) in r.iter().enumerate() {
        out[i] = v.as_i64().expect("tick") as f64 / TICKS_PER_BP;
    }
    out
}

fn on_wire() -> Wire {
    Wire { images: false, device_color: false, diagnostics: false, links: true }
}

fn off_wire() -> Wire {
    Wire { images: false, device_color: false, diagnostics: false, links: false }
}

// -- negotiation ----------------------------------------------------------

#[test]
fn the_capability_is_echoed_only_when_requested_next_to_display_list_v2() {
    if !lm_available() {
        return;
    }
    let _limit = REPLY_LIMIT.lock().unwrap();
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let text = doc(r"See \url{https://example.com} now.");

    let off = handle_line(&compile_line("off", &text, &[V2]), &fonts, &options, None);
    assert_eq!(echoed_caps(&off.line), vec![V2.to_string()]);
    assert!(sibling_navigation(&off.extra_lines).is_none(), "navigation must be absent when not negotiated");

    let on = handle_line(&compile_line("on", &text, &[V2, LINKS]), &fonts, &options, None);
    assert_eq!(echoed_caps(&on.line), vec![V2.to_string(), LINKS.to_string()]);
    let nav = sibling_navigation(&on.extra_lines).expect("navigation when negotiated");
    assert_eq!(links_of(&nav).len(), 1);

    // Asked for alone, the dependent capability is not accepted and no
    // sibling is produced at all.
    let alone = handle_line(&compile_line("alone", &text, &[LINKS]), &fonts, &options, None);
    assert!(alone.extra_lines.is_empty(), "links without display-list-v2 produces no sibling");
    assert!(!echoed_caps(&alone.line).iter().any(|c| c == LINKS), "{:?}", echoed_caps(&alone.line));
}

#[test]
fn links_and_delta_are_negotiated_together() {
    // GH-1003: `-links` used to decline `-delta` (the delta line had no
    // `navigation`), so the Mac app, which always asks for links, never got
    // a delta. A delta now carries `navigation` and its digest binds it
    // (`tests/display_list_delta_links.rs`), so both are accepted.
    let requested: Vec<String> = [V2, "display-list-v2-delta", LINKS].iter().map(|c| c.to_string()).collect();
    let (caps, accepted) = flashtex_render_pipeline::v1::Capabilities::negotiate(&requested);
    assert!(caps.links && caps.delta, "{accepted:?}");
    assert_eq!(accepted, requested);
}

// -- the line without links is unchanged ----------------------------------

#[test]
fn a_document_with_no_links_serialises_byte_identically() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc("Ordinary text with no link at all."));
    assert!(r.v2.navigation.is_none(), "no link means no navigation object");
    assert_eq!(r.v2.write_json_wire("t", on_wire()), r.v2.write_json_wire("t", off_wire()));
    assert!(!r.v2.write_json_wire("t", on_wire()).contains("navigation"));
}

#[test]
fn nolinkurl_is_typeset_but_never_linked() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc(r"See \nolinkurl{https://example.com} now."));
    assert!(r.v2.navigation.is_none(), "{:?}", r.v2.navigation);
}

// -- geometry against pdflatex --------------------------------------------

/// Measured with TeX Live's pdflatex on the exact document below and read
/// back with PyMuPDF (`page.get_links()`, whose `from` rectangle is already
/// y-down from the page's top-left, like this envelope):
///
/// ```text
/// https://example.com Rect(165.427001953125, 126.8499755859375, 266.7959899902344, 137.9749755859375)
/// https://ex.org/a    Rect(287.49700927734375, 126.8499755859375, 330.4739990234375, 137.9749755859375)
/// ```
///
/// Note both rectangles share a top and a bottom although one is typewriter
/// and the other roman: pdfTeX takes a link's height and depth from the line
/// box, which is what `links::line_extent` reproduces.
const ORACLE: [(&str, [f64; 4]); 2] = [
    ("https://example.com", [165.427_001_953_125, 126.849_975_585_937_5, 266.795_989_990_234_4, 137.974_975_585_937_5]),
    ("https://ex.org/a", [287.497_009_277_343_75, 126.849_975_585_937_5, 330.473_999_023_437_5, 137.974_975_585_937_5]),
];

#[test]
fn link_rectangles_match_pdflatex_within_half_a_point() {
    if !lm_available() {
        return;
    }
    let _limit = REPLY_LIMIT.lock().unwrap();
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let text = doc(r"See \url{https://example.com} and \href{https://ex.org/a}{click here} end.");
    let reply = handle_line(&compile_line("geom", &text, &[V2, LINKS]), &fonts, &options, None);
    let nav = sibling_navigation(&reply.extra_lines).expect("navigation");
    let links = links_of(&nav);
    assert_eq!(links.len(), 2, "{links:?}");

    for (link, (expect_uri, expect)) in links.iter().zip(ORACLE) {
        assert_eq!(uri(link), expect_uri);
        assert_eq!(link.get("page").and_then(Value::as_i64), Some(1));
        assert_eq!(link.get("class").and_then(Value::as_str), Some("url"));
        let got = rect_bp(link);
        for (i, (g, e)) in got.iter().zip(expect).enumerate() {
            assert!(
                (g - e).abs() <= 0.5,
                "{expect_uri}: coordinate {i} is {g} bp, pdflatex says {e} bp (over the 0.5 bp bar)"
            );
        }
    }

    // `destinations` is always present, empty included: the Mac model
    // decodes it as a non-optional dictionary.
    assert!(nav.get("destinations").is_some(), "{nav:?}");
    // `source` is the whole construct's byte span.
    let src = links[0].get("source").expect("source");
    assert_eq!(src.get("document").and_then(Value::as_str), Some("main.tex"));
    let (start, end) = (
        src.get("start").and_then(Value::as_i64).unwrap() as usize,
        src.get("end").and_then(Value::as_i64).unwrap() as usize,
    );
    assert_eq!(&text[start..end], r"\url{https://example.com}");
    let src1 = links[1].get("source").expect("source");
    let (start, end) = (
        src1.get("start").and_then(Value::as_i64).unwrap() as usize,
        src1.get("end").and_then(Value::as_i64).unwrap() as usize,
    );
    assert_eq!(&text[start..end], r"\href{https://ex.org/a}{click here}");
}

#[test]
fn a_link_broken_across_lines_is_one_entry_with_several_rects() {
    if !lm_available() {
        return;
    }
    // Link *text* long enough to wrap: ordinary interword spaces give the
    // line breaker guaranteed break points, where a long `\url` depends on
    // url.sty's own break characters.
    let words = "clickable ".repeat(24);
    let r = render_one(&doc(&format!("Padding words first. \\href{{https://ex.org/wrapped}}{{{words}}} after.")));
    let nav = r.v2.navigation.as_ref().expect("navigation");
    assert_eq!(nav.links.len(), 1, "{:?}", nav.links);
    assert_eq!(nav.links[0].uri, "https://ex.org/wrapped");
    assert!(nav.links[0].rects.len() >= 2, "a wrapped URL is several line pieces: {:?}", nav.links[0].rects);
    // Reading order: each piece starts below the one before it.
    for w in nav.links[0].rects.windows(2) {
        assert!(w[1].y0.0 > w[0].y0.0, "{:?}", nav.links[0].rects);
    }
    // Several pieces are written as `rects`, not `rect`.
    let line = r.v2.write_json_wire("t", on_wire());
    assert!(line.contains("\"rects\":["), "{}", &line[..line.len().min(400)]);
}

#[test]
fn the_two_json_writers_agree_with_navigation_present() {
    if !lm_available() {
        return;
    }
    let r = render_one(&doc(r"See \url{https://example.com} and \href{https://ex.org/a}{click here} end."));
    assert_eq!(r.v2.write_json_wire("t", on_wire()), json::write(&r.v2.to_json_wire("t", on_wire())));
}
