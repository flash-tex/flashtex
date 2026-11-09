//! Named objects as dvipdfmx keeps them (pdfnames.c's
//! pdf_names_add_object, spc_pdfm.c's spc_handler_pdfm_put): a second
//! `pdf:obj @x` is ignored ("Object @x already defined."), and `pdf:put`
//! to a name not yet defined makes nothing. TeX Live 2026's xetex +
//! xdvipdfmx on this input writes `/X <</First 1>>`, `/Y <</Obj 4>>` and
//! `/Z [1 2 (s)]` into the catalog (measured).

use flashtex_pdf::reader::{Obj, PdfFile};
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\shipout\hbox{%
\special{pdf:obj @x <</First 1>>}%
\special{pdf:obj @x <</Second 2>>}%
\special{pdf:put @y <</Put 3>>}%
\special{pdf:obj @y <</Obj 4>>}%
\special{pdf:obj @z [1]}%
\special{pdf:put @z 2 (s)}%
\special{pdf:put @catalog <</X @x /Y @y /Z @z>>}}
\end
"#;

#[test]
fn named_objects_keep_their_first_definition() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcnamed-{}", std::process::id()));
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
    let x = pdf.resolve(cat.get("X").unwrap()).as_dict().unwrap();
    assert!(x.get("First").is_some() && x.get("Second").is_none());
    let y = pdf.resolve(cat.get("Y").unwrap()).as_dict().unwrap();
    assert!(y.get("Obj").is_some() && y.get("Put").is_none());
    let z = pdf.resolve(cat.get("Z").unwrap()).as_array().unwrap();
    assert_eq!(z.len(), 3);
    assert_eq!(z[2], Obj::String(b"s".to_vec()));
    let _ = std::fs::remove_dir_all(&dir);
}
