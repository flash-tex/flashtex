//! `pdf:names` and `pdf:dest` as dvipdfmx reads them (spc_pdfm.c's
//! spc_handler_pdfm_names and spc_handler_pdfm_dest). TeX Live 2026's
//! xetex + xdvipdfmx on this input (measured): the catalog's `/Names` has
//! a `/JavaScript` tree `[(js1) n 0 R]` whose value is the action, and no
//! `/Dests`: a name (`/nm`) is not a destination's name and `@thispage`
//! alone is not a destination.

use flashtex_pdf::reader::{Obj, PdfFile};
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\shipout\hbox{%
\special{pdf:names /JavaScript [(js1) <</S /JavaScript /JS (app.alert(1))>>]}%
\special{pdf:dest /nm [@thispage /Fit]}%
\special{pdf:dest (dd) @thispage}%
\vrule width 10pt height 10pt}
\end
"#;

#[test]
fn names_and_dest_specials() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcnames-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.tex"), INPUT).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .expect("run flashtex-xetex");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("unknown special"), "{stdout}");
    let pdf = PdfFile::parse(&std::fs::read(dir.join("t.pdf")).unwrap()).unwrap();
    let cat = pdf.catalog().unwrap();
    let names = pdf.resolve(cat.get("Names").unwrap()).as_dict().unwrap();
    assert!(names.get("Dests").is_none(), "{names:?}");
    let js = pdf
        .resolve(names.get("JavaScript").unwrap())
        .as_dict()
        .unwrap();
    let list = js.get("Names").and_then(Obj::as_array).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0], Obj::String(b"js1".to_vec()));
    assert!(matches!(list[1], Obj::Ref(..)), "{list:?}");
    let action = pdf.resolve(&list[1]).as_dict().unwrap();
    assert_eq!(action.get("S"), Some(&Obj::Name("JavaScript".into())));
    let _ = std::fs::remove_dir_all(&dir);
}
