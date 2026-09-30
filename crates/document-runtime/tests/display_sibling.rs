#![cfg(unix)]
use flashtex_document_runtime::{Document, Event, Limits, Request, Session};
use std::{
    os::unix::fs::PermissionsExt,
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
            text: "東京 α".into(),
        }],
    }
}
fn caps() -> Vec<String> {
    vec!["display-list-v2".into()]
}
fn session(mode: &str) -> (tempfile::TempDir, Session) {
    session_mode(mode, false)
}
fn session_mode(mode: &str, raw: bool) -> (tempfile::TempDir, Session) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("worker.py");
    let script = format!(
        r#"#!/usr/bin/env python3
import json,sys,time,hashlib
mode={mode:?}
for line in sys.stdin:
 r=json.loads(line);p=r['payload'];cap=p.get('layout_capabilities',[])
 accepted=[] if mode=='declined' else cap
 status='failed' if mode=='failed' else 'ok'
 result={{'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{{'project_id':p['project_id'],'revision':p['revision'],'status':status,'pages':[],'diagnostics':[],'layout_capabilities':accepted}}}}
 if mode=='delay': time.sleep(.04)
 print(json.dumps(result),flush=True)
 if mode=='missing': time.sleep(10);continue
 if mode in ('declined','failed'):continue
 docs=[{{'path':d['path'],'revision':p['revision'],'sha256':hashlib.sha256(d['text'].encode()).hexdigest(),'byte_length':len(d['text'].encode())}} for d in p['documents']]
 v={{'protocol_version':2,'type':'display_list','id':r['id'],'payload':{{'project_id':p['project_id'],'revision':p['revision'],'render_format':'display-list-v2','documents':docs}}}}
 if mode=='oversized':v['payload']['padding']='x'*4096
 if mode=='hash':docs[0]['sha256']='0'*64
 if mode=='length':docs[0]['byte_length']=len(p['documents'][0]['text'])
 if mode=='duplicate_doc':docs.append(docs[0])
 if mode=='revision':v['payload']['revision']+=1
 if mode=='id':v['id']='other'
 if mode=='interleaved':v=result
 if mode=='malformed':print('{{',flush=True);continue
 if mode=='gap':time.sleep(.1)
 print(json.dumps(v),flush=True)
 if mode=='duplicate':print(json.dumps(v),flush=True)
"#
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let limits = Limits {
        max_frame: if mode == "oversized" {
            1024
        } else {
            Limits::default().max_frame
        },
        timeout: if mode == "missing" {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        },
        ..Limits::default()
    };
    let mut command = std::process::Command::new("python3");
    command.arg(path);
    let s = if raw {
        Session::spawn_command_raw_display_prototype(command, limits)
    } else {
        Session::spawn_command(command, limits)
    }
    .unwrap();
    (dir, s)
}
fn wait_until(s: &mut Session, condition: impl Fn(&Session, &[Event]) -> bool) -> Vec<Event> {
    let start = Instant::now();
    let mut events = vec![];
    loop {
        events.extend(s.poll());
        if condition(s, &events) {
            return events;
        }
        assert!(
            start.elapsed() < Duration::from_secs(7),
            "waiting for condition: {events:?}"
        );
        thread::sleep(Duration::from_millis(2));
    }
}
fn preview(s: &mut Session, revision: u64) -> Vec<Event> {
    wait_until(s, |s, e| {
        !s.is_alive()
            || e.iter()
                .any(|e| matches!(e,Event::Preview{revision:r,..} if *r==revision))
    })
}
fn take_wait(s: &mut Session) -> flashtex_document_runtime::UntrustedDisplayCandidate {
    let start = Instant::now();
    loop {
        s.poll();
        if let Some(c) = s.take_current_display_candidate() {
            return c;
        }
        assert!(s.is_alive() && start.elapsed() < Duration::from_secs(7));
        thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn opt_in_exact_utf8_identity_owned_slot_and_legacy_fallback() {
    let (_d, mut s) = session("ok");
    assert!(s.submit_with_capabilities(request(1), caps()).is_err());
    s.set_display_candidates_enabled(true).unwrap();
    assert!(s.set_completed_snapshots_enabled(true).is_err());
    s.submit_with_capabilities(request(1), caps()).unwrap();
    let e = preview(&mut s, 1);
    assert!(e
        .iter()
        .any(|e| matches!(e, Event::Preview { revision: 1, .. })));
    let c = take_wait(&mut s);
    assert_eq!(c.request_id(), "r1");
    assert_eq!(c.project_id(), "p");
    assert_eq!(c.revision(), 1);
    assert_eq!(c.sources()[0].byte_length, "東京 α".len());
    assert_eq!(
        c.sources()[0].sha256,
        flashtex_project_files::sha256_hex("東京 α".as_bytes())
    );
    assert_eq!(c.into_envelope()["protocol_version"], 2);
    assert!(s.take_current_display_candidate().is_none());
    assert!(s.is_alive());
}
#[test]
fn declined_failed_and_old_requests_finish_without_sibling() {
    for mode in ["declined", "failed"] {
        let (_d, mut s) = session(mode);
        s.set_display_candidates_enabled(true).unwrap();
        s.submit_with_capabilities(request(1), caps()).unwrap();
        preview(&mut s, 1);
        assert!(s.take_current_display_candidate().is_none());
        assert!(s.is_alive());
        s.submit_with_capabilities(request(2), caps()).unwrap();
        assert!(preview(&mut s, 2)
            .iter()
            .any(|e| matches!(e, Event::Preview { revision: 2, .. })));
    }
}
#[test]
fn malformed_mismatched_duplicate_interleaved_and_missing_fail_closed() {
    for mode in [
        "hash",
        "length",
        "duplicate_doc",
        "revision",
        "id",
        "interleaved",
        "malformed",
        "duplicate",
        "missing",
    ] {
        let (_d, mut s) = session(mode);
        s.set_display_candidates_enabled(true).unwrap();
        s.submit_with_capabilities(request(1), caps()).unwrap();
        wait_until(&mut s, |s, _| !s.is_alive());
        assert!(!s.is_alive(), "{mode}");
        assert!(s.take_current_display_candidate().is_none(), "{mode}");
    }
}
#[test]
fn stale_cancelled_toggle_and_submit_invalidate_candidates() {
    let (_d, mut s) = session("delay");
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    s.submit_with_capabilities(request(2), caps()).unwrap();
    assert_eq!(take_wait(&mut s).revision(), 2);
    s.submit_with_capabilities(request(3), caps()).unwrap();
    s.close_project("p").unwrap();
    s.submit_with_capabilities(request(4), caps()).unwrap();
    assert_eq!(take_wait(&mut s).revision(), 4);
    assert!(s.take_current_display_candidate().is_none());
    assert!(s.is_alive());
    let (_d, mut s) = session("gap");
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    preview(&mut s, 1);
    s.set_display_candidates_enabled(false).unwrap();
    s.set_display_candidates_enabled(true).unwrap();

    assert!(s.take_current_display_candidate().is_none());
    assert!(s.is_alive());
    s.submit_with_capabilities(request(2), caps()).unwrap();
    assert_eq!(take_wait(&mut s).revision(), 2);
    s.submit_with_capabilities(request(3), caps()).unwrap();
    assert!(s.take_current_display_candidate().is_none());
}

#[test]
fn unsolicited_and_closed_project_siblings_never_escape() {
    let (_d, mut s) = session("ok");
    s.submit(request(1)).unwrap();
    wait_until(&mut s, |s, _| !s.is_alive());
    assert!(!s.is_alive());
    assert!(s.take_current_display_candidate().is_none());
    let (_d, mut s) = session("gap");
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    preview(&mut s, 1);
    s.close_project("p").unwrap();
    s.submit_with_capabilities(request(2), caps()).unwrap();
    assert_eq!(take_wait(&mut s).revision(), 2);
    assert!(s.is_alive());
    assert!(s.take_current_display_candidate().is_none());
}

#[test]
fn promised_sibling_blocks_next_dispatch_and_preserves_timeout() {
    let (_d, mut s) = session("missing");
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    let mut events = preview(&mut s, 1);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Preview { revision: 1, .. })));
    s.submit_with_capabilities(request(2), caps()).unwrap();
    events.extend(wait_until(&mut s, |s, _| !s.is_alive()));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Preview { revision: 2, .. })));
    assert!(events.iter().any(
        |e| matches!(e, Event::Failed { id, reason } if id == "r1" && reason.contains("timeout"))
    ));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Failed { id, .. } if id == "r2")));
}

#[test]
fn helper_can_defer_take_without_extra_pending_value_and_close_invalidates() {
    let (_d, mut s) = session("ok");
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    preview(&mut s, 1); // Required v1 output could still be writing externally.
    thread::sleep(Duration::from_millis(40));
    assert_eq!(take_wait(&mut s).request_id(), "r1");
    s.submit_with_capabilities(request(2), caps()).unwrap();
    preview(&mut s, 2);
    s.close_project("p").unwrap();
    assert!(s.take_current_display_candidate().is_none());
    s.set_display_candidates_enabled(false).unwrap();
    s.set_completed_snapshots_enabled(true).unwrap();
    assert!(s.set_display_candidates_enabled(true).is_err());
    assert!(s
        .submit_with_snapshot_origin(request(3), vec![], "origin3".into())
        .is_ok());
}

#[test]
fn display_profile_is_current_scalar_only_and_clears_on_invalidation() {
    let (_d, mut s) = session("ok");
    assert!(s.last_display_profile().is_none());
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    take_wait(&mut s);
    let p = s.last_display_profile().unwrap();
    assert_eq!((&*p.request_id, &*p.project_id, p.revision), ("r1", "p", 1));
    assert!(p.response_bytes > 0);
    for elapsed in [
        p.parse_ms,
        p.decode_queue_wait_ms,
        p.reader_delivery_wait_ms,
        p.source_binding_ms,
    ] {
        assert!(elapsed.is_finite() && elapsed >= 0.0);
    }
    let epoch = p.display_epoch;
    let scalar = serde_json::to_value(p).unwrap();
    assert!(!scalar.to_string().contains("東京"));
    s.set_display_candidates_enabled(true).unwrap(); // same policy explicitly resets epoch
    assert!(s.last_display_profile().is_none());
    s.submit_with_capabilities(request(2), caps()).unwrap();
    take_wait(&mut s);
    assert!(s.last_display_profile().unwrap().display_epoch > epoch);
    s.submit_with_capabilities(request(3), caps()).unwrap();
    assert!(s.last_display_profile().is_none());
    s.submit_with_capabilities(request(4), caps()).unwrap();
    assert_eq!(take_wait(&mut s).revision(), 4);
    assert_eq!(s.last_display_profile().unwrap().revision, 4);
    s.close_project("p").unwrap();
    assert!(s.last_display_profile().is_none());
    let (_d, mut s) = session("duplicate");
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    wait_until(&mut s, |s, _| !s.is_alive());
    assert!(s.last_display_profile().is_none());
}

#[test]
fn raw_prototype_is_separate_current_only_and_preserves_failure_lifecycle() {
    let (_d, mut s) = session_mode("ok", true);
    assert!(s.submit_with_capabilities(request(1), caps()).is_err());
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    let start = Instant::now();
    let candidate = loop {
        s.poll();
        if let Some(c) = s.take_current_raw_display_candidate() {
            break c;
        }
        assert!(s.is_alive() && start.elapsed() < Duration::from_secs(7));
        thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(candidate.request_id(), "r1");
    assert!(s.take_current_display_candidate().is_none());
    assert_eq!(
        candidate.sources()[0].sha256,
        flashtex_project_files::sha256_hex("東京 α".as_bytes())
    );
    s.submit_with_capabilities(request(2), caps()).unwrap();
    s.set_display_candidates_enabled(false).unwrap();
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(3), caps()).unwrap();
    let start = Instant::now();
    loop {
        s.poll();
        if let Some(c) = s.take_current_raw_display_candidate() {
            assert_eq!(c.revision(), 3);
            break;
        }
        assert!(s.is_alive() && start.elapsed() < Duration::from_secs(7));
        thread::sleep(Duration::from_millis(2));
    }
    s.close_project("p").unwrap();
    assert!(s.take_current_raw_display_candidate().is_none());
    assert!(s.last_display_profile().is_none());
    for mode in [
        "hash",
        "duplicate",
        "malformed",
        "interleaved",
        "oversized",
        "missing",
    ] {
        let (_d, mut s) = session_mode(mode, true);
        s.set_display_candidates_enabled(true).unwrap();
        s.submit_with_capabilities(request(1), caps()).unwrap();
        wait_until(&mut s, |s, _| !s.is_alive());
        assert!(s.take_current_raw_display_candidate().is_none());
    }
}

#[test]
fn raw_cancelled_sibling_retains_old_budget_then_dispatches_new_document_set() {
    let (_dir, mut s) = session_mode("gap", true);
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(request(1), caps()).unwrap();
    preview(&mut s, 1);
    s.close_project("p").unwrap();
    let mut next = request(2);
    next.documents.push(Document {
        path: "chapters/東京-long.tex".into(),
        text: "next".into(),
    });
    s.submit_with_capabilities(next, caps()).unwrap();
    let start = Instant::now();
    loop {
        let events = s.poll();
        assert!(
            !events.iter().any(|e| matches!(e, Event::Failed { .. })),
            "{events:?}"
        );
        if let Some(c) = s.take_current_raw_display_candidate() {
            assert_eq!(c.request_id(), "r2");
            assert_eq!(c.sources().len(), 2);
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(7));
        thread::sleep(Duration::from_millis(2));
    }
}
