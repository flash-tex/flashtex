use flashtex_edit_ledger::{Document, Store};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
struct Client {
    child: Child,
    input: Option<ChildStdin>,
    output: Receiver<Value>,
    reader_progress: Arc<Mutex<ReaderProgress>>,
    reader_thread: Option<thread::JoinHandle<()>>,
}
#[derive(Debug, Default)]
struct ReaderProgress {
    phase: &'static str,
    frames: usize,
    last_frame_bytes: usize,
    last_decode_ms: f64,
}
#[test]
fn metadata_undo_redo_unread_ack_retry_and_later_edit() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("doc", "document", json!({"path":"main.tex"}));
    let original = client.reply("doc")["payload"]["document"].clone();
    let text = "β".repeat(250_000);
    client.send(
        "edit",
        "edit",
        json!({"path":"main.tex","expected_revision":1,
        "expected_sha256":original["source_sha256"],"text":text,"response_mode":"metadata"}),
    );
    let edited = client.reply("edit")["payload"]["document"].clone();
    let mut undo = json!({"path":"main.tex","response_mode":"invalid",
        "command":{"command_id":"undo-compact","expected_revision":2,
        "expected_sha256":edited["source_sha256"]}});
    client.send("invalid", "undo", undo.clone());
    assert_eq!(client.reply("invalid")["type"], "error");
    undo["response_mode"] = json!("metadata");
    client.send("undo", "undo", undo.clone());
    let result = client.reply("undo");
    assert_eq!(result["type"], "result");
    let restored = &result["payload"]["history"]["document"];
    assert_eq!(restored["revision"], 3);
    assert_eq!(restored["source_sha256"], original["source_sha256"]);
    assert!(restored.get("text").is_none());
    let redo = json!({"path":"main.tex","response_mode":"metadata",
        "command":{"command_id":"redo-compact","expected_revision":3,
        "expected_sha256":restored["source_sha256"]}});
    client.send("redo", "redo", redo.clone());
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let persisted: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("store/document.json")).unwrap())
                .unwrap();
        if persisted["document"]["revision"] == 4 {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    drop(client); // Redo is durable but its acknowledgement was not consumed.
    let mut client = Client::start(dir.path());
    client.send("redo", "redo", redo.clone());
    let replay = client.reply("redo");
    assert_eq!(replay["type"], "result");
    assert!(serde_json::to_vec(&replay).unwrap().len() < 1024);
    let history = &replay["payload"]["history"];
    assert_eq!(history["command_revision"], 4);
    assert_eq!(history["replayed_command"], true);
    assert_eq!(
        history["document"]["source_sha256"],
        edited["source_sha256"]
    );
    assert_eq!(history["document"]["byte_length"], 500000);
    assert!(history["document"].get("text").is_none());
    client.send("doc", "document", json!({"path":"main.tex"}));
    assert_eq!(client.reply("doc")["payload"]["document"]["text"], text);
    let mut conflict = redo.clone();
    conflict["command"]["expected_revision"] = json!(4);
    client.send("conflict", "redo", conflict);
    assert_eq!(client.reply("conflict")["type"], "error");
    let mut stale = undo.clone();
    stale["command"]["command_id"] = json!("new-stale-undo");
    client.send("stale", "undo", stale);
    assert_eq!(client.reply("stale")["type"], "error");
    client.send(
        "later",
        "edit",
        json!({"path":"main.tex","expected_revision":4,
        "expected_sha256":edited["source_sha256"],"text":"later source"}),
    );
    assert_eq!(client.reply("later")["payload"]["document"]["revision"], 5);
    for (kind, request, revision) in [("undo", undo, 3), ("redo", redo, 4)] {
        client.send("retry", kind, request.clone());
        let compact = client.reply("retry");
        let h = &compact["payload"]["history"];
        assert_eq!(h["command_revision"], revision);
        assert_eq!(h["document"]["revision"], 5);
        assert_eq!(h["replayed_command"], true);
        assert_eq!(h["can_redo"], false);
        let mut full = request;
        full.as_object_mut().unwrap().remove("response_mode");
        client.send("full", kind, full);
        let result = client.reply("full");
        let f = &result["payload"]["history"];
        assert_eq!(f["document"]["text"], "later source");
        assert_eq!(
            f["document"]["source_sha256"],
            h["document"]["source_sha256"]
        );
        for key in [
            "command_revision",
            "replayed_command",
            "can_undo",
            "can_redo",
        ] {
            assert_eq!(f[key], h[key]);
        }
    }
}
#[test]
fn metadata_group_ack_recovers_unread_reply_and_preserves_command_and_undo_identity() {
    let dir = tempfile::tempdir().unwrap();
    let source = "α".repeat(250_000);
    let original = Document::new("p".into(), "main.tex".into(), 1, source.clone()).unwrap();
    {
        let mut store = Store::open(dir.path().join("store")).unwrap();
        store.initialize(original.clone()).unwrap();
    }
    let mut client = Client::start(dir.path());
    let command = json!({"command_id":"unicode-group", "expected_revision":1,
        "expected_sha256":original.source_sha256,"label":"Two Unicode edits",
        "edits":[{"start_byte":0,"end_byte":2,"removed_text":"α","replacement":"β"},
        {"start_byte":499998,"end_byte":500000,"removed_text":"α","replacement":"γ"}]});
    let mut request = json!({"path":"main.tex","command":command,"response_mode":"bogus"});
    client.send("bad", "apply_group", request.clone());
    assert_eq!(client.reply("bad")["type"], "error");
    client.send("check", "document", json!({"path":"main.tex"}));
    assert_eq!(client.reply("check")["payload"]["document"]["revision"], 1);
    request["response_mode"] = json!("metadata");
    client.send("group", "apply_group", request.clone());
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let persisted: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("store/document.json")).unwrap())
                .unwrap();
        if persisted["document"]["revision"] == 2 {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    drop(client); // Application never consumed the first grouped-edit ACK.
    let mut client = Client::start(dir.path());
    client.send("group", "apply_group", request.clone());
    let compact = client.reply("group");
    assert_eq!(compact["type"], "result");
    assert_eq!(
        compact["payload"].get("compile_request_id"),
        Some(&Value::Null)
    );
    assert_eq!(
        compact["payload"].get("compile_revision"),
        Some(&Value::Null)
    );
    assert!(serde_json::to_vec(&compact).unwrap().len() < 1024);
    let history = &compact["payload"]["history"];
    assert_eq!(history["replayed_command"], true);
    assert_eq!(history["command_revision"], 2);
    assert_eq!(history["document"]["revision"], 2);
    assert_eq!(history["document"]["byte_length"], 500000);
    assert!(history["document"].get("text").is_none());
    let mut full_request = request.clone();
    full_request
        .as_object_mut()
        .unwrap()
        .remove("response_mode");
    client.send("full", "apply_group", full_request);
    let full = client.reply("full");
    let full_history = &full["payload"]["history"];
    for field in [
        "command_revision",
        "replayed_command",
        "can_undo",
        "can_redo",
    ] {
        assert_eq!(history[field], full_history[field]);
    }
    assert_eq!(
        history["document"]["source_sha256"],
        full_history["document"]["source_sha256"]
    );
    assert_eq!(
        full_history["document"]["text"],
        format!("β{}γ", "α".repeat(249_998))
    );
    let mut conflict = request.clone();
    conflict["command"]["label"] = json!("Different command");
    client.send("conflict", "apply_group", conflict);
    assert_eq!(client.reply("conflict")["type"], "error");
    client.send(
        "undo",
        "undo",
        json!({"path":"main.tex","command":{"command_id":"undo-group",
        "expected_revision":2,"expected_sha256":history["document"]["source_sha256"]}}),
    );
    let undo = client.reply("undo");
    assert!(undo["payload"].get("compile_request_id").is_none());
    assert_eq!(undo["payload"]["history"]["document"]["text"], source);
    client.send("group", "apply_group", request);
    let after = client.reply("group");
    assert_eq!(after["payload"]["history"]["command_revision"], 2);
    assert_eq!(after["payload"]["history"]["document"]["revision"], 3);
    assert_eq!(
        after["payload"]["history"]["document"]["source_sha256"],
        original.source_sha256
    );
    assert_eq!(after["payload"]["history"]["replayed_command"], true);
    assert_eq!(after["payload"]["history"]["can_redo"], true);
}
#[test]
fn metadata_edit_ack_preserves_large_source_recovery_and_default_response() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("before", "document", json!({"path":"main.tex"}));
    let before = client.reply("before")["payload"]["document"].clone();
    client.send(
        "invalid",
        "edit",
        json!({"path":"main.tex","expected_revision":1,
        "expected_sha256":before["source_sha256"],"text":"must not save","response_mode":true}),
    );
    assert_eq!(client.reply("invalid")["type"], "error");
    client.send("unchanged", "document", json!({"path":"main.tex"}));
    assert_eq!(client.reply("unchanged")["payload"]["document"], before);
    let text = "α".repeat(250_000);
    let request = json!({"path":"main.tex","expected_revision":1,
        "expected_sha256":before["source_sha256"],"text":text,"response_mode":"metadata"});
    client.send("save", "edit", request.clone());
    let ack = client.reply("save");
    assert_eq!(ack["type"], "result");
    assert_eq!(ack["payload"]["response_mode"], "metadata");
    let metadata = ack["payload"]["document"].clone();
    assert!(metadata.get("text").is_none());
    assert_eq!(metadata["project_id"], "p");
    assert_eq!(metadata["path"], "main.tex");
    assert_eq!(metadata["revision"], 2);
    assert_eq!(metadata["byte_length"], 500_000);
    assert!(serde_json::to_vec(&ack).unwrap().len() < 1024);
    drop(client);
    let mut client = Client::start(dir.path());
    client.send("recovered", "document", json!({"path":"main.tex"}));
    let recovered = client.reply("recovered")["payload"]["document"].clone();
    assert_eq!(recovered["text"], text);
    assert_eq!(recovered["source_sha256"], metadata["source_sha256"]);
    assert_eq!(recovered["revision"], 2);
    client.send("save", "edit", request);
    assert_eq!(client.reply("save")["type"], "error");
    client.send(
        "full",
        "edit",
        json!({"path":"main.tex","expected_revision":2,
        "expected_sha256":metadata["source_sha256"],"text":"default full response"}),
    );
    let full = client.reply("full");
    assert_eq!(full["payload"]["document"]["text"], "default full response");
    assert_eq!(full["payload"]["document"]["revision"], 3);
}
#[test]
fn unread_metadata_ack_reopens_source_and_stale_retry_does_not_apply_twice() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("before", "document", json!({"path":"main.tex"}));
    let before = client.reply("before")["payload"]["document"].clone();
    let request = json!({"path":"main.tex","expected_revision":1,
        "expected_sha256":before["source_sha256"],"text":"metadata acknowledgement not consumed",
        "response_mode":"metadata"});
    client.send("unread", "edit", request.clone());
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let persisted: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("store/document.json")).unwrap())
                .unwrap();
        if persisted["document"]["revision"] == 2 {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    // The application's reply receiver has deliberately not consumed the ACK.
    drop(client);
    let mut client = Client::start(dir.path());
    client.send("recovered", "document", json!({"path":"main.tex"}));
    let recovered = client.reply("recovered")["payload"]["document"].clone();
    assert_eq!(recovered["revision"], 2);
    assert_eq!(recovered["text"], request["text"]);
    client.send("unread", "edit", request);
    assert_eq!(client.reply("unread")["type"], "error");
    client.send("after", "document", json!({"path":"main.tex"}));
    assert_eq!(client.reply("after")["payload"]["document"], recovered);
}
#[test]
fn invalid_display_transport_is_rejected_before_source_import() {
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("must-not-be-created");
    for mode in [json!("unknown"), Value::Null, json!(true)] {
        let config = dir.path().join("bad-startup.json");
        std::fs::write(
            &config,
            serde_json::to_vec(&json!({
                "session_id":"session1", "display_transport":mode,
                "project_id":"p", "entry_path":"main.tex",
                "project_root":dir.path(), "private_ledger_root":private
            }))
            .unwrap(),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_flashtex-preview-controller"))
            .arg(&config)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr)
            .contains("display_transport must be value or raw-prototype"));
        assert!(!private.exists());
    }
}
/// The interpreter for the fake compilers (#207): `FLASHTEX_TEST_PYTHON`, else
/// `/usr/bin/python3` when it exists (what CI has always used), else the first
/// `python3` on `PATH` (NixOS has no `/usr/bin/python3`).
fn python3() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("FLASHTEX_TEST_PYTHON") {
        return path.into();
    }
    let system = std::path::PathBuf::from("/usr/bin/python3");
    if system.is_file() {
        return system;
    }
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join("python3"))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or(system)
}

/// A fake compiler script with a shebang for [`python3`].
fn script(body: &str) -> String {
    format!("#!{}\n{body}", python3().display())
}
fn bounded_diagnostics(path: &std::path::Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(path) else {
        return "unavailable".into();
    };
    let length = file.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(length.saturating_sub(8192)));
    let mut bytes = Vec::new();
    let _ = file.take(8192).read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}
impl Client {
    fn reader_snapshot(&self) -> String {
        format!(
            "{:?}",
            self.reader_progress
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        )
    }
    fn start(root: &std::path::Path) -> Self {
        Self::with_compiler(root, None)
    }
    fn with_compiler(root: &std::path::Path, compiler: Option<&std::path::Path>) -> Self {
        Self::with_transport(root, compiler, false)
    }
    fn with_transport(
        root: &std::path::Path,
        compiler: Option<&std::path::Path>,
        raw: bool,
    ) -> Self {
        let path = root.join("store");
        {
            let mut store = Store::open(&path).unwrap();
            if store.document().unwrap().is_none() {
                store
                    .initialize(
                        Document::new("p".into(), "main.tex".into(), 1, "α original".into())
                            .unwrap(),
                    )
                    .unwrap();
            }
        }
        Self::configured(
            root,
            json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","store_paths":[path],"compiler_path":compiler,"display_transport":if raw {"raw-prototype"} else {"value"}}),
        )
    }
    fn configured(root: &std::path::Path, value: Value) -> Self {
        Self::configured_stderr(root, value, Stdio::null())
    }
    fn configured_stderr(root: &std::path::Path, value: Value, stderr: Stdio) -> Self {
        let config = root.join("config.json");
        std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-preview-controller"))
            .arg(config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let input = child.stdin.take();
        let (tx, output) = mpsc::channel();
        let reader_progress = Arc::new(Mutex::new(ReaderProgress::default()));
        let progress = Arc::clone(&reader_progress);
        let reader_thread = thread::spawn(move || {
            progress.lock().unwrap().phase = "reading";
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    progress.lock().unwrap().phase = "io_error";
                    return;
                };
                {
                    let mut p = progress.lock().unwrap();
                    p.phase = "decoding";
                    p.last_frame_bytes = line.len();
                }
                let started = std::time::Instant::now();
                let value = match serde_json::from_str(&line) {
                    Ok(value) => value,
                    Err(_) => {
                        progress.lock().unwrap().phase = "json_error";
                        return;
                    }
                };
                {
                    let mut p = progress.lock().unwrap();
                    p.frames += 1;
                    p.last_decode_ms = started.elapsed().as_secs_f64() * 1000.0;
                    p.phase = "delivering";
                }
                if tx.send(value).is_err() {
                    break;
                }
                progress.lock().unwrap().phase = "reading";
            }
            progress.lock().unwrap().phase = "finished";
        });
        let client = Self {
            child,
            input,
            output,
            reader_progress,
            reader_thread: Some(reader_thread),
        };
        assert_eq!(
            client.output.recv_timeout(Duration::from_secs(3)).unwrap()["type"],
            "ready"
        );
        client
    }
    fn send(&mut self, id: &str, kind: &str, payload: Value) {
        let input = self.input.as_mut().unwrap();
        writeln!(input,"{}",json!({"protocol_version":1,"session_id":"session1","id":id,"type":kind,"payload":payload})).unwrap();
        input.flush().unwrap();
    }
    fn reply(&self, id: &str) -> Value {
        loop {
            let event = self.output.recv_timeout(Duration::from_secs(3)).unwrap();
            if event["id"] == id {
                return event;
            }
        }
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader_thread.take() {
            // Do not let a previous test's JSON decode continue into the next test.
            // A descendant retaining stdout must not cause an unbounded join.
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            while !reader.is_finished() && std::time::Instant::now() < deadline {
                thread::sleep(Duration::from_millis(2));
            }
            if reader.is_finished() {
                let result = reader.join();
                if !thread::panicking() {
                    assert!(result.is_ok(), "helper reader panicked");
                }
            } else if thread::panicking() {
                eprintln!(
                    "helper reader did not terminate after kill/reap: {:?}",
                    self.reader_snapshot()
                );
            } else {
                panic!(
                    "helper reader did not terminate after kill/reap: {:?}",
                    self.reader_snapshot()
                );
            }
        }
    }
}
#[test]
fn eof_drains_durable_edit_response_and_reopen_reads_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("get", "document", json!({"path":"main.tex"}));
    let document = client.reply("get")["payload"]["document"].clone();
    client.send("edit","edit",json!({"path":"main.tex","expected_revision":1,"expected_sha256":document["source_sha256"],"text":"β durable"}));
    client.input.take();
    let response = client.reply("edit");
    assert_eq!(response["payload"]["document"]["text"], "β durable");
    assert!(client.child.wait().unwrap().success());
    drop(client);
    let mut reopened = Client::start(dir.path());
    reopened.send("get", "document", json!({"path":"main.tex"}));
    assert_eq!(reopened.reply("get")["payload"]["document"]["revision"], 2);
}
#[test]
fn review_token_requires_explicit_approval_and_kill_preserves_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("get", "document", json!({"path":"main.tex"}));
    let doc = client.reply("get")["payload"]["document"].clone();
    let edit = json!({"capture_id":"capture1","edit_id":"edit1","project_id":"p","path":"main.tex","expected_revision":1,"start_byte":0,"end_byte":2,"removed_text":"α","replacement":"β","document_before_sha256":doc["source_sha256"]});
    client.send("review", "review", json!({"edit":edit}));
    let token = client.reply("review")["payload"]["approval_token"].clone();
    assert_eq!(token.as_str().unwrap().len(), 64);
    client.send(
        "no",
        "apply_reviewed",
        json!({"approval_token":token,"user_approved":false}),
    );
    assert_eq!(client.reply("no")["type"], "error");
    client.send(
        "yes",
        "apply_reviewed",
        json!({"approval_token":token,"user_approved":true}),
    );
    let applied = client.reply("yes");
    assert_eq!(applied["payload"]["receipt"]["new_revision"], 2);
    client.send(
        "again",
        "apply_reviewed",
        json!({"approval_token":token,"user_approved":true}),
    );
    assert_eq!(
        client.reply("again")["payload"]["receipt"],
        applied["payload"]["receipt"]
    );
    drop(client); // kill helper without graceful protocol close
    let mut reopened = Client::start(dir.path());
    reopened.send("recover", "recovery", json!({"path":"main.tex"}));
    let recovery = reopened.reply("recover");
    assert_eq!(
        recovery["payload"]["transactions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    reopened.send("get", "document", json!({"path":"main.tex"}));
    assert_eq!(
        reopened.reply("get")["payload"]["document"]["text"],
        "β original"
    );
}

#[test]
#[ignore = "requires explicitly configured original compiler"]
fn helper_streams_original_compiler_result_for_latest_durable_edit() {
    let binary = std::env::var_os("FLASHTEX_TEST_COMPILER").expect("original compiler required");
    for metadata_only in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut client = Client::with_compiler(dir.path(), Some(std::path::Path::new(&binary)));
        client.send("get", "document", json!({"path":"main.tex"}));
        let doc = client.reply("get")["payload"]["document"].clone();
        let source = "Actual streamed compiler preview";
        let mut request = json!({"path":"main.tex","expected_revision":1,
            "expected_sha256":doc["source_sha256"],"text":source});
        if metadata_only {
            request["response_mode"] = json!("metadata");
        }
        client.send("edit", "edit", request);
        let ack = client.reply("edit");
        assert!(ack["payload"]["preview_error"].is_null());
        assert_eq!(ack["payload"]["document"]["revision"], 2);
        assert_eq!(
            ack["payload"]["document"]["source_sha256"],
            flashtex_project_files::sha256_hex(source.as_bytes())
        );
        if metadata_only {
            assert_eq!(ack["payload"]["response_mode"], "metadata");
            assert!(ack["payload"]["document"].get("text").is_none());
            assert_eq!(ack["payload"]["document"]["byte_length"], source.len());
        } else {
            assert_eq!(ack["payload"]["document"]["text"], source);
        }
        loop {
            let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
            if event["type"] == "update"
                && event["payload"]["kind"] == "preview"
                && event["payload"]["source_versions"]["main.tex"] == 2
            {
                assert_eq!(event["payload"]["result"]["payload"]["status"], "ok");
                break;
            }
        }
    }
}

#[test]
fn native_queries_check_versions_and_return_utf8_source_navigation() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("get", "document", json!({"path":"main.tex"}));
    let doc = client.reply("get")["payload"]["document"].clone();
    let text = "α \\label{chapter} \\ref{chapter}";
    client.send("edit","edit",json!({"path":"main.tex","expected_revision":1,"expected_sha256":doc["source_sha256"],"text":text}));
    assert_eq!(client.reply("edit")["type"], "result");
    client.send("snapshot", "snapshot", json!({}));
    let versions = client.reply("snapshot")["payload"]["source_versions"].clone();
    client.send(
        "complete",
        "complete",
        json!({"source_versions":versions,"category":"label","prefix":"cha","limit":10}),
    );
    assert_eq!(
        client.reply("complete")["payload"]["completions"][0]["name"],
        "chapter"
    );
    client.send("nav","navigate",json!({"source_versions":versions,"path":"main.tex","byte_offset":text.rfind("chapter").unwrap()}));
    let navigation = client.reply("nav")["payload"]["navigation"].clone();
    assert_eq!(navigation["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(
        navigation["definitions"][0]["start_byte"],
        text.find("chapter").unwrap()
    );
    client.send(
        "stale",
        "complete",
        json!({"source_versions":{"main.tex":1},"category":"label","prefix":"cha","limit":10}),
    );
    assert_eq!(client.reply("stale")["type"], "error");
    client.send(
        "badutf8",
        "navigate",
        json!({"source_versions":versions,"path":"main.tex","byte_offset":1}),
    );
    assert_eq!(client.reply("badutf8")["type"], "error");
}

#[test]
fn undo_retry_after_helper_kill_is_idempotent_and_redo_remains_available() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("get", "document", json!({"path":"main.tex"}));
    let doc = client.reply("get")["payload"]["document"].clone();
    client.send("edit","edit",json!({"path":"main.tex","expected_revision":1,"expected_sha256":doc["source_sha256"],"text":"typed"}));
    let edited = client.reply("edit")["payload"]["document"].clone();
    let undo = json!({"command_id":"undo1","expected_revision":2,"expected_sha256":edited["source_sha256"]});
    client.send("undo", "undo", json!({"path":"main.tex","command":undo}));
    let undone = client.reply("undo")["payload"]["history"].clone();
    assert_eq!(undone["document"]["text"], "α original");
    assert_eq!(undone["document"]["revision"], 3);
    drop(client);
    let mut reopened = Client::start(dir.path());
    reopened.send("retry", "undo", json!({"path":"main.tex","command":undo}));
    let replayed = reopened.reply("retry")["payload"]["history"].clone();
    assert_eq!(replayed["replayed_command"], true);
    assert_eq!(replayed["document"]["revision"], 3);
    reopened.send("redo","redo",json!({"path":"main.tex","command":{"command_id":"redo1","expected_revision":3,"expected_sha256":undone["document"]["source_sha256"]}}));
    assert_eq!(
        reopened.reply("redo")["payload"]["history"]["document"]["text"],
        "typed"
    );
}

#[test]
fn stalled_output_reader_causes_bounded_failure_instead_of_unlimited_queueing() {
    stalled_reader(40);
}

#[test]
fn single_stalled_reply_times_out_without_filling_output_queue() {
    stalled_reader(1);
}

fn stalled_reader(requests: usize) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store");
    {
        let mut store = Store::open(&path).unwrap();
        store
            .initialize(
                Document::new("p".into(), "main.tex".into(), 1, "x".repeat(1024 * 1024)).unwrap(),
            )
            .unwrap();
    }
    let config = dir.path().join("config.json");
    std::fs::write(&config,serde_json::to_vec(&json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","store_paths":[path]})).unwrap()).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-preview-controller"))
        .arg(config)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut unread_output = BufReader::new(child.stdout.take().unwrap());
    let input = child.stdin.take();
    let (_sender, output) = mpsc::channel();
    let mut client = Client {
        child,
        input,
        output,
        reader_progress: Arc::new(Mutex::new(ReaderProgress {
            phase: "intentionally_unread",
            ..ReaderProgress::default()
        })),
        reader_thread: None,
    };
    let mut ready = String::new();
    unread_output.read_line(&mut ready).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&ready).unwrap()["type"],
        "ready"
    );
    // Keep stdout open but deliberately stop draining it. Each document reply is
    // larger than the OS pipe; bounded helper output admission must stop work.
    for id in 0..requests {
        let line = json!({"protocol_version":1,"session_id":"session1","id":format!("r{id}"),"type":"document","payload":{"path":"main.tex"}});
        if writeln!(client.input.as_mut().unwrap(), "{line}").is_err() {
            break;
        }
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = client.child.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "helper did not stop on output backpressure"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn wrong_session_cannot_edit_authoritative_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    writeln!(client.input.as_mut().unwrap(),"{}",json!({"protocol_version":1,"session_id":"another-session","id":"wrong","type":"edit","payload":{"path":"main.tex","expected_revision":1,"expected_sha256":"unused","text":"must not save"}})).unwrap();
    assert_eq!(client.reply("wrong")["type"], "error");
    client.send("get", "document", json!({"path":"main.tex"}));
    assert_eq!(
        client.reply("get")["payload"]["document"]["text"],
        "α original"
    );
}

#[test]
fn file_project_helper_reports_external_change_without_overwrite() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "initial source").unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","project_root":root.path(),"private_ledger_root":private.path()});
    let mut client = Client::configured(config_dir.path(), config.clone());
    client.send("status", "file_status", json!({"path":"main.tex"}));
    assert_eq!(
        client.reply("status")["payload"]["disk"]["state"],
        "matches_source"
    );
    std::fs::write(root.path().join("main.tex"), "external source").unwrap();
    client.send("status2", "file_status", json!({"path":"main.tex"}));
    assert_eq!(
        client.reply("status2")["payload"]["disk"]["state"],
        "differs_from_source"
    );
    client.send("export", "export", json!({"path":"main.tex"}));
    assert_eq!(client.reply("export")["type"], "error");
    drop(client);
    let mut reopened = Client::configured(config_dir.path(), config);
    reopened.send("get", "document", json!({"path":"main.tex"}));
    assert_eq!(
        reopened.reply("get")["payload"]["document"]["text"],
        "initial source"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("main.tex")).unwrap(),
        "external source"
    );
}

#[test]
fn literal_search_exposes_utf8_matches_work_limits_and_stale_version_errors() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("get", "document", json!({"path":"main.tex"}));
    let doc = client.reply("get")["payload"]["document"].clone();
    client.send("edit","edit",json!({"path":"main.tex","expected_revision":1,"expected_sha256":doc["source_sha256"],"text":"α x α"}));
    assert_eq!(client.reply("edit")["type"], "result");
    let query =
        json!({"source_versions":{"main.tex":2},"literal":"α","max_matches":10,"max_work":100});
    client.send("search", "search_literal", query.clone());
    let result = client.reply("search")["payload"].clone();
    assert_eq!(result["termination"], "complete");
    assert_eq!(result["matches"].as_array().unwrap().len(), 2);
    assert_eq!(result["matches"][1]["start_byte"], 5);
    let mut bounded = query.clone();
    bounded["max_work"] = json!(1);
    client.send("bounded", "search_literal", bounded);
    assert_eq!(
        client.reply("bounded")["payload"]["termination"],
        "work_limit"
    );
    let mut stale = query;
    stale["source_versions"] = json!({"main.tex":1});
    client.send("stale", "search_literal", stale);
    assert_eq!(client.reply("stale")["type"], "error");
}

#[test]
fn helper_exports_exact_source_and_refuses_conflicting_or_ambiguous_expectations() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "initial").unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","project_root":root.path(),"private_ledger_root":private.path()});
    let mut client = Client::configured(config_dir.path(), config.clone());
    client.send("get", "document", json!({"path":"main.tex"}));
    let old = client.reply("get")["payload"]["document"].clone();
    client.send("edit", "edit", json!({"path":"main.tex","expected_revision":1,"expected_sha256":old["source_sha256"],"text":"saved β"}));
    let new = client.reply("edit")["payload"]["document"].clone();
    let request = json!({"path":"main.tex","expected_revision":2,"expected_sha256":new["source_sha256"],"expected_disk_sha256":old["source_sha256"]});
    let mut missing = request.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove("expected_disk_sha256");
    client.send("missing", "export", missing);
    assert_eq!(client.reply("missing")["type"], "error");
    client.send("save", "export", request.clone());
    let saved = client.reply("save");
    assert_eq!(saved["type"], "result");
    assert_eq!(saved["payload"]["sha256"], new["source_sha256"]);
    assert_eq!(
        std::fs::read_to_string(root.path().join("main.tex")).unwrap(),
        "saved β"
    );
    std::fs::write(root.path().join("main.tex"), "external").unwrap();
    client.send("conflict", "export", request);
    assert_eq!(client.reply("conflict")["type"], "error");
    assert_eq!(
        std::fs::read_to_string(root.path().join("main.tex")).unwrap(),
        "external"
    );
    drop(client);
    let mut reopened = Client::configured(config_dir.path(), config);
    reopened.send("get", "document", json!({"path":"main.tex"}));
    assert_eq!(
        reopened.reply("get")["payload"]["document"]["text"],
        "saved β"
    );
}

#[test]
fn helper_reload_requires_approval_and_preserves_disk() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "initial").unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","project_root":root.path(),"private_ledger_root":private.path()});
    let mut client = Client::configured(config_dir.path(), config);
    client.send("get", "document", json!({"path":"main.tex"}));
    let old = client.reply("get")["payload"]["document"].clone();
    std::fs::write(root.path().join("main.tex"), "external").unwrap();
    let hash = flashtex_project_files::sha256_hex(b"external");
    let mut request = json!({"path":"main.tex","expected_revision":1,"expected_sha256":old["source_sha256"],"expected_disk_sha256":hash});
    client.send("refused", "reload", request.clone());
    assert_eq!(client.reply("refused")["type"], "error");
    client.send("unchanged", "document", json!({"path":"main.tex"}));
    assert_eq!(client.reply("unchanged")["payload"]["document"], old);
    request["user_approved"] = json!(true);
    client.send("reload", "reload", request.clone());
    assert_eq!(
        client.reply("reload")["payload"]["document"]["text"],
        "external"
    );
    client.send("retry", "reload", request);
    assert_eq!(client.reply("retry")["type"], "error");
    assert_eq!(
        std::fs::read_to_string(root.path().join("main.tex")).unwrap(),
        "external"
    );
}

#[test]
fn helper_membership_and_bounded_status_preserve_detached_source() {
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.tex"), "initial").unwrap();
    std::fs::write(root.path().join("extra.tex"), "extra α").unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","project_root":root.path(),"private_ledger_root":private.path()});
    let mut client = Client::configured(config_dir.path(), config);
    client.send("snapshot", "snapshot", json!({}));
    let initial = client.reply("snapshot")["payload"].clone();
    let request = json!({"path":"extra.tex","source_versions":initial["source_versions"],"membership_generation":initial["membership_generation"]});
    client.send("open", "open_document", request.clone());
    let added = client.reply("open")["payload"].clone();
    assert_eq!(added["document"]["text"], "extra α");
    client.send("stale", "detach_document", request);
    assert_eq!(client.reply("stale")["type"], "error");
    client.send("status", "project_status", json!({"max_documents":1}));
    let status = client.reply("status")["payload"].clone();
    assert_eq!(status["documents"].as_array().unwrap().len(), 1);
    assert_eq!(status["total_documents"], 2);
    assert_eq!(status["truncated"], true);
    client.send("detach", "detach_document", json!({"path":"extra.tex","source_versions":added["source_versions"],"membership_generation":added["membership_generation"]}));
    let removed = client.reply("detach")["payload"].clone();
    std::fs::remove_file(root.path().join("extra.tex")).unwrap();
    client.send("reopen", "open_document", json!({"path":"extra.tex","source_versions":removed["source_versions"],"membership_generation":removed["membership_generation"]}));
    assert_eq!(
        client.reply("reopen")["payload"]["document"]["text"],
        "extra α"
    );
    assert!(!root.path().join("extra.tex").exists());
}

#[test]
#[ignore = "requires explicitly configured original compiler"]
fn configured_large_result_reaches_real_helper_without_dropping_pages() {
    let compiler = std::env::var("FLASHTEX_TEST_COMPILER").unwrap();
    let root = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let config_dir = tempfile::tempdir().unwrap();
    let text = "Measured paragraph with ordinary words and spaces.\n\n".repeat(9804);
    std::fs::write(root.path().join("main.tex"), &text).unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","project_root":root.path(),"private_ledger_root":private.path(),"compiler_path":compiler,"compiler_max_frame_bytes":12*1024*1024,"diagnostic_timings":true});
    let diagnostic_path = config_dir.path().join("diagnostics.jsonl");
    let client = Client::configured_stderr(
        config_dir.path(),
        config,
        Stdio::from(std::fs::File::create(&diagnostic_path).unwrap()),
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let event = client
            .output
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap_or_else(|error| {
                panic!(
                    "large preview receive {error:?}; reader={:?}; helper={}",
                    client.reader_snapshot(),
                    bounded_diagnostics(&diagnostic_path)
                )
            });
        assert_ne!(event["type"], "error", "{event}");
        if event["type"] != "update" {
            continue;
        }
        assert_ne!(event["payload"]["kind"], "failed");
        if event["payload"]["kind"] == "preview" {
            let result = &event["payload"]["result"];
            assert_eq!(result["payload"]["status"], "ok");
            assert!(serde_json::to_vec(result).unwrap().len() > 8 * 1024 * 1024);
            let pages = result["payload"]["pages"].as_array().unwrap();
            assert!(pages.len() > 250);
            for (index, page) in pages.iter().enumerate() {
                assert_eq!(page["number"], index + 1);
            }
            break;
        }
    }
}

#[test]
#[cfg(unix)]
fn negotiated_history_echoes_original_token_and_restart_requires_renegotiation() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let compiler = dir.path().join("gated.py");
    std::fs::write(&compiler, script(r#"import json,sys,pathlib,time
root=pathlib.Path(__file__).parent
for line in sys.stdin:
 r=json.loads(line);p=r['payload'];revision=p['revision']
 (root/('started'+str(revision))).touch()
 while revision>1 and not (root/('release'+str(revision))).exists(): time.sleep(.001)
 print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':revision,'status':'ok','pages':[],'diagnostics':[]}}),flush=True)
"#)).unwrap();
    std::fs::set_permissions(&compiler, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut client = Client::with_compiler(dir.path(), Some(&compiler));
    loop {
        let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_ne!(event["payload"]["kind"], "completed_snapshot");
        if event["payload"]["kind"] == "preview" {
            break;
        }
    }
    client.send(
        "configure",
        "configure_completed_snapshots",
        json!({"capability":"completed-snapshots-v1","enabled":true}),
    );
    assert_eq!(client.reply("configure")["payload"]["enabled"], true);
    client.send(
        "a",
        "compile",
        json!({"source_binding_token":"editor-A:α\nopaque"}),
    );
    assert_eq!(client.reply("a")["type"], "result");
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !dir.path().join("started2").exists() {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    client.send("get", "document", json!({"path":"main.tex"}));
    let doc = client.reply("get")["payload"]["document"].clone();
    client.send("b", "edit", json!({"path":"main.tex","expected_revision":1,"expected_sha256":doc["source_sha256"],"text":"new durable source","source_binding_token":"editor-B"}));
    assert_eq!(client.reply("b")["payload"]["document"]["revision"], 2);
    std::fs::write(dir.path().join("release2"), b"ok").unwrap();
    let historical = loop {
        let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
        if event["payload"]["kind"] == "completed_snapshot" {
            break event["payload"].clone();
        }
    };
    assert_eq!(historical["source_binding_token"], "editor-A:α\nopaque");
    assert_eq!(historical["source_versions"]["main.tex"], 1);
    assert_eq!(historical["compile_revision"], 2);
    assert_eq!(historical["current_compile_revision"], 3);
    assert_eq!(historical["project_id"], "p");
    assert_eq!(historical["session_id"], "session1");
    assert_eq!(historical["is_current"], false);
    assert_eq!(historical["source_actions_enabled"], false);
    assert_eq!(historical["result"]["payload"]["revision"], 2);
    std::fs::write(dir.path().join("release3"), b"ok").unwrap();
    loop {
        let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
        if event["payload"]["kind"] == "preview" {
            assert_eq!(event["payload"]["source_versions"]["main.tex"], 2);
            break;
        }
    }
    client.send("restart", "restart", json!({}));
    assert_eq!(client.reply("restart")["type"], "result");
    std::fs::write(dir.path().join("release4"), b"ok").unwrap();
    loop {
        let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_ne!(event["payload"]["kind"], "completed_snapshot");
        if event["payload"]["kind"] == "preview" {
            break;
        }
    }
    client.send(
        "after-restart-a",
        "compile",
        json!({"source_binding_token":"must-not-enroll"}),
    );
    assert_eq!(client.reply("after-restart-a")["type"], "result");
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !dir.path().join("started5").exists() {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    client.send(
        "after-restart-b",
        "compile",
        json!({"source_binding_token":"newer"}),
    );
    assert_eq!(client.reply("after-restart-b")["type"], "result");
    std::fs::write(dir.path().join("release5"), b"ok").unwrap();
    loop {
        let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_ne!(event["payload"]["kind"], "completed_snapshot");
        if event["payload"]["kind"] == "stale" {
            assert_eq!(event["payload"]["compile_revision"], 5);
            break;
        }
    }
    client.send(
        "config-check",
        "configure_completed_snapshots",
        json!({"capability":"wrong","enabled":true}),
    );
    assert_eq!(client.reply("config-check")["type"], "error");
    client.send("get2", "document", json!({"path":"main.tex"}));
    let saved = client.reply("get2")["payload"]["document"].clone();
    client.send("badtoken", "edit", json!({"path":"main.tex","expected_revision":2,"expected_sha256":saved["source_sha256"],"text":"must not save","source_binding_token":"α".repeat(65)}));
    assert_eq!(client.reply("badtoken")["type"], "error");
    client.send("get3", "document", json!({"path":"main.tex"}));
    assert_eq!(client.reply("get3")["payload"]["document"], saved);
}

#[test]
fn source_plans_are_bounded_exact_snapshot_proposals_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let text = "α \\cite{old}\n% \\cite{old}\n\\begin{verbatim}\\cite{old}\\end{verbatim}\n";
    let bib = "@article{old,title={Title}}";
    let mut paths = Vec::new();
    for (i, (file, source)) in [("main.tex", text), ("refs.bib", bib)].iter().enumerate() {
        let path = dir.path().join(format!("store-{i}"));
        let mut store = Store::open(&path).unwrap();
        store
            .initialize(Document::new("p".into(), (*file).into(), 1, (*source).into()).unwrap())
            .unwrap();
        paths.push(path);
    }
    let mut client = Client::configured(
        dir.path(),
        json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","store_paths":paths,"bibliography_paths":["refs.bib"]}),
    );
    client.send("snapshot", "snapshot", json!({}));
    let snapshot = client.reply("snapshot")["payload"].clone();
    let mut request = snapshot.clone();
    request["max_bytes"] = json!(100000);
    request["old_name"] = json!("old");
    request["new_name"] = json!("new");
    client.send("rename", "plan_citation_rename", request.clone());
    let response = client.reply("rename");
    assert_eq!(response["type"], "result", "{response}");
    let plan = &response["payload"]["plan"];
    assert_eq!(plan["proposal_only"], true);
    assert_eq!(plan["requires_user_approval"], true);
    assert_eq!(plan["edits"].as_array().unwrap().len(), 2);
    let start = text.find("old").unwrap();
    request["path"] = json!("main.tex");
    request["start_byte"] = json!(start);
    request["end_byte"] = json!(start + 3);
    client.send("at", "plan_citation_rename_at", request.clone());
    assert_eq!(client.reply("at")["payload"]["plan"], *plan);
    request["max_bytes"] = json!(16);
    client.send("small", "plan_citation_rename", request.clone());
    assert_eq!(client.reply("small")["type"], "error");
    request["max_bytes"] = json!(100000);
    request["literal"] = json!("α");
    request["replacement"] = json!("βγ");
    request["max_matches"] = json!(100);
    request["max_work"] = json!(100000);
    client.send("literal", "plan_literal_replacement", request.clone());
    let literal = client.reply("literal");
    assert_eq!(literal["payload"]["plan"]["edits"][0]["end_byte"], "2");
    client.send("unchanged", "document", json!({"path":"main.tex"}));
    assert_eq!(
        client.reply("unchanged")["payload"]["document"]["text"],
        text
    );
    client.send("bib", "document", json!({"path":"refs.bib"}));
    let document = client.reply("bib")["payload"]["document"].clone();
    client.send("edit-bib", "edit", json!({"path":"refs.bib","expected_revision":1,"expected_sha256":document["source_sha256"],"text":"@article{old,title={Changed}}"}));
    assert_eq!(client.reply("edit-bib")["type"], "result");
    for kind in [
        "plan_literal_replacement",
        "plan_citation_rename",
        "plan_citation_rename_at",
    ] {
        client.send(kind, kind, request.clone());
        assert_eq!(client.reply(kind)["type"], "error");
    }
}

#[test]
fn rooted_bibliography_plans_survive_typed_attach_edit_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("main.tex"), "\\cite{old}").unwrap();
    std::fs::write(
        project.path().join("references.data"),
        "@article{old,title={Title}}",
    )
    .unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex","project_root":project.path(),"private_ledger_root":private.path(),"bibliography_paths":["references.data"]});
    let mut client = Client::configured(dir.path(), config.clone());
    let plan = |client: &mut Client| {
        client.send("snap", "snapshot", json!({}));
        let mut request = client.reply("snap")["payload"].clone();
        assert_eq!(
            request["document_kinds"],
            json!({"main.tex":"latex","references.data":"bibliography"})
        );
        request["max_bytes"] = json!(100000);
        request["old_name"] = json!("old");
        request["new_name"] = json!("new");
        client.send("plan", "plan_citation_rename", request);
        let response = client.reply("plan");
        assert_eq!(response["type"], "result", "{response}");
        assert_eq!(
            response["payload"]["plan"]["edits"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    };
    plan(&mut client);
    client.send("snap", "snapshot", json!({}));
    let mut detach = client.reply("snap")["payload"].clone();
    detach["path"] = json!("references.data");
    client.send("detach", "detach_document", detach.clone());
    let detached = client.reply("detach");
    assert_eq!(detached["type"], "result");
    let mut attach = detached["payload"].clone();
    attach["path"] = json!("references.data");
    attach["document_kind"] = json!("bibliography");
    client.send("attach", "open_document", attach);
    assert_eq!(client.reply("attach")["type"], "result");
    plan(&mut client);
    // Membership generation distinguishes detach/reattach with unchanged revisions.
    detach["max_bytes"] = json!(100000);
    detach["old_name"] = json!("old");
    detach["new_name"] = json!("new");
    client.send("stale", "plan_citation_rename", detach);
    assert_eq!(client.reply("stale")["type"], "error");
    client.send("doc", "document", json!({"path":"references.data"}));
    let doc = client.reply("doc")["payload"]["document"].clone();
    client.send("edit", "edit", json!({"path":"references.data","expected_revision":doc["revision"],"expected_sha256":doc["source_sha256"],"text":"@article{old,title={Durable}}"}));
    assert_eq!(client.reply("edit")["type"], "result");
    plan(&mut client);
    for (kind, id) in [("undo", "undo-bib"), ("redo", "redo-bib")] {
        client.send("history-doc", "document", json!({"path":"references.data"}));
        let document = client.reply("history-doc")["payload"]["document"].clone();
        client.send(id, kind, json!({"path":"references.data","command":{"command_id":id,"expected_revision":document["revision"],"expected_sha256":document["source_sha256"]}}));
        assert_eq!(client.reply(id)["type"], "result");
        plan(&mut client);
    }
    drop(client);
    std::fs::remove_file(project.path().join("references.data")).unwrap();
    let mut client = Client::configured(dir.path(), config);
    plan(&mut client);
    client.send("doc", "document", json!({"path":"references.data"}));
    assert_eq!(
        client.reply("doc")["payload"]["document"]["text"],
        "@article{old,title={Durable}}"
    );
}

#[test]
fn generated_literal_plan_applies_as_one_durable_group_and_retries_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    let proposal = |client: &mut Client| {
        client.send("snapshot", "snapshot", json!({}));
        let mut request = client.reply("snapshot")["payload"].clone();
        request["literal"] = json!("α");
        request["replacement"] = json!("βγ");
        request["max_matches"] = json!(100);
        request["max_work"] = json!(100000);
        request["max_bytes"] = json!(100000);
        client.send("proposal", "plan_literal_replacement", request);
        let response = client.reply("proposal");
        assert_eq!(response["type"], "result", "{response}");
        response["payload"]["plan"].clone()
    };
    let prepare_group = |plan: &Value, doc: &Value, id: &str| {
        // Native conversion must preserve exact decimal-string values. This test
        // exercises the real exported proposal, not independently invented edits.
        let edits: Vec<_> = plan["edits"].as_array().unwrap().iter().map(|edit| {
            assert_eq!(edit["file"], "main.tex");
            assert_eq!(edit["revision"].as_str().unwrap().parse::<u64>().unwrap(), doc["revision"].as_u64().unwrap());
            json!({"start_byte":edit["start_byte"].as_str().unwrap().parse::<u64>().unwrap(),"end_byte":edit["end_byte"].as_str().unwrap().parse::<u64>().unwrap(),"removed_text":edit["expected_text"],"replacement":edit["replacement"]})
        }).collect();
        json!({"path":"main.tex","command":{"command_id":id,"expected_revision":doc["revision"],"expected_sha256":doc["source_sha256"],"label":"Replace reviewed matches","edits":edits}})
    };
    let old = proposal(&mut client);
    client.send("doc", "document", json!({"path":"main.tex"}));
    let doc = client.reply("doc")["payload"]["document"].clone();
    let stale = prepare_group(&old, &doc, "stale-plan");
    client.send("typing", "edit", json!({"path":"main.tex","expected_revision":doc["revision"],"expected_sha256":doc["source_sha256"],"text":"α α original"}));
    assert_eq!(client.reply("typing")["type"], "result");
    client.send("stale", "apply_group", stale);
    assert_eq!(client.reply("stale")["type"], "error");
    let fresh = proposal(&mut client);
    client.send("doc", "document", json!({"path":"main.tex"}));
    let doc = client.reply("doc")["payload"]["document"].clone();
    let approved = prepare_group(&fresh, &doc, "approved-plan");
    assert_eq!(approved["command"]["edits"].as_array().unwrap().len(), 2);
    client.send("apply", "apply_group", approved.clone());
    let applied = client.reply("apply")["payload"]["history"].clone();
    assert_eq!(applied["document"]["text"], "βγ βγ original");
    assert_eq!(applied["replayed_command"], false);
    drop(client);
    let mut client = Client::start(dir.path());
    client.send("retry", "apply_group", approved.clone());
    let retry = client.reply("retry")["payload"]["history"].clone();
    assert_eq!(retry["replayed_command"], true);
    assert_eq!(retry["document"], applied["document"]);
    let mut changed = approved;
    changed["command"]["edits"][0]["replacement"] = json!("tampered");
    client.send("conflict", "apply_group", changed);
    assert_eq!(client.reply("conflict")["type"], "error");
    client.send("undo", "undo", json!({"path":"main.tex","command":{"command_id":"undo-plan","expected_revision":retry["document"]["revision"],"expected_sha256":retry["document"]["source_sha256"]}}));
    assert_eq!(
        client.reply("undo")["payload"]["history"]["document"]["text"],
        "α α original"
    );
}

#[test]
fn history_status_binds_labels_and_limits_to_current_source_without_text() {
    use flashtex_edit_ledger::history::{
        MAX_HISTORY_BYTES, MAX_HISTORY_COMMAND_IDS, MAX_HISTORY_ENTRIES,
    };
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    client.send("status", "history_status", json!({"path":"main.tex"}));
    let before = client.reply("status")["payload"].clone();
    assert_eq!(before["document"]["revision"], 1);
    assert!(before["document"].get("text").is_none());
    assert_eq!(
        before["limits"],
        json!({"history_bytes":MAX_HISTORY_BYTES,"history_entries":MAX_HISTORY_ENTRIES,"permanent_command_ids":MAX_HISTORY_COMMAND_IDS})
    );
    client.send("edit", "edit", json!({"path":"main.tex","expected_revision":1,"expected_sha256":before["document"]["source_sha256"],"text":"α next"}));
    assert_eq!(client.reply("edit")["type"], "result");
    client.send("status", "history_status", json!({"path":"main.tex"}));
    let after = client.reply("status")["payload"].clone();
    assert_eq!(after["document"]["revision"], 2);
    assert_eq!(after["history"]["undo_labels"].as_array().unwrap().len(), 1);
    client.send("undo", "undo", json!({"path":"main.tex","command":{"command_id":"status-undo","expected_revision":after["document"]["revision"],"expected_sha256":after["document"]["source_sha256"]}}));
    assert_eq!(
        client.reply("undo")["payload"]["history"]["document"]["text"],
        "α original"
    );
    client.send("status", "history_status", json!({"path":"main.tex"}));
    let undone = client.reply("status")["payload"].clone();
    assert_eq!(undone["document"]["revision"], 3);
    assert_eq!(undone["history"]["permanent_command_ids"], 1);
    assert_eq!(
        undone["history"]["redo_labels"].as_array().unwrap().len(),
        1
    );
}

#[test]
#[cfg(unix)]
fn display_candidate_opt_in_preserves_v1_and_individual_source_versions() {
    display_candidate_lifecycle(false);
}
#[test]
#[cfg(unix)]
fn raw_display_candidate_lifecycle_preserves_default_fallback_and_restart_strategy() {
    display_candidate_lifecycle(true);
}
#[cfg(unix)]
fn display_candidate_lifecycle(raw: bool) {
    let capability = if raw {
        "display-candidates-raw-v1"
    } else {
        "display-candidates-v1"
    };
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let compiler = dir.path().join("display-fixture.py");
    // Transport fixture only: intentionally not renderer-valid or a real compiler.
    std::fs::write(&compiler, r#"#!/usr/bin/env python3
import json,sys,hashlib
for line in sys.stdin:
 r=json.loads(line);p=r['payload'];caps=p.get('layout_capabilities',[])
 print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[],'layout_capabilities':caps}}),flush=True)
 if 'display-list-v2' in caps:
  docs=[{'path':d['path'],'revision':p['revision'],'sha256':hashlib.sha256(d['text'].encode()).hexdigest(),'byte_length':len(d['text'].encode())} for d in p['documents']]
  print(json.dumps({'protocol_version':2,'type':'display_list','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'render_format':'display-list-v2','documents':docs}}),flush=True)
"#).unwrap();
    std::fs::set_permissions(&compiler, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut client = Client::with_transport(dir.path(), Some(&compiler), raw);
    client.send("wrong-mode", "configure_display_candidates", json!({"capability":if raw {"display-candidates-v1"} else {"display-candidates-raw-v1"},"enabled":true,"renderer_support_confirmed":true}));
    assert_eq!(client.reply("wrong-mode")["type"], "error");
    client.send(
        "unconfirmed",
        "configure_display_candidates",
        json!({"capability":capability,"enabled":true}),
    );
    assert_eq!(client.reply("unconfirmed")["type"], "error");
    client.send(
        "enable",
        "configure_display_candidates",
        json!({"capability":capability,"enabled":true,"renderer_support_confirmed":true}),
    );
    assert_eq!(client.reply("enable")["payload"]["enabled"], true);
    let mut seen_v1 = Vec::new();
    loop {
        let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
        if event["payload"]["kind"] == "preview" {
            seen_v1.push((
                event["payload"]["request_id"].clone(),
                event["payload"]["compile_revision"].clone(),
            ));
        }
        if event["payload"]["kind"] == "display_candidate" {
            let p = &event["payload"];
            assert!(seen_v1.contains(&(p["request_id"].clone(), p["compile_revision"].clone())));
            assert_eq!(p["untrusted"], true);
            assert_eq!(p["source_actions_enabled"], false);
            assert_eq!(p["source_versions"]["main.tex"], 1);
            assert!(p["compile_revision"].as_u64().unwrap() > 1);
            assert_eq!(
                p["display_list"]["payload"]["documents"][0]["revision"],
                p["compile_revision"]
            );
            assert_eq!(
                p["display_list"]["payload"]["documents"][0]["byte_length"],
                "α original".len()
            );
            break;
        }
    }
    client.send(
        "conflict",
        "configure_completed_snapshots",
        json!({"capability":"completed-snapshots-v1","enabled":true}),
    );
    assert_eq!(client.reply("conflict")["type"], "error");
    client.send(
        "disable",
        "configure_display_candidates",
        json!({"capability":capability,"enabled":false}),
    );
    assert_eq!(client.reply("disable")["payload"]["enabled"], false);
    client.send(
        "history",
        "configure_completed_snapshots",
        json!({"capability":"completed-snapshots-v1","enabled":true}),
    );
    assert_eq!(client.reply("history")["payload"]["enabled"], true);
    client.send(
        "conflict2",
        "configure_display_candidates",
        json!({"capability":capability,"enabled":true,"renderer_support_confirmed":true}),
    );
    assert_eq!(client.reply("conflict2")["type"], "error");
    client.send("restart", "restart", json!({}));
    assert_eq!(client.reply("restart")["type"], "result");
    // A restart resets both negotiated optional modes. The fallback still arrives.
    loop {
        let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_ne!(event["payload"]["kind"], "display_candidate");
        if event["payload"]["kind"] == "preview" {
            break;
        }
    }
    client.send(
        "history-off",
        "configure_completed_snapshots",
        json!({"capability":"completed-snapshots-v1","enabled":false}),
    );
    assert_eq!(client.reply("history-off")["type"], "result");
    client.send(
        "reenable",
        "configure_display_candidates",
        json!({"capability":capability,"enabled":true,"renderer_support_confirmed":true}),
    );
    assert_eq!(
        client.reply("reenable")["payload"]["capability"],
        capability
    );
    loop {
        let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_ne!(event["payload"]["kind"], "failed");
        if event["payload"]["kind"] == "display_candidate" {
            break;
        }
    }
}

#[cfg(unix)]
fn display_failure_fixture(root: &std::path::Path, mode: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = root.join("display-failure.py");
    // Deliberately minimal transport fixture; never presented as renderer-valid.
    let script = r#"#!/usr/bin/env python3
import json,sys,hashlib,time,pathlib
mode='MODE'
held=False
for line in sys.stdin:
 r=json.loads(line);p=r['payload'];caps=p.get('layout_capabilities',[])
 print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[],'layout_capabilities':caps}}),flush=True)
 if 'display-list-v2' not in caps:continue
 if mode=='hold' and not held:
  held=True
  while not pathlib.Path(__file__+'.release').exists():time.sleep(.002)
 docs=[{'path':d['path'],'revision':p['revision'],'sha256':hashlib.sha256(d['text'].encode()).hexdigest(),'byte_length':len(d['text'].encode())} for d in p['documents']]
 if mode=='hash':docs[0]['sha256']='0'*64
 wire=json.dumps({'protocol_version':2,'type':'display_list','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'render_format':'display-list-v2','documents':docs}})
 if mode=='oversize':wire=wire[:-2]+',"stress":['+','.join(['1e9']*1400000)+']}}'
 if mode=='large':wire=wire[:-2]+',"stress":'+json.dumps('x'*1048576)+'}}'
 print(wire,flush=True)
"#.replace("MODE", mode);
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}
#[cfg(unix)]
fn enable_display(client: &mut Client) {
    enable_display_transport(client, false);
}
#[cfg(unix)]
fn enable_display_transport(client: &mut Client, raw: bool) {
    let capability = if raw {
        "display-candidates-raw-v1"
    } else {
        "display-candidates-v1"
    };
    client.send(
        "enable",
        "configure_display_candidates",
        json!({"capability":capability,"enabled":true,"renderer_support_confirmed":true}),
    );
    assert_eq!(client.reply("enable")["payload"]["enabled"], true);
}

#[test]
#[cfg(unix)]
fn stale_display_sibling_after_durable_edit_never_reaches_optional_output() {
    for raw in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let compiler = display_failure_fixture(dir.path(), "hold");
        let mut client = Client::with_transport(dir.path(), Some(&compiler), raw);
        enable_display_transport(&mut client, raw);
        let old_generation = loop {
            let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
            let p = &event["payload"];
            if p["kind"] == "preview"
                && p["result"]["payload"]["layout_capabilities"]
                    .as_array()
                    .is_some_and(|caps| caps.iter().any(|cap| cap == "display-list-v2"))
            {
                break p["compile_revision"].as_u64().unwrap();
            }
        };
        client.send("doc", "document", json!({"path":"main.tex"}));
        let doc = client.reply("doc")["payload"]["document"].clone();
        client.send("edit", "edit", json!({"path":"main.tex","expected_revision":doc["revision"],"expected_sha256":doc["source_sha256"],"text":"β current"}));
        let edited = client.reply("edit")["payload"]["document"].clone();
        assert_eq!(edited["revision"], 2);
        std::fs::write(
            compiler.with_file_name("display-failure.py.release"),
            "release",
        )
        .unwrap();
        loop {
            let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
            let p = &event["payload"];
            assert_ne!(p["kind"], "failed", "{event}");
            if p["kind"] == "display_candidate" {
                assert!(p["compile_revision"].as_u64().unwrap() > old_generation);
                assert_eq!(p["source_versions"]["main.tex"], 2);
                assert_eq!(
                    p["display_list"]["payload"]["documents"][0]["sha256"],
                    edited["source_sha256"]
                );
                break;
            }
        }
    }
}

#[test]
#[cfg(unix)]
fn corrupt_display_hash_fails_preview_but_preserves_durable_edit_and_reopen() {
    for raw in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let compiler = display_failure_fixture(dir.path(), "hash");
        let mut client = Client::with_transport(dir.path(), Some(&compiler), raw);
        enable_display_transport(&mut client, raw);
        loop {
            let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
            assert_ne!(event["payload"]["kind"], "display_candidate");
            if event["payload"]["kind"] == "failed" {
                break;
            }
        }
        client.send("doc", "document", json!({"path":"main.tex"}));
        let doc = client.reply("doc")["payload"]["document"].clone();
        client.send("edit", "edit", json!({"path":"main.tex","expected_revision":doc["revision"],"expected_sha256":doc["source_sha256"],"text":"durable despite compiler failure"}));
        let response = client.reply("edit");
        assert_eq!(response["type"], "result");
        assert!(response["payload"]["preview_error"].is_string());
        drop(client);
        let mut reopened = Client::start(dir.path());
        reopened.send("doc", "document", json!({"path":"main.tex"}));
        assert_eq!(
            reopened.reply("doc")["payload"]["document"]["text"],
            "durable despite compiler failure"
        );
    }
}

#[test]
#[cfg(unix)]
fn full_size_optional_expansion_drops_candidate_and_keeps_edit_ack_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    // Initialize the existing durable store without creating any renderer dependency.
    drop(Client::start(dir.path()));
    let compiler = display_failure_fixture(dir.path(), "oversize");
    let diagnostic_path = dir.path().join("diagnostics.jsonl");
    let diagnostic_file = std::fs::File::create(&diagnostic_path).unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex",
        "store_paths":[dir.path().join("store")],"compiler_path":compiler,"diagnostic_timings":true});
    let mut client = Client::configured_stderr(dir.path(), config, Stdio::from(diagnostic_file));
    enable_display(&mut client);
    // Fixture's 5.6 MB line fits the runtime 8 MiB limit but reserialization of
    // its compact numbers exceeds the helper's actual 16 MiB complete-frame limit.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let diagnostics = std::fs::read_to_string(&diagnostic_path).unwrap();
        if diagnostics
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|v| {
                v["phase"] == "optional_output"
                    && v["kind"] == "display_candidate"
                    && v["outcome"] == "serialization_refused"
            })
        {
            break;
        }
        assert!(
            client.child.try_wait().unwrap().is_none(),
            "helper terminated"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "no full-size serialization refusal; reader={:?}; helper={} ",
            client.reader_snapshot(),
            bounded_diagnostics(&diagnostic_path)
        );
        thread::sleep(Duration::from_millis(2));
    }
    for event in client.output.try_iter() {
        assert_ne!(event["payload"]["kind"], "display_candidate");
        assert_ne!(event["payload"]["kind"], "failed", "{event}");
    }
    client.send("doc", "document", json!({"path":"main.tex"}));
    let doc = client.reply("doc")["payload"]["document"].clone();
    let log = std::fs::read_to_string(&diagnostic_path).unwrap();
    let profiles: Vec<Value> = log
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|event| event["phase"] == "display_transport")
        .collect();
    assert_eq!(
        profiles.len(),
        1,
        "unchanged profile repeated on idle/document polls"
    );
    let profile = &profiles[0]["profile"];
    assert!(profile["response_bytes"].as_u64().unwrap() > 5_000_000);
    for field in [
        "parse_ms",
        "decode_queue_wait_ms",
        "reader_delivery_wait_ms",
        "source_binding_ms",
    ] {
        let duration = profile[field].as_f64().unwrap();
        assert!(duration.is_finite() && duration >= 0.0);
    }
    client.send(
        "edit",
        "edit",
        json!({"path":"main.tex","expected_revision":doc["revision"],
        "expected_sha256":doc["source_sha256"],"text":"durable after oversized optional frame"}),
    );
    let response = client.reply("edit");
    assert_eq!(response["type"], "result");
    assert_eq!(response["payload"]["document"]["revision"], 2);
    drop(client);
    let mut client = Client::start(dir.path());
    client.send("doc", "document", json!({"path":"main.tex"}));
    assert_eq!(
        client.reply("doc")["payload"]["document"]["text"],
        "durable after oversized optional frame"
    );
}

#[test]
#[cfg(unix)]
fn stalled_optional_display_write_triggers_watchdog_with_no_source_loss() {
    let dir = tempfile::tempdir().unwrap();
    drop(Client::start(dir.path()));
    let compiler = display_failure_fixture(dir.path(), "large");
    let diagnostics = dir.path().join("optional-stall.jsonl");
    let config = dir.path().join("stall-config.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&json!({"session_id":"session1","project_id":"p",
        "entry_path":"main.tex","store_paths":[dir.path().join("store")],"compiler_path":compiler,
        "diagnostic_timings":true}))
        .unwrap(),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-preview-controller"))
        .arg(config)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(std::fs::File::create(&diagnostics).unwrap()))
        .spawn()
        .unwrap();
    let mut unread = BufReader::new(child.stdout.take().unwrap());
    let input = child.stdin.take();
    let (_sender, output) = mpsc::channel();
    let mut client = Client {
        child,
        input,
        output,
        reader_progress: Arc::new(Mutex::new(ReaderProgress {
            phase: "intentionally_unread",
            ..ReaderProgress::default()
        })),
        reader_thread: None,
    };
    let mut line = String::new();
    unread.read_line(&mut line).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&line).unwrap()["type"],
        "ready"
    );
    client.send(
        "enable",
        "configure_display_candidates",
        json!({"capability":"display-candidates-v1",
        "enabled":true,"renderer_support_confirmed":true}),
    );
    // Keep stdout open but unread: small required frames fit, optional 1 MiB does not.
    let deadline = std::time::Instant::now() + Duration::from_secs(6);
    loop {
        if let Some(status) = client.child.try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "optional writer did not terminate"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let log = std::fs::read_to_string(&diagnostics).unwrap();
    assert!(log
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .any(|v| v["phase"] == "optional_output"
            && v["kind"] == "display_candidate"
            && v["outcome"] == "admitted"));
    let records: Vec<Value> = log
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let timeout = records
        .iter()
        .find(|v| v["phase"] == "output_watchdog")
        .unwrap();
    assert_eq!(timeout["outcome"], "timeout");
    let sequence = timeout["sequence"].as_u64().unwrap();
    assert!(records.iter().any(|v| v["phase"] == "output_frame"
        && v["sequence"] == sequence
        && v["class"] == "optional"
        && v["outcome"] == "write_started"));
    assert!(!records.iter().any(|v| v["phase"] == "output_frame"
        && v["sequence"] == sequence
        && v["outcome"] == "write_finished"));
    assert!(!log.contains("α original"));
    drop(client);
    let mut reopened = Client::start(dir.path());
    reopened.send("doc", "document", json!({"path":"main.tex"}));
    assert_eq!(
        reopened.reply("doc")["payload"]["document"]["text"],
        "α original"
    );
}

// POSIX-only fixture, like the other `#[cfg(unix)]` tests in this file: the
// producer is a `#!/usr/bin/python3` script marked executable with `chmod 0700`
// and then spawned by path. Windows has no shebang dispatch and no executable
// permission bit, so the fixture cannot run there at all — `PermissionsExt` and
// `Permissions::from_mode` do not even exist off Unix. Gated rather than
// ported: the controller logic under test is platform-independent, but giving
// it a Windows producer needs a different fixture shape (a `.cmd` shim around
// the interpreter), which belongs in a deliberate test-harness change.
#[test]
#[cfg(unix)]
fn producer_reply_limit_is_applied_on_startup_and_restart() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let compiler = dir.path().join("record-cap.py");
    std::fs::write(
        &compiler,
        script(
            r#"import os, pathlib, sys
with pathlib.Path(__file__).with_suffix('.log').open('a') as f:
    f.write(os.environ['FLASHTEX_MAX_REPLY_BYTES'] + '\n')
for line in sys.stdin:
    pass
"#,
        ),
    )
    .unwrap();
    std::fs::set_permissions(&compiler, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut client = Client::with_compiler(dir.path(), Some(&compiler));
    let expected = std::env::var("FLASHTEX_MAX_REPLY_BYTES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .map_or(8 * 1024 * 1024 - 1, |v| v.min(8 * 1024 * 1024 - 1));
    for count in 1..=2 {
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            let log = std::fs::read_to_string(compiler.with_extension("log")).unwrap_or_default();
            if log.lines().count() == count {
                assert!(log.lines().all(|line| line == expected.to_string()));
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "child did not record launch cap"
            );
            thread::sleep(Duration::from_millis(5));
        }
        if count == 1 {
            client.send("restart-cap", "restart", json!({}));
            assert_eq!(client.reply("restart-cap")["type"], "result");
        }
    }
}

#[test]
fn offline_full_and_metadata_edits_ack_save_without_compile_identity() {
    let dir = tempfile::tempdir().unwrap();
    let mut client = Client::start(dir.path());
    for mode in ["full", "metadata"] {
        client.send("source", "document", json!({"path":"main.tex"}));
        let doc = client.reply("source")["payload"]["document"].clone();
        client.send("save", "edit", json!({"path":"main.tex", "expected_revision":doc["revision"], "expected_sha256":doc["source_sha256"], "text":mode, "response_mode":mode}));
        let response = client.reply("save");
        assert_eq!(response["type"], "result");
        let payload = &response["payload"];
        assert_eq!(payload.get("compile_request_id"), Some(&Value::Null));
        assert_eq!(payload.get("compile_revision"), Some(&Value::Null));
        assert!(payload["preview_error"].is_string());
        assert_eq!(
            payload["document"]["revision"].as_u64(),
            Some(doc["revision"].as_u64().unwrap() + 1)
        );
    }
}

// POSIX-only fixture (executable `#!/usr/bin/python3` producer); see the note
// on `producer_reply_limit_is_applied_on_startup_and_restart`.
#[test]
#[cfg(unix)]
fn full_and_metadata_edit_admissions_match_wire_previews() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let compiler = dir.path().join("correlation.py");
    std::fs::write(&compiler, script(r#"import json,sys
for line in sys.stdin:
 r=json.loads(line);p=r['payload']
 print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[]}}),flush=True)
"#)).unwrap();
    std::fs::set_permissions(&compiler, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut client = Client::with_compiler(dir.path(), Some(&compiler));
    loop {
        if client.output.recv_timeout(Duration::from_secs(3)).unwrap()["payload"]["kind"]
            == "preview"
        {
            break;
        }
    }
    client.send("extra", "compile", json!({}));
    assert_eq!(client.reply("extra")["type"], "result");
    for mode in ["full", "metadata"] {
        client.send("document", "document", json!({"path":"main.tex"}));
        let doc = client.reply("document")["payload"]["document"].clone();
        client.send("edit", "edit", json!({"path":"main.tex","expected_revision":doc["revision"],"expected_sha256":doc["source_sha256"],"text":mode,"response_mode":mode}));
        let ack = client.reply("edit");
        let payload = &ack["payload"];
        assert!(payload["preview_error"].is_null());
        assert!(payload["compile_request_id"].is_string());
        assert_ne!(payload["compile_revision"], payload["document"]["revision"]);
        loop {
            let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
            if event["payload"]["kind"] == "preview"
                && event["payload"]["request_id"] == payload["compile_request_id"]
            {
                assert_eq!(
                    event["payload"]["compile_revision"],
                    payload["compile_revision"]
                );
                break;
            }
        }
    }
}

// POSIX-only fixture (executable `#!/usr/bin/python3` producer); see the note
// on `producer_reply_limit_is_applied_on_startup_and_restart`.
#[test]
#[cfg(unix)]
fn grouped_retry_retains_command_identity_but_admits_current_source_compile() {
    use std::os::unix::fs::PermissionsExt;
    for mode in ["full", "metadata"] {
        let dir = tempfile::tempdir().unwrap();
        let compiler = dir.path().join("group-gated.py");
        std::fs::write(&compiler, script(r#"import json,sys,pathlib,time
root=pathlib.Path(__file__).parent
for line in sys.stdin:
 r=json.loads(line);p=r['payload'];(root/'started').touch()
 while not (root/'release').exists(): time.sleep(.001)
 print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[]}}),flush=True)
"#)).unwrap();
        std::fs::set_permissions(&compiler, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut client = Client::with_compiler(dir.path(), Some(&compiler));
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while !dir.path().join("started").exists() {
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(Duration::from_millis(2));
        }
        client.send("doc", "document", json!({"path":"main.tex"}));
        let doc = client.reply("doc")["payload"]["document"].clone();
        let request = json!({"path":"main.tex","response_mode":mode,"command":{"command_id":"permanent-group","expected_revision":1,"expected_sha256":doc["source_sha256"],"label":"Replace alpha","edits":[{"start_byte":0,"end_byte":2,"removed_text":"α","replacement":"β"}]}});
        client.send("group", "apply_group", request.clone());
        let first = client.reply("group");
        assert!(first["payload"]["compile_request_id"].is_string());
        let saved = &first["payload"]["history"]["document"];
        client.send("later", "edit", json!({"path":"main.tex","expected_revision":saved["revision"],"expected_sha256":saved["source_sha256"],"text":"later source"}));
        let later = client.reply("later");
        client.send("retry", "apply_group", request);
        let retry = client.reply("retry");
        let payload = &retry["payload"];
        assert_eq!(payload["history"]["command_revision"], 2);
        assert_eq!(payload["history"]["replayed_command"], true);
        assert_eq!(payload["history"]["document"]["revision"], 3);
        assert_eq!(
            payload["history"]["document"]["source_sha256"],
            later["payload"]["document"]["source_sha256"]
        );
        assert_ne!(
            payload["compile_request_id"],
            first["payload"]["compile_request_id"]
        );
        assert_ne!(
            payload["compile_request_id"],
            later["payload"]["compile_request_id"]
        );
        std::fs::write(dir.path().join("release"), "").unwrap();
        loop {
            let event = client.output.recv_timeout(Duration::from_secs(3)).unwrap();
            if event["payload"]["kind"] == "preview"
                && event["payload"]["request_id"] == payload["compile_request_id"]
            {
                assert_eq!(
                    event["payload"]["compile_revision"],
                    payload["compile_revision"]
                );
                break;
            }
        }
    }
}

/// Transport fixture only: records the launch argv and each request's
/// `payload.project_root` (or `<absent>`) next to itself, replies `ok`.
#[cfg(unix)]
fn root_recording_producer(dir: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let compiler = dir.join("root-recorder.py");
    std::fs::write(&compiler, script(r#"import json,sys,pathlib
log=pathlib.Path(__file__).with_name('seen.jsonl')
for line in sys.stdin:
 r=json.loads(line);p=r['payload']
 with open(log,'a') as f: f.write(json.dumps({'argv':sys.argv[1:],'project_root':p.get('project_root','<absent>')})+'\n')
 print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[]}}),flush=True)
"#)).unwrap();
    std::fs::set_permissions(&compiler, std::fs::Permissions::from_mode(0o700)).unwrap();
    compiler
}
#[cfg(unix)]
fn next_preview(client: &Client) -> Value {
    loop {
        let event = client.output.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_ne!(event["type"], "error", "{event}");
        assert_ne!(event["payload"]["kind"], "failed", "{event}");
        if event["payload"]["kind"] == "preview" {
            return event;
        }
    }
}
#[cfg(unix)]
fn recorded(dir: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("seen.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
#[cfg(unix)]
fn file_backed_helper_forwards_canonical_project_root_at_launch_and_per_request() {
    let project = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("main.tex"), "Hello.").unwrap();
    let compiler = root_recording_producer(tools.path());
    // tempdir may be spelled through a symlink (macOS /var -> /private/var);
    // the helper forwards the canonical directory FileProject opened.
    let expected = project.path().canonicalize().unwrap();
    let expected = expected.to_str().unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex",
        "project_root":project.path(),"private_ledger_root":private.path(),"compiler_path":compiler});
    let mut client = Client::configured(tools.path(), config);
    next_preview(&client);
    client.send("restart", "restart", json!({}));
    assert_eq!(client.reply("restart")["payload"]["submitted"], true);
    next_preview(&client);
    let seen = recorded(tools.path());
    assert!(seen.len() >= 2, "{seen:?}");
    for record in seen {
        assert_eq!(record["argv"], json!(["--project-root", expected]));
        assert_eq!(record["project_root"], expected);
    }
}

#[test]
#[cfg(unix)]
fn store_backed_helper_launch_and_requests_are_unchanged_without_project_root() {
    let dir = tempfile::tempdir().unwrap();
    let compiler = root_recording_producer(dir.path());
    let client = Client::with_compiler(dir.path(), Some(&compiler));
    next_preview(&client);
    let seen = recorded(dir.path());
    assert!(!seen.is_empty());
    for record in seen {
        assert_eq!(record["argv"], json!([]));
        assert_eq!(record["project_root"], "<absent>");
    }
}

/// Real producer round trip: `FLASHTEX_TEST_RENDER=<path to flashtex-render>`.
/// A file-backed `\includegraphics` document reaches the helper's display
/// candidate as an `image` item with no `image_unavailable` diagnostic.
#[test]
#[cfg(unix)]
#[ignore = "requires FLASHTEX_TEST_RENDER pointing at a built flashtex-render"]
fn real_render_producer_resolves_includegraphics_through_helper_route() {
    let render = std::env::var("FLASHTEX_TEST_RENDER").unwrap();
    let fixtures = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../render-pipeline/fixtures/floats"
    );
    let project = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let tex = std::fs::read_to_string(format!("{fixtures}/01-here.tex")).unwrap();
    assert!(tex.contains("\\includegraphics{images/red-72.png}"));
    std::fs::write(project.path().join("main.tex"), tex).unwrap();
    std::fs::create_dir(project.path().join("images")).unwrap();
    std::fs::copy(
        format!("{fixtures}/images/red-72.png"),
        project.path().join("images/red-72.png"),
    )
    .unwrap();
    let config = json!({"session_id":"session1","project_id":"p","entry_path":"main.tex",
        "project_root":project.path(),"private_ledger_root":private.path(),"compiler_path":render});
    let mut client = Client::configured(tools.path(), config);
    client.send(
        "enable",
        "configure_display_candidates",
        json!({"capability":"display-candidates-v1","enabled":true,"renderer_support_confirmed":true}),
    );
    assert_eq!(client.reply("enable")["payload"]["enabled"], true);
    client.send(
        "layout",
        "configure_layout",
        json!({"layout_capabilities":["display-list-v2","display-list-v2-images"],"renderer_support_confirmed":true}),
    );
    assert_eq!(client.reply("layout")["type"], "result");
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let mut accepted_images = false;
    loop {
        let event = client
            .output
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        assert_ne!(event["type"], "error", "{event}");
        let p = &event["payload"];
        assert_ne!(p["kind"], "failed", "{event}");
        let text = event.to_string();
        if p["kind"] == "preview" {
            let result = &p["result"]["payload"];
            if result["layout_capabilities"]
                .as_array()
                .is_some_and(|caps| caps.iter().any(|c| c == "display-list-v2-images"))
            {
                assert_ne!(result["status"], "failed", "{result}");
                assert!(!text.contains("image_unavailable"), "{result}");
                accepted_images = true;
            }
        }
        if p["kind"] == "display_candidate" && text.contains("\"kind\":\"image\"") {
            assert!(
                accepted_images,
                "image candidate before its accepted preview"
            );
            assert!(!text.contains("image_unavailable"));
            assert!(text.contains("images/red-72.png"));
            eprintln!(
                "helper route image item: request={} revision={} bytes={}",
                p["request_id"],
                p["compile_revision"],
                text.len()
            );
            break;
        }
    }
}
