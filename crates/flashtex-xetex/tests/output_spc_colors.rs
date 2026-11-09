//! Colour specials as dvipdfmx reads them (spc_util.c): its dvips colour
//! names (color.sty's `named` model writes `color push  Apricot` under
//! xetex.def), `pdf:bcolor fill C stroke C'`, and the `color` special's
//! narrower syntax. TeX Live 2026's xetex + xdvipdfmx on this input fills
//! the rules `0 0.32 0.52 0 k`, `0.5 g`, `0.98 0.13 0 0.43 k`, black
//! twice (`grey 0.5` and a bare `0.25` are not colours there), then
//! `1 0 0 rg` (measured).

use flashtex_pdf::reader::PdfFile;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\pdfpagewidth=612bp \pdfpageheight=792bp
\def\r{\vrule width 10pt height 10pt}
\shipout\hbox{%
\special{color push  Apricot}\r\special{color pop}%
\special{color push Gray}\r\special{color pop}%
\special{pdf:bcolor MidnightBlue}\r\special{pdf:ecolor}%
\special{color push grey 0.5}\r\special{color pop}%
\special{color push 0.25}\r\special{color pop}%
\special{pdf:bcolor fill [1 0 0] stroke [0 1 0]}\r\special{pdf:ecolor}}
\end
"#;

#[test]
fn colour_specials_as_dvipdfmx_reads_them() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spccol-{}", std::process::id()));
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
    let mut at = 0;
    for want in ["0 0.32 0.52 0 k", "0.5 g", "0.98 0.13 0 0.43 k", "1 0 0 rg"] {
        let i = content[at..].find(want).expect(&content);
        at += i + want.len();
    }
    assert!(!content.contains("0.25 g"), "{content}");
    assert_eq!(content.matches("0.5 g").count(), 1, "{content}");
    let _ = std::fs::remove_dir_all(&dir);
}
