//! Replay published helper/producer wire bytes; never substitutes generated producer output.
use flashtex_document_runtime::{Document, Event, Limits, Request, Session};
use serde_json::Value;
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};
fn request(record: &Value) -> Request {
    let r = &record["request"];
    let p = &r["payload"];
    Request {
        id: r["id"].as_str().unwrap().into(),
        project_id: p["project_id"].as_str().unwrap().into(),
        revision: p["revision"].as_u64().unwrap(),
        entry_path: p["entry_path"].as_str().unwrap().into(),
        documents: p["documents"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| Document {
                path: d["path"].as_str().unwrap().into(),
                text: d["text"].as_str().unwrap().into(),
            })
            .collect(),
    }
}
fn submit(s: &mut Session, record: &Value) {
    let caps: Vec<String> = record["request"]["payload"]["layout_capabilities"]
        .as_array()
        .map(|a| a.iter().map(|c| c.as_str().unwrap().into()).collect())
        .unwrap_or_default();
    if !caps.is_empty() {
        s.set_display_candidates_enabled(true).unwrap();
    }
    s.submit_with_capabilities(request(record), caps).unwrap();
}
fn collect(s: &mut Session, record: &Value) -> Value {
    let id = record["request"]["id"].as_str().unwrap();
    let lines: Vec<Value> = record["response_wire"]
        .as_str()
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let expected_raw = record["response_wire"].as_str().unwrap().lines().nth(1);
    let start = Instant::now();
    let mut preview = None;
    let mut raw = None;
    let mut stale = vec![];
    loop {
        for event in s.poll() {
            match event {
                Event::Preview {
                    id: seen, result, ..
                } => {
                    assert_eq!(seen, id, "stale preview delivered");
                    preview = Some(result);
                }
                Event::Failed { reason, .. } => panic!("{reason}"),
                Event::Stale { id, .. } => stale.push(id),
                _ => {}
            }
        }
        if let Some(candidate) = s.take_current_raw_display_candidate() {
            assert_eq!(candidate.request_id(), id);
            assert_eq!(candidate.sources().len(), 3);
            assert!(candidate
                .sources()
                .iter()
                .all(|d| d.revision == request(record).revision));
            raw = Some(candidate.into_raw().get().to_owned());
        }
        if preview.is_some() && (expected_raw.is_none() || raw.is_some()) {
            break;
        }
        assert!(s.is_alive() && start.elapsed() < Duration::from_secs(10));
        thread::yield_now();
    }
    assert_eq!(preview.as_ref().unwrap(), &lines[0]);
    assert_eq!(raw.as_deref(), expected_raw);
    serde_json::json!({"request_id":id,"v1_equal":true,"raw_exact":raw.is_some(),"source_count":3,"stale_ids":stale})
}
#[test]
#[ignore = "requires verified published helper capture config; no real compiler execution"]
fn published_multidocument_helper_bytes_preserve_runtime_binding_and_supersession() {
    let path = std::env::var("FLASHTEX_HELPER_CAPTURE_CONFIG").unwrap();
    let config: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let mut evidence = vec![];
    for records in config["sessions"].as_array().unwrap() {
        let mut command = Command::new("python3");
        command
            .arg("-c")
            .arg(
                r#"import sys,json
c=json.load(open(sys.argv[1]));rows={r['request']['id']:r for s in c['sessions'] for r in s}
for line in sys.stdin.buffer:
 r=rows[json.loads(line)['id']]
 assert line==r['request_wire'].encode(),'runtime request bytes changed'
 sys.stdout.buffer.write(r['response_wire'].encode());sys.stdout.buffer.flush()
"#,
            )
            .arg(&path);
        let mut session =
            Session::spawn_command_raw_display_prototype(command, Limits::default()).unwrap();
        let rows = records.as_array().unwrap();
        assert!(rows[0]["request"]["payload"]
            .get("layout_capabilities")
            .is_none());
        for record in rows {
            submit(&mut session, record);
            if record["request"]["id"] == "preview-5" {
                continue;
            }
            let result = collect(&mut session, record);
            if record["request"]["id"] == "preview-6" {
                assert!(result["stale_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|id| id == "preview-5"));
            }
            evidence.push(result);
        }
    }
    if let Ok(path) = std::env::var("FLASHTEX_HELPER_RUNTIME_EVIDENCE") {
        std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
