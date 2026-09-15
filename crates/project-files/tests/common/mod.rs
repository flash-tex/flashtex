#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use flashtex_project_files::ProjectPath;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique directory under the system temp dir, removed on drop. Never
/// touches the repository.
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "flashtex-project-files-{tag}-{}-{nanos}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub fn write(&self, rel: &str, text: &str) -> PathBuf {
        let p = self.path.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, text).unwrap();
        p
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path.join(rel)).unwrap()
    }

    pub fn root(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn pp(s: &str) -> ProjectPath {
    ProjectPath::normalize(s).unwrap()
}

/// Win32 `ERROR_PRIVILEGE_NOT_HELD`: the caller may not create symbolic links.
#[cfg(windows)]
const ERROR_PRIVILEGE_NOT_HELD: i32 = 1314;

/// Turns the one Windows-specific failure worth explaining into a message
/// that says what to do about it, instead of a bare OS error.
///
/// Creating a symbolic link on Windows needs either Developer Mode or an
/// elevated process. The symlink-refusal tests are the regression suite for
/// issue #18, so they must fail loudly when they cannot run — silently
/// skipping would report a green suite that proved nothing.
#[cfg(windows)]
fn explain(result: std::io::Result<()>, kind: &str) {
    match result {
        Ok(()) => {}
        Err(e) if e.raw_os_error() == Some(ERROR_PRIVILEGE_NOT_HELD) => panic!(
            "cannot create a {kind} symlink: Windows requires Developer Mode or an \
             elevated process. Enable Settings > System > For developers > Developer Mode \
             (or check HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModelUnlock \
             \\AllowDevelopmentWithoutDevLicense) and re-run; this suite is the issue #18 \
             symlink-refusal regression suite and cannot be meaningfully skipped."
        ),
        Err(e) => panic!("creating a {kind} symlink failed: {e}"),
    }
}

/// Creates a symbolic link to a directory.
pub fn symlink_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    explain(std::os::windows::fs::symlink_dir(target, link), "directory");
}

/// Creates a symbolic link to a file.
pub fn symlink_file(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    explain(std::os::windows::fs::symlink_file(target, link), "file");
}

/// Best-effort file symlink for racing tests, where the link may legitimately
/// lose to a concurrent writer.
pub fn try_symlink_file(target: &Path, link: &Path) {
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let _ = std::os::windows::fs::symlink_file(target, link);
}
