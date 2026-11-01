//! One-process, one-frame comparison. Run raw/value separately with identical fixtures.
use flashtex_document_runtime::{Document, Event, Limits, Request, Session};
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 4, "mode request.json responses.jsonl");
    let r: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
    let p = &r["payload"];
    let request = Request {
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
    };
    let mut command = Command::new("python3");
    command.arg("-c").arg("import sys; sys.stdin.readline(); sys.stdout.buffer.write(open(sys.argv[1],'rb').read()); sys.stdout.flush(); sys.stdin.read()").arg(&args[3]);
    let raw = args[1] == "raw";
    let mut session = if raw {
        Session::spawn_command_raw_display_prototype(command, Limits::default())
    } else {
        Session::spawn_command(command, Limits::default())
    }
    .unwrap();
    session.set_display_candidates_enabled(true).unwrap();
    session
        .submit_with_capabilities(request, vec!["display-list-v2".into()])
        .unwrap();
    let start = Instant::now();
    loop {
        for event in session.poll() {
            if let Event::Failed { reason, .. } = event {
                panic!("{reason}");
            }
        }
        let result = if raw {
            session
                .take_current_raw_display_candidate()
                .map(|c| (c.into_raw(), None))
        } else {
            session.take_current_display_candidate().map(|c| {
                (
                    serde_json::value::RawValue::from_string("null".into()).unwrap(),
                    Some(c.into_envelope()),
                )
            })
        };
        if let Some((raw_value, value)) = result {
            let memory = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
            let hwm = memory
                .lines()
                .find(|line| line.starts_with("VmHWM:"))
                .unwrap_or("unavailable")
                .to_string();
            let profile = serde_json::to_value(session.last_display_profile().unwrap()).unwrap();
            let expected_bytes = std::fs::read(&args[3]).unwrap();
            let expected = expected_bytes.split(|b| *b == b'\n').nth(1).unwrap();
            let expected_value: serde_json::Value = serde_json::from_slice(expected).unwrap();
            let equal = if let Some(value) = value {
                value == expected_value
            } else {
                assert_eq!(raw_value.get().as_bytes(), expected);
                serde_json::from_str::<serde_json::Value>(raw_value.get()).unwrap()
                    == expected_value
            };
            assert!(equal);
            println!(
                "{}",
                serde_json::json!({"mode":args[1],"profile":profile,"v1_profile":session.last_profile(),"process_peak_before_verification":hwm,"semantically_equal":equal,"raw_exact":raw,"input_frame_sha256":flashtex_project_files::sha256_hex(expected),"scope":"isolated replay of producer-derived normalized fixture; no native timing"})
            );
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(10));
        thread::sleep(Duration::from_millis(1));
    }
}
