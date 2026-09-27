//! Lane `etoolbox-newrobustcmd`: etoolbox's robust-command definers
//! (`\newrobustcmd`, `\renewrobustcmd`, `\providerobustcmd`) run in the
//! expansion pass as gated host-prelude macros. With `etoolbox` loaded they
//! define `\protected` macros with `\newcommand` argument syntax (star,
//! `[n]`, `[n][default]`) and newcommand semantics (a `\newrobustcmd` of a
//! defined name errors and keeps the old definition, a `\renewrobustcmd` of
//! an undefined name errors and still defines, a `\providerobustcmd` defines
//! only when free). Without `etoolbox` they are rejected the way pdflatex
//! rejects them (`! Undefined control sequence`).
//!
//! Ground truth (TeX Live 2026, `pdflatex -interaction=nonstopmode`):
//! - `repro.tex` (the doc below): exit 0, no `!` lines,
//!   `pdftotext` gives `A ¡b¿ Y c.` (`<`/`>` print as the OT1
//!   inverted-exclamation glyphs; the character sequence is `A <b> Y c.`).
//! - `\newcommand{\x}{old}` + `\newrobustcmd{\x}{new}`:
//!   `! LaTeX Error: Command \x already defined.`
//! - `\renewrobustcmd{\zzz}{new}` (undefined):
//!   `! Package etoolbox Error: \zzz undefined.`
//! - the same preamble without `\usepackage{etoolbox}`:
//!   `! Undefined control sequence. ... \newrobustcmd`, then further
//!   follow-on errors (`\x` undefined, `#` misuse) and text `[1][1] A b c.`
//! - `\typeout{\meaning...}`: no-arg `\newrobustcmd` gives
//!   `\protected macro:->...`, `[1]` gives `\protected\long macro:#1->...`,
//!   starred `[1]` gives `\protected macro:#1->...`, `[1][D]` gives
//!   `\protected macro:->\@testopt \\cmd {D}` with an inner
//!   `\long macro:[#1]->...` (long, not protected).

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

fn with_etoolbox(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{etoolbox}}\n{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn new_renew_and_provide_match_pdflatex() {
    // The supervisor's repro: pdflatex exits 0 with text `A <b> Y c.`.
    let reply = compile(
        "robust-repro.tex",
        &with_etoolbox(
            "\\newrobustcmd{\\x}[1]{[#1]}\n\\renewrobustcmd{\\x}[1]{<#1>}\n\\providerobustcmd{\\y}{Y}\n",
            "A \\x{b} \\y{} c.",
        ),
    );
    assert!(diagnostics(&reply).is_empty(), "unexpected diagnostics: {:?}", diagnostics(&reply));
    // FlashTeX words split `<b>` into `<`, `b`, `>` (exactly like the
    // native `\newcommand` path does); the character sequence is
    // pdflatex's `A <b> Y c.`.
    assert_eq!(page_words(&reply).join(" "), "A < b > Y c.");
}

#[test]
fn newrobustcmd_of_a_defined_name_errors_and_keeps_the_old_definition() {
    // pdflatex: `! LaTeX Error: Command \myx already defined.`
    let reply = compile(
        "robust-new-defined.tex",
        &with_etoolbox("\\newcommand{\\myx}{old}\n\\newrobustcmd{\\myx}{new}\n", "See \\myx."),
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, _, msg)| sev == "error" && msg.contains("already defined") && msg.contains("\\myx")),
        "expected the already-defined error (all: {found:?})"
    );
    assert_eq!(page_words(&reply).join(" "), "See old .");
}

#[test]
fn renewrobustcmd_of_an_undefined_name_errors_but_still_defines() {
    // pdflatex: `! Package etoolbox Error: \myz undefined.`, and the
    // definition still takes (error-and-continue recovery).
    let reply = compile(
        "robust-renew-undefined.tex",
        &with_etoolbox("\\renewrobustcmd{\\myz}{new}\n", "See \\myz."),
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, _, msg)| sev == "error"
            && msg.contains("etoolbox")
            && msg.contains("undefined")
            && msg.contains("\\myz")),
        "expected the etoolbox undefined error (all: {found:?})"
    );
    assert_eq!(page_words(&reply).join(" "), "See new .");
}

#[test]
fn providerobustcmd_defines_only_when_free() {
    // Over an existing definition it is a silent no-op, like pdflatex.
    let reply = compile(
        "robust-provide-existing.tex",
        &with_etoolbox("\\newcommand{\\myw}{old}\n\\providerobustcmd{\\myw}{new}\n", "See \\myw."),
    );
    assert!(diagnostics(&reply).is_empty(), "unexpected diagnostics: {:?}", diagnostics(&reply));
    assert_eq!(page_words(&reply).join(" "), "See old .");
    // Over a free name it defines, like pdflatex.
    let reply = compile(
        "robust-provide-new.tex",
        &with_etoolbox("\\providerobustcmd{\\myv}{new}\n", "See \\myv."),
    );
    assert!(diagnostics(&reply).is_empty(), "unexpected diagnostics: {:?}", diagnostics(&reply));
    assert_eq!(page_words(&reply).join(" "), "See new .");
}

#[test]
fn star_and_optional_default_forms_work() {
    // Starred (short), two arguments, and `[n][default]`, all silent.
    let reply = compile(
        "robust-forms.tex",
        &with_etoolbox(
            "\\newrobustcmd*{\\mys}[1]{[#1]}\n\\newrobustcmd{\\myt}[2]{[#1][#2]}\n\\newrobustcmd{\\myo}[1][D]{[#1]}\n",
            "\\mys{b} \\myt{1}{2} \\myo \\myo{E}.",
        ),
    );
    assert!(diagnostics(&reply).is_empty(), "unexpected diagnostics: {:?}", diagnostics(&reply));
    // Same word splitting as the native path; the character sequence is
    // pdflatex's `[b] [1][2] [D] [D]E.` (`\myo{E}` takes the default).
    assert_eq!(page_words(&reply).join(" "), "[ b ] [ 1 ][ 2 ] [ D ] [ D ] E .");
}

#[test]
fn defined_macros_are_protected() {
    // `\meaning` typesets the flags: pdflatex reports
    // `\protected\long macro:#1->[#1]` for `\newrobustcmd{\myx}[1]{[#1]}`
    // and `\protected macro:->...` for the no-arg form.
    let reply = compile(
        "robust-meaning.tex",
        &with_etoolbox("\\newrobustcmd{\\myx}[1]{[#1]}\n", "\\meaning\\myx"),
    );
    let words = page_words(&reply).join(" ");
    assert!(
        words.contains("\\protected"),
        "expected the protected flag in the meaning ({words:?}); diagnostics: {:?}",
        diagnostics(&reply)
    );
    assert!(
        words.contains("\\long"),
        "expected the long flag in the meaning ({words:?}); diagnostics: {:?}",
        diagnostics(&reply)
    );
}

#[test]
fn robust_commands_are_rejected_without_etoolbox() {
    // pdflatex under plain article: `! Undefined control sequence.`
    // at `\newrobustcmd`. FlashTeX must diagnose it too.
    let reply = compile(
        "robust-noetoolbox.tex",
        "\\documentclass{article}\n\\newrobustcmd{\\x}[1]{[#1]}\n\\begin{document}\nA \\x{b} c.\n\\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, msg)| sev == "error"
            && code == "unknown_command"
            && msg.contains("newrobustcmd")),
        "expected an unknown_command error for \\newrobustcmd without etoolbox (all: {found:?})"
    );
}
