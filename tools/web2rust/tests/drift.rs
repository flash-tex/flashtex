//! Drift check: crates/flashtex-engine/src/generated/ and tex.pool must be
//! exactly what web2rust produces from third_party/knuth/tex.web with the
//! committed configuration (crates/flashtex-engine/web2rust-default.args).
//!
//!     cargo test --release -p web2rust --test drift
//!
//! On failure, regenerate with the command in tools/web2rust/README.md and
//! commit the result together with the translator change that caused it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn committed_engine_matches_a_fresh_translation() {
    let root = root();
    let engine = root.join("crates/flashtex-engine");
    let out = std::env::temp_dir().join(format!("web2rust-drift-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.join("generated")).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_web2rust"))
        .current_dir(&root)
        .arg("third_party/knuth/tex.web")
        .arg("@crates/flashtex-engine/web2rust-default.args")
        .arg("--out-dir")
        .arg(out.join("generated"))
        .arg("--pool")
        .arg(out.join("tex.pool"))
        .status()
        .expect("run web2rust");
    assert!(status.success(), "web2rust failed");

    let committed = engine.join("src/generated");
    let fresh = out.join("generated");
    assert_eq!(
        files(&committed),
        files(&fresh),
        "set of generated files differs"
    );
    let mut drift = vec![];
    for f in files(&committed) {
        if std::fs::read(committed.join(&f)).unwrap() != std::fs::read(fresh.join(&f)).unwrap() {
            drift.push(format!("src/generated/{f}"));
        }
    }
    if std::fs::read(engine.join("tex.pool")).unwrap()
        != std::fs::read(out.join("tex.pool")).unwrap()
    {
        drift.push("tex.pool".into());
    }
    let _ = std::fs::remove_dir_all(&out);
    assert!(
        drift.is_empty(),
        "committed engine differs from a fresh translation in: {}\n\
         regenerate with the command in tools/web2rust/README.md",
        drift.join(", ")
    );
}
