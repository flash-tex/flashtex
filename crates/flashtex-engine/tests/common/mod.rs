//! Shared by the integration tests that need a TeX Live installation.

/// What a test does where it would skip for want of TeX Live.
///
/// Locally, and on CI runners that have no TeX Live, it prints a line and the
/// test returns (a skip that reports as a pass). A CI job that is supposed to
/// have TeX Live sets `FLASHTEX_REQUIRE_TEXLIVE=1`, and then a missing TeX Live
/// fails the test: otherwise a runner whose TeX Live vanished, or whose PATH
/// lost it, turns every TeX Live test green without running one.
// write18.rs calls it only with the `kpathsea` feature.
#[allow(dead_code)]
pub fn no_texlive() {
    if std::env::var_os("FLASHTEX_REQUIRE_TEXLIVE").is_some_and(|v| v == "1") {
        panic!(
            "no TeX Live found, and FLASHTEX_REQUIRE_TEXLIVE=1 says this run must have one \
             (put TeX Live 2026's bin directory first on PATH)"
        );
    }
    eprintln!("no TeX Live found; skipping");
}
