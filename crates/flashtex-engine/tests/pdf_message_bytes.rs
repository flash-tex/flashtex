//! pdfTeX's backend messages keep their bytes (lane REVIEW-PDF), against
//! TeX Live's own `pdftex`.
//!
//! utils.c's `pdftex_fail` prints its message with `safe_print`, `print(*c)`
//! of each `const char`: on macOS and x86-64 Linux `char` is signed, so a
//! byte from 0x80 up is a negative character code and tex.web's `print`
//! writes `???` for it. `pdftex_warn` prints the bytes themselves. The
//! engine printed each byte as an unsigned code (`^^e9`), and built several
//! messages through `String::from_utf8_lossy`, so a byte that is not UTF-8
//! became U+FFFD's three bytes: an `.enc` line or a font line quoted in a
//! font error, a glyph name, an included PDF's resource key, a named
//! destination. Each case is a plain-TeX job; exit status, the PDF and the
//! log from the `**` line on (lines joined, the program name normalised)
//! must agree. Skips where there is no TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const SETUP: &[u8] = b"\\pdfoutput=1 \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 \
                       \\pdfsuppressptexinfo=-1 ";

/// (name, body after `SETUP`); `files` is the shared directory of the
/// `.enc` files (named by its absolute path, so both logs name one file).
fn cases(files: &Path) -> Vec<(&'static str, Vec<u8>)> {
    let f = files.display().to_string();
    let enc = |name: &str| {
        format!("\\pdfmapline{{=cmr10 CMR10 <{f}/{name} <cmr10.pfb}}A\\bye").into_bytes()
    };
    vec![
        // pdftex_fail quoting an .enc line: `???` for the byte
        ("enc-names", enc("latin1-names.enc")),
        ("enc-bracket", enc("latin1-bracket.enc")),
        // pdftex_warn: the byte itself
        (
            "pdf-key",
            b"\\pdfximage{key.pdf}\\pdfrefximage\\pdflastximage\\bye".to_vec(),
        ),
        // pdftex_fail with the destination's name
        (
            "pdf-dest",
            b"\\pdfximage named {d\xe9st}{key.pdf}\\pdfrefximage\\pdflastximage\\bye".to_vec(),
        ),
    ]
}

/// A one-page PDF whose resources hold `/X#E9y 5` (a key with byte 0xE9
/// that is not a dictionary) and no named destinations.
fn key_pdf() -> Vec<u8> {
    let objs: [&[u8]; 4] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] \
          /Resources << /X#E9y 5 /ProcSet [/PDF] >> /Contents 4 0 R >>",
        b"<< /Length 15 >>\nstream\n0 0 m 10 10 l S\nendstream",
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![];
    for (i, body) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn run(bin: &Path, dir: &Path, args: &[&str], ours: bool) -> Option<i32> {
    let mut c = Command::new(bin);
    c.args(args)
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    c.status().unwrap().code()
}

/// The log from the `**` line on, lines joined, with the program name
/// after "pdfTeX warning: " and "!pdfTeX error: " replaced (as bytes: the
/// log holds the bytes the messages had).
fn normalised_log(log: &[u8]) -> Vec<u8> {
    let from = log
        .windows(3)
        .position(|w| w == b"\n**")
        .map_or(0, |i| i + 1);
    let joined: Vec<u8> = log[from..]
        .iter()
        .copied()
        .filter(|&b| b != b'\n')
        .collect();
    let mut out = vec![];
    let mut rest = &joined[..];
    'scan: while !rest.is_empty() {
        for key in [&b"pdfTeX warning: "[..], b"!pdfTeX error: "] {
            if rest.starts_with(key) {
                out.extend_from_slice(key);
                rest = &rest[key.len()..];
                let end = rest
                    .iter()
                    .position(|&b| b == b' ' || b == b':')
                    .unwrap_or(rest.len());
                out.extend_from_slice(b"PROG");
                rest = &rest[end..];
                continue 'scan;
            }
        }
        out.push(rest[0]);
        rest = &rest[1..];
    }
    out
}

#[test]
fn backend_messages_keep_their_bytes() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-msgbytes");
    let (a, b) = (base.join("ours"), base.join("tex"));
    let files = base.join("files");
    std::fs::create_dir_all(&files).unwrap();
    std::fs::write(files.join("latin1-names.enc"), b"/T [ /a \xe9x ] def\n").unwrap();
    std::fs::write(files.join("latin1-bracket.enc"), b"T\xe9 [ /a ] def\n").unwrap();
    let cases = cases(&files);
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(d.join("key.pdf"), key_pdf()).unwrap();
        for (name, body) in &cases {
            std::fs::write(d.join(format!("{name}.tex")), [SETUP, body, b"\n"].concat()).unwrap();
        }
    }
    run(ours, &a, &["-ini", "\\input plain \\dump"], true);
    run(&theirs, &b, &["-ini", "\\input plain \\dump"], false);
    let mut failures = vec![];
    for (name, _) in &cases {
        let arg = format!("&plain {name}");
        let rx = run(ours, &a, &["-fmt=plain", &arg], true);
        let ry = run(&theirs, &b, &[&arg], false);
        if rx != ry {
            failures.push(format!("{name}: exit {rx:?} vs TeX Live's {ry:?}"));
        }
        let pdf = format!("{name}.pdf");
        if std::fs::read(a.join(&pdf)).ok() != std::fs::read(b.join(&pdf)).ok() {
            failures.push(format!("{pdf}: differs (or one engine wrote none)"));
        }
        let log = format!("{name}.log");
        let lx = normalised_log(&std::fs::read(a.join(&log)).unwrap_or_default());
        let ly = normalised_log(&std::fs::read(b.join(&log)).unwrap_or_default());
        if lx != ly || lx.is_empty() {
            failures.push(format!(
                "{log}:\n  ours {:?}\n  tex  {:?}",
                String::from_utf8_lossy(&lx),
                String::from_utf8_lossy(&ly)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
