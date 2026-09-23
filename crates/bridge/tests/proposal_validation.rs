use flashtex_bridge::{
    digest,
    validation::{CompilerValidator, ProposedCompilation},
    Document, PreparedEdit,
};
use serde_json::{json, Value};
use std::{path::PathBuf, time::Duration};
fn fixtures() -> (Vec<Document>, PreparedEdit) {
    let text = "α source";
    (
        vec![Document {
            project_id: "p".into(),
            path: "main.tex".into(),
            revision: 3,
            text: text.into(),
        }],
        PreparedEdit {
            capture_id: "capture".into(),
            edit_id: "edit".into(),
            project_id: "p".into(),
            path: "main.tex".into(),
            expected_revision: 3,
            start_byte: 3,
            end_byte: 9,
            removed_text: "source".into(),
            replacement: "$x$".into(),
            document_before_sha256: digest(text.as_bytes()),
            wrap: None,
        },
    )
}
fn reply() -> Value {
    json!({"protocol_version":1,"id":"test","type":"compile_result","payload":{"project_id":"p","revision":3,"status":"recovered","pages":[],"diagnostics":[{"severity":"warning","message":"subset only","source":{"path":"main.tex","start_byte":3,"end_byte":6},"recovery":"partial"}],"pdf_path":null}})
}
#[test]
fn hypothetical_snapshot_preserves_actual_source_and_checks_returned_spans() {
    let (docs, edit) = fixtures();
    let req = ProposedCompilation::new("test", "main.tex", &docs, &edit).unwrap();
    let wire: Value = serde_json::from_slice(&req.request_bytes().unwrap()).unwrap();
    assert_eq!(wire["payload"]["documents"][0]["text"], "α $x$");
    assert_eq!(docs[0].text, "α source");
    let evidence = req
        .response(&serde_json::to_vec(&reply()).unwrap())
        .unwrap();
    assert_eq!(evidence.status, "recovered");
    assert!(evidence.source_is_hypothetical);
    assert_eq!(
        evidence.snapshots[0].source_sha256,
        digest("α $x$".as_bytes())
    );
}
#[test]
fn stale_or_mismatched_edit_is_not_compiled() {
    let (docs, mut edit) = fixtures();
    edit.expected_revision = 4;
    assert!(ProposedCompilation::new("test", "main.tex", &docs, &edit).is_err());
    edit.expected_revision = 3;
    edit.removed_text = "other".into();
    assert!(ProposedCompilation::new("test", "main.tex", &docs, &edit).is_err());
    edit.removed_text = "source".into();
    edit.start_byte = 1;
    assert!(ProposedCompilation::new("test", "main.tex", &docs, &edit).is_err());
}
#[test]
fn response_correlation_and_unicode_ranges_are_required() {
    let (docs, edit) = fixtures();
    let req = ProposedCompilation::new("test", "main.tex", &docs, &edit).unwrap();
    for which in 0..4 {
        let mut value = reply();
        match which {
            0 => value["id"] = json!("stale"),
            1 => value["payload"]["revision"] = json!(2),
            2 => value["payload"]["diagnostics"][0]["source"]["start_byte"] = json!(1),
            _ => value["payload"]["project_id"] = json!("other"),
        };
        assert!(req.response(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    assert!(req.response(b"not json").is_err());
}
#[cfg(unix)]
fn fake(body: &str) -> (tempfile::TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("compiler");
    std::fs::write(&path, format!("#!/usr/bin/env python3\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    (dir, path)
}
#[test]
#[cfg(unix)]
fn compiler_process_success_and_timeout_leave_editor_unchanged() {
    let (docs, edit) = fixtures();
    let req = ProposedCompilation::new("test", "main.tex", &docs, &edit).unwrap();
    let (_dir, path) = fake(&format!(
        "print({:?})",
        serde_json::to_string(&reply()).unwrap()
    ));
    let compiler = CompilerValidator {
        executable: path,
        timeout: Duration::from_secs(2),
    };
    assert_eq!(compiler.validate(&req).unwrap().status, "recovered");
    let (_dir, path) = fake("import time\ntime.sleep(10)");
    let compiler = CompilerValidator {
        executable: path,
        timeout: Duration::from_millis(50),
    };
    assert_eq!(
        compiler.validate(&req).unwrap_err().code,
        "validation_timeout"
    );
    assert_eq!(docs[0].text, "α source");
}
#[test]
#[cfg(unix)]
fn compiler_output_overflow_and_bad_status_are_rejected() {
    let (docs, edit) = fixtures();
    let req = ProposedCompilation::new("test", "main.tex", &docs, &edit).unwrap();
    for body in [
        "import sys\nsys.stderr.write('x'*70000)",
        "raise SystemExit(3)",
        "print('bad')",
    ] {
        let (_dir, path) = fake(body);
        let compiler = CompilerValidator {
            executable: path,
            timeout: Duration::from_secs(2),
        };
        assert!(compiler.validate(&req).is_err());
    }
}

#[test]
#[ignore = "requires explicit original FlashTeX compiler binary"]
fn actual_original_compiler_returns_review_evidence() {
    let path =
        std::env::var_os("FLASHTEX_TEST_COMPILER").expect("set exact original compiler executable");
    let (docs, edit) = fixtures();
    let request = ProposedCompilation::new("test", "main.tex", &docs, &edit).unwrap();
    let evidence = CompilerValidator {
        executable: path.into(),
        timeout: Duration::from_secs(5),
    }
    .validate(&request)
    .unwrap();
    assert_eq!(evidence.project_id, "p");
    assert_eq!(evidence.revision, 3);
    assert!(!evidence.pages.is_empty());
    assert_eq!(docs[0].text, "α source");
}

struct CaptureFixtureConverter;
impl flashtex_bridge::Converter for CaptureFixtureConverter {
    fn convert(
        &self,
        _: &flashtex_bridge::CaptureSubmit,
        _: &flashtex_bridge::Context,
    ) -> flashtex_bridge::Result<flashtex_bridge::Proposal> {
        Ok(flashtex_bridge::Proposal {
            latex: "$x$".into(),
            ambiguities: vec![],
            required_dependencies: vec![],
        })
    }
}
fn seed_capture(path: &std::path::Path) -> flashtex_bridge::Bridge {
    use flashtex_bridge::{store::Store, Bridge, CaptureSubmit};
    let mut bridge = Bridge::new(Store::open(path).unwrap());
    let (docs, _) = fixtures();
    bridge.open_document(docs[0].clone()).unwrap();
    bridge
        .pin("fixture-anchor-1", "p", "main.tex", 3, 3, 9)
        .unwrap();
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../protocol/fixtures/capture-submission.json"
    ))
    .unwrap();
    let mut cap: CaptureSubmit = serde_json::from_value(fixture["payload"].clone()).unwrap();
    cap.base_revision = 3;
    bridge.receive(cap).unwrap();
    bridge
        .convert("fixture-capture-1", vec![], &CaptureFixtureConverter)
        .unwrap();
    bridge
}
#[test]
#[cfg(unix)]
fn capture_validation_does_not_issue_edit_and_refuses_stale_context() {
    let journal = tempfile::tempdir().unwrap();
    let mut bridge = seed_capture(journal.path());
    let(_dir,path)=fake("import json,sys\nr=json.loads(sys.stdin.readline())\nprint(json.dumps({'protocol_version':1,'id':r['id'],'type':'compile_result','payload':{'project_id':r['payload']['project_id'],'revision':r['payload']['revision'],'status':'ok','pages':[],'diagnostics':[]}}))");
    let compiler = CompilerValidator {
        executable: path,
        timeout: Duration::from_secs(2),
    };
    let before = bridge.store.require("fixture-capture-1").unwrap();
    bridge
        .validate_capture("request", "fixture-capture-1", 3, "main.tex", &compiler)
        .unwrap();
    assert_eq!(bridge.store.require("fixture-capture-1").unwrap(), before);
    assert_eq!(bridge.document("p", "main.tex").unwrap().text, "α source");
    bridge
        .edit(&flashtex_bridge::EditRequest {
            project_id: "p".into(),
            path: "main.tex".into(),
            base_revision: 3,
            revision: 4,
            start_byte: 0,
            end_byte: 0,
            replacement: "prefix ".into(),
        })
        .unwrap();
    assert_eq!(
        bridge
            .validate_capture("next", "fixture-capture-1", 4, "main.tex", &compiler)
            .unwrap_err()
            .code,
        "proposal_context_stale"
    );
}
#[test]
#[cfg(unix)]
fn real_bridge_cli_exposes_validation_without_preparing_source_edit() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let journal = tempfile::tempdir().unwrap();
    drop(seed_capture(journal.path()));
    let(_dir,path)=fake("import json,sys\nr=json.loads(sys.stdin.readline())\nprint(json.dumps({'protocol_version':1,'id':r['id'],'type':'compile_result','payload':{'project_id':r['payload']['project_id'],'revision':r['payload']['revision'],'status':'ok','pages':[],'diagnostics':[]}}))");
    let payloads = [
        (
            "document_open",
            json!({"project_id":"p","path":"main.tex","revision":3,"text":"α source"}),
        ),
        (
            "destination_pin",
            json!({"destination_id":"fixture-anchor-1","project_id":"p","path":"main.tex","revision":3,"start_byte":3,"end_byte":9}),
        ),
        (
            "capture_validate",
            json!({"capture_id":"fixture-capture-1","expected_revision":3}),
        ),
        ("capture_status", json!({"capture_id":"fixture-capture-1"})),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-bridge"))
        .arg("--store")
        .arg(journal.path())
        .arg("--compiler")
        .arg(path)
        .args(["--compiler-entry", "main.tex"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let input=payloads.iter().enumerate().map(|(i,(kind,payload))|format!("{}\n",json!({"protocol_version":1,"id":format!("request-{i}"),"type":kind,"payload":payload}))).collect::<String>();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let replies: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(replies[2]["type"], "capture_validation");
    assert_eq!(replies[2]["id"], "request-2");
    assert_eq!(replies[2]["payload"]["validation"]["status"], "ok");
    assert!(replies[3]["payload"]["prepared"].is_null());
    assert!(replies[3]["payload"]["applied"].is_null());
}
