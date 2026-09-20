//! Real helper process fixture for an eventual native adapter; no AppKit claims.
use flashtex_edit_ledger::{Document, PreparedEdit};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
struct Helper {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Helper {
    fn start(path: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-edit-ledger"))
            .args(["--store", path.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }
    fn ask(&mut self, request: Value) -> Value {
        writeln!(self.input, "{request}").unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["command_succeeded"], true, "{response}");
        response
    }
}
impl Drop for Helper {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn checkpoint_helper_kill_reopen_preserves_backup_source_history_and_ids() {
    let dir = tempfile::tempdir().unwrap();
    let mut helper = Helper::start(dir.path());
    let document = Document::new("demo".into(), "main.tex".into(), 1, "original".into()).unwrap();
    helper.ask(json!({"id":"init","operation":"initialize","document":document}));
    let edit = PreparedEdit {
        capture_id: "backup-capture".into(),
        edit_id: "backup-edit".into(),
        project_id: document.project_id.clone(),
        path: document.path.clone(),
        expected_revision: 1,
        start_byte: 0,
        end_byte: 8,
        removed_text: "original".into(),
        replacement: "revised".into(),
        document_before_sha256: document.source_sha256.clone(),
        wrap: None,
    };
    let applied = helper.ask(json!({"id":"apply","operation":"apply","edit":edit}));
    let rotate = json!({"id":"rotate","operation":"checkpoint_rotate",
        "authorization":{"acknowledge_private_source_export":true},
        "policy":{"acknowledge_checkpoint_deletion":true,"keep_latest":1,"max_total_bytes":1048576}});
    let first = helper.ask(rotate.clone());
    let old_session = first["session_id"].clone();
    // Submit another rotation, then terminate without receiving its result.
    // Scheduling may kill before, during, or after that rotation. Boundary
    // failpoint tests separately cover each durable write ordering.
    writeln!(helper.input, "{rotate}").unwrap();
    drop(helper);
    let mut helper = Helper::start(dir.path());
    let status = helper.ask(json!({"id":"status","operation":"checkpoint_status"}));
    assert_ne!(status["session_id"], old_session);
    assert_eq!(status["payload"]["current_revision"], 2);
    assert_eq!(
        status["payload"]["checkpoints"].as_array().unwrap().len(),
        1
    );
    let rotated = helper.ask(rotate);
    let generation = rotated["payload"]["created"]["generation"].clone();
    let backup =
        helper.ask(json!({"id":"read","operation":"checkpoint_read","generation":generation}));
    let checkpoint = backup["payload"].clone();
    let recovered = tempfile::tempdir().unwrap();
    let mut target = Helper::start(recovered.path());
    let plan = target.ask(json!({"id":"plan","operation":"checkpoint_plan",
        "checkpoint":checkpoint,"expected_identity":checkpoint["identity"]}));
    target.ask(
        json!({"id":"import","operation":"checkpoint_import","checkpoint":checkpoint,
        "plan":plan["payload"],"authorization":{"approve_plan_id":plan["payload"]["plan_id"],
        "allow_initialize_empty":true,"allow_same_source_metadata":false}}),
    );
    drop(target);
    let mut target = Helper::start(recovered.path());
    let retry = target.ask(json!({"id":"retry","operation":"apply","edit":edit}));
    assert_eq!(retry["payload"]["receipt"], applied["payload"]["receipt"]);
    assert_eq!(retry["payload"]["document"]["text"], "revised");
    let history = target.ask(json!({"id":"history","operation":"history_status"}));
    assert_eq!(
        history["payload"]["undo_labels"].as_array().unwrap().len(),
        1
    );
    let state = target.ask(json!({"id":"source","operation":"status"}));
    assert_eq!(
        state["payload"]["pending_receipts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn native_undo_restart_fixture_preserves_source_receipt_and_redo() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/native-undo-restart.json")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut helper = Helper::start(dir.path());
    let document = Document::new(
        "demo".into(),
        "main.tex".into(),
        1,
        fixture["initial_text"].as_str().unwrap().into(),
    )
    .unwrap();
    let edit = PreparedEdit {
        capture_id: fixture["capture_id"].as_str().unwrap().into(),
        edit_id: fixture["edit_id"].as_str().unwrap().into(),
        project_id: "demo".into(),
        path: "main.tex".into(),
        expected_revision: 1,
        start_byte: fixture["start_byte"].as_u64().unwrap() as usize,
        end_byte: fixture["end_byte"].as_u64().unwrap() as usize,
        removed_text: fixture["removed_text"].as_str().unwrap().into(),
        replacement: fixture["replacement"].as_str().unwrap().into(),
        document_before_sha256: document.source_sha256.clone(),
        wrap: None,
    };
    helper.ask(json!({"id":"init","operation":"initialize","document":document}));
    let applied = helper.ask(json!({"id":"apply","operation":"apply","edit":edit}));
    assert_eq!(
        applied["payload"]["document"]["text"],
        fixture["expected_after_capture"]
    );
    let undo = json!({"command_id":"undo-native","expected_revision":applied["document_revision"],"expected_sha256":applied["document_sha256"]});
    let undone = helper.ask(json!({"id":"undo","operation":"undo","command":undo}));
    let old_session = undone["session_id"].clone();
    drop(helper);
    let mut helper = Helper::start(dir.path());
    let state = helper.ask(json!({"id":"restart","operation":"status"}));
    assert_ne!(state["session_id"], old_session);
    assert_eq!(
        state["payload"]["document"]["text"],
        fixture["expected_after_undo"]
    );
    assert_eq!(
        state["payload"]["pending_receipts"]
            .as_array()
            .unwrap()
            .len() as u64,
        fixture["expected_pending_receipts"].as_u64().unwrap()
    );
    let history = helper.ask(json!({"id":"history","operation":"history_status"}));
    assert_eq!(
        history["payload"]["redo_labels"].as_array().unwrap().len() as u64,
        fixture["expected_redo_units"].as_u64().unwrap()
    );
    let retry = helper.ask(json!({"id":"retry-undo","operation":"undo","command":undo}));
    assert_eq!(retry["payload"]["replayed_command"], true);
    let duplicate = helper.ask(json!({"id":"duplicate-capture","operation":"apply","edit":edit}));
    assert_eq!(
        duplicate["payload"]["document"]["text"],
        fixture["expected_after_undo"]
    );
    let redo=helper.ask(json!({"id":"redo","operation":"redo","command":{"command_id":"redo-native","expected_revision":state["document_revision"],"expected_sha256":state["document_sha256"]}}));
    assert_eq!(
        redo["payload"]["document"]["text"],
        fixture["expected_after_capture"]
    );
}
