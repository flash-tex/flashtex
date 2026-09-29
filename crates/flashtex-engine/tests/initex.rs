//! End-to-end checks of the INITEX binary, with no TeX installation needed.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-engine-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn initex(dir: &Path, first_line: &str) -> (bool, String) {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("tex.pool");
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .arg(first_line)
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn relax_end() {
    let d = scratch("relax");
    let (ok, term) = initex(&d, "\\relax\\end");
    assert!(ok, "exit status; terminal:\n{term}");
    assert!(term.starts_with("This is TeX, Version 3.141592653 (INITEX)"), "{term}");
    assert!(term.contains("No pages of output."), "{term}");
    let log = std::fs::read_to_string(d.join("texput.log")).unwrap();
    assert_eq!(
        log,
        "This is TeX, Version 3.141592653 (INITEX)  4 JUL 1776 12:00\n\
         **\\relax\\end\n\
         No pages of output.\n"
    );
}

/// The committed configuration is TeX Live's: error_line=79 and
/// half_error_line=50 mean a 50-character context line is not truncated, and
/// `\showbox` reports a 32-bit glue ratio exactly.
#[test]
fn production_capacities_and_glue_display() {
    let d = scratch("show");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\showboxdepth=10 \\showboxbreadth=10\n\
         \\setbox0=\\hbox to 100pt{\\kern 3pt\\hskip 5pt plus 2fil\\kern 7pt}\\showbox0\n\
         \\skip0=3pt plus 1fill minus 2pt \\showthe\\skip0\n\
         \\end\n",
    )
    .unwrap();
    // \show... counts as an error, so the exit status is 1, as in web2c.
    let _ = initex(&d, "\\input t");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    assert!(log.contains("\\hbox(0.0+0.0)x100.0, glue set 42.5fil"), "{log}");
    assert!(log.contains("l.4 \\skip0=3pt plus 1fill minus 2pt \\showthe\\skip0"), "{log}");
    assert!(log.contains("> 3.0pt plus 1.0fill minus 2.0pt."), "{log}");
}
