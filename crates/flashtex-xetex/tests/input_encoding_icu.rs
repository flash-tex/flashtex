//! `\XeTeXinputencoding` and `\XeTeXdefaultencoding` with ICU converter
//! names (issue #2382): TeX Live's xetex opens the name with ICU's
//! `ucnv_open` and converts each line's bytes, read up to a LF or CR, with
//! `ucnv_toAlgorithmic` (XeTeX_ext.c's `input_line`). The cases and TeX
//! Live 2026's logs and XDV files are in tests/data/input-encoding (see its
//! README): single-byte and multi-byte tables (with extension tables,
//! SI/SO and GB 18030), ICU's name matching, the UTF-8, CESU-8,
//! UTF-16 and UTF-32 converters with ill-formed input, and documents that
//! switch to UTF-16. Before the port had ICU's converters it read every
//! such name but the UTF-8 and Latin-1 ones as bytes, after an "Unknown
//! encoding" warning.

use std::path::{Path, PathBuf};
use std::process::Command;

fn cases() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/input-encoding")
}

/// The log from its `**` line on, `(./` as `(`.
fn log_body(log: &[u8]) -> String {
    let s = String::from_utf8_lossy(log);
    let at = s.find("\n**").map_or(0, |i| i + 1);
    s[at..].replace("(./", "(")
}

fn run_case(name: &str) -> Result<(), String> {
    let src = cases().join(name);
    let dir =
        std::env::temp_dir().join(format!("flashtex-xetex-icu-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(&src).unwrap() {
        let p = e.unwrap().path();
        let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
        if ext == "tex" || ext == "dat" {
            std::fs::copy(&p, dir.join(p.file_name().unwrap())).unwrap();
        }
    }
    Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args([
            "-ini",
            "-etex",
            "-no-pdf",
            "-interaction=nonstopmode",
            "p.tex",
        ])
        .output()
        .expect("run flashtex-xetex");
    let want = log_body(&std::fs::read(src.join("p.log")).unwrap());
    let got = log_body(&std::fs::read(dir.join("p.log")).unwrap_or_default());
    let mut errs = Vec::new();
    if got != want {
        let line = want
            .lines()
            .zip(got.lines())
            .position(|(a, b)| a != b)
            .unwrap_or(want.lines().count().min(got.lines().count()));
        errs.push(format!(
            "{name}: the log differs from line {} on:\nTeX Live: {:?}\nport:     {:?}",
            line + 1,
            want.lines().nth(line),
            got.lines().nth(line)
        ));
    }
    let want_xdv = std::fs::read(src.join("p.xdv")).ok();
    let got_xdv = std::fs::read(dir.join("p.xdv")).ok();
    if want_xdv != got_xdv {
        errs.push(format!(
            "{name}: the XDV file differs (TeX Live {:?} bytes, port {:?})",
            want_xdv.map(|x| x.len()),
            got_xdv.map(|x| x.len())
        ));
    }
    let _ = std::fs::remove_dir_all(&dir);
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("\n"))
    }
}

fn check(names: &[&str]) {
    let errs: Vec<String> = names.iter().filter_map(|n| run_case(n).err()).collect();
    assert!(errs.is_empty(), "{}", errs.join("\n\n"));
}

#[test]
fn single_byte_converters_read_as_tex_live() {
    check(&["sbcs", "cp1252-text", "cr-lines"]);
}

#[test]
fn multi_byte_converters_read_as_tex_live() {
    check(&["mbcs"]);
}

#[test]
fn unicode_converters_read_as_tex_live() {
    check(&["unicode", "cesu-real", "utf8-bad"]);
}

#[test]
fn utf16_and_utf32_documents_run_as_tex_live() {
    check(&[
        "le-ascii",
        "unmarked-ascii",
        "be-ascii",
        "xbe-ascii",
        "u16-ascii",
        "ucs2-ascii",
        "u32-ascii",
        "be-real",
        "le-real",
        "u16-real-bom",
        "u32be-real",
        "u32le-real",
        "be-page",
        "default-input",
    ]);
}

#[test]
fn every_case_is_tested() {
    let mut on_disk: Vec<String> = std::fs::read_dir(cases())
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            e.path()
                .is_dir()
                .then(|| e.file_name().into_string().unwrap())
        })
        .collect();
    on_disk.sort();
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/input_encoding_icu.rs"),
    )
    .unwrap();
    for d in on_disk {
        assert!(src.contains(&format!("\"{d}\"")), "case {d} is not run");
    }
}
