//! A document that nests expansion deeply must not kill the worker.
//! `\csname` nested 5 000 deep overflowed `flashtex-render`'s 8 MiB main
//! thread (SIGABRT), so the preview died and the request after it was never
//! answered. pdflatex (TeX Live 2026) reports `! Missing \endcsname
//! inserted.` for this document and still writes a PDF.
//!
//! The worker now serves on a 512 MiB stack. The `expand_depth` guard that
//! bounds the recursion at TeX's 10 000 is in `crates/tex-expansion` and
//! reaches this crate with the next vendor re-pin; until then, nesting far
//! past 10 000 levels is still unbounded here.

use std::io::Write;
use std::process::{Command, Stdio};

use flashtex_compiler::json;

fn request(id: &str, body: &str) -> String {
    let text =
        format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
    let mut doc = json::Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(&text));
    let mut payload = json::Value::obj();
    payload.set("project_id", json::str_("p"));
    payload.set("revision", json::num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", json::Value::Arr(vec![doc]));
    let mut v = json::Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(id));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v) + "\n"
}

#[test]
fn deeply_nested_csname_is_answered_and_the_worker_keeps_serving() {
    let deep = format!(
        "{} x{}",
        r"\csname".repeat(5_000),
        r"\endcsname".repeat(5_000)
    );
    let input = request("deep", &deep) + &request("after", "Hello.");
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-render"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn flashtex-render");
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let out = child.wait_with_output().expect("worker output");
    let _ = writer.join().unwrap();
    assert!(
        out.status.success(),
        "the worker exited with {}",
        out.status
    );
    let replies: Vec<json::Value> = String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .map(|l| json::parse(l).expect("reply is JSON"))
        .filter(|r| r.get("type").and_then(|v| v.as_str()) == Some("compile_result"))
        .collect();
    let ids: Vec<&str> = replies
        .iter()
        .filter_map(|r| r.get("id").and_then(|v| v.as_str()))
        .collect();
    assert_eq!(ids, ["deep", "after"]);
    let messages: Vec<&str> = replies[0]
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|d| d.as_arr())
        .map(|d| {
            d.iter()
                .filter_map(|d| d.get("message").and_then(|m| m.as_str()))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        messages
            .iter()
            .any(|m| m.contains(r"Missing \endcsname inserted.")),
        "{messages:?}"
    );
}
