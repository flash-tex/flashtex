//! `pdf:outline` depths as dvipdfmx counts them (spc_pdfm.c's
//! spc_handler_pdfm_outline, pdfdoc.c's pdf_doc_bookmarks_down): from the
//! lowest level seen so far, with a `<No Title>` item for a level an item
//! jumps over. TeX Live 2026's xetex + xdvipdfmx on this input (measured):
//! the root's `/Count 5`; A2 > B3, C1 > `<No Title>` (`/Count -1`,
//! `/C [1 0 0]`, `/F 1`, a JavaScript action) > D3, E1 > F2, G2.

use flashtex_pdf::reader::{Obj, PdfFile};
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\shipout\hbox{%
\special{pdf:outline [] 2 <</Title (A2) /Dest [@thispage /Fit]>>}%
\special{pdf:outline [] 3 <</Title (B3) /Dest [@thispage /Fit]>>}%
\special{pdf:outline [] 1 <</Title (C1) /Dest [@thispage /Fit]>>}%
\special{pdf:outline [] 3 <</Title (D3) /Dest [@thispage /Fit]>>}%
\special{pdf:outline [-] 1 <</Title (E1) /Dest [@thispage /Fit]>>}%
\special{pdf:outline 2 <</Title (F2) /Dest [@thispage /Fit]>>}%
\special{pdf:outline [-] 2 <</Title (G2) /Dest [@thispage /Fit]>>}}
\end
"#;

fn title(d: &std::collections::BTreeMap<String, Obj>) -> Vec<u8> {
    match d.get("Title") {
        Some(Obj::String(s)) => s.clone(),
        o => panic!("{o:?}"),
    }
}

#[test]
fn outline_depths_and_empty_items() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcout-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.tex"), INPUT).unwrap();
    Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .expect("run flashtex-xetex");
    let pdf = PdfFile::parse(&std::fs::read(dir.join("t.pdf")).unwrap()).unwrap();
    let cat = pdf.catalog().unwrap();
    let root = pdf.resolve(cat.get("Outlines").unwrap()).as_dict().unwrap();
    assert_eq!(root.get("Count").and_then(Obj::as_number), Some("5"));
    // the top level: A2, C1, E1
    let mut tops = Vec::new();
    let mut at = root.get("First").cloned();
    while let Some(r) = at {
        let d = pdf.resolve(&r).as_dict().unwrap().clone();
        at = d.get("Next").cloned();
        tops.push(d);
    }
    let names: Vec<Vec<u8>> = tops.iter().map(title).collect();
    assert_eq!(names, [b"A2".to_vec(), b"C1".to_vec(), b"E1".to_vec()]);
    let c1 = &tops[1];
    assert_eq!(c1.get("Count").and_then(Obj::as_number), Some("1"));
    let empty = pdf.resolve(c1.get("First").unwrap()).as_dict().unwrap();
    assert_eq!(title(empty), b"<No Title>");
    assert_eq!(empty.get("Count").and_then(Obj::as_number), Some("-1"));
    assert_eq!(empty.get("F").and_then(Obj::as_number), Some("1"));
    let d3 = pdf.resolve(empty.get("First").unwrap()).as_dict().unwrap();
    assert_eq!(title(d3), b"D3");
    let _ = std::fs::remove_dir_all(&dir);
}
