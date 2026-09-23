//! Lane etoolbox-newbool-ifbool: etoolbox's TeX-bool booleans (`\newbool`,
//! `\providebool`, `\booltrue`/`\boolfalse`, `\setbool`,
//! `\ifbool`/`\notbool`) run in the expansion pass as host-prelude macros
//! over the same `\newif` representation etoolbox.sty itself uses.
//!
//! Reference (all measured, TeX Live 2026, `pdflatex -interaction=nonstopmode`):
//! - `probe.tex` (the slice-1 repro: preamble
//!   `\newbool{b}\booltrue{b}\newbool{c}`, body
//!   `\ifbool{b}{T}{F} \ifbool{c}{T}{F} \boolfalse{b}\ifbool{b}{T}{F} \notbool{c}{N}{Y}.`):
//!   0 errors, `pdftotext` gives `T F F N.`
//! - `probe2.tex` (`\setbool{b}{false}\ifbool{b}{T}{F}
//!   \setbool{b}{true}\ifbool{b}{T}{F} \providebool{b}\ifbool{b}{T}{F}
//!   \providebool{d}\ifbool{d}{T}{F}.`): 0 errors, `F T T F.`
//! - duplicate `\newbool{b}` after `\booltrue{b}`: one LaTeX error
//!   (`Command \ifb already defined`), recovery keeps the `true` state.
//! - `\ifbool{nope}{T}{F}.` with no such bool: one etoolbox error
//!   (`Boolean '\ifnope' undefined`), both branches gobbled, text `.`
//! - `\setbool{b}{maybe}\ifbool{b}{T}{F}.`: one etoolbox error
//!   (`Invalid boolean value 'maybe'`), state unchanged, text `F.`

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

fn document(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{etoolbox}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// The typeset probe text: words joined with spaces, with a sentence-final
/// `.` re-attached (the engine emits punctuation following a `}` group as
/// its own text item, so `Y}.` comes out as words `["Y", "."]`).
fn probe_text(reply: &Value) -> String {
    page_words(reply).join(" ").replace(" .", ".")
}

#[test]
fn bools_select_the_pdflatex_branches() {
    // The slice-1 repro verbatim: pdflatex typesets `T F F N.` with 0 errors.
    let reply = compile(
        "bools-repro.tex",
        &document(
            "\\newbool{b}\\booltrue{b}\\newbool{c}",
            "\\ifbool{b}{T}{F} \\ifbool{c}{T}{F} \\boolfalse{b}\\ifbool{b}{T}{F} \\notbool{c}{N}{Y}.",
        ),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(probe_text(&reply), "T F F N.");
}

#[test]
fn setbool_and_providebool_follow_pdflatex() {
    // Measured pdflatex: 0 errors, `F T T F.`
    let reply = compile(
        "bools-set-provide.tex",
        &document(
            "\\newbool{b}\\booltrue{b}",
            "\\setbool{b}{false}\\ifbool{b}{T}{F} \\setbool{b}{true}\\ifbool{b}{T}{F} \\providebool{b}\\ifbool{b}{T}{F} \\providebool{d}\\ifbool{d}{T}{F}.",
        ),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(probe_text(&reply), "F T T F.");
}

#[test]
fn duplicate_newbool_is_diagnosed_and_keeps_state() {
    // Real etoolbox errors (`Command \ifb already defined`) without
    // redefining the bool; the prelude has no `\errmessage` to borrow, so
    // the duplicate expands to a marker the parser reports at the use span
    // instead, exactly like the toggle prelude's duplicate handling.
    let reply = compile(
        "bools-dup.tex",
        &document(
            "\\newbool{b}\\booltrue{b}\\newbool{b}",
            "\\ifbool{b}{T}{F}",
        ),
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an error for the duplicate \\newbool (all: {found:?})"
    );
    assert_eq!(page_words(&reply).join(" "), "T");
}

#[test]
fn use_of_an_undefined_bool_is_diagnosed_and_typesets_nothing() {
    // Measured pdflatex: one `Boolean '\ifnope' undefined` error, both
    // branches gobbled, text `.`
    let reply = compile("bools-undef.tex", &document("", "\\ifbool{nope}{T}{F}."));
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an error for the undefined bool (all: {found:?})"
    );
    assert_eq!(probe_text(&reply), ".");
}

#[test]
fn setbool_with_a_bad_value_is_diagnosed_and_keeps_state() {
    // Measured pdflatex: one `Invalid boolean value 'maybe'` error, the
    // bool stays false, text `F.`
    let reply = compile(
        "bools-badval.tex",
        &document("\\newbool{b}", "\\setbool{b}{maybe}\\ifbool{b}{T}{F}."),
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an error for the bad \\setbool value (all: {found:?})"
    );
    assert_eq!(probe_text(&reply), "F.");
}

#[test]
fn booltrue_on_an_undefined_bool_is_diagnosed() {
    let reply = compile(
        "bools-set-undef.tex",
        &document("", "\\booltrue{nope}\\newbool{q}\\boolfalse{q}\\ifbool{q}{T}{F}"),
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an error for \\booltrue on an undefined bool (all: {found:?})"
    );
    assert_eq!(page_words(&reply).join(" "), "F");
}
