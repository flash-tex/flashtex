//! The `flashtex-project-files --root DIR` JSON Lines host, driven as a child
//! process the way the Mac shell drives it. Temp directories only.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use flashtex_project_files::json::Json;
use flashtex_project_files::sha256_hex;

mod common;

fn run(root: &std::path::Path, requests: &[&str]) -> Vec<Json> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-project-files"))
        .arg("--root")
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn helper");
    {
        let mut stdin = child.stdin.take().unwrap();
        for r in requests {
            writeln!(stdin, "{r}").unwrap();
        }
    }
    let stdout = child.stdout.take().unwrap();
    let replies: Vec<Json> = BufReader::new(stdout)
        .lines()
        .map(|l| Json::parse(&l.unwrap()).expect("reply is JSON"))
        .collect();
    let status = child.wait().unwrap();
    assert!(status.success(), "helper exited {status:?}");
    replies
}

fn payload<'a>(reply: &'a Json, id: &str) -> &'a Json {
    assert_eq!(reply.get("id").and_then(Json::as_str), Some(id));
    reply
        .get("payload")
        .unwrap_or_else(|| panic!("expected payload, got {}", reply.to_string_compact()))
}

fn error_code<'a>(reply: &'a Json, id: &str) -> &'a str {
    assert_eq!(reply.get("id").and_then(Json::as_str), Some(id));
    reply
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(Json::as_str)
        .unwrap_or_else(|| panic!("expected error, got {}", reply.to_string_compact()))
}

#[test]
fn ping_read_status_save_and_conflict_over_the_wire() {
    let tmp = common::TempDir::new("helper-bin");
    let root = tmp.root();
    std::fs::write(root.join("a.tex"), "hello\n").unwrap();
    let h1 = sha256_hex(b"hello\n");
    let h2 = sha256_hex(b"hello\nworld\n");
    let replies = run(
        root,
        &[
            r#"{"id":"1","operation":"ping"}"#,
            r#"{"id":"2","operation":"read","path":"a.tex"}"#,
            &format!(
                r#"{{"id":"3","operation":"status","path":"a.tex","expected_sha256":"{h1}"}}"#
            ),
            &format!(
                r#"{{"id":"4","operation":"save","path":"a.tex","text":"hello\nworld\n","expected":"{h1}"}}"#
            ),
            &format!(
                r#"{{"id":"5","operation":"save","path":"a.tex","text":"again\n","expected":"{h1}"}}"#
            ),
            &format!(
                r#"{{"id":"6","operation":"status","path":"a.tex","expected_sha256":"{h1}"}}"#
            ),
            r#"{"id":"7","operation":"read","path":"missing.tex"}"#,
            r#"{"id":"8","operation":"status","path":"missing.tex","expected_sha256":null}"#,
            r#"{"id":"9","operation":"save","path":"new/dir/n.tex","text":"n","expected":"new"}"#,
            r#"{"id":"10","operation":"save","path":"new/dir/n.tex","text":"n2","expected":"new"}"#,
            &format!(
                r#"{{"id":"11","operation":"save","path":"a.tex","text":"forced\n","expected":"{h1}","force":true}}"#
            ),
        ],
    );
    assert_eq!(replies.len(), 11);
    let p = payload(&replies[0], "1");
    assert_eq!(
        p.get("protocol").and_then(Json::as_str),
        Some("project-files-v1")
    );
    let p = payload(&replies[1], "2");
    assert_eq!(p.get("text").and_then(Json::as_str), Some("hello\n"));
    assert_eq!(p.get("sha256").and_then(Json::as_str), Some(h1.as_str()));
    assert_eq!(
        payload(&replies[2], "3")
            .get("state")
            .and_then(Json::as_str),
        Some("unchanged")
    );
    let p = payload(&replies[3], "4");
    assert_eq!(p.get("outcome").and_then(Json::as_str), Some("saved"));
    assert_eq!(
        p.get("receipt")
            .and_then(|r| r.get("sha256"))
            .and_then(Json::as_str),
        Some(h2.as_str())
    );
    let p = payload(&replies[4], "5");
    assert_eq!(p.get("outcome").and_then(Json::as_str), Some("conflict"));
    let c = p.get("conflict").unwrap();
    assert_eq!(
        c.get("kind").and_then(Json::as_str),
        Some("modified_externally")
    );
    assert_eq!(c.get("ours").and_then(Json::as_str), Some(h1.as_str()));
    assert_eq!(c.get("theirs").and_then(Json::as_str), Some(h2.as_str()));
    assert_eq!(
        payload(&replies[5], "6")
            .get("state")
            .and_then(Json::as_str),
        Some("modified")
    );
    assert_eq!(
        payload(&replies[6], "7").get("exists"),
        Some(&Json::Bool(false))
    );
    assert_eq!(
        payload(&replies[7], "8")
            .get("state")
            .and_then(Json::as_str),
        Some("unchanged")
    );
    assert_eq!(
        payload(&replies[8], "9")
            .get("outcome")
            .and_then(Json::as_str),
        Some("saved")
    );
    let c = payload(&replies[9], "10").get("conflict").unwrap();
    assert_eq!(c.get("kind").and_then(Json::as_str), Some("already_exists"));
    assert_eq!(
        payload(&replies[10], "11")
            .get("outcome")
            .and_then(Json::as_str),
        Some("saved")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("a.tex")).unwrap(),
        "forced\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("new/dir/n.tex")).unwrap(),
        "n"
    );
}

#[test]
fn refusals_and_bad_requests_are_errors_and_write_nothing() {
    let tmp = common::TempDir::new("helper-bin-refuse");
    let root = tmp.root();
    let outside = common::TempDir::new("helper-bin-outside");
    let secret = outside.root().join("secret.tex");
    std::fs::write(&secret, "secret\n").unwrap();
    common::symlink_file(&secret, &root.join("link.tex"));
    let replies = run(
        root,
        &[
            r#"{"id":"1","operation":"save","path":"link.tex","text":"x","expected":"any","force":true}"#,
            r#"{"id":"2","operation":"read","path":"link.tex"}"#,
            r#"{"id":"3","operation":"read","path":"../secret.tex"}"#,
            r#"{"id":"4","operation":"read","path":"/etc/hosts"}"#,
            r#"{"id":"5","operation":"nope"}"#,
            r#"{"id":"6","operation":"save","path":"a.tex","text":"x","expected":"zz"}"#,
            r#"{"id":"7","operation":"save","path":"a.tex","text":"x","expected":"new","force":"yes"}"#,
            r#"not json"#,
            r#"{"id":"8","operation":"read"}"#,
        ],
    );
    assert_eq!(replies.len(), 9);
    assert_eq!(error_code(&replies[0], "1"), "refused");
    assert_eq!(error_code(&replies[1], "2"), "refused");
    assert_eq!(error_code(&replies[2], "3"), "invalid_path");
    assert_eq!(error_code(&replies[3], "4"), "invalid_path");
    assert_eq!(error_code(&replies[4], "5"), "unsupported_operation");
    assert_eq!(error_code(&replies[5], "6"), "invalid_request");
    assert_eq!(error_code(&replies[6], "7"), "invalid_request");
    assert_eq!(replies[7].get("id"), Some(&Json::Null));
    assert_eq!(
        replies[7]
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Json::as_str),
        Some("invalid_request")
    );
    assert_eq!(error_code(&replies[8], "8"), "invalid_request");
    assert_eq!(std::fs::read_to_string(&secret).unwrap(), "secret\n");
    assert!(!root.join("a.tex").exists());
}

#[test]
fn symlinked_root_is_refused_at_startup() {
    let real = common::TempDir::new("helper-bin-realroot");
    let holder = common::TempDir::new("helper-bin-holder");
    let link = holder.root().join("root-link");
    common::symlink_dir(real.root(), &link);
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-project-files"))
        .arg("--root")
        .arg(&link)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot open root"));
}
