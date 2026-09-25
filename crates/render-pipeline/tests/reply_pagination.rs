//! GH-957: a `compile_result` past the 16 MiB reply limit is paginated, not
//! failed. The reply keeps the render's own status, every diagnostic, and
//! the leading pages that fit, plus one `reply_paginated` warning naming
//! the full size and the coverage -- pdflatex likewise still writes its
//! pages and reports every diagnostic when a document is long. Only when
//! not even the diagnostics fit does the worker refuse closed with the
//! exact byte count.
//!
//! The failure is a *ratio* between the reply and the limit rather than an
//! absolute size, so these tests put ordinary documents into the same ratio
//! through `FLASHTEX_MAX_REPLY_BYTES` -- the knob that exists for exactly
//! this. Nothing here hard-codes a page size: limits are derived from each
//! test's own permissive run, and every count in the expected messages is
//! read back out of that run.

mod common;

use std::sync::Mutex;

use common::*;
use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::protocol::handle_line;
use flashtex_render_pipeline::{FontSet, RenderOptions};

/// `FLASHTEX_MAX_REPLY_BYTES` is process-global; tests that lower it must
/// not run over a sibling that reads it.
static REPLY_LIMIT: Mutex<()> = Mutex::new(());

struct ReplyBytesGuard(Option<String>);

impl Drop for ReplyBytesGuard {
    fn drop(&mut self) {
        match self.0.take() {
            Some(v) => std::env::set_var("FLASHTEX_MAX_REPLY_BYTES", v),
            None => std::env::remove_var("FLASHTEX_MAX_REPLY_BYTES"),
        }
    }
}

fn compile_line(id: &str, text: &str) -> String {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("paginated"));
    payload.set("revision", json::num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut v = Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(id));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v)
}

fn payload_of(line: &str) -> Value {
    json::parse(line).expect("reply is JSON").get("payload").expect("reply has a payload").clone()
}

fn status_of(payload: &Value) -> &str {
    payload.get("status").and_then(Value::as_str).expect("payload has a status")
}

fn diagnostics_of(payload: &Value) -> Vec<Value> {
    payload.get("diagnostics").and_then(Value::as_arr).expect("payload has diagnostics").clone()
}

fn numbers_of(payload: &Value) -> Vec<i64> {
    payload
        .get("pages")
        .and_then(Value::as_arr)
        .expect("payload has pages")
        .iter()
        .map(|p| p.get("number").and_then(Value::as_i64).expect("page has a number"))
        .collect()
}

fn with_limit(line: &str, limit: usize) -> String {
    let _limit = REPLY_LIMIT.lock().unwrap();
    let _restore = ReplyBytesGuard(std::env::var("FLASHTEX_MAX_REPLY_BYTES").ok());
    std::env::set_var("FLASHTEX_MAX_REPLY_BYTES", limit.to_string());
    let fonts = FontSet::with_default_dirs(&[]);
    handle_line(line, &fonts, &RenderOptions::default(), None).line
}

/// A long document carrying a real diagnostic, answered over a limit near
/// three quarters of its size: the reply keeps the render's status and the
/// diagnostic, serves the leading pages that fit, and names the exact full
/// size and coverage in one warning.
#[test]
fn oversized_result_paginates_with_diagnostics_surviving() {
    if !lm_available() {
        return;
    }
    let mut text = "\\begin{document}\nText \\alpah here.\n\n".to_string();
    for i in 0..30 {
        text.push_str(&format!(
            "\\section{{Part {i}}}\nSome prose for part {i}, with enough words in it that the \
             paragraph breaks over several lines of the measure and the page fills up rather \
             than holding one short line. More words follow, and then more again.\n\n"
        ));
    }
    text.push_str("\\end{document}\n");
    let line = compile_line("long", &text);

    let fonts = FontSet::with_default_dirs(&[]);
    let full = handle_line(&line, &fonts, &RenderOptions::default(), None);
    assert!(full.extra_lines.is_empty(), "no sibling without capabilities");
    let full_payload = payload_of(&full.line);
    let full_len = full.line.len();
    let total = numbers_of(&full_payload).len();
    assert!(total > 1, "the document must span pages to paginate (got {total})");
    assert!(
        diagnostics_of(&full_payload).iter().any(|d| d.get("code").and_then(Value::as_str) == Some("unknown_command")),
        "the permissive run carries the \\alpah diagnostic: {:?}",
        diagnostics_of(&full_payload)
    );

    // Three quarters of the reply: well above one page, well below the whole
    // document, so a leading prefix is served but not everything.
    let limit = full_len * 3 / 4;
    let paginated = with_limit(&line, limit);
    assert!(paginated.len() <= limit, "the paginated line fits the limit ({} over {limit})", paginated.len());
    let p = payload_of(&paginated);
    let expected_status = if status_of(&full_payload) == "ok" { "recovered" } else { status_of(&full_payload) };
    assert_ne!(status_of(&p), "failed", "pagination is not a failure: {p:?}");
    assert_eq!(status_of(&p), expected_status, "{p:?}");
    let served = numbers_of(&p);
    assert!(!served.is_empty(), "at least the first page fits a three-quarter limit: {p:?}");
    assert!(served.len() < total, "not every page fits: {served:?} of {total}");
    assert_eq!(served, (1..=served.len() as i64).collect::<Vec<_>>(), "a dense 1-based prefix: {served:?}");
    let diags = diagnostics_of(&p);
    assert!(
        diags.iter().any(|d| d.get("code").and_then(Value::as_str) == Some("unknown_command")),
        "the real diagnostic survives pagination: {diags:?}"
    );
    let warnings: Vec<&Value> = diags
        .iter()
        .filter(|d| d.get("code").and_then(Value::as_str) == Some("reply_paginated"))
        .collect();
    assert_eq!(warnings.len(), 1, "exactly one pagination warning: {diags:?}");
    assert_eq!(warnings[0].get("severity").and_then(Value::as_str), Some("warning"));
    let expected = format!(
        "compile_result would be {full_len} bytes for {total} pages, over the {limit}-byte reply limit; serving the first {} of {total} pages",
        served.len()
    );
    assert_eq!(warnings[0].get("message").and_then(Value::as_str), Some(expected.as_str()), "{diags:?}");
}

/// Diagnostics alone past the limit: nothing paginatable is left, so the
/// worker refuses closed -- and the refusal still names the exact byte
/// count the permissive run actually produces.
#[test]
fn diagnostics_past_the_limit_refuse_closed_with_exact_size() {
    if !lm_available() {
        return;
    }
    let mut text = "\\begin{document}\n".to_string();
    for i in 0..200 {
        text.push_str(&format!("Word \\bad{i} with more words on the line to give the diagnostic a source span.\n\n"));
    }
    text.push_str("\\end{document}\n");
    let line = compile_line("many", &text);

    let fonts = FontSet::with_default_dirs(&[]);
    let full = handle_line(&line, &fonts, &RenderOptions::default(), None);
    let full_payload = payload_of(&full.line);
    let full_len = full.line.len();
    let total = numbers_of(&full_payload).len();
    assert!(diagnostics_of(&full_payload).len() >= 100, "the document must be diagnostic-heavy");

    let limit = 6000;
    assert!(full_len > limit, "the reply must overflow the tiny limit (got {full_len} bytes)");
    let refused = with_limit(&line, limit);
    let p = payload_of(&refused);
    assert_eq!(status_of(&p), "failed");
    assert!(numbers_of(&p).is_empty(), "a refusal carries no pages");
    let diags = diagnostics_of(&p);
    assert_eq!(diags.len(), 1, "{diags:?}");
    let expected = format!(
        "compile_result would be {full_len} bytes for {total} pages, over the {limit}-byte reply limit; split the project or compile fewer pages"
    );
    assert_eq!(diags[0].get("message").and_then(Value::as_str), Some(expected.as_str()), "{diags:?}");
}
