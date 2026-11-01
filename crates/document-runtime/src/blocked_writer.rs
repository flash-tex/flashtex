//! Linux process/pipeline acceptance; no production worker or queue is added.
use super::*;
use std::os::unix::net::UnixDatagram;
#[test]
#[ignore = "near-limit stopped-reader lifecycle evidence; run explicitly"]
fn stopped_stdin_child_timeout_cancel_drop_reaps_and_writer_exits() {
    let mut records = vec![];
    for mode in ["timeout", "close_drop", "direct_drop"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ready.sock");
        let handshake = UnixDatagram::bind(&path).unwrap();
        handshake
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut command = Command::new("python3");
        command
            .arg("-c")
            .arg(
                r#"import os,sys,socket,signal,fcntl
s=socket.socket(socket.AF_UNIX,socket.SOCK_DGRAM)
capacity=fcntl.fcntl(0,fcntl.F_GETPIPE_SZ)
b=os.read(0,1)
assert len(b)==1
s.sendto(str(capacity).encode(),sys.argv[1])
while True: signal.pause()
"#,
            )
            .arg(path);
        let mut session = Session::spawn_command(command, Limits::default()).unwrap();
        let process = session.process.as_mut().unwrap();
        let pid = process.child.id();
        let writer_done = process.writer_finished.take().unwrap();
        let request = Request {
            id: "blocked".into(),
            project_id: "p".into(),
            revision: 1,
            entry_path: "main.tex".into(),
            documents: vec![Document {
                path: "main.tex".into(),
                text: "x".repeat(Limits::default().max_frame - 512),
            }],
        };
        let framed_bytes = encode(&request, Limits::default().max_frame, &[])
            .unwrap()
            .len();
        session.submit(request).unwrap();
        session
            .submit(Request {
                id: "queued".into(),
                project_id: "q".into(),
                revision: 1,
                entry_path: "main.tex".into(),
                documents: vec![Document {
                    path: "main.tex".into(),
                    text: "queued".into(),
                }],
            })
            .unwrap();
        let mut message = [0; 64];
        let n = handshake.recv(&mut message).unwrap();
        let pipe_capacity = std::str::from_utf8(&message[..n])
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert!(framed_bytes > pipe_capacity + 1 && framed_bytes <= Limits::default().max_frame);
        assert!(matches!(
            writer_done.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        let start = Instant::now();
        let mut events = vec![];
        match mode {
            "timeout" => {
                // Deterministic deadline expiry; elapsed waiting is not the evidence.
                session.active.as_mut().unwrap().sent =
                    Some(Instant::now() - session.limits.timeout - Duration::from_millis(1));
                events = session.poll();
                assert!(!session.is_alive());
                assert!(events.iter().any(|e|matches!(e,Event::Failed{id,reason} if id=="blocked" && reason.contains("timeout"))));
            }
            "close_drop" => {
                session.close_project("p").unwrap();
                assert!(session.active.as_ref().unwrap().cancelled);
                assert!(session.is_alive()); // Close invalidates delivery; it does not kill a shared session.
                events.extend(session.events.drain(..));
                assert!(events
                    .iter()
                    .any(|e| matches!(e,Event::Cancelled{id} if id=="blocked")));
            }
            _ => {}
        }
        assert!(!events.iter().any(|e| matches!(e, Event::Preview { .. })));
        drop(session);
        let drop_or_fail_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "child not reaped"
        );
        writer_done.recv_timeout(Duration::from_secs(10)).unwrap();
        records.push(serde_json::json!({"mode":mode,"framed_bytes":framed_bytes,"pipe_capacity":pipe_capacity,"one_byte_read_handshake":true,"writer_exit_observed":true,"child_reaped":true,"preview_delivered":false,"drop_or_fail_ms":drop_or_fail_ms,"writer_terminal_ms":start.elapsed().as_secs_f64()*1000.0}));
    }
    println!(
        "{}",
        serde_json::json!({"cases":records,"scope":"Linux direct child, explicit one-byte handshake; timeout timestamp expired deterministically; writer exit witnessed separately from decoder join; no worst-case elapsed guarantee"})
    );
}

fn os_counts() -> (usize, usize, String) {
    (
        std::fs::read_dir("/proc/self/fd").unwrap().count(),
        std::fs::read_dir("/proc/self/task").unwrap().count(),
        std::fs::read_to_string("/proc/thread-self/children").unwrap(),
    )
}
#[test]
#[ignore = "30-cycle fd/thread/child lifecycle acceptance; run alone"]
fn repeated_sessions_return_to_os_baseline_after_worker_witnesses() {
    let baseline = os_counts();
    let mut records = vec![];
    for cycle in 0..30 {
        let stopped = cycle % 3 != 0;
        let raw = cycle % 2 == 0;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ready.sock");
        let ready = UnixDatagram::bind(&path).unwrap();
        ready
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut command = Command::new("python3");
        command.arg("-c").arg(r#"import os,sys,json,socket,signal,fcntl
s=socket.socket(socket.AF_UNIX,socket.SOCK_DGRAM)
capacity=fcntl.fcntl(0,fcntl.F_GETPIPE_SZ)
if sys.argv[2]=='stopped':
 assert len(os.read(0,1))==1
else:
 r=json.loads(sys.stdin.readline());p=r['payload']
 print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[]}}),flush=True)
s.sendto(str(capacity).encode(),sys.argv[1])
while True:signal.pause()
"#).arg(path).arg(if stopped {"stopped"}else{"normal"});
        let mut session = if raw {
            Session::spawn_command_raw_display_prototype(command, Limits::default())
        } else {
            Session::spawn_command(command, Limits::default())
        }
        .unwrap();
        let process = session.process.as_mut().unwrap();
        let pid = process.child.id();
        let writer = process.writer_finished.take().unwrap();
        let io = process.io_finished.take().unwrap();
        let request = Request {
            id: format!("cycle-{cycle}"),
            project_id: "p".into(),
            revision: 1,
            entry_path: "main.tex".into(),
            documents: vec![Document {
                path: "main.tex".into(),
                text: if stopped {
                    "x".repeat(Limits::default().max_frame - 512)
                } else {
                    "normal".into()
                },
            }],
        };
        let framed_bytes = encode(&request, Limits::default().max_frame, &[])
            .unwrap()
            .len();
        session.submit(request).unwrap();
        let mut buffer = [0; 64];
        let n = ready.recv(&mut buffer).unwrap();
        let capacity = std::str::from_utf8(&buffer[..n])
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let mut preview = false;
        if stopped {
            assert!(framed_bytes > capacity + 1);
            if cycle % 3 == 1 {
                session.active.as_mut().unwrap().sent =
                    Some(Instant::now() - session.limits.timeout - Duration::from_millis(1));
                let events = session.poll();
                assert!(events.iter().any(|e| matches!(e, Event::Failed { .. })));
                assert!(!events.iter().any(|e| matches!(e, Event::Preview { .. })));
            } else {
                session.close_project("p").unwrap();
            }
        } else {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !preview {
                let events = session.poll();
                assert!(!events.iter().any(|e| matches!(e, Event::Failed { .. })));
                preview = events.iter().any(|e| matches!(e, Event::Preview { .. }));
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
        drop(session);
        writer.recv_timeout(Duration::from_secs(10)).unwrap();
        for witness in io {
            witness.recv_timeout(Duration::from_secs(10)).unwrap();
        }
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
        drop(ready);
        drop(dir);
        // Exit messages precede the final thread epilogue. Wait for observed OS
        // counters, not an arbitrary sleep, before comparing the next cycle.
        let deadline = Instant::now() + Duration::from_secs(10);
        let counters = loop {
            let counters = os_counts();
            if counters == baseline {
                break counters;
            }
            assert!(
                Instant::now() < deadline,
                "baseline={baseline:?}, current={counters:?}"
            );
            std::thread::yield_now();
        };
        records.push(serde_json::json!({"cycle":cycle,"pid":pid,"raw_constructor":raw,"mode":if !stopped {"normal_drop"}else if cycle%3==1 {"stopped_timeout"}else{"stopped_close_drop"},"framed_bytes":framed_bytes,"pipe_capacity":capacity,"preview":preview,"all_three_io_witnesses":true,"child_reaped":true,"fd_count":counters.0,"thread_count":counters.1,"children":counters.2}));
    }
    let result = serde_json::json!({"cycles":records,"baseline_fd":baseline.0,"baseline_threads":baseline.1,"baseline_children":baseline.2,"scope":"One isolated Linux test process; OS counters after terminal witnesses, no allocator/RSS or all-production-thread-join claim"});
    if let Ok(path) = std::env::var("FLASHTEX_CYCLE_EVIDENCE") {
        std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    }
    println!(
        "30 cycles restored fd/thread/child baseline: {:?}",
        baseline
    );
}
