//! `pdf:bop` and `pdf:eop` (xdvipdfmx's spc_pdfm.c `spc_handler_pdfm_bop`
//! /`eop`, pdfdoc.c `pdf_doc_set_bop_content`, `pdf_doc_finish_page`,
//! `doc_flush_page`, `pdf_doc_close_page_tree`; issue #2390): a page's
//! `/Contents` is `[bop page eop]`, with the bop and eop in effect when the
//! page ended, or else the document's last ones (pages are flushed when the
//! document closes). Each special makes one stream shared by the pages that
//! use it; an empty one changes nothing; the last ones get a newline.
//!
//! Measured with TeX Live 2026's `xetex -ini` + xdvipdfmx 20260317 on this
//! input: page 1 `[q 0.5 g\n, page, Q\n]`, pages 2 and 3 `[0 0 1 rg, page,
//! 1 0 0 rg]` (one object each, shared), pages 4 and 5 `[q 0.5 g\n, page,
//! Q\n]` (the same objects as page 1's).

use flashtex_pdf::reader::{Obj, PdfFile};
use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6
\font\x="[lmmono10-regular.otf]" at 10pt
\shipout\hbox{\x A}
\shipout\hbox{\x B\special{pdf:bop 0 0 1 rg}\special{pdf:eop 1 0 0 rg}}
\shipout\hbox{\x C\special{pdf:bop}}
\shipout\hbox{\x D\special{pdf:bop  q 0.5 g}\special{pdf:eop Q}}
\shipout\hbox{\x E}
\end
"#;

#[test]
fn bop_and_eop_streams_are_xdvipdfmxs() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-bopeop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let font =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmmono10-regular.otf");
    std::fs::copy(&font, dir.join("lmmono10-regular.otf")).unwrap();
    std::fs::write(dir.join("t.tex"), INPUT).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .expect("run flashtex-xetex");
    let log = std::fs::read_to_string(dir.join("t.log")).unwrap();
    assert!(
        log.contains("Output written on t.pdf (5 pages)."),
        "{log}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&out.stderr).contains("pdf:bop"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(dir.join("t.pdf")).unwrap();
    let pdf = PdfFile::parse(&bytes).unwrap();
    let pages = pdf.pages().unwrap();
    assert_eq!(pages.len(), 5);
    let contents: Vec<Vec<Obj>> = pages
        .iter()
        .map(|p| {
            pdf.page_attr(p, "Contents")
                .and_then(Obj::as_array)
                .expect("a /Contents array")
                .to_vec()
        })
        .collect();
    let data = |o: &Obj| pdf.decode_stream(pdf.resolve(o)).unwrap();
    let want: [(&[u8], &[u8]); 5] = [
        (b"q 0.5 g\n", b"Q\n"),
        (b"0 0 1 rg", b"1 0 0 rg"),
        (b"0 0 1 rg", b"1 0 0 rg"),
        (b"q 0.5 g\n", b"Q\n"),
        (b"q 0.5 g\n", b"Q\n"),
    ];
    for (i, (c, (bop, eop))) in contents.iter().zip(want).enumerate() {
        assert_eq!(c.len(), 3, "page {}", i + 1);
        assert_eq!(data(&c[0]), bop, "page {} bop", i + 1);
        assert_eq!(data(&c[2]), eop, "page {} eop", i + 1);
    }
    // one object per special, shared
    assert_eq!(contents[1][0], contents[2][0]);
    assert_eq!(contents[1][2], contents[2][2]);
    assert_eq!(contents[0][0], contents[3][0]);
    assert_eq!(contents[3][0], contents[4][0]);
    assert_eq!(contents[0][2], contents[4][2]);
    assert_ne!(contents[0][0], contents[1][0]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The page's own stream between a bop and an eop (review of #2482): as
/// xdvipdfmx's (`pdf_dev_bop`, `pdf_dev_eop`), it starts with white space
/// and `q`, sets the colours of the colour stack at the page's start before
/// it draws, and closes its `q`s, so the bop's `0 0 1 rg` colours neither
/// the page's text nor survives into the eop. TeX Live 2026's streams for
/// this input (`xetex -ini` + xdvipdfmx 20260317, `-z 0`):
/// page 1 ` q 1 0 0 1 72 720 cm 0 G 0 g BT ... ET 1 0 0 RG 1 0 0 rg BT ...
/// ET 0 G 0 g Q`, page 2 ` q ... 0 G 0 g 0 1 0 RG 0 1 0 rg BT ... ET Q`,
/// page 3 ` q ... 0 1 0 RG 0 1 0 rg BT ... ET 0 G 0 g Q`: A black, B red,
/// C and D green, and every eop square in the bop's blue.
const COLOURS: &str = r#"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6
\font\x="[lmmono10-regular.otf]" at 10pt
\shipout\hbox{\special{pdf:bop 0 0 1 rg 2 w}\special{pdf:eop 0 0 20 20 re f}\x A\special{color push rgb 1 0 0}\x B\special{color pop}}
\shipout\hbox{\special{color push rgb 0 1 0}\x C}
\shipout\hbox{\x D\special{color pop}}
\end
"#;

#[test]
fn the_page_between_bop_and_eop_keeps_its_own_colours() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-bopeop2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let font =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmmono10-regular.otf");
    std::fs::copy(&font, dir.join("lmmono10-regular.otf")).unwrap();
    std::fs::write(dir.join("t.tex"), COLOURS).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .expect("run flashtex-xetex");
    let log = std::fs::read_to_string(dir.join("t.log")).unwrap();
    assert!(
        log.contains("Output written on t.pdf (3 pages)."),
        "{log}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(dir.join("t.pdf")).unwrap();
    let pdf = PdfFile::parse(&bytes).unwrap();
    let pages = pdf.pages().unwrap();
    // the fill colour of each glyph shown, in order
    let mut shown: Vec<String> = Vec::new();
    for (i, p) in pages.iter().enumerate() {
        let c = pdf
            .page_attr(p, "Contents")
            .and_then(Obj::as_array)
            .expect("[bop page eop]");
        assert_eq!(c.len(), 3, "page {}", i + 1);
        assert_eq!(
            pdf.decode_stream(pdf.resolve(&c[0])).unwrap(),
            b"0 0 1 rg 2 w\n"
        );
        assert_eq!(
            pdf.decode_stream(pdf.resolve(&c[2])).unwrap(),
            b"0 0 20 20 re f\n"
        );
        let page = pdf.decode_stream(pdf.resolve(&c[1])).unwrap();
        assert!(
            page.first().is_some_and(u8::is_ascii_whitespace),
            "page {}: {:?}",
            i + 1,
            String::from_utf8_lossy(&page)
        );
        let s = String::from_utf8_lossy(&page).into_owned();
        let toks: Vec<&str> = s.split_ascii_whitespace().collect();
        assert_eq!(toks.first(), Some(&"q"), "page {}: {s}", i + 1);
        let depth = toks.iter().fold(0i32, |d, t| match *t {
            "q" => d + 1,
            "Q" => {
                assert!(d > 0, "page {}: {s}", i + 1);
                d - 1
            }
            _ => d,
        });
        assert_eq!(depth, 0, "page {}: the page's q left open: {s}", i + 1);
        // the colours of the page's start, right after its `q`
        let start: &[&str] = match i {
            0 | 1 => &["0", "G", "0", "g"],
            _ => &["0", "1", "0", "RG", "0", "1", "0", "rg"],
        };
        assert_eq!(&toks[1..1 + start.len()], start, "page {}: {s}", i + 1);
        let mut fill = None;
        for (k, t) in toks.iter().enumerate() {
            match *t {
                "g" => fill = Some(toks[k - 1..=k].join(" ")),
                "rg" => fill = Some(toks[k - 3..=k].join(" ")),
                t if t.ends_with("Tj") || t.ends_with("TJ") => {
                    shown.push(fill.clone().unwrap_or_else(|| "none".into()))
                }
                _ => {}
            }
        }
    }
    assert_eq!(shown, ["0 g", "1 0 0 rg", "0 1 0 rg", "0 1 0 rg"]);
    let _ = std::fs::remove_dir_all(&dir);
}
