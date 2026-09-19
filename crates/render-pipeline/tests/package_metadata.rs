//! `metadata.packages` on the runtime-v1 `compile_result` this worker
//! writes: the section the compiler's own `compile_result` carries
//! (`docs/contracts/runtime-v1.md` "Optional `metadata` object"), so the
//! app's real compile path hands the editor what each project `.sty`/`.cls`
//! defined, with the engine's byte spans, instead of a lexical scan of the
//! package text. Absent -- never null -- for a project without package
//! files, so those replies are byte-identical to before the section existed.

mod common;

use flashtex_compiler::json::{self, Value};
use flashtex_render_pipeline::{protocol, FontSet, RenderOptions};

const MAIN: &str = "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\n\\hello, \\emphx{world}.\n\\end{document}\n";
const MYSTYLE: &str = "\\NeedsTeXFormat{LaTeX2e}\n\\ProvidesPackage{mystyle}[2026/01/01 v1.0 my macros]\n\\newcommand{\\hello}{Hello from mystyle}\n\\newcommand{\\emphx}[1]{\\textbf{#1}}\n\\newenvironment{aside}{\\begin{quote}}{\\end{quote}}\n";

/// One compile line over `docs`, `main.tex` the entry.
fn compile_line(docs: &[(&str, &str)]) -> String {
    let documents: Vec<String> = docs
        .iter()
        .map(|(path, text)| format!(r#"{{"path":{},"text":{}}}"#, json::write(&json::str_(*path)), json::write(&json::str_(*text))))
        .collect();
    format!(
        r#"{{"protocol_version":1,"id":"m1","type":"compile","payload":{{"project_id":"p","revision":3,"entry_path":"main.tex","documents":[{}]}}}}"#,
        documents.join(",")
    )
}

fn reply(docs: &[(&str, &str)]) -> (String, Value) {
    let fonts = FontSet::with_default_dirs(&[]);
    let line = protocol::handle_line(&compile_line(docs), &fonts, &RenderOptions::default(), None).line;
    let value = json::parse(&line).unwrap_or_else(|e| panic!("reply is not JSON: {}\n{line}", e.0));
    (line, value)
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_else(|| panic!("{key} is not a string in {}", json::write(v)))
}

/// The bytes a `{path, start, end}` span covers in `docs`.
fn slice<'a>(span: &Value, docs: &'a [(&str, &str)]) -> &'a str {
    let path = str_of(span, "path");
    let start = span.get("start").and_then(Value::as_i64).expect("start") as usize;
    let end = span.get("end").and_then(Value::as_i64).expect("end") as usize;
    let text = docs.iter().find(|(p, _)| *p == path).unwrap_or_else(|| panic!("span names {path}, not a document")).1;
    &text[start..end]
}

#[test]
fn a_project_package_puts_its_definitions_and_spans_on_the_wire() {
    if !common::lm_available() {
        return;
    }
    let docs = [("main.tex", MAIN), ("mystyle.sty", MYSTYLE)];
    let (line, value) = reply(&docs);
    let payload = value.get("payload").expect("payload");
    assert_eq!(payload.get("status").and_then(Value::as_str), Some("ok"), "{line}");
    let packages = payload.get("metadata").and_then(|m| m.get("packages")).and_then(Value::as_arr).unwrap_or_else(|| panic!("no metadata.packages in {line}"));
    assert_eq!(packages.len(), 1, "{line}");
    let record = &packages[0];
    assert_eq!(str_of(record, "path"), "mystyle.sty");
    assert_eq!(str_of(record, "kind"), "package");
    // The loading command token, as the compiler's own payload spans it.
    assert_eq!(slice(record.get("loaded_by").expect("loaded_by"), &docs), "\\usepackage");
    let provides = record.get("provides").expect("provides");
    assert_eq!(str_of(provides, "name"), "mystyle");
    assert_eq!(str_of(provides, "date"), "2026/01/01");
    assert_eq!(str_of(provides, "version"), "v1.0");
    assert_eq!(str_of(provides, "description"), "my macros");
    assert_eq!(slice(provides.get("span").expect("span"), &docs), "\\ProvidesPackage{mystyle}[2026/01/01 v1.0 my macros]");
    let definitions = record.get("definitions").and_then(Value::as_arr).expect("definitions");
    let names: Vec<(&str, &str, &str, i64)> = definitions
        .iter()
        .map(|d| (str_of(d, "name"), str_of(d, "kind"), str_of(d, "definer"), d.get("arity").and_then(Value::as_i64).expect("arity")))
        .collect();
    assert_eq!(
        names,
        vec![("hello", "macro", "newcommand", 0), ("emphx", "macro", "newcommand", 1), ("aside", "environment", "newenvironment", 0)],
        "{line}"
    );
    // Every span is the whole defining statement in the package file.
    let statements: Vec<&str> = definitions.iter().map(|d| slice(d.get("span").expect("span"), &docs)).collect();
    assert_eq!(
        statements,
        vec!["\\newcommand{\\hello}{Hello from mystyle}", "\\newcommand{\\emphx}[1]{\\textbf{#1}}", "\\newenvironment{aside}{\\begin{quote}}{\\end{quote}}"]
    );
    assert_eq!(definitions[1].get("signature").and_then(Value::as_str), Some("[1]"));
    assert_eq!(definitions[1].get("optional_default"), Some(&Value::Null));
    // The section sits in sorted key order, and the direct writer agrees
    // with the value tree: the line re-serialises to itself.
    assert!(line.contains(r#"],"metadata":{"packages":[{"definitions":["#), "{line}");
    assert_eq!(json::write(&value), line);
}

#[test]
fn a_project_without_package_files_has_no_metadata_object() {
    if !common::lm_available() {
        return;
    }
    let plain = "\\documentclass{article}\n\\begin{document}\nHello, \\textbf{world}.\n\\end{document}\n";
    let (line, value) = reply(&[("main.tex", plain)]);
    let payload = value.get("payload").expect("payload");
    assert_eq!(payload.get("status").and_then(Value::as_str), Some("ok"), "{line}");
    assert!(payload.get("metadata").is_none(), "absent, never null: {line}");
    assert!(!line.contains("metadata"), "{line}");
    // A package file the document never loads is not a record either.
    let (line, value) = reply(&[("main.tex", plain), ("unused.sty", MYSTYLE)]);
    assert!(value.get("payload").and_then(|p| p.get("metadata")).is_none(), "{line}");
}
