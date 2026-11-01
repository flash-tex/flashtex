//! Issue #33: the compiler bounds its `compile_result` to the consumer's
//! frame, not a fixed 8 MiB. `preview-controller` passes its configured
//! `compiler_max_frame_bytes` (less the newline) as `FLASHTEX_MAX_REPLY_BYTES`,
//! the variable `flashtex-render` already honours. Before this, a 12 MiB-frame
//! preview of a 307-page document got 244 pages and `recovered`, and a frame
//! smaller than 8 MiB could still be overrun.
//!
//! These tests run the real binary, because the limit is read from the
//! process environment and setting it inside the shared test process would
//! race the other tests' replies.

use flashtex_compiler::json::{self, Value};
use std::io::Write;
use std::process::{Command, Stdio};

const MIB: usize = 1024 * 1024;

/// The `preview-controller` stdio test's document: 9804 short paragraphs,
/// 307 pages, a ~10 MB reply.
fn large_request() -> String {
    let text = "Measured paragraph with ordinary words and spaces.\n\n".repeat(9804);
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
    env.set("id", json::str_("large"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env) + "\n"
}

/// The first reply line, with `FLASHTEX_MAX_REPLY_BYTES` set to `limit`
/// (removed when `None`).
fn reply(limit: Option<&str>) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_flashtex-compiler"));
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    match limit {
        Some(limit) => command.env("FLASHTEX_MAX_REPLY_BYTES", limit),
        None => command.env_remove("FLASHTEX_MAX_REPLY_BYTES"),
    };
    let mut child = command.spawn().expect("spawn flashtex-compiler");
    let request = large_request();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(request.as_bytes()));
    let output = child.wait_with_output().expect("compiler output");
    writer.join().unwrap().expect("request written");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 reply");
    stdout.lines().next().expect("one reply line").to_string()
}

fn summary(line: &str) -> (String, usize, Vec<String>) {
    let reply = json::parse(line).expect("reply is JSON");
    let payload = reply.get("payload").expect("payload");
    let status = payload.get("status").and_then(|s| s.as_str()).unwrap_or("").to_string();
    let pages = payload.get("pages").and_then(|p| p.as_arr()).map_or(0, |p| p.len());
    let messages = payload
        .get("diagnostics")
        .and_then(|d| d.as_arr())
        .map(|d| {
            d.iter()
                .filter_map(|d| d.get("message").and_then(|m| m.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    (status, pages, messages)
}

/// No variable: the documented 8 MiB default, unchanged.
#[test]
fn default_bound_is_eight_mib() {
    let line = reply(None);
    assert!(line.len() <= 8 * MIB, "{} bytes", line.len());
    let (status, pages, messages) = summary(&line);
    assert_eq!(status, "recovered");
    assert!(pages < 307, "{pages} pages");
    assert!(
        messages.iter().any(|m| m.contains("8 MiB transport frame")
            && m.contains(&format!("{} of 307 pages were not delivered", 307 - pages))),
        "{messages:?}"
    );
}

/// `preview-controller`'s value for a 12 MiB configured frame: every page
/// fits, so the reply is complete and `ok`, past the old fixed 8 MiB.
#[test]
fn a_larger_consumer_frame_gets_every_page() {
    let line = reply(Some(&(12 * MIB - 1).to_string()));
    assert!(line.len() > 8 * MIB && line.len() < 12 * MIB, "{} bytes", line.len());
    let (status, pages, messages) = summary(&line);
    assert_eq!(status, "ok", "{messages:?}");
    assert_eq!(pages, 307);
}

/// A frame smaller than the default is never overrun, and the diagnostic
/// names that frame.
#[test]
fn a_smaller_consumer_frame_is_never_overrun() {
    let line = reply(Some(&(4 * MIB - 1).to_string()));
    assert!(line.len() < 4 * MIB, "{} bytes", line.len());
    let (status, pages, messages) = summary(&line);
    assert_eq!(status, "recovered");
    assert!(pages > 0);
    assert!(
        messages.iter().any(|m| m.contains("4 MiB transport frame")),
        "{messages:?}"
    );
}

/// Past the Mac reader's 16 MiB line limit the variable is capped, and a
/// value that is not a positive integer falls back to the default.
#[test]
fn out_of_range_values_are_capped_or_ignored() {
    assert_eq!(reply(Some("0")).len(), reply(None).len());
    assert_eq!(reply(Some("not-a-number")).len(), reply(None).len());
    let (status, pages, _) = summary(&reply(Some(&(64 * MIB).to_string())));
    assert_eq!((status.as_str(), pages), ("ok", 307));
}
