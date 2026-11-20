//! tex.ch [51.1333], texmfmp.c's `calledit`: `E` at an error prompt runs the
//! editor command of `TEXEDIT` (`%d` the line, `%s` the file name) and the
//! program ends with status 1. XeTeX's `%s` is each UTF-16 unit of the name
//! through xetex.h's `Xchr(x)=x` into a `char`: its low byte. Expected
//! results are TeX Live 2026 XeTeX's (MacTeX 2026, macOS arm64) on the same
//! input: `printf 'E\n' | TEXEDIT='echo ED %d [%s] >edit.out' xetex -ini
//! FILE` writes `ED 2 [t.tex]`, and for `é日.tex` the bytes E9 E5 of
//! `ED 2 [\xe9\xe5.tex]`; both exit 1. Before, the port ended without
//! running the editor. No TeX installation needed.
#![cfg(unix)]

use std::io::Write;
use std::process::{Command, Stdio};

const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\\undefined\n\\end\n";

fn edit(tag: &str, file: &str) -> (Option<i32>, Option<Vec<u8>>, String) {
    let d = std::env::temp_dir().join(format!(
        "flashtex-xetex-call-edit-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(file), DOC).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .args(["-ini", "-no-pdf", file])
        .current_dir(&d)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("TEXEDIT", "echo ED %d [%s] >edit.out")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run flashtex-xetex");
    let _ = child.stdin.take().unwrap().write_all(b"E\n");
    let out = child.wait_with_output().unwrap();
    let edited = std::fs::read(d.join("edit.out")).ok();
    let _ = std::fs::remove_dir_all(&d);
    (
        out.status.code(),
        edited,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn e_at_the_prompt_runs_the_editor() {
    let (code, edited, err) = edit("ascii", "t.tex");
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(edited.as_deref(), Some(&b"ED 2 [t.tex]\n"[..]), "{err}");
    assert!(!err.contains("Trouble"), "{err}");
}

#[test]
fn the_editor_gets_each_utf16_units_low_byte() {
    let (code, edited, err) = edit("unicode", "é日.tex");
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(
        edited.as_deref(),
        Some(&b"ED 2 [\xe9\xe5.tex]\n"[..]),
        "{err}"
    );
}
