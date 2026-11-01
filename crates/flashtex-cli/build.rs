//! Records the release version as `FLASHTEX_VERSION` and the Git revision
//! the binary was built from as `FLASHTEX_GIT_SHA`
//! (for `flashtex --version`). `FLASHTEX_GIT_SHA` in the build environment
//! wins (an archive export has no repository); otherwise `git rev-parse` of
//! the crate's checkout, with `-dirty` when the tree has local changes;
//! `unknown` when neither is available. Never fails the build.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).current_dir(env!("CARGO_MANIFEST_DIR")).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn main() {
    // The crate version is 0.1.0 and is not bumped per release, so a release
    // build is told its own version the same way it is told its revision.
    // Without this `flashtex --version` printed "0.1.0" out of a v0.1.4
    // tarball.
    println!("cargo:rerun-if-env-changed=FLASHTEX_VERSION");
    let version = std::env::var("FLASHTEX_VERSION")
        .ok()
        .map(|v| v.trim().trim_start_matches('v').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=FLASHTEX_VERSION={version}");
    println!("cargo:rerun-if-env-changed=FLASHTEX_GIT_SHA");
    let sha = std::env::var("FLASHTEX_GIT_SHA").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| {
        let short = git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".into());
        // `status --porcelain` is empty on a clean tree; a failure (no
        // repository) leaves the plain revision.
        let dirty = git(&["status", "--porcelain", "--untracked-files=no", "--", "."]).is_some();
        if short != "unknown" && dirty {
            format!("{short}-dirty")
        } else {
            short
        }
    });
    println!("cargo:rustc-env=FLASHTEX_GIT_SHA={sha}");
    // Rebuild when HEAD moves so the SHA stays truthful in a checkout.
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
