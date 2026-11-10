//! `\XeTeXinputencoding` with ICU's alias spellings of UTF-8 and ISO-8859-1
//! (issue #2382): TeX Live's xetex opens them with ICU's `ucnv_open`, whose
//! names compare ignoring case and `-`, `_` and ` `. The expected log lines
//! are TeX Live 2026's `xetex -ini -etex -no-pdf` output for the same input
//! (MacTeX 2026, verified 2026-11-09).

use std::path::PathBuf;
use std::process::Command;

fn run_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-encalias-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Each name, then a line with `é`: UTF-8 bytes after the UTF-8 names,
/// the byte 0xE9 after the Latin-1 names.
fn input() -> Vec<u8> {
    let mut doc = b"\\catcode`\\{=1 \\catcode`\\}=2\n".to_vec();
    let utf8 = [
        "\"UTF-8\"",
        "utf-8",
        "UTF8",
        "\"unicode-1-1-utf-8\"",
        "\"x-UTF_8J\"",
        "\"windows-65001\"",
        "cp1208",
        "\"U T F 8\"",
    ];
    let latin1 = [
        "latin1",
        "\"ISO-8859-1\"",
        "l1",
        "cp819",
        "IBM819",
        "csISOLatin1",
        "\"iso-ir-100\"",
        "\"ISO_8859-1:1987\"",
        "8859_1",
        "819",
        "\"ibm-819\"",
        "\"Latin-1\"",
        "\"iso88591\"",
    ];
    let utf8_e: &[u8] = &[0xc3, 0xa9];
    let latin1_e: &[u8] = &[0xe9];
    for (names, e_acute) in [(&utf8[..], utf8_e), (&latin1[..], latin1_e)] {
        for n in names {
            doc.extend_from_slice(format!("\\XeTeXinputencoding {n}\n").as_bytes());
            let shown = n.replace('"', "");
            doc.extend_from_slice(format!("\\message{{{shown}}}\\message{{[\\number`").as_bytes());
            doc.extend_from_slice(e_acute);
            doc.extend_from_slice(b"]}\n");
        }
    }
    doc.extend_from_slice(b"\\XeTeXinputencoding \"utf8\"\n\\end\n");
    doc
}

#[test]
fn icu_aliases_of_utf8_and_latin1_decode_input() {
    let dir = run_dir();
    std::fs::write(dir.join("a.tex"), input()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args([
            "-ini",
            "-etex",
            "-no-pdf",
            "-interaction=nonstopmode",
            "a.tex",
        ])
        .output()
        .expect("run flashtex-xetex");
    let log = std::fs::read_to_string(dir.join("a.log")).unwrap();
    // xetex prints `(./a.tex`; the cwd resolver names the file `a.tex`.
    let want = "a.tex UTF-8 [233] utf-8 [233] UTF8 [233] unicode-1-1-utf-8 [233] x-UTF_8J
[233] windows-65001 [233] cp1208 [233] U T F 8 [233] latin1 [233] ISO-8859-1
[233] l1 [233] cp819 [233] IBM819 [233] csISOLatin1 [233] iso-ir-100 [233]
ISO_8859-1:1987 [233] 8859_1 [233] 819 [233] ibm-819 [233] Latin-1 [233]
iso88591 [233] )
No pages of output.";
    assert!(log.contains(want), "want\n{want}\nin:\n{log}");
    assert!(!log.contains("Unknown encoding"), "{log}");
    assert_eq!(out.status.code(), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}
