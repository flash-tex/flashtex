//! Issue #521: the `ifthen` package's conditionals (`\ifthenelse`, `\newif`
//! switches) run in the expansion pass, so a document using them directly in
//! the body must compile with no `unsupported_feature` diagnostic.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

/// The exact repro from the issue.
const REPRO: &str = "\\documentclass{article}\n\\usepackage{ifthen}\n\\begin{document}\n\\newif\\ifmyflag\n\\myflagtrue\n\\ifthenelse{\\equal{a}{a}}{yes}{no}\n\\ifmyflag Y\\else N\\fi\n\\end{document}\n";

fn compile(path: &str, text: &str) -> Vec<Value> {
    let mut doc = Value::obj();
    doc.set("path", json::str_(path));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("ifthen"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(path));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("ifthen"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    let reply = json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply");
    reply
        .get("payload")
        .expect("payload")
        .get("diagnostics")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap_or_default()
}

fn messages(diags: &[Value]) -> Vec<(String, String, String)> {
    diags
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

#[test]
fn ifthenelse_and_newif_in_the_body_are_supported() {
    let diags = compile("ifthen2.tex", REPRO);
    let found = messages(&diags);
    let unsupported: Vec<_> = found
        .iter()
        .filter(|(_, code, _)| code == "unsupported_feature")
        .collect();
    assert!(
        unsupported.is_empty(),
        "unsupported_feature diagnostics remain: {unsupported:?} (all: {found:?})"
    );
    let errors: Vec<_> = found.iter().filter(|(sev, _, _)| sev == "error").collect();
    assert!(errors.is_empty(), "error diagnostics remain: {errors:?} (all: {found:?})");
}

#[test]
fn ifthen_package_options_still_warn() {
    // `ifthen.sty` takes no options; an unknown one keeps the package warning.
    let diags = compile(
        "ifthen-opt.tex",
        "\\documentclass{article}\n\\usepackage[foo]{ifthen}\n\\begin{document}\nx\n\\end{document}\n",
    );
    let found = messages(&diags);
    assert!(
        found
            .iter()
            .any(|(_, code, msg)| code == "unsupported_feature" && msg.contains("ifthen")),
        "expected the ifthen package warning for an unknown option (all: {found:?})"
    );
}
