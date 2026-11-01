//! A `\newlength` register used as an enumitem key value
//! (`leftmargin=\mylen`): the engine absorbs `\setlist`'s arguments with
//! one expansion pass and splices the register's current value, so the
//! bare register behaves exactly like `\the\mylen` (and, in `pt`, exactly
//! like the literal dimension) instead of reaching the stomach as a
//! register assignment, which reported "Missing number, treated as zero."
//! and "Illegal unit of measure (pt inserted)."

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

fn compile_line(id: &str, text: &str) -> String {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("setlist-register-lengths"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_(id));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env)
}

fn reply(line: &str) -> Value {
    json::parse(&handle_line(line)).expect("reply must be valid JSON")
}

fn items(v: &Value) -> Vec<Value> {
    v.get("payload")
        .and_then(|p| p.get("pages"))
        .and_then(Value::as_arr)
        .map(|pages| {
            pages
                .iter()
                .flat_map(|pg| {
                    pg.get("items")
                        .and_then(Value::as_arr)
                        .cloned()
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn baseline_of(response: &Value, text: &str) -> f64 {
    items(response)
        .iter()
        .find(|item| item.get("text").and_then(Value::as_str) == Some(text))
        .and_then(|item| item.get("baseline_y_pt"))
        .and_then(|v| match v {
            Value::Num(n) => Some(*n),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no positioned item with text {text:?} in {response:?}"))
}

fn x_of(response: &Value, text: &str) -> f64 {
    items(response)
        .iter()
        .find(|item| item.get("text").and_then(Value::as_str) == Some(text))
        .and_then(|item| item.get("x_pt"))
        .and_then(|v| match v {
            Value::Num(n) => Some(*n),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no positioned item with text {text:?} in {response:?}"))
}

fn messages(response: &Value) -> Vec<String> {
    response
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(Value::as_arr)
        .into_iter()
        .flatten()
        .filter_map(|d| d.get("message").and_then(Value::as_str).map(str::to_string))
        .collect()
}

/// A short paragraph, a 3-item `enumerate`, then another paragraph, with
/// `extra` inserted right after `\documentclass`.
fn doc(extra: &str) -> String {
    format!(
        r"\documentclass{{article}}{extra}\begin{{document}}Intro.\begin{{enumerate}}\item Alpha\item Beta\item Gamma\end{{enumerate}}Outro.\end{{document}}"
    )
}

const TOLERANCE_PT: f64 = 0.02;

#[test]
fn length_register_as_leftmargin_reports_no_diagnostic() {
    // The exact reported reproduction: pdflatex accepts a length-register
    // macro as an enumitem key value without complaint; the compiler used
    // to answer "Missing number, treated as zero." plus "Illegal unit of
    // measure (pt inserted).", then "leftmargin ... recognised but not
    // implemented".
    let response = reply(&compile_line(
        "register-leftmargin",
        r"\newlength{\mylen}\setlength{\mylen}{2em}\setlist[itemize]{leftmargin=\mylen}\begin{itemize}\item a\end{itemize}",
    ));
    assert!(messages(&response).is_empty(), "{:?}", messages(&response));
}

#[test]
fn length_register_leftmargin_matches_literal_dimension() {
    // `pt` units: no `em` base is involved, so the register form must
    // resolve to exactly the internal result of the literal form.
    let registered = reply(&compile_line(
        "register-leftmargin-value",
        &doc(r"\newlength{\mylen}\setlength{\mylen}{24pt}\setlist[enumerate]{leftmargin=\mylen}"),
    ));
    let literal = reply(&compile_line(
        "literal-leftmargin-value",
        &doc(r"\setlist[enumerate]{leftmargin=24pt}"),
    ));
    assert!(messages(&registered).is_empty(), "{:?}", messages(&registered));
    assert!(
        (x_of(&registered, "Alpha") - x_of(&literal, "Alpha")).abs() < TOLERANCE_PT,
        "register leftmargin must indent exactly like the literal: {} vs {}",
        x_of(&registered, "Alpha"),
        x_of(&literal, "Alpha")
    );
}

#[test]
fn length_register_leftmargin_matches_the_form() {
    // The explicit `\the` always worked (it expands to literal text in
    // the main loop); the bare register must now behave identically.
    let bare = reply(&compile_line(
        "bare-leftmargin",
        &doc(r"\newlength{\mylen}\setlength{\mylen}{2em}\setlist[enumerate]{leftmargin=\mylen}"),
    ));
    let the_form = reply(&compile_line(
        "the-leftmargin",
        &doc(r"\newlength{\mylen}\setlength{\mylen}{2em}\setlist[enumerate]{leftmargin=\the\mylen}"),
    ));
    assert!(messages(&bare).is_empty(), "{:?}", messages(&bare));
    assert!(messages(&the_form).is_empty(), "{:?}", messages(&the_form));
    assert!(
        (x_of(&bare, "Alpha") - x_of(&the_form, "Alpha")).abs() < TOLERANCE_PT,
        "bare register must match the \\the form: {} vs {}",
        x_of(&bare, "Alpha"),
        x_of(&the_form, "Alpha")
    );
}

#[test]
fn length_register_as_itemsep_matches_literal() {
    let registered = reply(&compile_line(
        "register-itemsep",
        &doc(r"\newlength{\mylen}\setlength{\mylen}{20pt}\setlist[enumerate]{itemsep=\mylen}"),
    ));
    let literal = reply(&compile_line(
        "literal-itemsep",
        &doc(r"\setlist[enumerate]{itemsep=20pt}"),
    ));
    assert!(messages(&registered).is_empty(), "{:?}", messages(&registered));
    let reg_gap = baseline_of(&registered, "Beta") - baseline_of(&registered, "Alpha");
    let lit_gap = baseline_of(&literal, "Beta") - baseline_of(&literal, "Alpha");
    assert!(
        (reg_gap - lit_gap).abs() < TOLERANCE_PT,
        "register itemsep must space items exactly like the literal: {reg_gap} vs {lit_gap}"
    );
}

#[test]
fn length_register_as_topsep_matches_literal() {
    let registered = reply(&compile_line(
        "register-topsep",
        &doc(r"\newlength{\mylen}\setlength{\mylen}{20pt}\setlist[enumerate]{topsep=\mylen}"),
    ));
    let literal = reply(&compile_line(
        "literal-topsep",
        &doc(r"\setlist[enumerate]{topsep=20pt}"),
    ));
    assert!(messages(&registered).is_empty(), "{:?}", messages(&registered));
    let reg_gap = baseline_of(&registered, "Alpha") - baseline_of(&registered, "Intro.");
    let lit_gap = baseline_of(&literal, "Alpha") - baseline_of(&literal, "Intro.");
    assert!(
        (reg_gap - lit_gap).abs() < TOLERANCE_PT,
        "register topsep must space the list exactly like the literal: {reg_gap} vs {lit_gap}"
    );
}
