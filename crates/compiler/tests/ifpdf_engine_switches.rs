//! `ifpdf` / `iftex` engine switches under a pdflatex-equivalent engine.
//!
//! Oracle: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), `-interaction=nonstopmode`:
//! - `\usepackage{ifpdf}` + `\ifpdf\usepackage{graphicx}\else\usepackage{epsfig}\fi`
//!   in the preamble: exit 0, 0 errors, true branch taken;
//! - `\usepackage{iftex}` + `\ifPDFTeX` / `\ifpdftex` / `\ifpdf`: exit 0,
//!   0 errors, PDF text reads `PDFYES-ptYES-pdfYES` (all three true).
//! (`iftex.sty` v1.0g: `\ifpdftex`+`\ifPDFTeX` alias true under pdfTeX,
//! `\ifpdf` true when `\pdfoutput>0`; `ifpdf.sty` is a stub over `iftex`.)
//!
//! FlashTeX is pdflatex-equivalent, so all three switches are true here.
//! `\ifpdf` is defined by the expansion prelude unconditionally -- real
//! documents test it without loading `ifpdf` (hyperref uses it, defined
//! through its own `iftex` requirement).

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

fn compile(path: &str, text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_(path));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("ifpdf-switches"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(path));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("ifpdf-switches"));
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

fn errors(found: &[(String, String, String)]) -> Vec<(String, String, String)> {
    found.iter().filter(|(sev, _, _)| sev == "error").cloned().collect()
}

#[test]
fn ifpdf_preamble_conditional_takes_the_true_branch() {
    // The reported reproducer: on the base this is `error \ifpdf is not
    // supported in the document preamble` plus `Extra \else.` / `Extra \fi.`
    // The true branch loads graphicx (silent); the false branch would load
    // epsfig (warns), so silence also proves the branch choice.
    let reply = compile(
        "ifpdf-preamble.tex",
        "\\documentclass{article}\n\
         \\usepackage{ifpdf}\n\
         \\ifpdf\\usepackage{graphicx}\\else\\usepackage{epsfig}\\fi\n\
         \\begin{document}\n\
         \\ifpdf TRUEPDF \\else FALSEPDF \\fi\n\
         \\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(errors(&found).is_empty(), "error diagnostics remain: {found:?}");
    assert!(
        !found.iter().any(|(_, _, msg)| msg.contains("epsfig")),
        "false branch was taken (all: {found:?})"
    );
    assert_eq!(page_words(&reply).join(" "), "TRUEPDF");
}

#[test]
fn iftex_pdftex_switches_are_true() {
    // `\ifPDFTeX` (and lowercase `\ifpdftex`) are iftex.sty's engine test,
    // true under pdflatex; `\ifpdf` is the output-mode test, true when
    // `\pdfoutput>0`. Oracle body text: `PDFYES-ptYES-pdfYES`.
    let reply = compile(
        "iftex-pdftex.tex",
        "\\documentclass{article}\n\
         \\usepackage{iftex}\n\
         \\begin{document}\n\
         \\ifPDFTeX PDFYES\\else PDFNO\\fi-\\ifpdftex ptYES\\else ptNO\\fi-\\ifpdf pdfYES\\else pdfNO\\fi\n\
         \\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(errors(&found).is_empty(), "error diagnostics remain: {found:?}");
    assert_eq!(page_words(&reply).join(""), "PDFYES-ptYES-pdfYES");
}

#[test]
fn ifpdf_works_without_the_package() {
    // Real documents test `\ifpdf` without loading ifpdf (hyperref uses
    // it, defined through its own `iftex` requirement); the prelude
    // defines it unconditionally, exactly like `\ifxetex`/`\ifluatex`.
    let reply = compile(
        "ifpdf-nopackage.tex",
        "\\documentclass{article}\n\
         \\begin{document}\n\
         \\ifpdf TRUEPDF \\else FALSEPDF \\fi\n\
         \\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(errors(&found).is_empty(), "error diagnostics remain: {found:?}");
    assert_eq!(page_words(&reply).join(" "), "TRUEPDF");
}

#[test]
fn ifpdf_package_load_is_silent_but_options_still_warn() {
    // `ifpdf.sty` takes no options; like `iftex`, loading it is silent
    // because the switches run in the expansion pass.
    let reply = compile(
        "ifpdf-load.tex",
        "\\documentclass{article}\n\\usepackage{ifpdf}\n\\begin{document}\nx\n\\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(
        !found.iter().any(|(_, code, _)| code == "unsupported_feature"),
        "option-free \\usepackage{{ifpdf}} must be silent (all: {found:?})"
    );
    let reply = compile(
        "ifpdf-opt.tex",
        "\\documentclass{article}\n\\usepackage[foo]{ifpdf}\n\\begin{document}\nx\n\\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(
        found
            .iter()
            .any(|(_, code, msg)| code == "unsupported_feature" && msg.contains("ifpdf")),
        "expected the ifpdf package warning for an unknown option (all: {found:?})"
    );
}
