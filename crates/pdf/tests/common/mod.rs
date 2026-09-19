//! Locating the TeX oracles, and refusing to skip silently when they are
//! absent.
//!
//! Two separate defects lived in the copies of this logic that used to sit
//! inline in `exact.rs` and `type1.rs`:
//!
//! 1. The probes only ever looked at two hardcoded MacTeX paths
//!    (`/usr/local/texlive/2026/bin/universal-darwin/...` and
//!    `/Library/TeX/texbin/...`). `PATH` was never consulted, so a perfectly
//!    good TeX Live -- a distro package, a Nix profile, a GitHub runner that
//!    installed one, anything not MacTeX at that exact release -- was invisible.
//!    On both CI legs the oracle was therefore never found.
//!
//! 2. Not finding it printed `eprintln!("skipped: ...")` and returned. libtest
//!    captures the `eprintln!` macro, so for a passing test that line is never
//!    shown. The result was a byte-fidelity check that could not fail and said
//!    nothing about it: green on a machine with no TeX at all.
//!
//! `crates/render-pipeline/tests/common/mod.rs` already treats a missing Latin
//! Modern the same way and calls it out in the same terms. This module is the
//! PDF-crate counterpart, with one deliberate difference: a missing TeX
//! *distribution* is a normal state for a contributor's laptop in a way that a
//! missing bundled font is not, so the default here is a loud-but-non-fatal
//! skip rather than a panic. [`require_oracles`] turns it into a panic for the
//! machines -- CI legs, this project's oracle runs -- that are supposed to have
//! TeX and want to be told when they silently lose it.

#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};

/// `FLASHTEX_REQUIRE_ORACLES=1` makes every missing oracle a hard failure
/// instead of a skip. Set it wherever TeX is provisioned on purpose.
pub const REQUIRE_ENV_VAR: &str = "FLASHTEX_REQUIRE_ORACLES";

/// Explicit overrides, checked before `PATH`. An override that does not point
/// at a real file is an error, not a reason to fall back: if you named a
/// binary you meant that binary.
pub const PDFLATEX_ENV_VAR: &str = "FLASHTEX_PDFLATEX";
pub const XELATEX_ENV_VAR: &str = "FLASHTEX_XELATEX";
pub const KPSEWHICH_ENV_VAR: &str = "FLASHTEX_KPSEWHICH";
/// Directory holding Latin Modern's Type 1 (`.pfb`) files, in the family of
/// `FLASHTEX_LM_DIR` (OpenType) and `FLASHTEX_LM_TFM_DIR` (metrics).
pub const LM_TYPE1_DIR_ENV_VAR: &str = "FLASHTEX_LM_TYPE1_DIR";

/// MacTeX locations, kept as fallbacks so a Mac whose `PATH` lacks TeX (a
/// non-login shell, Xcode's test runner) still finds the oracle.
const MACTEX_BIN_DIRS: [&str; 2] =
    ["/usr/local/texlive/2026/bin/universal-darwin", "/Library/TeX/texbin"];

/// Fixed Latin Modern Type 1 directories, tried after `kpsewhich`.
const LM_TYPE1_FIXED: [&str; 4] = [
    "/usr/local/texlive/2026/texmf-dist/fonts/type1/public/lm",
    "/usr/local/texlive/2025/texmf-dist/fonts/type1/public/lm",
    "/usr/share/texmf/fonts/type1/public/lm",
    "/usr/share/texlive/texmf-dist/fonts/type1/public/lm",
];

/// Write to the process's real stderr, bypassing libtest's capture.
///
/// `eprintln!` goes through `std::io`'s output-capture hook, which libtest
/// swaps out per test and only replays for a *failing* test. A direct write to
/// the `Stderr` handle does not, so this is visible in a plain `cargo test`
/// run -- which is the whole point: a skipped oracle has to be something you
/// can see without passing `--nocapture`.
pub fn announce(msg: &str) {
    let mut err = std::io::stderr();
    let _ = writeln!(err, "{msg}");
    let _ = err.flush();
}

/// Whether a missing oracle should fail the run rather than skip it.
pub fn require_oracles() -> bool {
    match std::env::var(REQUIRE_ENV_VAR) {
        Ok(v) => !v.is_empty() && v != "0",
        Err(_) => false,
    }
}

/// Record that `test` did not run, loudly.
///
/// Panics under [`REQUIRE_ENV_VAR`]; otherwise prints to the uncaptured
/// stderr and lets the caller return. Either way the skip leaves a trace.
pub fn skip(test: &str, reason: &str) {
    let msg = format!(
        "{test}: ORACLE MISSING -- {reason}. This test measured nothing. \
         Set {PDFLATEX_ENV_VAR}/{XELATEX_ENV_VAR} or put a TeX distribution on \
         PATH to run it; set {REQUIRE_ENV_VAR}=1 to make this state a failure."
    );
    if require_oracles() {
        panic!("{msg}");
    }
    announce(&format!("SKIPPED {msg}"));
}

/// Something the oracle *did* produce, but wrongly. Always fatal: the oracle
/// was found, so a failure to build the reference is a real problem and must
/// never be laundered into a skip the way it used to be.
pub fn oracle_failed(test: &str, what: &str, detail: &str) -> ! {
    panic!("{test}: the oracle was found but {what}. This is not a skip.\n{detail}");
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// A plain `PATH` search: split `$PATH` the platform's way and take the first
/// executable entry. Deliberately not a shell-out to `which`/`where`, which
/// differ between platforms and between shells on the same platform.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| is_executable(p))
}

/// Find a TeX program: `$env_var` if set, then `PATH`, then MacTeX's fixed
/// locations.
pub fn find_tex_program(name: &str, env_var: &str) -> Option<PathBuf> {
    if let Some(v) = std::env::var_os(env_var) {
        let p = PathBuf::from(&v);
        assert!(
            p.is_file(),
            "{env_var} is set to {} but that is not a file",
            p.display()
        );
        return Some(p);
    }
    which(name).or_else(|| {
        MACTEX_BIN_DIRS
            .iter()
            .map(|d| Path::new(d).join(name))
            .find(|p| p.is_file())
    })
}

pub fn pdflatex() -> Option<PathBuf> {
    find_tex_program("pdflatex", PDFLATEX_ENV_VAR)
}

pub fn xelatex() -> Option<PathBuf> {
    find_tex_program("xelatex", XELATEX_ENV_VAR)
}

/// Ask the TeX installation where one of its own files lives.
///
/// This is the supported way to locate anything in a `texmf` tree and the only
/// way that works for an installation not laid out under one of the four
/// hardcoded prefixes (a Nix store path, for instance).
pub fn kpsewhich(name: &str) -> Option<PathBuf> {
    let exe = find_tex_program("kpsewhich", KPSEWHICH_ENV_VAR)?;
    let out = std::process::Command::new(exe).arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();
    let p = PathBuf::from(first);
    p.is_file().then_some(p)
}

/// A Latin Modern Type 1 program: `$FLASHTEX_LM_TYPE1_DIR`, then `kpsewhich`,
/// then the fixed TeX Live directories.
pub fn lm_type1(name: &str) -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(LM_TYPE1_DIR_ENV_VAR) {
        let p = PathBuf::from(dir).join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    kpsewhich(name).or_else(|| {
        LM_TYPE1_FIXED
            .iter()
            .map(|d| Path::new(d).join(name))
            .find(|p| p.is_file())
    })
}

/// Latin Modern's OpenType face: the crate's own candidate list (which honours
/// `FLASHTEX_LM_DIR` and the TeX Live roots), then `kpsewhich`, so a TeX tree
/// outside those roots still resolves.
pub fn latin_modern_otf() -> Option<PathBuf> {
    flashtex_pdf::embed::candidate_paths()
        .into_iter()
        .find(|p| p.ends_with(flashtex_pdf::embed::LATIN_MODERN_FILE) && p.is_file())
        .or_else(|| kpsewhich(flashtex_pdf::embed::LATIN_MODERN_FILE))
}
