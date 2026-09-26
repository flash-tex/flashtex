//! Lane etoolbox-ifdefempty-ifdefvoid: etoolbox's emptiness/voidness
//! conditionals (`\ifdefempty`/`\ifcsempty`, `\ifdefvoid`/`\ifcsvoid`) run in
//! the expansion pass as host-prelude macros mirroring etoolbox.sty's own
//! `\ifundef`/`\ifdefmacro`/`\ifdefparam`/`\etb@ifdefempty` chain, and
//! `\usepackage{etoolbox}` loads silently when option-free.
//!
//! Measured against pdflatex (TeX Live 2026):
//! `pdflatex -interaction=nonstopmode repro.tex` (body
//! `\newcommand\e{}\newcommand\f{F}` + `\ifdefempty{\e}{empty}{full}
//! \ifdefempty{\f}{empty}{full} \ifdefvoid{\zz}{void}{set}.`) exits 0 with no
//! errors and typesets `empty full void.`; the extended probe below
//! (`extended.tex`: empty, non-empty, undefined and `\relax` macros, a macro
//! with a parameter, and every `cs` variant) exits 0 with no errors and
//! typesets `empty full void ce cf cv rf rv gf gs rcf rcv`. Without
//! `\usepackage{etoolbox}` pdflatex answers `! Undefined control sequence.`
//! for `\ifdefempty`.

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

fn etoolbox_document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{etoolbox}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn etoolbox_preamble() -> &'static str {
    "\\newcommand\\e{}\n\\newcommand\\f{F}\n\\let\\r=\\relax\n\\newcommand\\g[1]{#1}\n"
}

#[test]
fn ifdefempty_and_ifdefvoid_match_pdflatex() {
    // The reported repro: pdflatex typesets `empty full void.` with no errors.
    let reply = compile(
        "ifdefempty-repro.tex",
        &etoolbox_document(&format!(
            "{}\\ifdefempty{{\\e}}{{empty}}{{full}} \\ifdefempty{{\\f}}{{empty}}{{full}} \\ifdefvoid{{\\zz}}{{void}}{{set}}.",
            etoolbox_preamble()
        )),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    // pdflatex (`pdflatex -interaction=nonstopmode repro.tex`) typesets
    // `empty full void.`. The compiler keeps macro-expanded text and the
    // source `.` that follows it as separate text items (a plain
    // `\newcommand\foo{void}` + `\foo.` splits identically, as does
    // `\iftoggle{draft}{void}{set}.`), glued with no space, so the word
    // vector carries the trailing `.` on its own.
    assert_eq!(
        page_words(&reply),
        vec!["empty", "full", "void", "."],
        "wrong branch selected"
    );
}

#[test]
fn cs_variants_relax_and_param_macros_match_pdflatex() {
    // `extended.tex` under pdflatex: exit 0, no errors,
    // `empty full void ce cf cv rf rv gf gs rcf rcv`.
    let body = format!(
        "{}\\ifdefempty{{\\e}}{{empty}}{{full}} \\ifdefempty{{\\f}}{{empty}}{{full}} \\ifdefvoid{{\\zz}}{{void}}{{set}}\n\
         \\ifcsempty{{e}}{{ce}}{{cf}} \\ifcsempty{{f}}{{ce}}{{cf}} \\ifcsvoid{{zz}}{{cv}}{{cs}}\n\
         \\ifdefempty{{\\r}}{{re}}{{rf}} \\ifdefvoid{{\\r}}{{rv}}{{rs}}\n\
         \\ifdefempty{{\\g}}{{ge}}{{gf}} \\ifdefvoid{{\\g}}{{gv}}{{gs}}\n\
         \\ifcsempty{{r}}{{rce}}{{rcf}} \\ifcsvoid{{r}}{{rcv}}{{rcs}}",
        etoolbox_preamble()
    );
    let reply = compile("ifcsempty-void.tex", &etoolbox_document(&body));
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(
        page_words(&reply).join(" "),
        "empty full void ce cf cv rf rv gf gs rcf rcv",
        "pdflatex (`pdflatex -interaction=nonstopmode extended.tex`) typesets this"
    );
}

#[test]
fn conditionals_are_rejected_without_etoolbox_like_pdflatex() {
    // Without the package pdflatex answers `! Undefined control sequence.`
    // for `\ifdefempty`; here each use must be an `unknown_command` error.
    for (name, use_) in [
        ("ifdefempty", "\\ifdefempty{\\e}{empty}{full}"),
        ("ifcsempty", "\\ifcsempty{e}{ce}{cf}"),
        ("ifdefvoid", "\\ifdefvoid{\\zz}{void}{set}"),
        ("ifcsvoid", "\\ifcsvoid{zz}{cv}{cs}"),
    ] {
        let reply = compile(
            &format!("no-etoolbox-{name}.tex"),
            &format!(
                "\\documentclass{{article}}\n\\newcommand\\e{{}}\n\\begin{{document}}\n{use_}.\n\\end{{document}}\n"
            ),
        );
        let found = diagnostics(&reply);
        assert!(
            found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
            "expected an `unknown_command` error for \\{name} without etoolbox (all: {found:?})"
        );
    }
}
