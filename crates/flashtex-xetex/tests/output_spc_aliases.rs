//! dvipdfmx's other names of its `pdf:` specials (spc_pdfm.c's
//! `pdfm_handlers`): `pdf:bc`, `pdf:sc`, `pdf:ec`, `pdf:bg`, `pdf:eg` are
//! colour specials as `pdf:bcolor`, `pdf:scolor`, `pdf:ecolor` are. TeX
//! Live 2026's xetex + xdvipdfmx on this input writes the page's rules
//! `1 0 0 rg`, `0.5 g` and `0 0 1 rg` (measured); the writer had reported
//! the five as unknown and drawn all three black.

use flashtex_pdf::reader::PdfFile;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\pdfpagewidth=612bp \pdfpageheight=792bp
\shipout\hbox{%
\special{pdf:bc [1 0 0]}\vrule width 10pt height 10pt\special{pdf:ec}%
\special{pdf:bg [0.5]}\vrule width 10pt height 10pt\special{pdf:eg}%
\special{pdf:sc [0 0 1]}\vrule width 10pt height 10pt}
\end
"#;

#[test]
fn pdf_special_aliases() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcalias-{}", std::process::id()));
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
    let pages = pdf.pages().unwrap();
    let content = String::from_utf8(pdf.page_content(pages[0]).unwrap()).unwrap();
    let red = content.find("1 0 0 rg").expect(&content);
    let gray = content.find("0.5 g").expect(&content);
    let blue = content.find("0 0 1 rg").expect(&content);
    assert!(red < gray && gray < blue, "{content}");
    let _ = std::fs::remove_dir_all(&dir);
}
