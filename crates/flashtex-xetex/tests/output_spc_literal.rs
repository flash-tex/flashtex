//! `pdf:literal`'s keywords as dvipdfmx reads them (spc_pdfm.c's
//! spc_handler_pdfm_literal): `reverse` and `direct` in any order. TeX Live
//! 2026's xetex + xdvipdfmx on this input writes `0.5 g` and `0.25 g`
//! directly (measured); the writer had written the second keyword into the
//! content stream as an operator (`direct`, `reverse`).

use flashtex_pdf::reader::PdfFile;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\shipout\hbox{%
\special{pdf:literal reverse direct 0.5 g}\vrule width 10pt height 10pt
\special{pdf:literal direct reverse 0.25 g}\vrule width 10pt height 10pt}
\end
"#;

#[test]
fn literal_keywords_in_any_order() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spclit-{}", std::process::id()));
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
    let content = String::from_utf8(pdf.page_content(pages[0]).unwrap()).unwrap();
    let a = content.find("0.5 g").expect(&content);
    let b = content.find("0.25 g").expect(&content);
    assert!(a < b, "{content}");
    assert!(
        !content.contains("direct") && !content.contains("reverse"),
        "{content}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
