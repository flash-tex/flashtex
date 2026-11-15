//! `pdf:put @resources` as dvipdfmx merges it (spc_pdfm.c's
//! safeputresdict): a category's entries join the page's, and an entry
//! already there keeps its value. TeX Live 2026's xetex + xdvipdfmx on
//! this input (measured): `/ExtGState <</GSa <</ca .5>> /GSb <</CA .25>>>>`
//! and `/Pattern <</P1 <</PatternType 2>>>>`.

use flashtex_pdf::reader::{Obj, PdfFile};
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\shipout\hbox{%
\special{pdf:put @resources <</ExtGState <</GSa <</ca 0.5>> >> >>}%
\special{pdf:put @resources <</ExtGState <</GSb <</CA 0.25>> /GSa <</ca 0.75>> >> /Pattern <</P1 <</PatternType 2>> >> >>}%
\vrule width 10pt height 10pt}
\end
"#;

fn num(o: Option<&Obj>) -> f64 {
    o.and_then(Obj::as_number).unwrap().parse().unwrap()
}

#[test]
fn put_resources_keeps_entries_already_there() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcres-{}", std::process::id()));
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
    let pages = pdf.pages().unwrap();
    let res = pdf
        .resolve(pdf.page_attr(pages[0], "Resources").unwrap())
        .as_dict()
        .unwrap();
    let gs = pdf
        .resolve(res.get("ExtGState").unwrap())
        .as_dict()
        .unwrap();
    let gsa = pdf.resolve(gs.get("GSa").unwrap()).as_dict().unwrap();
    assert_eq!(num(gsa.get("ca")), 0.5);
    let gsb = pdf.resolve(gs.get("GSb").unwrap()).as_dict().unwrap();
    assert_eq!(num(gsb.get("CA")), 0.25);
    let pat = pdf.resolve(res.get("Pattern").unwrap()).as_dict().unwrap();
    assert!(pat.get("P1").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}
