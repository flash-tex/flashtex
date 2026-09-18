//! Issue #837 (slice 1): etoolbox's toggle booleans (`\newtoggle`,
//! `\providetoggle`, `\toggletrue`/`\togglefalse`, `\iftoggle`) run in the
//! expansion pass as host-prelude macros over the same `\@firstoftwo` /
//! `\@secondoftwo` representation etoolbox.sty itself uses, and
//! `\usepackage{etoolbox}` loads silently when option-free.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

fn compile(path: &str, text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_(path));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("etoolbox"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(path));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("etoolbox"));
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
fn toggle_true_selects_the_true_branch_only() {
    let reply = compile(
        "toggle-true.tex",
        &document("\\newtoggle{draft}\\toggletrue{draft}\\iftoggle{draft}{This is a draft.}{Final version.}"),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(
        page_words(&reply).join(" "),
        "This is a draft.",
        "true branch only"
    );
}

#[test]
fn toggle_starts_false_and_togglefalse_resets_it() {
    let reply = compile(
        "toggle-false.tex",
        &document("\\newtoggle{draft}\\iftoggle{draft}{This is a draft.}{Final version.}"),
    );
    assert!(diagnostics(&reply).is_empty());
    assert_eq!(page_words(&reply).join(" "), "Final version.");

    let reply = compile(
        "toggle-reset.tex",
        &document("\\newtoggle{draft}\\toggletrue{draft}\\togglefalse{draft}\\iftoggle{draft}{T}{F}"),
    );
    assert!(diagnostics(&reply).is_empty());
    assert_eq!(page_words(&reply).join(" "), "F");
}

#[test]
fn providetoggle_is_a_noop_on_an_existing_toggle() {
    // Keeps the `true` state a plain `\newtoggle` would also keep: no
    // diagnostic, and the true branch still wins.
    let reply = compile(
        "provide-existing.tex",
        &document("\\newtoggle{draft}\\toggletrue{draft}\\providetoggle{draft}\\iftoggle{draft}{T}{F}"),
    );
    assert!(diagnostics(&reply).is_empty());
    assert_eq!(page_words(&reply).join(" "), "T");
}

#[test]
fn providetoggle_declares_a_new_toggle_as_false() {
    let reply = compile(
        "provide-new.tex",
        &document("\\providetoggle{draft}\\iftoggle{draft}{T}{F}"),
    );
    assert!(diagnostics(&reply).is_empty());
    assert_eq!(page_words(&reply).join(" "), "F");
}

#[test]
fn duplicate_newtoggle_is_diagnosed_and_keeps_state() {
    // Real etoolbox errors here without redefining the toggle; the
    // prelude has no `\errmessage` to borrow, so the duplicate expands
    // to a marker the parser reports at the use span instead.
    let reply = compile(
        "toggle-dup.tex",
        &document("\\newtoggle{draft}\\toggletrue{draft}\\newtoggle{draft}\\iftoggle{draft}{T}{F}"),
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an error for the duplicate \\newtoggle (all: {found:?})"
    );
    assert_eq!(page_words(&reply).join(" "), "T");
}

#[test]
fn use_of_an_undefined_toggle_is_diagnosed_and_typesets_nothing() {
    let reply = compile("toggle-undef.tex", &document("\\iftoggle{nope}{T}{F}"));
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an error for the undefined toggle (all: {found:?})"
    );
    assert!(
        page_words(&reply).is_empty(),
        "undefined toggle must gobble both branches, like etoolbox: {:?}",
        page_words(&reply)
    );
}

#[test]
fn etoolbox_package_load_is_silent_but_options_still_warn() {
    let reply = compile(
        "etoolbox-load.tex",
        "\\documentclass{article}\n\\usepackage{etoolbox}\n\\begin{document}\nx\n\\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(
        !found.iter().any(|(_, code, _)| code == "unsupported_feature"),
        "option-free \\usepackage{{etoolbox}} must be silent (all: {found:?})"
    );
    let reply = compile(
        "etoolbox-opt.tex",
        "\\documentclass{article}\n\\usepackage[foo]{etoolbox}\n\\begin{document}\nx\n\\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(
        found
            .iter()
            .any(|(_, code, msg)| code == "unsupported_feature" && msg.contains("etoolbox")),
        "expected the etoolbox package warning for an unknown option (all: {found:?})"
    );
}
