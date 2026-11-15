//! `pdf:stream`'s dictionary as dvipdfmx takes it (spc_pdfm.c's
//! spc_handler_pdfm_stream_with_type): the special's string is the
//! stream's decoded data, and `/Length` is dropped from the dictionary, or
//! else `/Filter`. TeX Live 2026's xetex + xdvipdfmx (measured, #2370):
//! `@a`'s stream has no `/ASCIIHexDecode` and its data decodes to
//! `48656C6C6F>`; `@b`, which gives `/Length`, keeps its `/ASCIIHexDecode`.

use flashtex_pdf::reader::{Obj, PdfFile};
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\shipout\hbox to 100pt{\vrule height 10pt\hfil
\special{pdf:stream @a (48656C6C6F>) <</Filter /ASCIIHexDecode>>}%
\special{pdf:stream @b (48656C6C6F>) <</Length 11 /Filter [/ASCIIHexDecode]>>}%
\special{pdf:put @catalog <</A @a /B @b>>}}
\end
"#;

fn has_hex(f: Option<&Obj>) -> bool {
    match f {
        Some(Obj::Name(n)) => n == "ASCIIHexDecode",
        Some(Obj::Array(a)) => a.iter().any(|o| has_hex(Some(o))),
        _ => false,
    }
}

#[test]
fn stream_dictionary_drops_length_or_filter() {
    let dir = std::env::temp_dir().join(format!("flashtex-xetex-spcstream-{}", std::process::id()));
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
    let a = pdf.resolve(cat.get("A").unwrap());
    assert!(!has_hex(a.as_dict().unwrap().get("Filter")), "{a:?}");
    assert_eq!(pdf.decode_stream(a).unwrap(), b"48656C6C6F>");
    let b = pdf.resolve(cat.get("B").unwrap());
    assert!(has_hex(b.as_dict().unwrap().get("Filter")), "{b:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
