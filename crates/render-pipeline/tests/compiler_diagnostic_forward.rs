//! GH-277: render-pipeline forwards the compiler's diagnostic `code` and
//! `suggestion`. runtime-v1 JSON carries both; display-list-v2 carries
//! `suggestion` only when `display-list-v2-diagnostics` is negotiated
//! (`additionalProperties: false` on the frozen four-key object).

mod common;

use std::sync::Mutex;

use common::*;
use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::display;
use flashtex_render_pipeline::protocol::handle_line;
use flashtex_render_pipeline::v1::{self, Capabilities};
use flashtex_render_pipeline::{FontSet, RenderOptions};

/// `FLASHTEX_MAX_REPLY_BYTES` is process-global; tests that call `handle_line`
/// must not run over a sibling that temporarily lowers it.
static REPLY_LIMIT: Mutex<()> = Mutex::new(());

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\\begin{{document}}{body}\\end{{document}}")
}

fn by_needle<'a>(r: &'a flashtex_render_pipeline::Rendered, needle: &str) -> &'a display::Diagnostic {
    r.v2
        .diagnostics
        .iter()
        .find(|d| d.message.contains(needle))
        .unwrap_or_else(|| panic!("no diagnostic containing {needle:?} in {:?}", r.v2.diagnostics))
}

fn field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

fn v2_diag_objects(r: &flashtex_render_pipeline::Rendered) -> Vec<Value> {
    let parsed = json::parse(&r.v2.write_json("fwd")).expect("v2 JSON");
    parsed
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(Value::as_arr)
        .cloned()
        .unwrap_or_default()
}

fn v1_diag_objects(r: &flashtex_render_pipeline::Rendered) -> Vec<Value> {
    let v1 = v1_of(r, Capabilities::default());
    let parsed = json::parse(&v1.write_envelope("fwd")).expect("v1 JSON");
    parsed
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(Value::as_arr)
        .cloned()
        .unwrap_or_default()
}

fn compile_line(id: &str, body: &str, caps: &[&str]) -> String {
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("fwd"));
    payload.set("revision", json::num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(body));
    payload.set("documents", Value::Arr(vec![doc]));
    payload.set("layout_capabilities", Value::Arr(caps.iter().map(|c| json::str_(*c)).collect()));
    let mut v = Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(id));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v)
}

fn diag_row_for<'a>(diags: &'a [Value], needle: &str) -> &'a Value {
    diags
        .iter()
        .find(|row| field(row, "message").is_some_and(|m| m.contains(needle)))
        .unwrap_or_else(|| panic!("no diagnostic containing {needle:?} in {diags:?}"))
}

fn sibling_diagnostics(extra: &[String]) -> Vec<Value> {
    let parsed = json::parse(extra.first().expect("display_list sibling")).expect("v2 JSON");
    parsed
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(Value::as_arr)
        .cloned()
        .unwrap_or_default()
}

fn echoed_caps(line: &str) -> Vec<String> {
    json::parse(line)
        .unwrap()
        .get("payload")
        .and_then(|p| p.get("layout_capabilities"))
        .and_then(|a| a.as_arr().map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect()))
        .unwrap_or_default()
}

#[test]
fn typo_alpah_is_unknown_command_with_alpha_suggestion_in_v1_only() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&doc(r"Text \alpah here."));
    let d = by_needle(&r, r"\alpah");
    assert_eq!(d.code, "unknown_command", "{d:?}");
    assert_eq!(d.suggestion.as_deref(), Some(r"\alpha"), "{d:?}");

    let v1j = v1::diagnostic_json(d);
    assert_eq!(field(&v1j, "code"), Some("unknown_command"), "{v1j:?}");
    assert_eq!(field(&v1j, "suggestion"), Some(r"\alpha"), "{v1j:?}");

    let v2j = display::diagnostic_json(d);
    assert_eq!(field(&v2j, "code"), Some("unknown_command"), "{v2j:?}");
    assert!(v2j.get("suggestion").is_none(), "v2 must omit suggestion: {v2j:?}");

    let v2_wire = v2_diag_objects(&r);
    let v2_row = v2_wire
        .iter()
        .find(|row| field(row, "message").is_some_and(|m| m.contains(r"\alpah")))
        .expect("v2 wire diagnostic for \\alpah");
    assert_eq!(field(v2_row, "code"), Some("unknown_command"));
    assert!(v2_row.get("suggestion").is_none(), "{v2_row:?}");

    let v1_wire = v1_diag_objects(&r);
    let v1_row = v1_wire
        .iter()
        .find(|row| field(row, "message").is_some_and(|m| m.contains(r"\alpah")))
        .expect("v1 wire diagnostic for \\alpah");
    assert_eq!(field(v1_row, "code"), Some("unknown_command"));
    assert_eq!(field(v1_row, "suggestion"), Some(r"\alpha"));
}

#[test]
fn negotiated_v2_diagnostics_emits_suggestion() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let _limit = REPLY_LIMIT.lock().unwrap();
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let text = doc(r"Text \alpah here.");

    let off = handle_line(&compile_line("off", &text, &["display-list-v2"]), &fonts, &options, None);
    let off_caps = echoed_caps(&off.line);
    assert!(off_caps.iter().any(|c| c == "display-list-v2"), "{off_caps:?}");
    assert!(!off_caps.iter().any(|c| c == "display-list-v2-diagnostics"), "{off_caps:?}");
    let off_diags = sibling_diagnostics(&off.extra_lines);
    let off_row = diag_row_for(&off_diags, r"\alpah");
    assert_eq!(field(off_row, "code"), Some("unknown_command"));
    assert!(off_row.get("suggestion").is_none(), "capability off must omit suggestion: {off_row:?}");

    let on = handle_line(
        &compile_line("on", &text, &["display-list-v2", "display-list-v2-diagnostics"]),
        &fonts,
        &options,
        None,
    );
    let on_caps = echoed_caps(&on.line);
    assert_eq!(
        on_caps,
        vec!["display-list-v2".to_string(), "display-list-v2-diagnostics".to_string()],
        "{on_caps:?}"
    );
    let on_diags = sibling_diagnostics(&on.extra_lines);
    let on_row = diag_row_for(&on_diags, r"\alpah");
    assert_eq!(field(on_row, "code"), Some("unknown_command"));
    assert_eq!(field(on_row, "suggestion"), Some(r"\alpha"), "{on_row:?}");
    assert!(on_row.get("labels").is_none(), "{on_row:?}");
    assert!(on_row.get("notes").is_none(), "{on_row:?}");
    assert!(on_row.get("help").is_none(), "{on_row:?}");

    let alone = handle_line(&compile_line("alone", &text, &["display-list-v2-diagnostics"]), &fonts, &options, None);
    assert!(alone.extra_lines.is_empty(), "diagnostics cap without display-list-v2 is rejected");
    assert!(!echoed_caps(&alone.line).iter().any(|c| c == "display-list-v2-diagnostics"));
}

struct ReplyBytesGuard(Option<String>);

impl Drop for ReplyBytesGuard {
    fn drop(&mut self) {
        match self.0.take() {
            Some(v) => std::env::set_var("FLASHTEX_MAX_REPLY_BYTES", v),
            None => std::env::remove_var("FLASHTEX_MAX_REPLY_BYTES"),
        }
    }
}

#[test]
fn declining_display_list_also_drops_dependent_diagnostics_capability() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let _limit = REPLY_LIMIT.lock().unwrap();
    let _restore = ReplyBytesGuard(std::env::var("FLASHTEX_MAX_REPLY_BYTES").ok());
    std::env::set_var("FLASHTEX_MAX_REPLY_BYTES", "6000");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    let text = "\\begin{document}Hello $\\frac{1}{2}$ wörld.\\end{document}";
    let reply = handle_line(
        &compile_line("big", text, &["display-list-v2", "display-list-v2-diagnostics"]),
        &fonts,
        &options,
        None,
    );
    assert!(reply.extra_lines.is_empty(), "no display-list sibling: {:?}", reply.extra_lines.len());
    let caps = echoed_caps(&reply.line);
    assert!(!caps.iter().any(|c| c == "display-list-v2"), "{caps:?}");
    assert!(!caps.iter().any(|c| c == "display-list-v2-diagnostics"), "dependent cap must drop with display-list-v2: {caps:?}");
}

#[test]
fn unimplemented_tikz_is_unsupported_feature() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&doc(r"Text \tikz here."));
    let d = by_needle(&r, r"\tikz");
    assert_eq!(d.code, "unsupported_feature", "{d:?}");
    assert_eq!(d.suggestion, None, "{d:?}");

    let v1j = v1::diagnostic_json(d);
    assert_eq!(field(&v1j, "code"), Some("unsupported_feature"));
    assert!(v1j.get("suggestion").is_none(), "absent, not null: {v1j:?}");

    let v2j = display::diagnostic_json(d);
    assert_eq!(field(&v2j, "code"), Some("unsupported_feature"));
    assert!(v2j.get("suggestion").is_none(), "{v2j:?}");
}
