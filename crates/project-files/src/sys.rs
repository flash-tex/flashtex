//! Minimal direct bindings to the C library for directory-relative file
//! operations (`openat`, `renameat`, `unlinkat`, `mkdirat`, `flock`).
//!
//! Rust `std` exposes no `openat` family, so a path-string API cannot pin a
//! walked directory handle: every `std::fs` call re-resolves the whole path
//! string, and a prefix swapped for a symlink between `symlink_metadata` and
//! `open` is followed. Rather than approximate, this module declares the
//! handful of POSIX symbols it needs; `std` already links the platform C
//! library, so this adds no external crate. Flag values are only known for
//! macOS and Linux x86_64/aarch64; every other target gets an
//! `io::ErrorKind::Unsupported` error, which the save layer surfaces as
//! [`Refused::Unsupported`](crate::save::Refused::Unsupported).

use std::fs::File;
use std::io;

#[cfg(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod imp {
    use std::ffi::CString;
    use std::fs::File;
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::raw::{c_char, c_int, c_uint};

    unsafe extern "C" {
        fn openat(dirfd: c_int, path: *const c_char, flags: c_int, ...) -> c_int;
        fn renameat(
            olddirfd: c_int,
            old: *const c_char,
            newdirfd: c_int,
            new: *const c_char,
        ) -> c_int;
        fn unlinkat(dirfd: c_int, path: *const c_char, flags: c_int) -> c_int;
        fn mkdirat(dirfd: c_int, path: *const c_char, mode: c_uint) -> c_int;
        fn flock(fd: c_int, operation: c_int) -> c_int;
    }

    pub const SUPPORTED: bool = true;
    pub const O_RDONLY: c_int = 0;
    pub const O_WRONLY: c_int = 1;
    pub const O_RDWR: c_int = 2;

    #[cfg(target_os = "macos")]
    mod flags {
        use std::os::raw::c_int;
        pub const O_CREAT: c_int = 0x0200;
        pub const O_EXCL: c_int = 0x0800;
        pub const O_NOFOLLOW: c_int = 0x0100;
        pub const O_DIRECTORY: c_int = 0x0010_0000;
        pub const O_CLOEXEC: c_int = 0x0100_0000;
        pub const O_NONBLOCK: c_int = 0x0004;
        pub const ELOOP: i32 = 62;
        pub const EWOULDBLOCK: i32 = 35;
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    mod flags {
        use std::os::raw::c_int;
        pub const O_CREAT: c_int = 0o100;
        pub const O_EXCL: c_int = 0o200;
        pub const O_NOFOLLOW: c_int = 0o400000;
        pub const O_DIRECTORY: c_int = 0o200000;
        pub const O_CLOEXEC: c_int = 0o2000000;
        pub const O_NONBLOCK: c_int = 0o4000;
        pub const ELOOP: i32 = 40;
        pub const EWOULDBLOCK: i32 = 11;
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    mod flags {
        use std::os::raw::c_int;
        pub const O_CREAT: c_int = 0o100;
        pub const O_EXCL: c_int = 0o200;
        pub const O_NOFOLLOW: c_int = 0o100000;
        pub const O_DIRECTORY: c_int = 0o40000;
        pub const O_CLOEXEC: c_int = 0o2000000;
        pub const O_NONBLOCK: c_int = 0o4000;
        pub const ELOOP: i32 = 40;
        pub const EWOULDBLOCK: i32 = 11;
    }
    pub use flags::*;

    pub const ENOENT: i32 = 2;
    pub const EEXIST: i32 = 17;
    pub const ENOTDIR: i32 = 20;
    const LOCK_EX: c_int = 2;
    const LOCK_NB: c_int = 4;
    const LOCK_UN: c_int = 8;

    fn cstr(name: &str) -> io::Result<CString> {
        CString::new(name)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in path component"))
    }

    /// `openat(dirfd, name, flags | O_CLOEXEC | O_NONBLOCK, mode)`.
    ///
    /// `O_NONBLOCK` is always added so that opening a name which is (or was
    /// swapped to) a FIFO or a device returns at once instead of blocking
    /// until a writer or carrier appears. It has no effect on reads or writes
    /// of regular files or on directories; callers that go on to read
    /// `fstat` the descriptor and refuse anything but a regular file.
    pub fn open_at(dir: &File, name: &str, flags: c_int, mode: u32) -> io::Result<File> {
        let c = cstr(name)?;
        // SAFETY: `c` is a valid NUL-terminated string that outlives the call
        // and `dir` is an open descriptor; the returned fd is owned by exactly
        // one `File`.
        let fd = unsafe {
            openat(
                dir.as_raw_fd(),
                c.as_ptr(),
                flags | O_CLOEXEC | O_NONBLOCK,
                mode as c_uint,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    pub fn rename_at(dir: &File, old: &str, new: &str) -> io::Result<()> {
        let (o, n) = (cstr(old)?, cstr(new)?);
        // SAFETY: valid C strings and an open directory descriptor.
        let rc = unsafe { renameat(dir.as_raw_fd(), o.as_ptr(), dir.as_raw_fd(), n.as_ptr()) };
        if rc != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn unlink_at(dir: &File, name: &str) -> io::Result<()> {
        let c = cstr(name)?;
        // SAFETY: valid C string and an open directory descriptor.
        let rc = unsafe { unlinkat(dir.as_raw_fd(), c.as_ptr(), 0) };
        if rc != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn mkdir_at(dir: &File, name: &str, mode: u32) -> io::Result<()> {
        let c = cstr(name)?;
        // SAFETY: valid C string and an open directory descriptor.
        let rc = unsafe { mkdirat(dir.as_raw_fd(), c.as_ptr(), mode as c_uint) };
        if rc != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Advisory exclusive lock; `Ok(false)` when another open file
    /// description (any process, or another handle in this one) holds it.
    pub fn try_lock_exclusive(file: &File) -> io::Result<bool> {
        // SAFETY: `file` is an open descriptor.
        let rc = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
        if rc == 0 {
            return Ok(true);
        }
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(EWOULDBLOCK) {
            Ok(false)
        } else {
            Err(err)
        }
    }

    pub fn unlock(file: &File) {
        // SAFETY: `file` is an open descriptor; failure is irrelevant at drop.
        unsafe {
            flock(file.as_raw_fd(), LOCK_UN);
        }
    }
}

#[cfg(not(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
mod imp {
    use std::fs::File;
    use std::io;
    use std::os::raw::c_int;

    pub const SUPPORTED: bool = false;
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
pub fn open_dir_nofollow(path: &std::path::Path) -> io::Result<File> {
    // Walk from the filesystem root is unnecessary here: the caller chooses
    // the root; only its final component must not be a symlink. `std` has no
    // O_NOFOLLOW knob, so open the parent with std and the last component
    // with `openat`.
    let (parent, name) = match (path.parent(), path.file_name().and_then(|n| n.to_str())) {
        (Some(p), Some(n)) if !n.is_empty() => (p, n),
        _ => return open_dir_std(path), // "/" or similar: nothing to refuse
    };
    let parent = open_dir_std(if parent.as_os_str().is_empty() {
        std::path::Path::new(".")
    } else {
        parent
    })?;
    open_dir_at_nofollow(&parent, name)
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
/// refusal consistently, an `ENOTDIR` result is probed once more without
/// `O_DIRECTORY` and mapped to `ELOOP` when that probe reports a symlink.
pub fn open_dir_at_nofollow(dir: &File, name: &str) -> io::Result<File> {
    match open_at(dir, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW, 0) {
        Err(e) if errno_is(&e, ENOTDIR) => match open_at(dir, name, O_RDONLY | O_NOFOLLOW, 0) {
            Err(probe) if errno_is(&probe, ELOOP) => Err(probe),
            _ => Err(e),
        },
        other => other,
    }
}
