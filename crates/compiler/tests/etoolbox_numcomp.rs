//! Lane etoolbox-ifnumcomp-ifdimcomp (slice 1): etoolbox's numeric and
//! dimension comparison conditionals (`\ifnumcomp` with its `\ifnumequal` /
//! `\ifnumgreater` / `\ifnumless` shortcuts, `\ifnumodd`, `\ifdimcomp` with
//! its `\ifdimequal` / `\ifdimgreater` / `\ifdimless` shortcuts) run in the
//! expansion pass as host-prelude macros over the engine's own `\ifnum` /
//! `\ifdim` / `\ifodd` with `\numexpr` / `\dimexpr` operands -- the exact
//! shape etoolbox.sty (TeX Live 2026) uses. They are gated on the
//! `etoolbox` package load (`\@ifpackageloaded`): without it they expand to
//! a never-defined marker the parser reports as `unknown_command`, the way
//! pdflatex reports `Undefined control sequence`.
//!
//! Oracle: `pdflatex -interaction=nonstopmode` (TeX Live 2026) on the same
//! bodies. Measured 2026-09-23:
//! `etb.tex` (`\usepackage{etoolbox}`, body below) exits 0 and typesets
//! "gt eq lt odd. big small same."; `noetb.tex` (plain article, body
//! `\ifnumcomp{3}{>}{2}{gt}{le}.`) reports `! Undefined control sequence.`
//! and typesets "3>2gtle.".

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

/// Probe text as rendered: words joined with spaces, except that punctuation
/// following an expansion-produced group itemizes separately (`"odd", "."`
/// where source text gives `"odd."`) while rendering attached, as in
/// pdflatex -- so the join collapses a space before `.`.
fn display_text(reply: &Value) -> String {
    page_words(reply).join(" ").replace(" .", ".")
}

fn plain_article(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn repro_matches_pdflatex_branches_and_zero_diagnostics() {
    // Exact task repro body; pdflatex typesets "gt eq lt odd." with 0 errors.
    let reply = compile(
        "repro.tex",
        &document("\\ifnumcomp{3}{>}{2}{gt}{le} \\ifnumequal{2}{2}{eq}{ne} \\ifdimcomp{1pt}{<}{2pt}{lt}{ge} \\ifnumodd{3}{odd}{even}."),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(display_text(&reply), "gt eq lt odd.");
}

#[test]
fn numcomp_selects_true_and_false_branches() {
    for (cond, first, second, want) in [
        ("{3}{>}{2}", "gt", "le", "gt"),
        ("{3}{<}{2}", "gt", "le", "le"),
        ("{2}{=}{2}", "eq", "ne", "eq"),
        ("{2}{=}{3}", "eq", "ne", "ne"),
    ] {
        let reply = compile(
            "numcomp.tex",
            &document(&format!("\\ifnumcomp{cond}{{{first}}}{{{second}}}")),
        );
        assert!(diagnostics(&reply).is_empty());
        assert_eq!(page_words(&reply).join(" "), want, "for {cond}");
    }
}

#[test]
fn num_shortcuts_accept_numexpr_operands() {
    // `2*3` exercises the `\numexpr` operand grammar, not just literals.
    // Measured pdflatex (`etb.tex`): "big small same."
    let reply = compile(
        "num-short.tex",
        &document("\\ifnumgreater{2*3}{5}{big}{small} \\ifnumless{1}{2}{small}{big} \\ifdimequal{2pt}{2pt}{same}{diff}."),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(display_text(&reply), "big small same.");
}

#[test]
fn numodd_selects_parity() {
    // Measured pdflatex (`oracle.tex`): "odd even."
    let reply = compile(
        "numodd.tex",
        &document("\\ifnumodd{3}{odd}{even} \\ifnumodd{4}{odd}{even}."),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(display_text(&reply), "odd even.");
}

#[test]
fn dimcomp_and_shortcuts_accept_dimexpr_operands() {
    // `1pt+1pt` exercises the `\dimexpr` operand grammar. Measured pdflatex
    // (`oracle.tex`): "lt same gt lt."
    let reply = compile(
        "dimcomp.tex",
        &document("\\ifdimcomp{1pt}{<}{2pt}{lt}{ge} \\ifdimequal{2pt}{2pt}{same}{diff} \\ifdimgreater{1pt+1pt}{1pt}{gt}{le} \\ifdimless{1pt}{2pt}{lt}{ge}."),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(display_text(&reply), "lt same gt lt.");
}

#[test]
fn comparisons_are_rejected_without_etoolbox_like_pdflatex() {
    // pdflatex under plain article: `! Undefined control sequence.` Here the
    // never-defined gate marker must surface as an `unknown_command` error.
    for (name, use_) in [
        ("numcomp", "\\ifnumcomp{3}{>}{2}{gt}{le}"),
        ("numequal", "\\ifnumequal{2}{2}{eq}{ne}"),
        ("numgreater", "\\ifnumgreater{3}{2}{gt}{le}"),
        ("numless", "\\ifnumless{1}{2}{lt}{ge}"),
        ("numodd", "\\ifnumodd{3}{odd}{even}"),
        ("dimcomp", "\\ifdimcomp{1pt}{<}{2pt}{lt}{ge}"),
        ("dimequal", "\\ifdimequal{2pt}{2pt}{same}{diff}"),
        ("dimgreater", "\\ifdimgreater{2pt}{1pt}{gt}{le}"),
        ("dimless", "\\ifdimless{1pt}{2pt}{lt}{ge}"),
    ] {
        let reply = compile(&format!("nogate-{name}.tex"), &plain_article(use_));
        let found = diagnostics(&reply);
        assert!(
            found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
            "expected an unknown_command error for \\{name} without etoolbox (all: {found:?})"
        );
    }
}
