use super::*;
use std::{hint::black_box, mem::size_of};

struct Proof { bytes: usize, hash: String, boundaries: Option<Box<[u8]>> }
impl Proof {
    fn new(text: &str) -> Self {
        let boundaries = if text.is_ascii() { None } else {
            let mut bits = vec![0u8; (text.len() + 1).div_ceil(8)];
            for (i, _) in text.char_indices() { bits[i / 8] |= 1 << (i % 8); }
            bits[text.len() / 8] |= 1 << (text.len() % 8);
            Some(bits.into_boxed_slice())
        };
        Self { bytes: text.len(), hash: flashtex_project_files::sha256_hex(text.as_bytes()), boundaries }
    }
    fn len(&self) -> usize { self.bytes }
    fn is_char_boundary(&self, i: usize) -> bool {
        i <= self.bytes && self.boundaries.as_ref().is_none_or(|b| b[i / 8] & (1 << (i % 8)) != 0)
    }
}
struct ProofDocument { path: String, text: Proof }
struct ProofRequest { id: String, project_id: String, revision: u64, documents: Vec<ProofDocument> }
impl ProofRequest {
    fn new(r: &Request) -> Self {
        Self { id: r.id.clone(), project_id: r.project_id.clone(), revision: r.revision,
            documents: r.documents.iter().map(|d| ProofDocument { path: d.path.clone(), text: Proof::new(&d.text) }).collect() }
    }
    fn retained(&self) -> usize {
        size_of::<Self>() + self.id.capacity() + self.project_id.capacity()
            + self.documents.capacity() * size_of::<ProofDocument>()
            + self.documents.iter().map(|d| d.path.capacity() + d.text.hash.capacity() + d.text.boundaries.as_ref().map_or(0, |b| b.len())).sum::<usize>()
    }
}
fn request(text: &str) -> Request {
    Request { id:"p1".into(), project_id:"p".into(), revision:1, entry_path:"main.tex".into(), documents:vec![Document {path:"main.tex".into(),text:text.into()}] }
}
fn span_reply(start: usize, end: usize) -> Value {
    serde_json::json!({"protocol_version":1,"type":"compile_result","id":"p1","payload":{"project_id":"p","revision":1,"status":"ok","pages":[],"diagnostics":[{"severity":"warning","message":"fixture","source":{"path":"main.tex","start_byte":start,"end_byte":end}}]}})
}
fn owned_retained(r: &Request) -> usize {
    size_of::<Request>() + r.id.capacity() + r.project_id.capacity() + r.entry_path.capacity()
        + r.documents.capacity()*size_of::<Document>()
        + r.documents.iter().map(|d| d.path.capacity()+d.text.capacity()).sum::<usize>()
}
#[test]
fn proof_boundaries_and_original_validator_equivalence() {
    for text in ["", "ascii", "éa", "aé", "東京", "😀", "e\u{301}", "\0x\r\n"] {
        let r=request(text); let p=ProofRequest::new(&r);
        for start in 0..=text.len()+1 { for end in 0..=text.len()+1 {
            assert_eq!(p.documents[0].text.is_char_boundary(start), text.is_char_boundary(start));
            let v=span_reply(start,end);
            assert_eq!(validate_reply_value(v.clone(),&r,&[]),validate_reply_proof(v,&p,&[]));
        }}
    }
}
#[test]
fn immutable_equal_length_snapshot_and_failed_preparation() {
    let mut r=request("éa"); let original=ProofRequest::new(&r);
    r.documents[0].text="aé".into(); let latest=ProofRequest::new(&r);
    assert!(!original.documents[0].text.is_char_boundary(1));
    assert!(latest.documents[0].text.is_char_boundary(1));
    assert!(original.documents[0].text.is_char_boundary(2));
    assert!(!latest.documents[0].text.is_char_boundary(2));
    assert_ne!(original.documents[0].text.hash,latest.documents[0].text.hash);
    assert_ne!(Proof::new("abc").hash,Proof::new("abd").hash);
    // Preparation failure cannot replace an earlier proof. No queue implementation
    // is modeled here: real Session lifecycle remains a separate acceptance gate.
    let previous_hash=original.documents[0].text.hash.clone();
    r.documents[0].text="x".repeat(2048);
    assert!(encode(&r,512,&[]).is_err());
    assert_eq!(original.documents[0].text.hash,previous_hash);
}
fn captured() -> Vec<(Request,Value,Option<Value>,Vec<String>)> {
    let cases:Value=serde_json::from_slice(&std::fs::read(std::env::var("PROOF_CASES").unwrap()).unwrap()).unwrap();
    cases.as_array().unwrap().iter().map(|c| {
        let p=&c["request"]["payload"];
        let r=Request {id:c["request"]["id"].as_str().unwrap().into(),project_id:p["project_id"].as_str().unwrap().into(),revision:p["revision"].as_u64().unwrap(),entry_path:p["entry_path"].as_str().unwrap().into(),documents:serde_json::from_value(p["documents"].clone()).unwrap()};
        let caps=serde_json::from_value(p.get("layout_capabilities").cloned().unwrap_or(serde_json::json!([]))).unwrap();
        (r,c["result"].clone(),c.get("candidate").filter(|v|!v.is_null()).cloned(),caps)
    }).collect()
}
#[test]
fn captured_v1_and_sibling_source_validation_equivalence() {
    let mut pairs=0;let mut siblings=0;
    for (r,v,sibling,caps) in captured() {
        let proof=ProofRequest::new(&r);
        assert_eq!(validate_reply_value(v.clone(),&r,&caps),validate_reply_proof(v.clone(),&proof,&caps));
        let mut bad=v;bad["payload"]["diagnostics"]=serde_json::json!([{"severity":"warning","message":"bad","source":{"path":"missing.tex","start_byte":0,"end_byte":1}}]);
        assert_eq!(validate_reply_value(bad.clone(),&r,&caps),validate_reply_proof(bad,&proof,&caps));
        pairs+=1;
        if let Some(envelope)=sibling {
            // Full existing typed raw decoder remains in the reference path.
            let raw=serde_json::to_vec(&envelope).unwrap();
            assert!(raw_display::Parsed::parse(raw).unwrap().validate(&r).is_ok());
            for (i,doc) in envelope["payload"]["documents"].as_array().unwrap().iter().enumerate() {
                let path=doc["path"].as_str().unwrap();let proof_doc=proof.documents.iter().find(|d|d.path==path).unwrap();
                assert_eq!(doc["sha256"],proof_doc.text.hash);assert_eq!(doc["byte_length"].as_u64(),Some(proof_doc.text.bytes as u64));
                let mut altered=envelope.clone();altered["payload"]["documents"][i]["sha256"]=serde_json::json!("0".repeat(64));
                assert!(raw_display::Parsed::parse(serde_json::to_vec(&altered).unwrap()).unwrap().validate(&r).is_err());
                assert_ne!(altered["payload"]["documents"][i]["sha256"],proof_doc.text.hash);
            }
            siblings+=1;
        }
    }
    eprintln!("captured_v1_pairs={pairs} captured_siblings={siblings}");
}
#[test]
#[ignore="isolated paired preparation cost observation; no runtime admission/paint claim"]
fn paired_preparation_cost() {
    let mut results=Vec::new();
    for (label,text) in [("ascii50k","a".repeat(50_000)),("utf8_50k","éa".repeat(16_667)),("ascii1m","a".repeat(1_000_000))] {
        let source=request(&text);let mut old=Vec::new();let mut new=Vec::new();let mut stored=0;let mut proof_stored=0;let mut encoded=0;
        for i in 0..24 { for proof_first in [i%2==0,i%2!=0] {
            if proof_first {
                let start=Instant::now();let bytes=encode(&source,8*1024*1024,&[]).unwrap().into_boxed_slice().into_vec();let p=ProofRequest::new(&source);
                let elapsed=start.elapsed().as_secs_f64()*1000.;black_box((&p,&bytes));new.push(elapsed);proof_stored=p.retained()+bytes.capacity();
            } else {
                let start=Instant::now();let owned=source.clone();let bytes=encode(&owned,8*1024*1024,&[]).unwrap();let owned=compact_request(owned);let bytes=bytes.into_boxed_slice().into_vec();
                let elapsed=start.elapsed().as_secs_f64()*1000.;black_box((&owned,&bytes));old.push(elapsed);stored=owned_retained(&owned)+bytes.capacity();encoded=bytes.len();
            }
        }}
        old.sort_by(f64::total_cmp);new.sort_by(f64::total_cmp);
        results.push(serde_json::json!({"label":label,"source_bytes":text.len(),"encoded_bytes":encoded,"owned_preparation_median_ms":old[old.len()/2],"proof_preparation_median_ms":new[new.len()/2],"owned_retained_capacity_bytes":stored,"proof_retained_capacity_bytes":proof_stored,"samples_each":old.len()}));
    }
    std::fs::write(std::env::var("PROOF_RESULTS").unwrap(),serde_json::to_vec_pretty(&results).unwrap()).unwrap();
}

#[test]
fn real_session_mutated_input_stale_pair_and_failed_admission() {
    let dir=tempfile::tempdir().unwrap();let started=dir.path().join("started");let release=dir.path().join("release");
    let mut command=Command::new("python3");
    command.arg("-c").arg(r#"import sys,json,pathlib,time,hashlib
root=pathlib.Path(sys.argv[1])
for line in sys.stdin:
 r=json.loads(line);p=r['payload'];text=p['documents'][0]['text']
 if p['revision']==1:
  (root/'started').touch()
  while not (root/'release').exists():time.sleep(.001)
 start,end=(2,3) if text=='éa' else (0,1)
 body={'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'layout_capabilities':['display-list-v2'],'diagnostics':[{'severity':'warning','message':'span','source':{'path':'main.tex','start_byte':start,'end_byte':end}}]}
 print(json.dumps({'protocol_version':1,'type':'compile_result','id':r['id'],'payload':body}),flush=True)
 doc={'path':'main.tex','revision':p['revision'],'sha256':hashlib.sha256(text.encode()).hexdigest(),'byte_length':len(text.encode())}
 print(json.dumps({'protocol_version':2,'type':'display_list','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'render_format':'display-list-v2','documents':[doc]}}),flush=True)
"#).arg(dir.path());
    let mut session=Session::spawn_command_raw_display_prototype(command,Limits {max_frame:2048,timeout:Duration::from_secs(10),..Limits::default()}).unwrap();
    session.set_display_candidates_enabled(true).unwrap();
    let mut caller=request("éa");let original=ProofRequest::new(&caller);
    session.submit_with_capabilities(caller.clone(),vec!["display-list-v2".into()]).unwrap();
    let deadline=Instant::now()+Duration::from_secs(10);
    while !started.exists() {assert!(Instant::now()<deadline);std::thread::yield_now();}
    caller.documents[0].text="aé".into();caller.id="p2".into();caller.revision=2;
    let current=ProofRequest::new(&caller);
    session.submit_with_capabilities(caller.clone(),vec!["display-list-v2".into()]).unwrap();
    let mut invalid=caller.clone();invalid.id="refused".into();invalid.revision=3;invalid.documents[0].text="x".repeat(4096);
    assert!(session.submit_with_capabilities(invalid,vec!["display-list-v2".into()]).is_err());
    assert_eq!(session.latest["p"],(2,"p2".into()));assert_eq!(session.queue.len(),1);
    assert_eq!(original.documents[0].text.hash,Proof::new(&session.active.as_ref().unwrap().request.documents[0].text).hash);
    std::fs::write(release,b"go").unwrap();
    let mut events=Vec::new();
    while session.active.is_some() || !session.queue.is_empty() {events.extend(session.poll());assert!(Instant::now()<deadline);std::thread::yield_now();}
    assert!(events.iter().any(|e|matches!(e,Event::Stale{id,..} if id=="p1")));
    assert!(!events.iter().any(|e|matches!(e,Event::Preview{id,..} if id=="p1")));
    assert!(events.iter().any(|e|matches!(e,Event::Preview{id,..} if id=="p2")));
    let candidate=session.take_current_raw_display_candidate().unwrap();
    assert_eq!(candidate.request_id(),"p2");assert_ne!(original.documents[0].text.hash,current.documents[0].text.hash);
    // This exercises original Session ownership; proof equivalence is separate,
    // not a claim that Pending has already adopted compact proofs.
}
