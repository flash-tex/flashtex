use flashtex_document_runtime::{Document, Event, Limits, Request, Session};
use std::{
    thread,
    time::{Duration, Instant},
};
fn request(rev: u64) -> Request {
    Request {
        id: format!("r{rev}"),
        project_id: "p".into(),
        revision: rev,
        entry_path: "main.tex".into(),
        documents: vec![Document {
            path: "main.tex".into(),
            text: format!("α revision {rev}"),
        }],
    }
}
#[cfg(unix)]
fn executable(body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("compiler");
    std::fs::write(&p, format!("#!/usr/bin/env python3\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
    (d, p)
}
#[cfg(unix)]
const ECHO:&str="import json,sys,time\nfor line in sys.stdin:\n r=json.loads(line);p=r['payload'];time.sleep(0.03)\n print(json.dumps({'protocol_version':1,'id':r['id'],'type':'compile_result','payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[]}}),flush=True)";
fn collect_until(session: &mut Session, predicate: impl Fn(&[Event]) -> bool) -> Vec<Event> {
    let start = Instant::now();
    let mut events = vec![];
    while start.elapsed() < Duration::from_secs(3) {
        events.extend(session.poll());
        if predicate(&events) {
            return events;
        }
        thread::sleep(Duration::from_millis(3));
    }
    panic!("events timed out: {events:?}")
}
#[test]
#[cfg(unix)]
fn coalesces_queued_edits_and_never_displays_stale_revision() {
    let (_d, path) = executable(ECHO);
    let mut s = fake_session(path, Limits::default()).unwrap();
    s.submit(request(1)).unwrap();
    s.submit(request(2)).unwrap();
    s.submit(request(3)).unwrap();
    let events = collect_until(&mut s, |es| {
        es.iter()
            .any(|e| matches!(e, Event::Preview { revision: 3, .. }))
    });
    assert!(events
        .iter()
        .any(|e| matches!(e,Event::Superseded{id,by_id}if id=="r2"&&by_id=="r3")));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Stale { revision: 1, .. })));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Preview { .. }))
            .count(),
        1
    );
    assert!(s.is_alive());
}
#[test]
#[cfg(unix)]
fn monotonic_revisions_and_paths_are_checked_before_sending() {
    let (_d, path) = executable(ECHO);
    let mut s = fake_session(path, Limits::default()).unwrap();
    s.submit(request(2)).unwrap();
    assert!(s.submit(request(1)).is_err());
    assert!(s.submit(request(2)).is_err());
    let mut invalid = request(3);
    invalid.entry_path = "../secret.tex".into();
    assert!(s.submit(invalid).is_err());
}
#[test]
#[cfg(unix)]
fn correlation_failure_invalidates_queued_work_explicitly() {
    let (_d, path) = executable("import sys\nsys.stdin.readline()\nprint('{}',flush=True)");
    let mut s = fake_session(path, Limits::default()).unwrap();
    s.submit(request(1)).unwrap();
    s.submit(request(2)).unwrap();
    let events = collect_until(&mut s, |es| {
        es.iter()
            .filter(|e| matches!(e, Event::Failed { .. }))
            .count()
            == 2
    });
    assert!(!s.is_alive());
    assert_eq!(events.len(), 2);
    assert!(s.submit(request(3)).is_err());
}
#[test]
#[cfg(unix)]
fn timeout_and_crash_fail_without_approximate_preview() {
    for body in ["import time\ntime.sleep(10)", "raise SystemExit(3)"] {
        let (_d, path) = executable(body);
        let mut s = fake_session(
            path,
            Limits {
                timeout: Duration::from_millis(150),
                ..Limits::default()
            },
        )
        .unwrap();
        s.submit(request(1)).unwrap();
        let events = collect_until(&mut s, |es| {
            es.iter().any(|e| matches!(e, Event::Failed { .. }))
        });
        assert!(!events.iter().any(|e| matches!(e, Event::Preview { .. })));
        assert!(!s.is_alive());
    }
}
#[test]
#[cfg(unix)]
fn stderr_backpressure_does_not_block_real_reply() {
    let body = ECHO.replace(
        "r=json.loads(line);",
        "sys.stderr.write('x'*100000);sys.stderr.flush();r=json.loads(line);",
    );
    let (_d, path) = executable(&body);
    let mut s = fake_session(path, Limits::default()).unwrap();
    s.submit(request(1)).unwrap();
    let events = collect_until(&mut s, |es| {
        es.iter().any(|e| matches!(e, Event::Preview { .. }))
    });
    assert!(matches!(&events[0],Event::Preview{total_ms,..} if *total_ms>=0.0));
}
#[test]
#[ignore = "requires explicitly configured original compiler"]
fn original_persistent_outputs_equal_clean_processes() {
    let binary = std::env::var_os("FLASHTEX_TEST_COMPILER").expect("set original compiler binary");
    let mut warm = Session::spawn(&binary, Limits::default()).unwrap();
    for revision in 1..=3 {
        let r = request(revision);
        warm.submit(r.clone()).unwrap();
        let w = collect_until(&mut warm, |es| {
            es.iter().any(|e| matches!(e, Event::Preview { .. }))
        });
        let mut fresh = Session::spawn(&binary, Limits::default()).unwrap();
        fresh.submit(r).unwrap();
        let f = collect_until(&mut fresh, |es| {
            es.iter().any(|e| matches!(e, Event::Preview { .. }))
        });
        let result = |es: Vec<Event>| {
            es.into_iter()
                .find_map(|e| {
                    if let Event::Preview { result, .. } = e {
                        Some(result)
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        assert_eq!(result(w), result(f));
    }
}

#[test]
#[cfg(unix)]
fn submission_backpressure_bounds_unconsumed_supersession_events() {
    let (_d, path) = executable("import time\ntime.sleep(10)");
    let mut s = fake_session(
        path,
        Limits {
            max_pending_events: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    for revision in 1..=4 {
        s.submit(request(revision)).unwrap();
    }
    assert!(s.submit(request(5)).unwrap_err().contains("poll pending"));
    let events = s.poll();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Superseded { .. }))
            .count(),
        2
    );
    s.submit(request(5)).unwrap();
}
#[test]
#[cfg(unix)]
fn malformed_unicode_source_is_never_delivered_to_preview() {
    let body=ECHO.replace("'pages':[]", "'pages':[{'number':1,'width_pt':612,'height_pt':792,'items':[{'kind':'text','text':'x','x_pt':1,'baseline_y_pt':12,'font_size_pt':12,'source':{'path':'main.tex','start_byte':1,'end_byte':2}}]}]");
    let (_d, path) = executable(&body);
    let mut s = fake_session(path, Limits::default()).unwrap();
    s.submit(request(1)).unwrap();
    let events = collect_until(&mut s, |es| {
        es.iter().any(|e| matches!(e, Event::Failed { .. }))
    });
    assert!(!events.iter().any(|e| matches!(e, Event::Preview { .. })));
}

#[cfg(unix)]
fn fake_session(path: std::path::PathBuf, limits: Limits) -> Result<Session, String> {
    let mut command = std::process::Command::new("python3");
    command.arg(path);
    Session::spawn_command(command, limits)
}

#[test]
#[cfg(unix)]
fn oversized_and_truncated_frames_fail_then_fresh_session_recovers() {
    for body in [
        "import sys\nsys.stdin.readline()\nsys.stdout.write('x'*1025);sys.stdout.flush()",
        "import sys\nsys.stdin.readline()\nsys.stdout.write('{}');sys.stdout.flush()",
    ] {
        let (_dir, path) = executable(body);
        let mut failed = fake_session(
            path,
            Limits {
                max_frame: 1024,
                ..Limits::default()
            },
        )
        .unwrap();
        failed.submit(request(1)).unwrap();
        let events = collect_until(&mut failed, |events| {
            events
                .iter()
                .any(|event| matches!(event, Event::Failed { .. }))
        });
        assert_eq!(events.len(), 1);
        assert!(!failed.is_alive());
        assert!(failed.submit(request(2)).is_err());
        let (_recovered_dir, path) = executable(ECHO);
        let mut recovered = fake_session(path, Limits::default()).unwrap();
        recovered.submit(request(2)).unwrap();
        let events = collect_until(&mut recovered, |events| {
            events
                .iter()
                .any(|event| matches!(event, Event::Preview { revision: 2, .. }))
        });
        assert_eq!(events.len(), 1);
    }
}

#[test]
#[cfg(unix)]
fn project_revisions_are_independent_and_queued_projects_are_not_lost() {
    let (_dir, path) = executable(ECHO);
    let mut session = fake_session(path, Limits::default()).unwrap();
    session.submit(request(10)).unwrap();
    let mut second = request(1);
    second.project_id = "other".into();
    second.id = "other-1".into();
    session.submit(second).unwrap();
    session.submit(request(11)).unwrap();
    let events = collect_until(&mut session, |events| {
        events
            .iter()
            .filter(|event| matches!(event, Event::Preview { .. }))
            .count()
            == 2
    });
    let projects: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Preview {
                project_id,
                revision,
                ..
            } => Some((project_id.as_str(), *revision)),
            _ => None,
        })
        .collect();
    assert_eq!(projects, vec![("other", 1), ("p", 11)]);
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::Stale { revision: 10, .. })));
}

#[test]
#[cfg(unix)]
fn closing_project_cancels_once_and_reclaims_slot_without_showing_old_result() {
    let (_dir, path) = executable(ECHO);
    let mut session = fake_session(
        path,
        Limits {
            max_projects: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    session.submit(request(1)).unwrap();
    session.submit(request(2)).unwrap();
    session.close_project("p").unwrap();
    session.close_project("p").unwrap();
    let mut reopened = request(0);
    reopened.id = "reopened".into();
    session.submit(reopened).unwrap();
    let events = collect_until(&mut session, |events| {
        events
            .iter()
            .any(|event| matches!(event, Event::Preview { revision: 0, .. }))
    });
    assert_eq!(events.len(), 3);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Cancelled { .. }))
            .count(),
        2
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::Stale { .. } | Event::Failed { .. })));
    session.close_project("p").unwrap();
    let mut other = request(1);
    other.project_id = "other".into();
    session.submit(other).unwrap();
}

#[cfg(unix)]
const LAYOUT: &str = "import json,sys,time\nfor line in sys.stdin:\n r=json.loads(line);p=r['payload'];caps=p.get('layout_capabilities',[]);time.sleep(.02)\n items=[]\n if 'rules-v1' in caps: items.append({'kind':'rule','x_pt':12,'y_pt':24,'width_pt':18,'height_pt':0.5,'source':{'path':'main.tex','start_byte':0,'end_byte':2}})\n if 'font-hints-v1' in caps: items.append({'kind':'text','text':'α','x_pt':12,'baseline_y_pt':24,'font_size_pt':12,'font':{'family':'Latin Modern Roman','weight':'normal','style':'italic'},'source':{'path':'main.tex','start_byte':0,'end_byte':2}})\n print(json.dumps({'protocol_version':1,'id':r['id'],'type':'compile_result','payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','layout_capabilities':caps,'pages':[{'number':1,'width_pt':612,'height_pt':792,'items':items}],'diagnostics':[]}}),flush=True)";
#[test]
#[cfg(unix)]
fn negotiated_rules_and_fonts_preserve_exact_output() {
    let (_dir, path) = executable(LAYOUT);
    let mut session = fake_session(path, Limits::default()).unwrap();
    session
        .submit_with_capabilities(request(1), vec!["rules-v1".into(), "font-hints-v1".into()])
        .unwrap();
    let events = collect_until(&mut session, |events| {
        events
            .iter()
            .any(|event| matches!(event, Event::Preview { .. }))
    });
    let result = events
        .into_iter()
        .find_map(|event| {
            if let Event::Preview { result, .. } = event {
                Some(result)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        result["payload"]["pages"][0]["items"][0],
        serde_json::json!({"kind":"rule","x_pt":12,"y_pt":24,"width_pt":18,"height_pt":0.5,"source":{"path":"main.tex","start_byte":0,"end_byte":2}})
    );
    assert_eq!(
        result["payload"]["pages"][0]["items"][1]["font"]["style"],
        "italic"
    );
}
#[test]
#[cfg(unix)]
fn late_extended_response_does_not_activate_legacy_preview() {
    let (_dir, path) = executable(LAYOUT);
    let mut session = fake_session(path, Limits::default()).unwrap();
    session
        .submit_with_capabilities(request(1), vec!["rules-v1".into()])
        .unwrap();
    session.submit(request(2)).unwrap();
    let events = collect_until(&mut session, |events| {
        events
            .iter()
            .any(|event| matches!(event, Event::Preview { revision: 2, .. }))
    });
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::Stale { revision: 1, .. })));
    for event in events {
        if let Event::Preview { result, .. } = event {
            assert_eq!(
                result["payload"]["layout_capabilities"],
                serde_json::json!([])
            );
            assert!(result["payload"]["pages"][0]["items"]
                .as_array()
                .unwrap()
                .is_empty());
        }
    }
}
#[test]
#[cfg(unix)]
fn malformed_and_unrequested_layout_cannot_reach_preview() {
    for body in [
        LAYOUT.replace(
            "caps=p.get('layout_capabilities',[])",
            "caps=['rules-v1','font-hints-v1']",
        ),
        LAYOUT.replace("'height_pt':0.5", "'height_pt':0"),
        LAYOUT.replace("'x_pt':12", "'x_pt':1000001"),
        LAYOUT.replace("'style':'italic'", "'style':'unknown'"),
        LAYOUT.replace("'kind':'rule'", "'kind':'unknown'"),
    ] {
        let (_dir, path) = executable(&body);
        let mut session = fake_session(path, Limits::default()).unwrap();
        let caps = if body.contains("caps=['rules-v1','font-hints-v1']") {
            vec![]
        } else {
            vec!["rules-v1".into(), "font-hints-v1".into()]
        };
        session.submit_with_capabilities(request(1), caps).unwrap();
        let events = collect_until(&mut session, |events| {
            events
                .iter()
                .any(|event| matches!(event, Event::Failed { .. }))
        });
        assert!(!events
            .iter()
            .any(|event| matches!(event, Event::Preview { .. })));
    }
}

#[test]
#[cfg(unix)]
fn invalid_utf8_json_frame_is_terminal_without_preview() {
    let (_dir, path) = executable(
        r#"import sys
sys.stdin.readline()
sys.stdout.buffer.write(b'{"bad":"\xff"}\n')
sys.stdout.buffer.flush()
"#,
    );
    let mut session = fake_session(path, Limits::default()).unwrap();
    session.submit(request(1)).unwrap();
    let events = collect_until(&mut session, |events| {
        events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
    });
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::Preview { .. })));
    assert!(!session.is_alive());
}

#[cfg(unix)]
fn historical_compiler() -> (tempfile::TempDir, std::path::PathBuf) {
    let body = format!("import pathlib\n{}", ECHO.replace("time.sleep(0.03)",
        "\n while p['revision'] > 1 and not pathlib.Path(__file__).with_name('release').exists(): time.sleep(0.001)"));
    executable(&body)
}

#[test]
#[cfg(unix)]
fn historical_snapshot_is_opt_in_and_preserves_original_identity() {
    let (dir, path) = historical_compiler();
    let mut session = fake_session(path, Limits::default()).unwrap();
    assert!(session
        .submit_with_snapshot_origin(request(1), vec![], "session-a/source-1".into())
        .is_err());
    session.set_completed_snapshots_enabled(true).unwrap();
    session
        .submit_with_snapshot_origin(request(1), vec![], "session-a/source-1".into())
        .unwrap();
    session.submit(request(2)).unwrap();
    let events = collect_until(&mut session, |events| {
        events
            .iter()
            .any(|event| matches!(event, Event::Stale { revision: 1, .. }))
    });
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::Preview { revision: 1, .. })));
    let snapshot = session.take_completed_snapshot().unwrap();
    assert_eq!(snapshot.origin, "session-a/source-1");
    assert_eq!(snapshot.request_id, "r1");
    assert_eq!(snapshot.project_id, "p");
    assert_eq!(snapshot.revision, 1);
    assert_eq!(snapshot.result["id"], "r1");
    assert_eq!(snapshot.result["payload"]["revision"], 1);
    assert!(session.take_completed_snapshot().is_none());
    std::fs::write(dir.path().join("release"), b"ok").unwrap();
    collect_until(&mut session, |events| {
        events
            .iter()
            .any(|event| matches!(event, Event::Preview { revision: 2, .. }))
    });
    assert!(session.take_completed_snapshot().is_none());
}

#[test]
#[cfg(unix)]
fn ordinary_submissions_never_retain_historical_results() {
    for enabled in [false, true] {
        let (_dir, path) = historical_compiler();
        let mut session = fake_session(path, Limits::default()).unwrap();
        session.set_completed_snapshots_enabled(enabled).unwrap();
        session.submit(request(1)).unwrap();
        session.submit(request(2)).unwrap();
        collect_until(&mut session, |events| {
            events
                .iter()
                .any(|event| matches!(event, Event::Stale { .. }))
        });
        assert!(session.take_completed_snapshot().is_none());
    }
}

#[test]
#[cfg(unix)]
fn policy_toggle_invalidates_inflight_snapshot_origins() {
    let (_dir, path) = historical_compiler();
    let mut session = fake_session(path, Limits::default()).unwrap();
    session.set_completed_snapshots_enabled(true).unwrap();
    session
        .submit_with_snapshot_origin(request(1), vec![], "old-epoch".into())
        .unwrap();
    session.submit(request(2)).unwrap();
    session.set_completed_snapshots_enabled(false).unwrap();
    session.set_completed_snapshots_enabled(true).unwrap();
    collect_until(&mut session, |events| {
        events
            .iter()
            .any(|event| matches!(event, Event::Stale { .. }))
    });
    assert!(session.take_completed_snapshot().is_none());
}

#[test]
#[cfg(unix)]
fn fresh_result_and_project_close_clear_retained_snapshots() {
    for close in [false, true] {
        let (dir, path) = historical_compiler();
        let mut session = fake_session(path, Limits::default()).unwrap();
        session.set_completed_snapshots_enabled(true).unwrap();
        session
            .submit_with_snapshot_origin(request(1), vec![], "origin".into())
            .unwrap();
        session.submit(request(2)).unwrap();
        collect_until(&mut session, |events| {
            events
                .iter()
                .any(|event| matches!(event, Event::Stale { .. }))
        });
        if close {
            session.close_project("p").unwrap();
        } else {
            std::fs::write(dir.path().join("release"), b"ok").unwrap();
            collect_until(&mut session, |events| {
                events
                    .iter()
                    .any(|event| matches!(event, Event::Preview { revision: 2, .. }))
            });
        }
        assert!(session.take_completed_snapshot().is_none());
    }
}

#[test]
#[cfg(unix)]
fn malformed_historical_result_never_enters_snapshot_slot() {
    let (_dir, path) = executable(&ECHO.replace("'pages':[]", "'pages':False"));
    let mut session = fake_session(path, Limits::default()).unwrap();
    session.set_completed_snapshots_enabled(true).unwrap();
    session
        .submit_with_snapshot_origin(request(1), vec![], "origin".into())
        .unwrap();
    session.submit(request(2)).unwrap();
    collect_until(&mut session, |events| {
        events
            .iter()
            .any(|event| matches!(event, Event::Failed { .. }))
    });
    assert!(session.take_completed_snapshot().is_none());
}
