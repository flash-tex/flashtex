//! Change files against Knuth's TANGLE.
//!
//! tests/changefile/sample.tangle.{p,pool,toks} are what TANGLE 4.6 (TeX Live
//! 2026) writes for `tangle sample.web sample.ch` (regenerate.sh in that
//! directory remakes them). web2rust must produce the same token stream and a
//! byte-identical string pool, must translate the result, and must reject the
//! two change files TANGLE rejects.

use std::path::{Path, PathBuf};
use std::process::Command;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/changefile")
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("web2rust-ch-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn sample_change_file_tangles_like_tangle() {
    let out = scratch("ok");
    let status = Command::new(env!("CARGO_BIN_EXE_web2rust"))
        .arg(dir().join("sample.web"))
        .arg("--change")
        .arg(dir().join("sample.ch"))
        .arg("--pool")
        .arg(out.join("sample.pool"))
        .arg("--emit-pascal")
        .arg(out.join("sample.toks"))
        .arg("--out-dir")
        .arg(out.join("rs"))
        .status()
        .unwrap();
    assert!(status.success());
    let toks = std::fs::read_to_string(out.join("sample.toks")).unwrap();
    let want = std::fs::read_to_string(dir().join("sample.tangle.toks")).unwrap();
    assert_eq!(toks, want, "token stream differs from TANGLE's");
    let pool = std::fs::read(out.join("sample.pool")).unwrap();
    let want = std::fs::read(dir().join("sample.tangle.pool")).unwrap();
    assert_eq!(pool, want, "string pool differs from TANGLE's");
    // The section the change file adds is §6, as in TANGLE's `{6:}`.
    let main = std::fs::read_to_string(out.join("rs/main_body.rs")).unwrap();
    assert!(main.contains("// §6\n"), "{main}");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn change_files_tangle_rejects_are_rejected() {
    for (bad, msg) in [
        ("partial.ch", "1 of the preceding lines failed to match"),
        ("nomatch.ch", "did not match"),
    ] {
        let out = scratch("bad");
        let o = Command::new(env!("CARGO_BIN_EXE_web2rust"))
            .arg(dir().join("sample.web"))
            .arg("--change")
            .arg(dir().join(bad))
            .arg("--pool")
            .arg(out.join("sample.pool"))
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(!o.status.success(), "{bad} was accepted");
        assert!(err.contains(msg), "{bad}: {err}");
        let _ = std::fs::remove_dir_all(&out);
    }
}
