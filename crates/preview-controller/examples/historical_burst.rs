//! Real original compiler burst replay of internal historical display. No native activation.
use flashtex_document_runtime::{Document as InputDocument, Event, Limits, Request, Session};
use flashtex_edit_ledger::{Document, Store};
use flashtex_preview_controller::{Controller, Update};
use serde_json::json;
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let compiler = args.get(1).expect("compiler path");
    let enabled = args.get(2).is_some_and(|value| value == "on");
    let size: usize = args.get(3).map(|n| n.parse().unwrap()).unwrap_or(50000);
    assert!((100..=500000).contains(&size));
    let dir = tempfile::tempdir().unwrap();
    let base = format!(
        "\\documentclass{{article}}\n\\begin{{document}}\n{}\\end{{document}}\n",
        "Alpha beta gamma delta.\n".repeat(size / 23)
    );
    let mut store = Store::open(dir.path().join("source")).unwrap();
    store
        .initialize(Document::new("p".into(), "main.tex".into(), 1, base.clone()).unwrap())
        .unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store],
        Command::new(compiler),
        Limits {
            max_frame: 12 * 1024 * 1024,
            ..Limits::default()
        },
    )
    .unwrap();
    controller.configure_completed_snapshots(enabled).unwrap();
    controller.compile_current().unwrap();
    let startup = Instant::now();
    loop {
        assert!(startup.elapsed() < Duration::from_secs(15));
        if controller
            .poll()
            .into_iter()
            .any(|event| matches!(event, Update::Preview(_)))
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let start = Instant::now();
    let mut sent = Vec::new();
    let mut displays = Vec::new();
    let mut final_result = None;
    let mut final_source = base.clone();
    while sent.len() < 20 || final_result.is_none() {
        assert!(start.elapsed() < Duration::from_secs(30), "burst deadline");
        if sent.len() < 20 && start.elapsed() >= Duration::from_millis(sent.len() as u64 * 30) {
            let index = sent.len();
            sent.push(start.elapsed().as_secs_f64() * 1000.0);
            final_source = base.replacen("Alpha", &format!("Edit{index:03}"), 1);
            let previous = controller.document("main.tex").unwrap().clone();
            let saved = controller
                .replace_document(
                    "main.tex",
                    previous.revision,
                    &previous.source_sha256,
                    final_source.clone(),
                )
                .unwrap();
            assert!(saved.preview_error.is_none());
            assert_eq!(saved.document.text, final_source);
        }
        for update in controller.poll() {
            if let Update::Preview(preview) = update {
                assert!(controller.is_current_preview(&preview));
                assert_eq!(
                    preview.source_versions.documents["main.tex"],
                    sent.len() as u64 + 1
                );
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                displays.push(json!({"kind":"current","revision":preview.compile_revision,"received_ms":elapsed}));
                if sent.len() == 20 {
                    final_result = Some(preview.result);
                }
            }
        }
        if let Some(historical) = controller.take_completed_snapshot() {
            assert!(enabled && controller.claim_historical_display(&historical));
            assert_eq!(
                historical.source_versions().documents["main.tex"],
                historical.compile_revision()
            );
            assert!(historical.compile_revision() < sent.len() as u64 + 1);
            displays.push(json!({"kind":"historical","revision":historical.compile_revision(),
                "current_revision":sent.len() as u64+1,"received_ms":start.elapsed().as_secs_f64()*1000.0}));
        }
        thread::sleep(Duration::from_millis(1));
    }
    let final_ms = displays.last().unwrap()["received_ms"].as_f64().unwrap();
    let result = final_result.unwrap();
    let mut clean = Session::spawn(
        compiler,
        Limits {
            max_frame: 12 * 1024 * 1024,
            ..Limits::default()
        },
    )
    .unwrap();
    clean
        .submit(Request {
            id: result["id"].as_str().unwrap().into(),
            project_id: "p".into(),
            revision: result["payload"]["revision"].as_u64().unwrap(),
            entry_path: "main.tex".into(),
            documents: vec![InputDocument {
                path: "main.tex".into(),
                text: final_source.clone(),
            }],
        })
        .unwrap();
    let clean_started = Instant::now();
    let clean_result = loop {
        assert!(
            clean_started.elapsed() < Duration::from_secs(15),
            "clean result deadline"
        );
        let mut completed = None;
        for event in clean.poll() {
            match event {
                Event::Preview { result, .. } => completed = Some(result),
                other => panic!("unexpected clean event: {other:?}"),
            }
        }
        if let Some(result) = completed {
            break result;
        }
        thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(clean_result, result);
    drop(controller);
    let reopened = Store::open(dir.path().join("source")).unwrap();
    assert_eq!(reopened.document().unwrap().unwrap().text, final_source);
    println!(
        "{}",
        json!({"enabled":enabled,"source_bytes":final_source.len(),"edits":20,
        "intended_interval_ms":30,"actual_send_ms":sent,"displays":displays,
        "final_after_last_send_ms":final_ms-sent.last().unwrap(),"exact_clean_final":true,
        "exact_durable_reopen":true,"native_paint_measured":false,
        "measurement":"single controller worker; edits and poll serialized; clean comparison after burst"})
    );
}
