use flashtex_document_runtime::{Event, Limits};
use flashtex_edit_ledger::{Document, Store};
use flashtex_preview_controller::{Controller, Update};
use flashtex_project_index::Category;
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};
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
fn command(dir: &std::path::Path, body: &str) -> Command {
    let path = dir.join("compiler.py");
    std::fs::write(&path, body).unwrap();
    let mut command = Command::new(python3());
    command.arg(path);
    command
}
const ECHO: &str = "import json,sys,time\nfor line in sys.stdin:\n r=json.loads(line);p=r['payload'];time.sleep(.02)\n print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[]}}),flush=True)\n";
fn store(dir: &std::path::Path) -> Store {
    let mut store = Store::open(dir.join("source")).unwrap();
    if store.document().unwrap().is_none() {
        store
            .initialize(
                Document::new("p".into(), "main.tex".into(), 1, "α \\label{old}".into()).unwrap(),
            )
            .unwrap();
    }
    store
}
fn wait(controller: &mut Controller, predicate: impl Fn(&[Update]) -> bool) -> Vec<Update> {
    let start = Instant::now();
    let mut events = Vec::new();
    while start.elapsed() < Duration::from_secs(3) {
        events.extend(controller.poll());
        if predicate(&events) {
            return events;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("controller timed out: {events:?}");
}
#[test]
fn edit_persists_indexes_and_only_previews_current_revision() {
    let dir = tempfile::tempdir().unwrap();
    let source = store(dir.path());
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![source],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    controller.compile_current().unwrap();
    let before = controller.document("main.tex").unwrap().clone();
    let outcome = controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "β \\label{new}".into(),
        )
        .unwrap();
    assert!(outcome.preview_error.is_none());
    let snapshot = controller.index().snapshot();
    assert_eq!(
        controller
            .index()
            .definitions(&snapshot, Category::Label, "new")
            .unwrap()
            .len(),
        1
    );
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let previews: Vec<_> = events
        .iter()
        .filter_map(|event| {
            if let Update::Preview(preview) = event {
                Some(preview)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(previews.len(), 1);
    assert!(previews[0].controller_total_ms >= previews[0].runtime_total_ms);
    assert!(previews[0].controller_total_ms >= outcome.save_and_submit_ms);
    assert_eq!(previews[0].source_versions.documents["main.tex"], 2);
    drop(controller);
    assert_eq!(
        store(dir.path()).document().unwrap().unwrap().text,
        "β \\label{new}"
    );
}
#[test]
fn compiler_failure_does_not_lose_typing_and_restart_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), "raise SystemExit(3)"),
        Limits::default(),
    )
    .unwrap();
    controller.compile_current().unwrap();
    wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Runtime(Event::Failed { .. })))
    });
    let before = controller.document("main.tex").unwrap().clone();
    let saved = controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "saved despite crash".into(),
        )
        .unwrap();
    assert!(saved.preview_error.is_some());
    assert_eq!(
        controller.document("main.tex").unwrap().text,
        "saved despite crash"
    );
    controller
        .restart(command(dir.path(), ECHO), Limits::default())
        .unwrap();
    wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
}
#[test]
fn close_never_publishes_inflight_preview_or_accepts_typing() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    controller.compile_current().unwrap();
    controller.close().unwrap();
    thread::sleep(Duration::from_millis(100));
    assert!(!controller
        .poll()
        .iter()
        .any(|event| matches!(event, Update::Preview(_))));
    let before = controller.document("main.tex").unwrap().clone();
    assert!(controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "late".into()
        )
        .is_err());
}
#[test]
#[ignore = "requires explicit original compiler"]
fn original_compiler_renders_durable_updated_source() {
    let binary = std::env::var_os("FLASHTEX_TEST_COMPILER").expect("set original compiler path");
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        Command::new(binary),
        Limits::default(),
    )
    .unwrap();
    let before = controller.document("main.tex").unwrap().clone();
    let outcome = controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "Hello persistent preview".into(),
        )
        .unwrap();
    assert!(outcome.preview_error.is_none());
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let preview = events
        .into_iter()
        .find_map(|event| {
            if let Update::Preview(p) = event {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(preview.source_versions.documents["main.tex"], 2);
    assert_eq!(preview.result["payload"]["status"], "ok");
    assert!(!preview.result["payload"]["pages"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn retained_preview_is_invalid_after_new_edit_and_after_close() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    controller.compile_current().unwrap();
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let preview = events
        .into_iter()
        .find_map(|event| {
            if let Update::Preview(p) = event {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert!(controller.is_current_preview(&preview));
    let before = controller.document("main.tex").unwrap().clone();
    controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "new source".into(),
        )
        .unwrap();
    assert!(!controller.is_current_preview(&preview));
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let current = events
        .into_iter()
        .find_map(|event| {
            if let Update::Preview(p) = event {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert!(controller.is_current_preview(&current));
    controller.close().unwrap();
    assert!(!controller.is_current_preview(&current));
}

#[test]
fn stale_typing_cannot_overwrite_durable_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    let before = controller.document("main.tex").unwrap().clone();
    controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "saved".into(),
        )
        .unwrap();
    assert!(controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "stale overwrite".into()
        )
        .is_err());
    assert_eq!(controller.document("main.tex").unwrap().text, "saved");
    drop(controller);
    let recovered = store(dir.path());
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![recovered],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    controller.compile_current().unwrap();
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let preview = events
        .into_iter()
        .find_map(|event| {
            if let Update::Preview(p) = event {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(preview.source_versions.documents["main.tex"], 2);
}

#[test]
fn approved_insertion_is_durable_and_retry_never_inserts_twice() {
    use flashtex_edit_ledger::PreparedEdit;
    use flashtex_preview_controller::ApprovedEdit;
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    let before = controller.document("main.tex").unwrap().clone();
    let edit = PreparedEdit {
        capture_id: "capture1".into(),
        edit_id: "edit1".into(),
        project_id: "p".into(),
        path: "main.tex".into(),
        expected_revision: before.revision,
        start_byte: 0,
        end_byte: 2,
        removed_text: "α".into(),
        replacement: "β".into(),
        document_before_sha256: before.source_sha256,
    };
    let applied = controller
        .apply_reviewed(ApprovedEdit::from_explicit_user_approval(edit.clone()))
        .unwrap();
    assert!(applied.source.preview_error.is_none());
    assert_eq!(applied.receipt.new_revision, 2);
    assert_eq!(controller.recovery("main.tex").unwrap().len(), 1);
    drop(controller);
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    let retry = controller
        .apply_reviewed(ApprovedEdit::from_explicit_user_approval(edit))
        .unwrap();
    assert_eq!(retry.receipt, applied.receipt);
    assert_eq!(retry.source.document.revision, 2);
    assert_eq!(retry.source.document.text, "β \\label{old}");
    assert!(retry.source.preview_error.is_none());
    controller
        .confirm_receipt("main.tex", &retry.receipt)
        .unwrap();
    assert!(controller.recovery("main.tex").unwrap().is_empty());
}

#[test]
fn approved_edit_conflict_cannot_modify_source() {
    use flashtex_edit_ledger::PreparedEdit;
    use flashtex_preview_controller::ApprovedEdit;
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    let before = controller.document("main.tex").unwrap().clone();
    let edit = PreparedEdit {
        capture_id: "capture1".into(),
        edit_id: "edit1".into(),
        project_id: "p".into(),
        path: "main.tex".into(),
        expected_revision: before.revision,
        start_byte: 0,
        end_byte: 1,
        removed_text: "α".into(),
        replacement: "β".into(),
        document_before_sha256: before.source_sha256.clone(),
    };
    assert!(controller
        .apply_reviewed(ApprovedEdit::from_explicit_user_approval(edit))
        .is_err());
    assert_eq!(controller.document("main.tex").unwrap(), &before);
    assert!(controller.recovery("main.tex").unwrap().is_empty());
}

#[test]
fn included_file_edit_updates_complete_source_versions_and_navigation() {
    let dir = tempfile::tempdir().unwrap();
    let mut chapter = Store::open(dir.path().join("chapter-store")).unwrap();
    chapter
        .initialize(
            Document::new(
                "p".into(),
                "chapter.tex".into(),
                3,
                "\\label{chapter}".into(),
            )
            .unwrap(),
        )
        .unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path()), chapter],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    let before = controller.document("chapter.tex").unwrap().clone();
    let outcome = controller
        .replace_document(
            "chapter.tex",
            before.revision,
            &before.source_sha256,
            "\\label{revised}".into(),
        )
        .unwrap();
    assert!(outcome.preview_error.is_none());
    let snapshot = controller.index().snapshot();
    let definitions = controller
        .index()
        .definitions(&snapshot, Category::Label, "revised")
        .unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].source.file, "chapter.tex");
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let preview = events
        .into_iter()
        .find_map(|event| {
            if let Update::Preview(p) = event {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(preview.source_versions.documents["main.tex"], 1);
    assert_eq!(preview.source_versions.documents["chapter.tex"], 4);
}

#[test]
fn editor_operates_without_compiler_then_attaches_and_renders() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller =
        Controller::open_without_compiler("p".into(), "main.tex".into(), vec![store(dir.path())])
            .unwrap();
    let before = controller.document("main.tex").unwrap().clone();
    let saved = controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "\\label{offline}".into(),
        )
        .unwrap();
    assert!(saved
        .preview_error
        .as_ref()
        .unwrap()
        .contains("compiler unavailable"));
    assert!(controller.poll().is_empty());
    assert_eq!(
        controller
            .index()
            .definitions(&controller.index().snapshot(), Category::Label, "offline")
            .unwrap()
            .len(),
        1
    );
    assert!(controller
        .restart(
            Command::new(dir.path().join("missing-compiler")),
            Limits::default()
        )
        .is_err());
    assert_eq!(controller.document("main.tex").unwrap().revision, 2);
    controller
        .restart(command(dir.path(), ECHO), Limits::default())
        .unwrap();
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    assert!(events.iter().any(|event| matches!(event, Update::Preview(preview) if preview.source_versions.documents["main.tex"] == 2)));
}

#[test]
fn missing_negotiated_capability_is_explicit_and_legacy_switch_invalidates_preview() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    controller
        .configure_layout(vec!["rules-v1".into()])
        .unwrap();
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let extended = events
        .into_iter()
        .find_map(|event| {
            if let Update::Preview(p) = event {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(extended.missing_layout_capabilities, vec!["rules-v1"]);
    controller.configure_layout(vec![]).unwrap();
    assert!(!controller.is_current_preview(&extended));
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let legacy = events
        .into_iter()
        .find_map(|event| {
            if let Update::Preview(p) = event {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert!(legacy.missing_layout_capabilities.is_empty());
    assert!(controller.is_current_preview(&legacy));
}

#[test]
fn membership_change_discards_pending_preview_and_reopens_same_source_revision() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    controller.compile_current().unwrap();
    let mut extra = Store::open(dir.path().join("extra")).unwrap();
    extra
        .initialize(Document::new("p".into(), "extra.tex".into(), 1, "extra".into()).unwrap())
        .unwrap();
    let initial = controller.index().snapshot();
    controller.attach_document(&initial, extra).unwrap();
    let added = controller.index().snapshot();
    controller.detach_document(&added, "extra.tex").unwrap();
    let removed = controller.index().snapshot();
    controller
        .attach_document(&removed, Store::open(dir.path().join("extra")).unwrap())
        .unwrap();
    let current = controller.index().snapshot();
    let events = wait(&mut controller, |items| {
        items
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    for event in events {
        if let Update::Preview(preview) = event {
            assert_eq!(preview.source_versions, current);
            assert!(controller.is_current_preview(&preview));
        }
    }
    assert!(current.generation > added.generation);
    assert_eq!(controller.document("extra.tex").unwrap().revision, 1);
}

#[test]
fn compiler_restart_never_revalidates_older_index_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller =
        Controller::open_without_compiler("p".into(), "main.tex".into(), vec![store(dir.path())])
            .unwrap();
    let initial = controller.index().snapshot();
    controller
        .restart(command(dir.path(), ECHO), Limits::default())
        .unwrap();
    let restarted = controller.index().snapshot();
    assert!(restarted.generation > initial.generation);
    assert_eq!(restarted.documents, initial.documents);
    assert!(controller.index().symbols(&initial).is_err());
    controller
        .restart(command(dir.path(), ECHO), Limits::default())
        .unwrap();
    assert!(controller.index().snapshot().generation > restarted.generation);
}

fn historical_fixture(dir: &std::path::Path) -> Controller {
    let body = format!("import pathlib\n{}", ECHO.replace("time.sleep(.02)",
        "\n while p['revision'] > 1 and not pathlib.Path(__file__).with_name('release').exists(): time.sleep(.001)"));
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir)],
        command(dir, &body),
        Limits::default(),
    )
    .unwrap();
    controller.configure_completed_snapshots(true).unwrap();
    controller.compile_current().unwrap();
    let before = controller.document("main.tex").unwrap().clone();
    controller
        .replace_document(
            "main.tex",
            before.revision,
            &before.source_sha256,
            "new source with changed offsets".into(),
        )
        .unwrap();
    wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Runtime(Event::Stale { revision: 1, .. })))
    });
    controller
}

#[test]
fn historical_preview_binds_original_versions_and_cannot_regress_after_current_output() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = historical_fixture(dir.path());
    let historical = controller.take_completed_snapshot().unwrap();
    assert_eq!(historical.source_versions().documents["main.tex"], 1);
    assert_eq!(controller.index().snapshot().documents["main.tex"], 2);
    assert_eq!(historical.request_id(), "preview-1");
    assert_eq!(historical.result()["payload"]["revision"], 1);
    assert!(controller.claim_historical_display(&historical));
    assert!(!controller.claim_historical_display(&historical));
    std::fs::write(dir.path().join("release"), b"ok").unwrap();
    let events = wait(&mut controller, |events| {
        events
            .iter()
            .any(|event| matches!(event, Update::Preview(_)))
    });
    let current = events
        .into_iter()
        .find_map(|event| match event {
            Update::Preview(p) => Some(p),
            _ => None,
        })
        .unwrap();
    assert!(controller.is_current_preview(&current));
    assert!(!controller.claim_historical_display(&historical));
    assert!(controller.take_completed_snapshot().is_none());
}

#[test]
fn historical_callbacks_are_invalid_after_policy_layout_restart_close_or_other_controller() {
    for transition in ["policy", "layout", "restart", "close", "other"] {
        let dir = tempfile::tempdir().unwrap();
        let mut controller = historical_fixture(dir.path());
        let historical = controller.take_completed_snapshot().unwrap();
        match transition {
            "policy" => {
                controller.configure_completed_snapshots(false).unwrap();
                controller.configure_completed_snapshots(true).unwrap();
            }
            "layout" => {
                controller.configure_layout(vec![]).unwrap();
            }
            "restart" => {
                controller
                    .restart(command(dir.path(), ECHO), Limits::default())
                    .unwrap();
            }
            "close" => {
                controller.close().unwrap();
            }
            "other" => {
                let other_dir = tempfile::tempdir().unwrap();
                let mut other = historical_fixture(other_dir.path());
                assert!(!other.claim_historical_display(&historical));
                continue;
            }
            _ => unreachable!(),
        }
        assert!(!controller.claim_historical_display(&historical));
    }
}

#[test]
fn optional_historical_metadata_eviction_never_rejects_a_durable_edit() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    controller.configure_completed_snapshots(true).unwrap();
    for index in 0..80 {
        let previous = controller.document("main.tex").unwrap().clone();
        let outcome = controller
            .replace_document(
                "main.tex",
                previous.revision,
                &previous.source_sha256,
                format!("durable edit {index}"),
            )
            .unwrap();
        assert!(outcome.preview_error.is_none());
        assert!(controller.historical_binding_count() <= 64);
    }
    assert_eq!(
        controller.document("main.tex").unwrap().text,
        "durable edit 79"
    );
    drop(controller);
    assert_eq!(
        store(dir.path()).document().unwrap().unwrap().text,
        "durable edit 79"
    );
}

#[test]
fn preview_currentness_checks_compile_generation_even_with_matching_id_and_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::new(
        "p".into(),
        "main.tex".into(),
        vec![store(dir.path())],
        command(dir.path(), ECHO),
        Limits::default(),
    )
    .unwrap();
    controller.compile_current().unwrap();
    let mut preview = wait(&mut controller, |events| {
        events.iter().any(|e| matches!(e, Update::Preview(_)))
    })
    .into_iter()
    .find_map(|event| match event {
        Update::Preview(p) => Some(p),
        _ => None,
    })
    .unwrap();
    assert!(controller.is_current_preview(&preview));
    let generation = preview.compile_revision;
    // Keep the real request ID and complete source snapshot unchanged.
    preview.compile_revision = generation.checked_add(1).unwrap();
    assert!(!controller.is_current_preview(&preview));
    preview.compile_revision = generation.saturating_sub(1);
    assert!(!controller.is_current_preview(&preview));
    preview.compile_revision = generation;
    assert!(controller.is_current_preview(&preview));
    controller
        .restart(command(dir.path(), ECHO), Limits::default())
        .unwrap();
    assert!(!controller.is_current_preview(&preview));
    let current = wait(&mut controller, |events| {
        events.iter().any(|e| matches!(e, Update::Preview(_)))
    })
    .into_iter()
    .find_map(|event| match event {
        Update::Preview(p) => Some(p),
        _ => None,
    })
    .unwrap();
    assert_eq!(
        current.source_versions.documents,
        preview.source_versions.documents
    );
    assert_ne!(current.compile_revision, generation);
    assert!(controller.is_current_preview(&current));
    assert!(!controller.is_current_preview(&preview));
}

#[test]
fn edit_admission_tracks_queued_supersession_and_failed_admission() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller =
        Controller::open_without_compiler("p".into(), "main.tex".into(), vec![store(dir.path())])
            .unwrap();
    let old = controller.document("main.tex").unwrap().clone();
    let saved = controller
        .replace_document(
            "main.tex",
            old.revision,
            &old.source_sha256,
            "saved offline".into(),
        )
        .unwrap();
    assert!(saved.compile_admission.is_none());
    assert!(saved.preview_error.is_some());
    let gated = "import json,sys,pathlib,time\nroot=pathlib.Path(__file__).parent\nfor line in sys.stdin:\n r=json.loads(line);p=r['payload'];(root/'started').touch()\n while not (root/'release').exists(): time.sleep(.001)\n print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[]}}),flush=True)\n";
    controller
        .restart(command(dir.path(), gated), Limits::default())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !dir.path().join("started").exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    let mut admissions = Vec::new();
    for text in ["queued first", "queued second"] {
        let doc = controller.document("main.tex").unwrap().clone();
        let result = controller
            .replace_document_metadata("main.tex", doc.revision, &doc.source_sha256, text.into())
            .unwrap();
        assert!(result.preview_error.is_none());
        let admission = result.compile_admission.unwrap();
        assert_ne!(admission.compile_revision, result.document.revision);
        admissions.push(admission);
    }
    std::fs::write(dir.path().join("release"), "").unwrap();
    let events = wait(&mut controller, |events| {
        events.iter().any(
            |event| matches!(event, Update::Preview(p) if p.request_id == admissions[1].request_id),
        )
    });
    assert!(events.iter().any(|event| matches!(event, Update::Runtime(Event::Superseded {id,by_id}) if id == &admissions[0].request_id && by_id == &admissions[1].request_id)));
    let limits = Limits {
        max_frame: 512,
        ..Limits::default()
    };
    controller
        .restart(command(dir.path(), ECHO), limits)
        .unwrap();
    let doc = controller.document("main.tex").unwrap().clone();
    let rejected = controller
        .replace_document(
            "main.tex",
            doc.revision,
            &doc.source_sha256,
            "x".repeat(2048),
        )
        .unwrap();
    assert!(rejected.preview_error.is_some());
    assert!(rejected.compile_admission.is_none());
    assert_eq!(
        controller.document("main.tex").unwrap().text,
        "x".repeat(2048)
    );
    controller.close().unwrap();
}

#[test]
fn grouped_encoding_refusal_preserves_source_and_permanent_retry() {
    use flashtex_preview_controller::HistoryAction;
    for metadata in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let limits = Limits {
            max_frame: 512,
            ..Limits::default()
        };
        let mut controller = Controller::new(
            "p".into(),
            "main.tex".into(),
            vec![store(dir.path())],
            command(dir.path(), ECHO),
            limits,
        )
        .unwrap();
        let doc = controller.document("main.tex").unwrap().clone();
        let command: flashtex_edit_ledger::history::GroupedEdit = serde_json::from_value(serde_json::json!({"command_id":"large-group","expected_revision":doc.revision,"expected_sha256":doc.source_sha256,"label":"large replacement","edits":[{"start_byte":0,"end_byte":doc.text.len(),"removed_text":doc.text,"replacement":"x".repeat(2048)}]})).unwrap();
        for replayed in [false, true] {
            if metadata {
                let result = controller
                    .apply_group_metadata("main.tex", command.clone())
                    .unwrap();
                assert!(result.compile_admission.is_none());
                assert!(result.preview_error.is_some());
                assert_eq!(result.history.command_revision, 2);
                assert_eq!(result.history.replayed_command, replayed);
            } else {
                let result = controller
                    .apply_history("main.tex", HistoryAction::Group(command.clone()))
                    .unwrap();
                assert!(result.source.compile_admission.is_none());
                assert!(result.source.preview_error.is_some());
                assert_eq!(result.history.command_revision, 2);
                assert_eq!(result.history.replayed_command, replayed);
            }
            assert_eq!(controller.document("main.tex").unwrap().revision, 2);
            assert_eq!(
                controller.document("main.tex").unwrap().text,
                "x".repeat(2048)
            );
        }
        controller.close().unwrap();
        drop(controller);
        assert_eq!(
            store(dir.path()).document().unwrap().unwrap().text,
            "x".repeat(2048)
        );
    }
}
