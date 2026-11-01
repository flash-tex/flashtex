//! Bounded malformed metadata memory probe, not a throughput benchmark.
use flashtex_document_runtime::{Document, Event, Limits, Request, Session};
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};
fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(
        a.len(),
        4,
        "document count, identity bytes, captured output file"
    );
    let mut command = Command::new("python3");
    command.arg("-c").arg(r#"import sys,json,hashlib
r=json.loads(sys.stdin.readline());p=r['payload'];n=int(sys.argv[1]);length=int(sys.argv[2])
v1={'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[],'layout_capabilities':['display-list-v2']}}
docs=[{}]*n if not length else [{'path':'main.tex','revision':1,'sha256':hashlib.sha256(b'a').hexdigest(),'byte_length':1}]
v2={'protocol_version':2,'type':'display_list','id':'x'*length if length else r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'render_format':'display-list-v2','documents':docs}}
wire=('\n'.join(json.dumps(v,separators=(',',':')) for v in [v1,v2])+'\n').encode();open(sys.argv[3],'wb').write(wire);sys.stdout.buffer.write(wire);sys.stdout.flush();sys.stdin.read()
"#).args(&a[1..]);
    let mut s = Session::spawn_command_raw_display_prototype(command, Limits::default()).unwrap();
    s.set_display_candidates_enabled(true).unwrap();
    s.submit_with_capabilities(
        Request {
            id: "r1".into(),
            project_id: "p".into(),
            revision: 1,
            entry_path: "main.tex".into(),
            documents: vec![Document {
                path: "main.tex".into(),
                text: "a".into(),
            }],
        },
        vec!["display-list-v2".into()],
    )
    .unwrap();
    let start = Instant::now();
    let mut failed = false;
    while start.elapsed() < Duration::from_secs(10) {
        for e in s.poll() {
            if matches!(e, Event::Failed { .. }) {
                failed = true;
            }
        }
        if !s.is_alive() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(failed && !s.is_alive());
    assert!(s.take_current_raw_display_candidate().is_none());
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let peak = status
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .unwrap()
        .to_string();
    let bytes = std::fs::read(&a[3]).unwrap();
    let frame = bytes.split(|b| *b == b'\n').nth(1).unwrap();
    assert!(frame.len() + 1 < Limits::default().max_frame);
    println!(
        "{}",
        serde_json::json!({"documents":a[1],"identity_bytes":a[2],"frame_bytes":frame.len()+1,"frame_sha256":flashtex_project_files::sha256_hex(frame),"process_peak_before_fixture_read":peak,"failed":failed,"candidate_delivered":false,"scope":"process residency after malformed fixture, not allocation count or timing"})
    );
}
