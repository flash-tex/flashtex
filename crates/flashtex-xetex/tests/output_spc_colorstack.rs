//! dvipdfmx's colour stack (pdfcolor.c): `color SPEC` sets its bottom
//! entry, a pop at the bottom is ignored, and a push beyond 128 entries
//! is ignored. TeX Live 2026's xetex + xdvipdfmx on this input fills the
//! three rules of page 1 `0.5 g` and the rule of page 2 `0.25 g`
//! (measured).

use flashtex_pdf::reader::PdfFile;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\def\r{\vrule width 10pt height 10pt}
\shipout\hbox{%
\special{color gray 0.5}\r\special{color pop}\r
\special{color push rgb 1 0 0}\special{color pop}\special{color pop}\r}
\count1=0
\def\p{\special{color push gray 0.25}\advance\count1 by 1
  \ifnum\count1<127 \expandafter\p\fi}
\shipout\hbox{\p\special{color push rgb 1 0 0}\r}
\end
"#;

#[test]
fn colour_stack_bottom_and_limits() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcstack-{}", std::process::id()));
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
    let one = String::from_utf8(pdf.page_content(pages[0]).unwrap()).unwrap();
    assert!(one.contains("0.5 g"), "{one}");
    assert!(!one.lines().any(|l| l.trim() == "0 g"), "{one}");
    let two = String::from_utf8(pdf.page_content(pages[1]).unwrap()).unwrap();
    assert!(two.contains("0.25 g"), "{two}");
    assert!(!two.contains("1 0 0 rg"), "{two}");
    let _ = std::fs::remove_dir_all(&dir);
}
