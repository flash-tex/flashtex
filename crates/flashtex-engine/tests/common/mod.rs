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

/// A scratch directory that belongs to this test process alone:
/// `$TMPDIR/<prefix>-<pid>-<nanos>`, created here, so it never existed before.
///
/// The tests used to work in `$TMPDIR/<prefix>-<pid>` and keep whatever was
/// there, so a process that got a reused pid ran another build's `pdftex`
/// symlink and `pdflatex.fmt` (incremental.rs: every test then differed from
/// its scratch run). Directories of this prefix last modified over a day ago
/// are removed on the way, so finished runs do not pile up; no test run takes
/// that long.
#[allow(dead_code)]
pub fn fresh_dir(prefix: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir();
    sweep_old(&tmp, prefix);
    loop {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = tmp.join(format!("{prefix}-{}-{nanos}", std::process::id()));
        match std::fs::create_dir(&d) {
            Ok(()) => return d,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => panic!("creating {}: {e}", d.display()),
        }
    }
}

/// Remove the `<prefix>-*` directories in `tmp` last modified over a day ago.
#[allow(dead_code)]
fn sweep_old(tmp: &std::path::Path, prefix: &str) {
    let Ok(rd) = std::fs::read_dir(tmp) else {
        return;
    };
    let lead = format!("{prefix}-");
    let day = std::time::Duration::from_secs(24 * 3600);
    for e in rd.flatten() {
        let name = e.file_name();
        if !name.to_str().is_some_and(|n| n.starts_with(&lead)) {
            continue;
        }
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > day);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Link `link` to `target`, which must be what it points to afterwards (a
/// silently failed link used to leave another build's engine in place).
#[allow(dead_code)]
pub fn link_engine(target: &std::path::Path, link: &std::path::Path) {
    if std::fs::read_link(link).ok().as_deref() != Some(target) {
        let _ = std::fs::remove_file(link);
        std::os::unix::fs::symlink(target, link)
            .unwrap_or_else(|e| panic!("linking {} to {}: {e}", link.display(), target.display()));
    }
    assert_eq!(
        std::fs::read_link(link).ok().as_deref(),
        Some(target),
        "{} does not point to this build's engine",
        link.display()
    );
}
