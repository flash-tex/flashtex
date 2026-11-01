//! Isolated test consumer of the real bridge library; fake conversion, no provider calls.
use flashtex_bridge::{store::Store, *};
use serde_json::{json, Value};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Cursor;

struct Fake;
impl Converter for Fake {
    fn convert(&self, _: &CaptureSubmit, _: &Context) -> Result<Proposal> {
        Ok(Proposal { latex: "$x$".into(), ambiguities: vec![], required_dependencies: vec![] })
    }
}
fn cap() -> CaptureSubmit {
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(1,1).write_to(&mut png,image::ImageFormat::Png).unwrap();
    CaptureSubmit { capture_id:"capture".into(),destination_id:"target".into(),base_revision:1,
        image:CaptureImage{mime_type:"image/png".into(),data_base64:STANDARD.encode(png.into_inner())},instructions:"Preserve notation".into() }
}
fn open(b:&mut Bridge,path:&str,revision:u64,text:&str) {
    b.open_document(Document{project_id:"corpus".into(),path:path.into(),revision,text:text.into()}).unwrap();
}
fn compile_request(id:&str,path:&str,revision:u64,text:&str)->Value {
    json!({"protocol_version":1,"id":id,"type":"compile","payload":{"project_id":"corpus","entry_path":path,"revision":revision,"documents":[{"path":path,"text":text}]}})
}
fn main() {
    let mut results=Vec::new();
    {
        let dir=tempfile::tempdir().unwrap();let mut b=Bridge::new(Store::open(dir.path()).unwrap());
        let root="\\newcommand{\\energy}{E=mc^2}\n\\input{parts/section}";
        let child="Selected source";
        open(&mut b,"main.tex",1,root);open(&mut b,"parts/section.tex",1,child);
        b.pin("target","corpus","parts/section.tex",1,0,8).unwrap();b.receive(cap()).unwrap();
        let context=b.context(&cap(),vec![]).unwrap();
        let root_definition_present=context.definitions.iter().any(|d|d.contains("\\energy"));
        results.push(json!({"scenario":"included-file-context","status":if root_definition_present{"pass"}else{"fail"},
            "scope":"required complete project context remains absent; contract currently promises only local excerpts",
            "context":context,"expected_root_definition":"\\newcommand{\\energy}{E=mc^2}",
            "compile_request":{"protocol_version":1,"id":"included-context","type":"compile","payload":{"project_id":"corpus","revision":1,"entry_path":"main.tex","documents":[{"path":"main.tex","text":root},{"path":"parts/section.tex","text":child}]}}}));
    }
    {
        let dir=tempfile::tempdir().unwrap();let mut b=Bridge::new(Store::open(dir.path()).unwrap());
        let text="\\newcommand{\\symbol}{a}\nTarget";
        open(&mut b,"main.tex",1,text);b.pin("target","corpus","main.tex",1,text.len(),text.len()).unwrap();b.receive(cap()).unwrap();
        let original=b.convert("capture",vec![],&Fake).unwrap();
        let index=text.find("{a}").unwrap()+1;b.edit("corpus","main.tex",1,2,index,index+1,"b").unwrap();
        let edit=b.prepare_insert("capture",2,true).unwrap();
        let mut candidate=b.document("corpus","main.tex").unwrap().text.clone();
        candidate.replace_range(edit.start_byte..edit.end_byte,&edit.replacement);
        results.push(json!({"compile_request":compile_request("macro-candidate","main.tex",3,&candidate),"scenario":"macro-context-changed","status":"unverified","context_revision":original.context.unwrap().revision,
            "prepared":edit,"current_source":b.document("corpus","main.tex").unwrap(),
            "required_native_gate":"Show stale conversion context and compile candidate against current source before approval; bridge allows newer reviewed revision."}));
    }
    {
        let dir=tempfile::tempdir().unwrap();let mut b=Bridge::new(Store::open(dir.path()).unwrap());
        open(&mut b,"main.tex",1,"αβ world");b.pin("target","corpus","main.tex",1,5,5).unwrap();b.receive(cap()).unwrap();
        b.edit("corpus","main.tex",1,2,0,0,"東京").unwrap();b.convert("capture",vec![],&Fake).unwrap();
        let edit=b.prepare_insert("capture",2,true).unwrap();assert_eq!(edit.start_byte,11);
        assert_eq!(edit.document_before_sha256,digest("東京αβ world".as_bytes()));
        b.confirm_insert("capture",&edit.edit_id,3).unwrap();let text=&b.document("corpus","main.tex").unwrap().text;
        assert_eq!(text,"東京αβ $x$world");
        results.push(json!({"scenario":"multibyte-rebase","status":"pass","prepared":edit,"source":text,
            "compile_request":compile_request("rebase","main.tex",3,text)}));
    }
    {
        let dir=tempfile::tempdir().unwrap();let mut b=Bridge::new(Store::open(dir.path()).unwrap());
        open(&mut b,"main.tex",1,"αβ world");b.pin("target","corpus","main.tex",1,5,5).unwrap();b.receive(cap()).unwrap();b.convert("capture",vec![],&Fake).unwrap();
        let edit=b.prepare_insert("capture",1,true).unwrap();let receipt=b.confirm_insert("capture",&edit.edit_id,2).unwrap();
        let after=b.document("corpus","main.tex").unwrap().text.clone();drop(b);
        let mut restored=Bridge::new(Store::open(dir.path()).unwrap());open(&mut restored,"main.tex",2,&after);
        let retry=restored.confirm_insert("capture",&edit.edit_id,2).unwrap();assert_eq!(receipt,retry);
        assert_eq!(restored.document("corpus","main.tex").unwrap().text,after);
        results.push(json!({"scenario":"confirm-restart-idempotency","status":"unverified","bridge_receipt_retry":"pass",
            "native_ledger":"unverified: this simulates restored durable source; no native transaction/crash injection executed",
            "prepared":edit,"receipt":receipt,"compile_request":compile_request("receipt","main.tex",2,&after)}));
    }
    println!("{}",json!({"schema_version":1,"converter":"deterministic fake; no provider request", "results":results}));
}
