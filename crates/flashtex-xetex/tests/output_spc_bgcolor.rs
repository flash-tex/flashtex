//! The page background as dvipdfmx keeps it (pdfdoc.c's
//! pdf_doc_set_bgcolor / doc_fill_page_background): one colour, the last
//! one given, filled at the end of each page and kept for the following
//! pages; white is no background. TeX Live 2026's xetex + xdvipdfmx on
//! this input (measured): pages 1 and 2 `0 1 0 rg` (not the yellow given
//! first), page 3 none, pages 4 and 5 `0 0 1 rg`.

use flashtex_pdf::reader::PdfFile;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\pdfpagewidth=612bp \pdfpageheight=792bp
\def\r{\vrule width 10pt height 10pt}
\shipout\hbox{\special{background rgb 1 1 0}\special{background rgb 0 1 0}\r}
\shipout\hbox{\r}
\shipout\hbox{\special{background gray 1}\r}
\shipout\hbox{\special{pdf:bgcolor [0 0 1]}\r}
\shipout\hbox{\r}
\end
"#;

#[test]
fn background_lasts_across_pages() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcbg-{}", std::process::id()));
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
    assert_eq!(pages.len(), 5);
    let want = [
        Some("0 1 0 rg"),
        Some("0 1 0 rg"),
        None,
        Some("0 0 1 rg"),
        Some("0 0 1 rg"),
    ];
    for (i, (pg, w)) in pages.iter().zip(want).enumerate() {
        let c = String::from_utf8(pdf.page_content(pg).unwrap()).unwrap();
        let fills: Vec<&str> = c.lines().filter(|l| l.ends_with(" rg")).collect();
        match w {
            Some(w) => assert_eq!(fills, [w], "page {}: {c}", i + 1),
            None => assert!(fills.is_empty(), "page {}: {c}", i + 1),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
