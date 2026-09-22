//! `\patchcmd{\cmd}{search}{replace}{success}{failure}` (slice 1): a
//! literal find-and-replace on the macro's stored replacement text,
//! run as an engine primitive. A present search string redefines the
//! macro and expands the success branch; an absent one leaves the
//! macro alone and expands the failure branch, silently either way.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

fn compile(path: &str, text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_(path));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("patchcmd"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(path));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("patchcmd"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply")
}

fn diagnostics(reply: &Value) -> Vec<(String, String, String)> {
    reply
        .get("payload")
        .expect("payload")
        .get("diagnostics")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|d| {
            (
                d.get("severity").and_then(|s| s.as_str()).unwrap_or("?").to_string(),
                d.get("code").and_then(|c| c.as_str()).unwrap_or("?").to_string(),
                d.get("message").and_then(|m| m.as_str()).unwrap_or("?").to_string(),
            )
        })
        .collect()
}

/// The words typeset on every page, in order.
fn page_words(reply: &Value) -> Vec<String> {
    reply
        .get("payload")
        .expect("payload")
        .get("pages")
        .and_then(|p| p.as_arr())
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|pg| {
            pg.get("items").and_then(|i| i.as_arr()).cloned().unwrap_or_default()
        })
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("text"))
        .filter_map(|item| item.get("text").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{etoolbox}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn patchcmd_success_redefines_the_macro() {
    let reply = compile(
        "patch-success.tex",
        &document("\\newcommand{\\greet}{Hello}\\patchcmd{\\greet}{Hello}{Hi}{}{}\\greet"),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(page_words(&reply).join(" "), "Hi", "patched body must typeset");
}

#[test]
fn patchcmd_failure_typesets_failure_and_keeps_the_macro() {
    let reply = compile(
        "patch-failure.tex",
        &document("\\newcommand{\\greet}{Hello}\\patchcmd{\\greet}{Nope}{X}{success text}{failure text} \\greet"),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(
        page_words(&reply).join(" "),
        "failure text Hello",
        "failure branch typesets and the macro keeps its original text"
    );
}
