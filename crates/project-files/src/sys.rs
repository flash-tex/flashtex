//! Directory-relative file operations (`openat`, `renameat`, `unlinkat`,
//! `mkdirat`, `fstatat`, `flock`) through the [`libc`] crate.
//!
//! Rust `std` exposes no `openat` family, so a path-string API cannot pin a
//! walked directory handle: every `std::fs` call re-resolves the whole path
//! string, and a prefix swapped for a symlink between `symlink_metadata` and
//! `open` is followed. Every symbol, flag, errno value and struct layout used
//! here comes from `libc`, which maintains them per target (including the
//! macOS `mode_t` width, the `$INODE64` symbol variants and each Linux
//! architecture's `struct stat`); nothing is declared by hand. Rooted
//! operations are enabled on macOS and on Linux with glibc or musl; every
//! other target gets an `io::ErrorKind::Unsupported` error, which the save
//! layer surfaces as [`Refused::Unsupported`](crate::save::Refused::Unsupported).

use std::fs::File;
use std::io;

#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", any(target_env = "gnu", target_env = "musl"))
))]
mod imp {
    use std::ffi::CString;
    use std::fs::File;
    use std::io;
    use std::mem::MaybeUninit;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::raw::c_int;

    pub const SUPPORTED: bool = true;
    pub use libc::{
        EEXIST, ELOOP, ENOENT, ENOTDIR, O_CREAT, O_DIRECTORY, O_EXCL, O_NOFOLLOW, O_NONBLOCK,
        O_RDONLY, O_RDWR, O_WRONLY,
    };

    fn cstr(name: impl AsRef<[u8]>) -> io::Result<CString> {
        CString::new(name.as_ref())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in path component"))
    }

    fn check(rc: c_int) -> io::Result<()> {
        if rc != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// `openat(dirfd, name, flags | O_CLOEXEC | O_NONBLOCK, mode)`.
    ///
    /// `O_NONBLOCK` is always added so that opening a name which is (or was
    /// swapped to) a FIFO returns at once instead of blocking until a writer
    /// appears. It has no effect on reads or writes of regular files or on
    /// directories. For a device node it is best-effort only: whether a
    /// driver's open honours it, and what the open itself does, is up to the
    /// driver. It is a backstop, not the check: callers first classify an
    /// existing name with [`stat_at_nofollow`] and open only a regular file,
    /// then `fstat` the descriptor and refuse anything but a regular file
    /// (the name can change between the two).
    pub fn open_at(dir: &File, name: &str, flags: c_int, mode: u32) -> io::Result<File> {
        open_at_bytes(dir, name.as_bytes(), flags, mode)
    }

    /// [`open_at`] for a name that need not be UTF-8.
    pub fn open_at_bytes(dir: &File, name: &[u8], flags: c_int, mode: u32) -> io::Result<File> {
        let c = cstr(name)?;
        // SAFETY: `c` is a valid NUL-terminated string that outlives the call
        // and `dir` is an open descriptor; the variadic mode is passed as the
        // promoted `c_uint`. The returned fd is owned by exactly one `File`.
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                c.as_ptr(),
                flags | libc::O_CLOEXEC | libc::O_NONBLOCK,
                mode as libc::c_uint,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `fd` is a freshly opened descriptor nothing else owns.
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    pub fn rename_at(dir: &File, old: &str, new: &str) -> io::Result<()> {
        let (o, n) = (cstr(old)?, cstr(new)?);
        // SAFETY: valid C strings and an open directory descriptor.
        check(unsafe { libc::renameat(dir.as_raw_fd(), o.as_ptr(), dir.as_raw_fd(), n.as_ptr()) })
    }

    pub fn unlink_at(dir: &File, name: &str) -> io::Result<()> {
        let c = cstr(name)?;
        // SAFETY: valid C string and an open directory descriptor.
        check(unsafe { libc::unlinkat(dir.as_raw_fd(), c.as_ptr(), 0) })
    }

    pub fn mkdir_at(dir: &File, name: &str, mode: u32) -> io::Result<()> {
        let c = cstr(name)?;
        // SAFETY: valid C string and an open directory descriptor.
        check(unsafe { libc::mkdirat(dir.as_raw_fd(), c.as_ptr(), mode as libc::mode_t) })
    }

    /// `fstatat(dirfd, name, AT_SYMLINK_NOFOLLOW)`: classifies the entry
    /// `name` of the open directory `dir` without following a symlink and
    /// without opening it (so a device's open side effects never run).
    #[allow(clippy::unnecessary_cast)] // field widths differ per target
    pub fn stat_at_nofollow(dir: &File, name: &[u8]) -> io::Result<super::EntryStat> {
        let c = cstr(name)?;
        let mut st = MaybeUninit::<libc::stat>::uninit();
        // SAFETY: valid C string, an open directory descriptor, and a
        // `libc::stat` buffer of the target's own layout.
        check(unsafe {
            libc::fstatat(
                dir.as_raw_fd(),
                c.as_ptr(),
                st.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        })?;
        // SAFETY: `fstatat` returned 0, so it filled the whole struct.
        let st = unsafe { st.assume_init() };
        Ok(super::EntryStat {
            dev: st.st_dev as u64,
            ino: st.st_ino as u64,
            mode: st.st_mode as u32,
        })
    }

    /// Advisory exclusive lock; `Ok(false)` when another open file
    /// description (any process, or another handle in this one) holds it.
    pub fn try_lock_exclusive(file: &File) -> io::Result<bool> {
        // SAFETY: `file` is an open descriptor.
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc == 0 {
            return Ok(true);
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
            Ok(false)
        } else {
            Err(err)
        }
    }

    pub fn unlock(file: &File) {
        // SAFETY: `file` is an open descriptor; failure is irrelevant at drop.
        unsafe {
            libc::flock(file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[cfg(not(any(
    target_os = "macos",
    all(target_os = "linux", any(target_env = "gnu", target_env = "musl"))
)))]
mod imp {
    use std::fs::File;
    use std::io;
    use std::os::raw::c_int;

    pub const SUPPORTED: bool = false;
    // Inert placeholders so the rest of the crate type-checks: nothing on
    // this target reaches the OS with them, because `ProjectRoot::open`
    // refuses with `Refused::Unsupported` before any rooted call is made.
    pub const O_RDONLY: c_int = 0;
    pub const O_WRONLY: c_int = 0;
    pub const O_RDWR: c_int = 0;
    pub const O_CREAT: c_int = 0;
    pub const O_EXCL: c_int = 0;
    pub const O_NOFOLLOW: c_int = 0;
    pub const O_DIRECTORY: c_int = 0;
    pub const O_NONBLOCK: c_int = 0;
    pub const ELOOP: i32 = -1;
    pub const ENOENT: i32 = -1;
    pub const EEXIST: i32 = -1;
    pub const ENOTDIR: i32 = -1;

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "rooted file operations are not available on this target",
        )
    }
    pub fn open_at(_: &File, _: &str, _: c_int, _: u32) -> io::Result<File> {
        Err(unsupported())
    }
    pub fn open_at_bytes(_: &File, _: &[u8], _: c_int, _: u32) -> io::Result<File> {
        Err(unsupported())
    }
    pub fn stat_at_nofollow(_: &File, _: &[u8]) -> io::Result<super::EntryStat> {
        Err(unsupported())
    }
    pub fn rename_at(_: &File, _: &str, _: &str) -> io::Result<()> {
        Err(unsupported())
    }
    pub fn unlink_at(_: &File, _: &str) -> io::Result<()> {
        Err(unsupported())
    }
    pub fn mkdir_at(_: &File, _: &str, _: u32) -> io::Result<()> {
        Err(unsupported())
    }
    pub fn try_lock_exclusive(_: &File) -> io::Result<bool> {
        Err(unsupported())
    }
    pub fn unlock(_: &File) {}
}

pub use imp::*;

pub fn errno_is(err: &io::Error, code: i32) -> bool {
    code >= 0 && err.raw_os_error() == Some(code)
}

/// Opens `path` itself as a directory without following a final symlink.
///
/// **Symlinks above the root are followed, deliberately.** Only the final
/// component is opened with `O_NOFOLLOW`; its parent is opened by path string
/// with ordinary resolution, so a symlink among the root's *ancestors*
/// (`/Users/me/link/project` with `link -> /Volumes/work`) is followed and
/// the directory it leads to becomes the pinned root. The caller chose that
/// path, and symlinked home directories, checkouts and volume mounts are
/// common; refusing them would reject real projects without protecting
/// anything the caller did not already select. The refuse-all-symlinks
/// policy applies *inside* the root: once the root descriptor is pinned,
/// every project path is walked from it with `O_NOFOLLOW` at each component,
/// and a later change to the ancestors does not move the pinned root.
pub fn open_dir_nofollow(path: &std::path::Path) -> io::Result<File> {
    // `std` has no O_NOFOLLOW knob, so open the parent with std and the last
    // component with `openat`. The name is passed as raw bytes: a root whose
    // final component is not valid UTF-8 gets the same `O_NOFOLLOW` open,
    // never a symlink-following fallback.
    use std::os::unix::ffi::OsStrExt;
    let (parent, name) = match (path.parent(), path.file_name()) {
        (Some(p), Some(n)) if !n.is_empty() => (p, n),
        _ => return open_dir_std(path), // "/" or similar: nothing to refuse
    };
    let parent = open_dir_std(if parent.as_os_str().is_empty() {
        std::path::Path::new(".")
    } else {
        parent
    })?;
    open_dir_at_nofollow_bytes(&parent, name.as_bytes())
}

/// `std` open of a caller-chosen directory path. On unix it carries
/// `O_DIRECTORY | O_NONBLOCK`, so a FIFO at that path is refused instead of
/// blocking the open.
fn open_dir_std(path: &std::path::Path) -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECTORY | O_NONBLOCK)
            .open(path)
    }
    #[cfg(not(unix))]
    {
        File::open(path)
    }
}

/// Opens directory `name` inside `dir` with `O_DIRECTORY | O_NOFOLLOW`.
///
/// Linux reports a symlink as `ELOOP`; macOS reports `ENOTDIR` for a
/// symlink-to-directory in this mode. Both refuse to follow; to classify the
/// refusal consistently, an `ENOTDIR` result is classified with
/// [`stat_at_nofollow`] (never a second open, which could run a device's
/// open side effects) and mapped to `ELOOP` when the entry is a symlink.
pub fn open_dir_at_nofollow(dir: &File, name: &str) -> io::Result<File> {
    open_dir_at_nofollow_bytes(dir, name.as_bytes())
}

fn open_dir_at_nofollow_bytes(dir: &File, name: &[u8]) -> io::Result<File> {
    match open_at_bytes(dir, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW, 0) {
        Err(e) if errno_is(&e, ENOTDIR) => match stat_at_nofollow(dir, name) {
            Ok(st) if st.is_symlink() => Err(io::Error::from_raw_os_error(ELOOP)),
            _ => Err(e),
        },
        other => other,
    }
}

/// File type bits of `st_mode`, from `libc` (`mode_t` is 16 bits on macOS
/// and 32 on Linux; the values are widened, never truncated).
#[allow(clippy::unnecessary_cast)]
pub const S_IFMT: u32 = libc::S_IFMT as u32;
#[allow(clippy::unnecessary_cast)]
pub const S_IFREG: u32 = libc::S_IFREG as u32;
#[allow(clippy::unnecessary_cast)]
pub const S_IFDIR: u32 = libc::S_IFDIR as u32;
#[allow(clippy::unnecessary_cast)]
pub const S_IFLNK: u32 = libc::S_IFLNK as u32;

/// What [`stat_at_nofollow`] reports about one directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryStat {
    /// `st_dev`, widened the same way `std::os::unix::fs::MetadataExt::dev` does.
    pub dev: u64,
    pub ino: u64,
    /// `st_mode`: file type and permission bits.
    pub mode: u32,
}

impl EntryStat {
    pub fn is_file(&self) -> bool {
        self.mode & S_IFMT == S_IFREG
    }
    pub fn is_dir(&self) -> bool {
        self.mode & S_IFMT == S_IFDIR
    }
    pub fn is_symlink(&self) -> bool {
        self.mode & S_IFMT == S_IFLNK
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    /// The `fstatat` fields must agree with `std`'s own `lstat` for every
    /// file type this crate distinguishes.
    #[test]
    fn stat_at_nofollow_matches_std_symlink_metadata() {
        if !SUPPORTED {
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "flashtex-sys-stat-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("file"), "x").unwrap();
        std::os::unix::fs::symlink(dir.join("file"), dir.join("link")).unwrap();
        std::os::unix::fs::symlink(dir.join("absent"), dir.join("broken")).unwrap();
        let handle = open_dir_nofollow(&dir).unwrap();
        for name in ["file", "sub", "link", "broken"] {
            let st = stat_at_nofollow(&handle, name.as_bytes()).unwrap();
            let meta = std::fs::symlink_metadata(dir.join(name)).unwrap();
            assert_eq!(st.mode, meta.mode(), "{name}");
            assert_eq!(st.ino, meta.ino(), "{name}");
            assert_eq!(st.dev, meta.dev(), "{name}");
            let ft = meta.file_type();
            assert_eq!(
                (st.is_file(), st.is_dir(), st.is_symlink()),
                (ft.is_file(), ft.is_dir(), ft.is_symlink()),
                "{name}"
            );
        }
        let err = stat_at_nofollow(&handle, b"absent").unwrap_err();
        assert!(errno_is(&err, ENOENT), "{err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
