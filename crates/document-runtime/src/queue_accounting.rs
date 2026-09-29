//! Deterministic owned-buffer accounting; excludes allocator headers and writer buffers.
use super::*;
fn request(id: &str, project: &str, revision: u64, reserve: usize) -> Request {
    let mut text = String::with_capacity(reserve);
    text.push_str("東京 x");
    let mut documents = Vec::with_capacity(16);
    documents.push(Document {
        path: "main.tex".into(),
        text,
    });
    Request {
        id: id.into(),
        project_id: project.into(),
        revision,
        entry_path: "main.tex".into(),
        documents,
    }
}
fn retained(s: &Session) -> Value {
    let pending = s.active.iter().chain(s.queue.iter());
    let mut source_bytes = 0;
    let mut string_capacity = 0;
    let mut encoded_bytes = 0;
    let mut encoded_capacity = 0;
    let mut document_slots = 0;
    for p in pending {
        source_bytes += p
            .request
            .documents
            .iter()
            .map(|d| d.text.len())
            .sum::<usize>();
        string_capacity += p.request.id.capacity()
            + p.request.project_id.capacity()
            + p.request.entry_path.capacity();
        string_capacity += p
            .request
            .documents
            .iter()
            .map(|d| d.path.capacity() + d.text.capacity())
            .sum::<usize>();
        encoded_bytes += p.bytes.len();
        encoded_capacity += p.bytes.capacity();
        document_slots += p.request.documents.capacity();
    }
    serde_json::json!({"source_bytes":source_bytes,"string_capacity":string_capacity,"encoded_bytes":encoded_bytes,"encoded_capacity":encoded_capacity,"document_slots":document_slots,"active":s.active.as_ref().map(|p|p.request.id.as_str()),"queue":s.queue.iter().map(|p|p.request.id.as_str()).collect::<Vec<_>>()})
}
#[test]
fn retained_buffers_follow_coalescing_close_and_failed_admission() {
    let mut command = Command::new("python3");
    command.arg("-c").arg("import sys;sys.stdin.read()");
    let mut s = Session::spawn_command(
        command,
        Limits {
            max_projects: 3,
            ..Limits::default()
        },
    )
    .unwrap();
    let mut evidence = vec![];
    s.submit(request("a1", "a", 1, 1048576)).unwrap();
    s.submit(request("a2", "a", 2, 1048576)).unwrap();
    s.submit(request("b1", "b", 1, 1048576)).unwrap();
    s.submit(request("c1", "c", 1, 1048576)).unwrap();
    assert_eq!(
        s.queue
            .iter()
            .map(|p| p.request.id.as_str())
            .collect::<Vec<_>>(),
        ["a2", "b1", "c1"]
    );
    evidence.push(retained(&s));
    let before = retained(&s);
    assert!(s.submit(request("d1", "d", 1, 1048576)).is_err());
    assert_eq!(retained(&s), before);
    assert!(s.submit(request("b1", "b", 2, 1048576)).is_err());
    assert_eq!(retained(&s), before);
    let mut oversized = request("b2", "b", 2, 0);
    oversized.documents[0].text = "x".repeat(s.limits.max_frame + 1);
    assert!(s.submit(oversized).is_err());
    assert_eq!(retained(&s), before);
    s.submit(request("a3", "a", 3, 1048576)).unwrap();
    assert_eq!(
        s.queue
            .iter()
            .map(|p| p.request.id.as_str())
            .collect::<Vec<_>>(),
        ["b1", "c1", "a3"]
    );
    assert!(s
        .events
        .iter()
        .any(|e| matches!(e,Event::Superseded{id,by_id} if id=="a2" && by_id=="a3")));
    evidence.push(retained(&s));
    s.close_project("a").unwrap();
    assert!(s.active.as_ref().unwrap().cancelled);
    assert_eq!(s.queue.len(), 2);
    evidence.push(retained(&s));
    s.submit(request("d2", "d", 2, 1048576)).unwrap();
    assert_eq!(s.queue.len(), 3);
    evidence.push(retained(&s));
    for p in s.active.iter().chain(s.queue.iter()) {
        assert_eq!(p.request.documents.capacity(), p.request.documents.len());
        assert_eq!(p.bytes.capacity(), p.bytes.len());
        assert_eq!(
            p.request.documents[0].text.capacity(),
            p.request.documents[0].text.len()
        );
    }
    for p in &s.queue {
        assert_eq!(p.request.documents[0].text, "東京 x");
        assert_eq!(
            p.bytes,
            encode(&p.request, s.limits.max_frame, &p.capabilities).unwrap()
        );
    }
    s.close_project("b").unwrap();
    assert_eq!(s.queue.len(), 2);
    evidence.push(retained(&s));
    s.fail("test failure");
    assert!(s.queue.is_empty() && s.active.is_none());
    evidence.push(retained(&s));
    if let Ok(path) = std::env::var("FLASHTEX_QUEUE_EVIDENCE") {
        std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "ordinary-size compaction cost observation; run in quiet window"]
fn ordinary_compaction_cost_observation() {
    let mut inputs = Vec::new();
    for _ in 0..1000 {
        let mut r = request("ordinary", "p", 1, 32768);
        r.documents[0].text.clear();
        r.documents[0].text.push_str(&"a".repeat(16384));
        inputs.push(r);
    }
    let encodings: Vec<_> = inputs
        .iter()
        .map(|r| encode(r, Limits::default().max_frame, &[]).unwrap())
        .collect();
    let start = Instant::now();
    let outputs: Vec<_> = inputs.into_iter().map(compact_request).collect();
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let encoded: Vec<_> = encodings
        .into_iter()
        .map(|b| b.into_boxed_slice().into_vec())
        .collect();
    let encoded_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(encoded.iter().all(|b| b.len() == b.capacity()));
    assert!(outputs
        .iter()
        .all(|r| r.documents[0].text.len() == 16384 && r.documents.capacity() == 1));
    println!(
        "{}",
        serde_json::json!({"requests":1000,"source_bytes_each":16384,"elapsed_ms":elapsed_ms,"per_request_ms":elapsed_ms/1000.0,"encoded_trim_ms":encoded_ms,"encoded_trim_per_request_ms":encoded_ms/1000.0,"scope":"debug-process batched ownership compaction only; inputs constructed before timing; no native throughput claim"})
    );
}
