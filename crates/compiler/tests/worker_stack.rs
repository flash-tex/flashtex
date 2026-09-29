//! Input that nests deeply must get an answer, not kill the worker.
//!
//! **Expansion.** A document that nests expansion deeply must get a
//! diagnostic. `\csname` nested a few thousand deep (likewise `\number`,
//! `\romannumeral`, `\the`, `\if`, `\ifnum` and `\ifcsname`) overflowed the
//! 8 MiB main thread. That aborts the process with SIGABRT, so the request
//! after it was never answered.
//!
//! pdflatex (TeX Live 2026), measured:
//! * 5 000 nested `\csname`: `! Missing \endcsname inserted.` and a PDF.
//! * 20 000 nested `\csname`: `! TeX capacity exceeded, sorry [expansion
//!   depth=10000].` and no PDF.
//!
//! **Request JSON.** A request line of 100 000 `[` overflowed the JSON
//! parser's recursion the same way. It is now `malformed_json` past
//! `json::MAX_DEPTH` (128, serde_json's default, which `document-runtime`
//! already applies to the same requests).
//!
//! These run the real binary, because the failure was the process dying.

use flashtex_compiler::json::{self, Value};
use std::io::Write;
use std::process::{Command, Stdio};

fn request(id: &str, body: &str) -> String {
    let text =
        format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(&text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("p"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_(id));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env) + "\n"
}

/// Sends `deep` then an ordinary request; returns each reply's
/// (id, status, diagnostic messages) and the exit status.
fn serve(deep: &str) -> (Vec<(String, String, Vec<String>)>, std::process::ExitStatus) {
    serve_lines(request("deep", deep))
}

/// Sends `first` (one or more request lines) then an ordinary request.
fn serve_lines(first: String) -> (Vec<(String, String, Vec<String>)>, std::process::ExitStatus) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-compiler"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn flashtex-compiler");
    let input = first + &request("after", "Hello.");
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let output = child.wait_with_output().expect("compiler output");
    let _ = writer.join().unwrap();
    let replies = String::from_utf8(output.stdout)
        .expect("utf-8")
        .lines()
        .map(|line| {
            let reply = json::parse(line).expect("reply is JSON");
            let id = reply
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let payload = reply.get("payload").expect("payload");
            let status = payload
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let messages = payload
                .get("diagnostics")
                .and_then(|d| d.as_arr())
                .map(|d| {
                    d.iter()
                        .filter_map(|d| {
                            d.get("message")
                                .and_then(|m| m.as_str())
                                .map(str::to_string)
                        })
                        .collect()
                })
                .unwrap_or_default();
            (id, status, messages)
        })
        .collect();
    (replies, output.status)
}

const CAPACITY: &str = "TeX capacity exceeded, sorry [expansion depth=10000].";

#[test]
fn nesting_past_the_expand_depth_is_answered_and_the_worker_keeps_serving() {
    let deep = format!(
        "{} x{}",
        r"\csname".repeat(20_000),
        r"\endcsname".repeat(20_000)
    );
    let (replies, status) = serve(&deep);
    assert!(status.success(), "the worker exited with {status}");
    assert_eq!(replies.len(), 2, "{replies:?}");
    let (id, _, messages) = &replies[0];
    assert_eq!(id, "deep");
    assert!(
        messages.iter().any(|m| m.contains(CAPACITY)),
        "{messages:?}"
    );
    assert_eq!(replies[1].0, "after");
    assert_eq!(replies[1].1, "ok", "{:?}", replies[1]);
}

#[test]
fn nesting_below_the_expand_depth_is_not_a_capacity_error() {
    let deep = format!(
        "{} x{}",
        r"\csname".repeat(5_000),
        r"\endcsname".repeat(5_000)
    );
    let (replies, status) = serve(&deep);
    assert!(status.success(), "the worker exited with {status}");
    assert_eq!(replies.len(), 2, "{replies:?}");
    let messages = &replies[0].2;
    assert!(
        !messages.iter().any(|m| m.contains(CAPACITY)),
        "{:?}",
        &messages[..messages.len().min(3)]
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains(r"Missing \endcsname inserted.")),
        "{:?}",
        &messages[..messages.len().min(3)]
    );
    assert_eq!(replies[1].1, "ok", "{:?}", replies[1]);
}

#[test]
fn a_request_nested_past_the_json_depth_is_malformed_and_the_worker_keeps_serving() {
    for line in ["[".repeat(100_000), "{\"a\":".repeat(100_000)] {
        let (replies, status) = serve_lines(line + "\n");
        assert!(status.success(), "the worker exited with {status}");
        assert_eq!(replies.len(), 2, "{replies:?}");
        assert!(
            replies[0]
                .2
                .iter()
                .any(|m| m.contains("nesting deeper than 128 levels")),
            "{:?}",
            replies[0]
        );
        assert_eq!(
            (replies[1].0.as_str(), replies[1].1.as_str()),
            ("after", "ok")
        );
    }
}

#[test]
fn json_at_the_depth_limit_still_parses() {
    let at = format!(
        "{}{}",
        "[".repeat(json::MAX_DEPTH),
        "]".repeat(json::MAX_DEPTH)
    );
    assert!(json::parse(&at).is_ok());
    let past = format!(
        "{}{}",
        "[".repeat(json::MAX_DEPTH + 1),
        "]".repeat(json::MAX_DEPTH + 1)
    );
    assert_eq!(
        json::parse(&past).unwrap_err().to_string(),
        "nesting deeper than 128 levels"
    );
}
