//! Lane etoolbox-atbeginenvironment (v0.2.0 gate B8): etoolbox's
//! environment hooks. `\AtBeginEnvironment{env}{code}` runs `code` inside
//! the environment's group ahead of `\env`, `\AtEndEnvironment` ahead of
//! `\endenv`, `\BeforeBeginEnvironment` before the group opens and
//! `\AfterEndEnvironment` after it closes (etoolbox.sty on the kernel's
//! `env/<name>/<kind>` hooks). They used to be passed through unexpanded:
//! three "not supported in the preamble" errors and every hook dropped.
//!
//! Expected text is pdflatex's (TeX Live 2026, `pdflatex
//! -interaction=nonstopmode`, 0 `!` lines, read back with pymupdf).

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

fn compile(text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("etoolbox-hooks"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    // Per-run font hints (`weight`/`style`), to tell italic from upright.
    payload.set("layout_capabilities", Value::Arr(vec![json::str_("font-hints-v1")]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("etoolbox-hooks"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply")
}

/// `(severity, message)` of every error and warning.
fn problems(reply: &Value) -> Vec<(String, String)> {
    reply
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|d| {
            (
                d.get("severity").and_then(|s| s.as_str()).unwrap_or("?").to_string(),
                d.get("message").and_then(|m| m.as_str()).unwrap_or("?").to_string(),
            )
        })
        .filter(|(s, _)| s == "error" || s == "warning")
        .collect()
}

/// `(text, font)` of every text item on every page, in order.
fn runs(reply: &Value) -> Vec<(String, String)> {
    reply
        .get("payload")
        .and_then(|p| p.get("pages"))
        .and_then(|p| p.as_arr())
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|pg| pg.get("items").and_then(|i| i.as_arr()).cloned().unwrap_or_default())
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("text"))
        .map(|item| {
            let text = item.get("text").and_then(Value::as_str).unwrap_or("").to_string();
            let font = item.get("font").map(json::write).unwrap_or_default();
            (text, font)
        })
        .collect()
}

fn document(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{etoolbox}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn text(runs: &[(String, String)]) -> String {
    runs.iter().map(|r| r.0.as_str()).collect::<Vec<_>>().join(" ")
}

#[test]
fn at_begin_and_at_end_environment_run_inside_the_group() {
    // pdflatex: "Before." / "Quoted text End." in CMTI10 / "After." upright.
    let reply = compile(&document(
        "\\AtBeginEnvironment{quote}{\\itshape}\n\\AtEndEnvironment{quote}{ End.}",
        "Before.\n\\begin{quote}Quoted text\\end{quote}\nAfter.",
    ));
    assert_eq!(problems(&reply), Vec::<(String, String)>::new());
    let r = runs(&reply);
    let t = text(&r);
    assert!(t.contains("Quoted") && t.contains("End."), "{t}");
    let font_of = |w: &str| r.iter().find(|x| x.0.contains(w)).map(|x| x.1.clone()).unwrap_or_default();
    let quoted = font_of("Quoted");
    assert_ne!(quoted, font_of("Before"), "the hook's \\itshape must reach the quote body: {r:?}");
    assert_eq!(font_of("End"), quoted, "the end hook runs inside the group, still italic: {r:?}");
    assert_eq!(font_of("After"), font_of("Before"), "the group closes the italic: {r:?}");
}

#[test]
fn before_and_after_hooks_run_outside_the_group() {
    // pdflatex: "Before." / "[B]" / "Quoted text" / "[A] After.", and
    // `\itshape` set in the `before` hook stays on after `\end{quote}`.
    let reply = compile(&document(
        "\\BeforeBeginEnvironment{quote}{[B]}\n\\AfterEndEnvironment{quote}{[A]}",
        "Before.\n\n\\begin{quote}Quoted text\\end{quote}\nAfter.",
    ));
    assert_eq!(problems(&reply), Vec::<(String, String)>::new());
    let t = text(&runs(&reply));
    let (b, q, a) = (t.find("[B]"), t.find("Quoted"), t.find("[A]"));
    assert!(b.is_some() && q.is_some() && a.is_some() && b < q && q < a, "{t}");
}

#[test]
fn a_before_hook_declaration_leaks_past_the_environment() {
    let reply = compile(&document(
        "\\BeforeBeginEnvironment{quote}{\\bfseries}",
        "Before.\n\n\\begin{quote}Quoted text\\end{quote}\nAfter.",
    ));
    assert_eq!(problems(&reply), Vec::<(String, String)>::new());
    let r = runs(&reply);
    let font_of = |w: &str| r.iter().find(|x| x.0.contains(w)).map(|x| x.1.clone()).unwrap_or_default();
    assert_ne!(font_of("After"), font_of("Before"), "\\bfseries ran outside the group: {r:?}");
}

#[test]
fn hooks_accumulate_and_only_touch_their_environment() {
    let reply = compile(&document(
        "\\AtBeginEnvironment{quote}{[1]}\n\\AtBeginEnvironment{quote}{[2]}",
        "\\begin{quote}Q\\end{quote}\n\\begin{center}C\\end{center}\n\\begin{quote}R\\end{quote}",
    ));
    assert_eq!(problems(&reply), Vec::<(String, String)>::new());
    let t = text(&runs(&reply)).replace(' ', "");
    assert_eq!(t, "[1][2]QC[1][2]R", "{t}");
}

#[test]
fn without_etoolbox_the_hooks_are_undefined() {
    // pdflatex: "Undefined control sequence" for each.
    let reply = compile("\\documentclass{article}\n\\begin{document}\n\\AtBeginEnvironment{quote}{x}Text.\n\\end{document}\n");
    assert!(problems(&reply).iter().any(|(s, _)| s == "error"), "{:?}", problems(&reply));
}
