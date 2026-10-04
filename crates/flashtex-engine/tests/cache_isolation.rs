//! Tests never use the user's real FlashTeX caches.
//!
//! Every engine binary a test starts (flashtex-host, flashtex-initex,
//! flashtex-v3, the CLI) inherits the test's environment, and the repository's
//! `.cargo/config.toml` gives every process cargo starts a format, bundle and
//! package cache under `target/`. This test fails if that stops being so --
//! the configuration removed, or a cache resolved under the real per-user
//! root (`~/Library/Caches/FlashTeX`, `$XDG_CACHE_HOME/flashtex`,
//! `%LOCALAPPDATA%\FlashTeX`) anyway. `scripts/gate.sh` checks the other side:
//! it runs `cargo test` with an empty HOME and fails if a FlashTeX cache
//! appears there.
#![cfg(all(feature = "distribution", not(feature = "tex82")))]

use flashtex_engine::{bundle, formats};
use std::path::{Path, PathBuf};

const VARS: [&str; 3] = [
    "FLASHTEX_FORMAT_CACHE_DIR",
    "FLASHTEX_BUNDLE_CACHE_DIR",
    "FLASHTEX_PACKAGE_CACHE",
];

/// Where the user's own caches live: the engine's cache root and the
/// package resolver's default (`~/Library/Application Support/FlashTeX` on
/// macOS, under the cache root elsewhere).
fn user_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = formats::cache_dir_default_root().into_iter().collect();
    if let Some(h) = std::env::var_os("HOME").filter(|h| !h.is_empty()) {
        v.push(Path::new(&h).join("Library/Application Support/FlashTeX"));
    }
    v
}

fn assert_private(what: &str, p: &Path) {
    for root in user_roots() {
        assert!(
            !p.starts_with(&root),
            "{what} is {}, inside the user's real cache {}: tests would write it. \
             The repository's .cargo/config.toml [env] should point it under target/.",
            p.display(),
            root.display()
        );
    }
}

#[test]
fn every_cache_variable_is_set_for_tests() {
    for k in VARS {
        let v = std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                panic!(
                    "{k} is not set in the test environment, so engine binaries started by \
                 tests would use the user's real cache. It comes from the [env] table of \
                 the repository's .cargo/config.toml."
                )
            });
        assert_private(k, Path::new(&v));
    }
}

#[test]
fn the_engine_resolves_its_caches_outside_the_user_root() {
    let f = formats::cache_dir().expect("a format cache directory");
    assert_private("formats::cache_dir()", &f);
    let b = bundle::default_cache_dir().expect("a bundle cache directory");
    assert_private("bundle::default_cache_dir()", &b);
}
