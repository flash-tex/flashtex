//! Records, for `flashtex --version` and `flashtex licenses`
//! (src/licenses.rs):
//!
//! - `FLASHTEX_VERSION`: the release version. A release build sets it in the
//!   environment (.github/workflows/release.yml), because the crate version
//!   is 0.1.0 and is not bumped per release; otherwise the crate version.
//! - `FLASHTEX_RELEASE`: `1` when the environment named the version (a
//!   release build, whose source is the tag `v<version>`), else empty.
//! - `FLASHTEX_GIT_SHA`: the Git revision the binary was built from.
//!   `FLASHTEX_GIT_SHA` in the build environment wins (an archive export has
//!   no repository); otherwise `git rev-parse` of the crate's checkout, with
//!   `-dirty` when the tree has local changes; `unknown` when neither is
//!   available.
//!
//! Never fails the build. (The same scheme as the old engine's CLI,
//! crates/flashtex-cli, which this program replaced in retirement stage S7.)

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn main() {
    println!("cargo:rerun-if-env-changed=FLASHTEX_VERSION");
    let released = std::env::var("FLASHTEX_VERSION")
        .ok()
        .map(|v| v.trim().trim_start_matches('v').to_string())
        .filter(|v| !v.is_empty());
    let release = if released.is_some() { "1" } else { "" };
    let version = released.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=FLASHTEX_VERSION={version}");
    println!("cargo:rustc-env=FLASHTEX_RELEASE={release}");
    println!("cargo:rerun-if-env-changed=FLASHTEX_GIT_SHA");
    let sha = std::env::var("FLASHTEX_GIT_SHA")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let short =
                git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".into());
            // `status --porcelain` is empty on a clean tree; a failure (no
            // repository) leaves the plain revision.
            let dirty =
                git(&["status", "--porcelain", "--untracked-files=no", "--", "."]).is_some();
            if short != "unknown" && dirty {
                format!("{short}-dirty")
            } else {
                short
            }
        });
    println!("cargo:rustc-env=FLASHTEX_GIT_SHA={sha}");
    // Rebuild when HEAD moves so the revision stays truthful in a checkout.
    if let Some(dir) = git(&["rev-parse", "--git-dir"]) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        for f in ["HEAD", "index"] {
            let p = dir.join(f);
            if p.exists() {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }
    }
}
