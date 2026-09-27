//! Lane `etoolbox-ifdef-ifcsdef`: etoolbox's definedness tests (`\ifdef`,
//! `\ifundef`, `\ifcsdef`, `\ifcsundef`, `\ifdefmacro`) run in the
//! expansion pass as host-prelude macros using etoolbox.sty's own
//! `\ifdefined`/`\ifcsname`/`\meaning` implementation, installed only when
//! `etoolbox` is loaded. Branch expectations are the values pdflatex
//! (TeX Live 2026) printed for the same probes (see check-in).

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

fn compile(path: &str, text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_(path));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("etoolbox-ifdef"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(path));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("etoolbox-ifdef"));
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

/// `\newcommand\foo{F}` plus a `\relax`-valued name, like the pdflatex probe.
fn defs() -> &'static str {
    "\\newcommand\\foo{F}\\let\\baz\\relax"
}

#[test]
fn repro_row_matches_pdflatex() {
    // pdflatex PROBE-MAIN=[d d cd cu], 0 errors.
    let reply = compile(
        "ifdef-repro.tex",
        &document(&format!(
            "{}\\ifdef{{\\foo}}{{d}}{{u}} \\ifundef{{\\bar}}{{u}}{{d}} \\ifcsdef{{foo}}{{cd}}{{cu}} \\ifcsundef{{nope}}{{cu}}{{cd}}",
            defs()
        )),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(page_words(&reply).join(" "), "d d cd cu");
}

#[test]
fn ifdef_defined_undefined_and_relax() {
    // pdflatex: [d] [u] [d] — a `\relax`-valued name counts as defined.
    let reply = compile(
        "ifdef-relax.tex",
        &document(&format!(
            "{}\\ifdef{{\\foo}}{{d}}{{u}} \\ifdef{{\\nope}}{{d}}{{u}} \\ifdef{{\\baz}}{{d}}{{u}}",
            defs()
        )),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(page_words(&reply).join(" "), "d u d");
}

#[test]
fn ifundef_defined_undefined_kernel_and_relax() {
    // pdflatex: [d] [u] [d] [u] — `\bar` is kernel-defined so the false
    // branch wins, while a `\relax`-valued name counts as undefined.
    let reply = compile(
        "ifundef-relax.tex",
        &document(&format!(
            "{}\\ifundef{{\\foo}}{{u}}{{d}} \\ifundef{{\\nope}}{{u}}{{d}} \\ifundef{{\\bar}}{{u}}{{d}} \\ifundef{{\\baz}}{{u}}{{d}}",
            defs()
        )),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(page_words(&reply).join(" "), "d u d u");
}

#[test]
fn ifcsdef_defined_relax_and_undefined() {
    // pdflatex: [cd] [cd] [cu] — unlike `\ifdef`, the csname form treats a
    // `\relax`-valued name as defined here (`\ifcsname` is true for it).
    let reply = compile(
        "ifcsdef-relax.tex",
        &document(&format!(
            "{}\\ifcsdef{{foo}}{{cd}}{{cu}} \\ifcsdef{{baz}}{{cd}}{{cu}} \\ifcsdef{{nope}}{{cd}}{{cu}}",
            defs()
        )),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(page_words(&reply).join(" "), "cd cd cu");
}

#[test]
fn ifcsundef_defined_relax_and_undefined() {
    // pdflatex: [cd] [cu] [cu] — the inner `\ifx...\relax` test sends a
    // `\relax`-valued name to the true (undefined) branch.
    let reply = compile(
        "ifcsundef-relax.tex",
        &document(&format!(
            "{}\\ifcsundef{{foo}}{{cu}}{{cd}} \\ifcsundef{{baz}}{{cu}}{{cd}} \\ifcsundef{{nope}}{{cu}}{{cd}}",
            defs()
        )),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(page_words(&reply).join(" "), "cd cu cu");
}

#[test]
fn ifdefmacro_macro_undefined_and_relax() {
    // pdflatex: [m] [n] [n] for these three. (`\bar` also answers [m]
    // under pdflatex, but it is a host command in this compiler rather
    // than a macro, so `\ifdefmacro` reports it as a non-macro here;
    // that kernel-accent gap is out of scope for this lane.)
    let reply = compile(
        "ifdefmacro.tex",
        &document(&format!(
            "{}\\ifdefmacro{{\\foo}}{{m}}{{n}} \\ifdefmacro{{\\nope}}{{m}}{{n}} \\ifdefmacro{{\\baz}}{{m}}{{n}}",
            defs()
        )),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(page_words(&reply).join(" "), "m n n");
}

#[test]
fn ifdef_family_rejected_without_etoolbox() {
    // pdflatex under plain article: `! Undefined control sequence.` for
    // `\ifdef`, so the family must not exist without the package.
    let reply = compile(
        "ifdef-no-package.tex",
        "\\documentclass{article}\n\\begin{document}\n\\newcommand\\foo{F}\\ifdef{\\foo}{d}{u} \\ifcsdef{foo}{cd}{cu} \\ifdefmacro{\\foo}{m}{n}\n\\end{document}\n",
    );
    let found = diagnostics(&reply);
    for name in ["\\ifdef", "\\ifcsdef", "\\ifdefmacro"] {
        assert!(
            found
                .iter()
                .any(|(sev, code, msg)| sev == "error" && code == "unknown_command" && msg.contains(name)),
            "expected an unknown-command error for {name} without etoolbox (all: {found:?})"
        );
    }
}
