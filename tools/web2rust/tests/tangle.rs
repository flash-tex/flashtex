//! The tangle stage against the numbers Knuth's TANGLE gives for tex.web
//! 3.141592653. scripts/web2rust-oracle-check.sh does the full token-by-token
//! comparison where `tangle` is installed; this pins the totals everywhere.

use std::path::Path;
use std::process::Command;

#[test]
fn tex_web_tangles_like_tangle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("web2rust-tangle-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_web2rust"))
        .arg(root.join("third_party/knuth/tex.web"))
        .arg("--pool")
        .arg(out.join("tex.pool"))
        .arg("--emit-pascal")
        .arg(out.join("tex.toks"))
        .status()
        .unwrap();
    assert!(status.success());
    let pool = std::fs::read_to_string(out.join("tex.pool")).unwrap();
    let toks = std::fs::read_to_string(out.join("tex.toks")).unwrap();
    let _ = std::fs::remove_dir_all(&out);
    // "1045 strings written to string pool file.", checksum line last.
    assert_eq!(pool.lines().count(), 1046);
    assert_eq!(pool.lines().next(), Some("11buffer size"));
    assert_eq!(pool.lines().last(), Some("*504454778"));
    assert_eq!(toks.lines().count(), 120_560);
}
