//! Drift check: crates/flashtex-engine/src/generated/ and pdftex.pool must be
//! exactly what web2rust produces from third_party/pdftex/pdftex.web and the
//! change files of crates/flashtex-engine/changes/ with the committed
//! configuration (crates/flashtex-engine/web2rust-default.args).
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
        .arg("third_party/pdftex/pdftex.web")
        .arg("@crates/flashtex-engine/web2rust-default.args")
        .arg("--out-dir")
        .arg(out.join("generated"))
        .arg("--pool")
        .arg(out.join("pdftex.pool"))
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
    if std::fs::read(engine.join("pdftex.pool")).unwrap()
        != std::fs::read(out.join("pdftex.pool")).unwrap()
    {
        drift.push("pdftex.pool".into());
    }
    let _ = std::fs::remove_dir_all(&out);
    assert!(
        drift.is_empty(),
        "committed engine differs from a fresh translation in: {}\n\
         regenerate with the command in tools/web2rust/README.md",
        drift.join(", ")
    );
}

/// The same check for the XeTeX port (docs/design/xetex/PLAN.md):
/// crates/flashtex-xetex/src/generated/ and xetex.pool must be what web2rust
/// produces from third_party/xetex/xetex.web with
/// crates/flashtex-xetex/web2rust-default.args. That crate is built outside
/// the workspace; its translation is checked here, so that a translator
/// change that alters it cannot land unnoticed.
#[test]
fn committed_xetex_matches_a_fresh_translation() {
    let root = root();
    let crate_dir = root.join("crates/flashtex-xetex");
    let out = std::env::temp_dir().join(format!("web2rust-drift-xetex-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.join("generated")).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_web2rust"))
        .current_dir(&root)
        .arg("third_party/xetex/xetex.web")
        .arg("@crates/flashtex-xetex/web2rust-default.args")
        .arg("--out-dir")
        .arg(out.join("generated"))
        .arg("--pool")
        .arg(out.join("xetex.pool"))
        .status()
        .expect("run web2rust");
    assert!(status.success(), "web2rust failed");

    let committed = crate_dir.join("src/generated");
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
    if std::fs::read(crate_dir.join("xetex.pool")).unwrap()
        != std::fs::read(out.join("xetex.pool")).unwrap()
    {
        drift.push("xetex.pool".into());
    }
    let _ = std::fs::remove_dir_all(&out);
    assert!(
        drift.is_empty(),
        "committed XeTeX port differs from a fresh translation in: {}\n\
         regenerate with scripts/xetex-regenerate.sh",
        drift.join(", ")
    );
}
