use flashtex_edit_ledger::{digest, Document, PreparedEdit};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

fn run(path: &std::path::Path, input: &str) -> (bool, Vec<Value>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-edit-ledger"))
        .args(["--store", path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let rows = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    (output.status.success(), rows)
}

#[test]
fn subprocess_apply_crash_before_receipt_delivery_recovers_source_and_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let document = Document::new("demo".into(), "main.tex".into(), 1, "aé😀z".into()).unwrap();
    let edit = PreparedEdit {
        capture_id: "capture-1".into(),
        edit_id: "edit-1".into(),
        project_id: "demo".into(),
        path: "main.tex".into(),
        expected_revision: 1,
        start_byte: 1,
        end_byte: 7,
        removed_text: "é😀".into(),
        replacement: "$x$".into(),
        document_before_sha256: digest("aé😀z"),
        wrap: None,
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-edit-ledger"))
        .args(["--store", dir.path().to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    for request in [
        json!({"id":"init","operation":"initialize","document":document}),
        json!({"id":"apply","operation":"apply","edit":edit}),
    ] {
        writeln!(input, "{request}").unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let response: Value = serde_json::from_str(&line).unwrap();
        assert!(response.get("error").is_none(), "{response}");
    }
    // No capture_applied was sent to a bridge. Kill the owner, then load the
    // bundled durable source and pending receipt in a new OS process.
    child.kill().unwrap();
    child.wait().unwrap();
    let (success, rows) = run(
        dir.path(),
        &format!(
            "{}\n{}\n",
            json!({"id":"status","operation":"status"}),
            json!({"id":"retry","operation":"apply","edit":edit})
        ),
    );
    assert!(success);
    assert_eq!(rows[0]["payload"]["document"]["text"], "a$x$z");
    assert_eq!(
        rows[0]["payload"]["pending_receipts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(rows[1]["payload"]["receipt"]["new_revision"], 2);
    assert_eq!(rows[1]["payload"]["document"]["revision"], 2);
}

#[test]
fn malformed_request_is_reported_and_next_request_is_processed() {
    let dir = tempfile::tempdir().unwrap();
    let (success, rows) = run(
        dir.path(),
        "{bad}\n{\"id\":\"status\",\"operation\":\"status\"}\n",
    );
    assert!(success);
    assert_eq!(rows[0]["error"]["code"], "invalid_request");
    assert_eq!(rows[1]["id"], "status");
    assert!(rows[1]["payload"]["document"].is_null());
}

#[test]
fn unterminated_frame_is_rejected_without_applying() {
    let dir = tempfile::tempdir().unwrap();
    let (success, rows) = run(dir.path(), "{\"id\":\"status\",\"operation\":\"status\"}");
    assert!(!success);
    assert_eq!(rows[0]["error"]["code"], "invalid_frame");
}
