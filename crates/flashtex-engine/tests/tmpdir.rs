//! The scratch directories of the engine's integration tests
//! (`common::fresh_dir`, `common::link_engine`).
//!
//! The tests used to work in `$TMPDIR/<prefix>-<pid>` and keep what was
//! there. A process that got a reused pid then ran another build's `pdftex`
//! symlink and `pdflatex.fmt`: with a stale link in place, every test of
//! `tests/incremental.rs` failed its comparison with a scratch run. These
//! tests plant exactly such a stale directory and check that it is not used.

mod common;

use std::path::{Path, PathBuf};

/// The directory the old code used for `prefix` in this process, made stale:
/// `fmt/pdftex` links to a program that is not this build's engine, and a
/// `pdflatex.fmt` is already there.
fn plant_stale(prefix: &str) -> PathBuf {
    let old = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&old);
    std::fs::create_dir_all(old.join("fmt")).unwrap();
    std::os::unix::fs::symlink("/usr/bin/false", old.join("fmt/pdftex")).unwrap();
    std::fs::write(old.join("fmt/pdflatex.fmt"), b"not this build's format").unwrap();
    old
}

#[test]
fn a_stale_directory_at_the_old_path_is_never_used() {
    let prefix = "flashtex-tmpdir-selftest-a";
    let old = plant_stale(prefix);
    let a = common::fresh_dir(prefix);
    let b = common::fresh_dir(prefix);
    assert_ne!(a, old, "fresh_dir returned the old pid-named directory");
    assert_ne!(a, b, "two calls returned the same directory");
    for d in [&a, &b] {
        assert!(d.is_dir());
        assert_eq!(
            std::fs::read_dir(d).unwrap().count(),
            0,
            "{} is not empty",
            d.display()
        );
    }
    for d in [&old, &a, &b] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn link_engine_replaces_another_builds_link() {
    let prefix = "flashtex-tmpdir-selftest-b";
    let old = plant_stale(prefix);
    let link = old.join("fmt/pdftex");
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    common::link_engine(ours, &link);
    assert_eq!(std::fs::read_link(&link).unwrap(), ours);
    // a second call leaves a correct link alone
    common::link_engine(ours, &link);
    assert_eq!(std::fs::read_link(&link).unwrap(), ours);
    std::fs::remove_dir_all(&old).unwrap();
}

#[test]
fn directories_older_than_a_day_are_swept() {
    let prefix = "flashtex-tmpdir-selftest-c";
    let stale = std::env::temp_dir().join(format!("{prefix}-1-1"));
    let recent = std::env::temp_dir().join(format!("{prefix}-2-2"));
    for d in [&stale, &recent] {
        let _ = std::fs::remove_dir_all(d);
        std::fs::create_dir_all(d).unwrap();
    }
    let two_days = std::time::Duration::from_secs(2 * 24 * 3600);
    std::fs::File::open(&stale)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - two_days)
        .unwrap();
    let made = common::fresh_dir(prefix);
    assert!(
        !stale.exists(),
        "a directory older than a day was not swept"
    );
    assert!(recent.exists(), "a recent directory was swept");
    for d in [&recent, &made] {
        std::fs::remove_dir_all(d).unwrap();
    }
}
