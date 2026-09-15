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
    use std::ffi::{CStr, c_void};
    use std::fs::File;
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
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
        // `buf` is a `struct stat`; only the fields decoded in `flags::decode_stat`
        // are read, from a zeroed buffer larger than any supported layout.
        #[cfg_attr(
            all(target_os = "macos", target_arch = "x86_64"),
            link_name = "fstatat$INODE64"
        )]
        fn fstatat(dirfd: c_int, path: *const c_char, buf: *mut u8, flags: c_int) -> c_int;
        #[cfg_attr(
            all(target_os = "macos", target_arch = "x86_64"),
            link_name = "fdopendir$INODE64"
        )]
        fn fdopendir(fd: c_int) -> *mut c_void;
        // Returns a `struct dirent`; only `d_name` (at `DIRENT_NAME_OFFSET`)
        // is read.
        #[cfg_attr(
            all(target_os = "macos", target_arch = "x86_64"),
            link_name = "readdir$INODE64"
        )]
        fn readdir(dirp: *mut c_void) -> *const u8;
        fn closedir(dirp: *mut c_void) -> c_int;
        #[cfg(target_os = "macos")]
        fn __error() -> *mut c_int;
        #[cfg(target_os = "linux")]
        fn __errno_location() -> *mut c_int;
        #[cfg(target_os = "macos")]
        fn renameatx_np(
            fromfd: c_int,
            from: *const c_char,
            tofd: c_int,
            to: *const c_char,
            flags: c_uint,
        ) -> c_int;
        #[cfg(target_os = "linux")]
        fn renameat2(
            olddirfd: c_int,
            old: *const c_char,
            newdirfd: c_int,
            new: *const c_char,
            flags: c_uint,
        ) -> c_int;
    }

    /// The thread's `errno` lvalue.
    fn errno_ptr() -> *mut c_int {
        // SAFETY: both functions return the calling thread's errno location.
        #[cfg(target_os = "macos")]
        unsafe {
            __error()
        }
        #[cfg(target_os = "linux")]
        unsafe {
            __errno_location()
        }
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
        pub const AT_SYMLINK_NOFOLLOW: c_int = 0x0020;
        pub const ELOOP: i32 = 62;
        pub const EWOULDBLOCK: i32 = 35;
        pub const ENOSYS: i32 = 78;
        pub const ENOTSUP: i32 = 45;
        /// `renameatx_np`'s `RENAME_EXCL`.
        pub(super) const RENAME_NOREPLACE: std::os::raw::c_uint = 0x0004;
        /// `struct dirent`: `d_ino: u64`, `d_seekoff: u64`, `d_reclen: u16`,
        /// `d_namlen: u16`, `d_type: u8`, then `d_name`.
        pub(super) const DIRENT_NAME_OFFSET: usize = 21;

        /// `struct stat` (64-bit inode): `st_dev: i32` at 0, `st_mode: u16`
        /// at 4, `st_ino: u64` at 8.
        pub(super) fn decode_stat(b: &[u8]) -> super::super::EntryStat {
            super::super::EntryStat {
                dev: i32::from_ne_bytes(b[0..4].try_into().unwrap()) as u64,
                mode: u16::from_ne_bytes(b[4..6].try_into().unwrap()) as u32,
                ino: u64::from_ne_bytes(b[8..16].try_into().unwrap()),
            }
        }
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
        pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
        pub const ELOOP: i32 = 40;
        pub const EWOULDBLOCK: i32 = 11;
        pub const ENOSYS: i32 = 38;
        pub const ENOTSUP: i32 = 95;
        /// `renameat2`'s `RENAME_NOREPLACE`.
        pub(super) const RENAME_NOREPLACE: std::os::raw::c_uint = 1;
        /// `struct dirent`: `d_ino: u64`, `d_off: i64`, `d_reclen: u16`,
        /// `d_type: u8`, then `d_name`.
        pub(super) const DIRENT_NAME_OFFSET: usize = 19;

        /// `struct stat`: `st_dev: u64` at 0, `st_ino: u64` at 8,
        /// `st_nlink: u64` at 16, `st_mode: u32` at 24.
        pub(super) fn decode_stat(b: &[u8]) -> super::super::EntryStat {
            super::super::EntryStat {
                dev: u64::from_ne_bytes(b[0..8].try_into().unwrap()),
                ino: u64::from_ne_bytes(b[8..16].try_into().unwrap()),
                mode: u32::from_ne_bytes(b[24..28].try_into().unwrap()),
            }
        }
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
        pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
        pub const ELOOP: i32 = 40;
        pub const EWOULDBLOCK: i32 = 11;
        pub const ENOSYS: i32 = 38;
        pub const ENOTSUP: i32 = 95;
        /// `renameat2`'s `RENAME_NOREPLACE`.
        pub(super) const RENAME_NOREPLACE: std::os::raw::c_uint = 1;
        /// `struct dirent`: `d_ino: u64`, `d_off: i64`, `d_reclen: u16`,
        /// `d_type: u8`, then `d_name`.
        pub(super) const DIRENT_NAME_OFFSET: usize = 19;

        /// Generic `struct stat`: `st_dev: u64` at 0, `st_ino: u64` at 8,
        /// `st_mode: u32` at 16.
        pub(super) fn decode_stat(b: &[u8]) -> super::super::EntryStat {
            super::super::EntryStat {
                dev: u64::from_ne_bytes(b[0..8].try_into().unwrap()),
                ino: u64::from_ne_bytes(b[8..16].try_into().unwrap()),
                mode: u32::from_ne_bytes(b[16..20].try_into().unwrap()),
            }
        }
    }
    pub use flags::*;

    pub const ENOENT: i32 = 2;
    pub const EEXIST: i32 = 17;
    pub const ENOTDIR: i32 = 20;
    pub const EINVAL: i32 = 22;
    const LOCK_EX: c_int = 2;
    const LOCK_NB: c_int = 4;
    const LOCK_UN: c_int = 8;

    fn cstr(name: impl AsRef<[u8]>) -> io::Result<CString> {
        CString::new(name.as_ref())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in path component"))
    }

    /// `openat(dirfd, name, flags | O_CLOEXEC | O_NONBLOCK, mode)`.
    ///
    /// `O_NONBLOCK` is always added so that opening a name which is (or was
    /// swapped to) a FIFO or a device returns at once instead of blocking
    /// until a writer or carrier appears. It has no effect on reads or writes
    /// of regular files or on directories. It is a backstop, not the check:
    /// callers first classify an existing name with [`stat_at_nofollow`] and
    /// open only a regular file, then `fstat` the descriptor and refuse
    /// anything but a regular file (the name can change between the two).
    pub fn open_at(dir: &File, name: &str, flags: c_int, mode: u32) -> io::Result<File> {
        open_at_bytes(dir, name.as_bytes(), flags, mode)
    }

    /// [`open_at`] for a name that need not be UTF-8.
    pub fn open_at_bytes(dir: &File, name: &[u8], flags: c_int, mode: u32) -> io::Result<File> {
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

    /// Renames `old` to `new` inside `dir` only if `new` does not exist
    /// (`renameatx_np(RENAME_EXCL)` on macOS, `renameat2(RENAME_NOREPLACE)`
    /// on Linux): an entry created at `new` after the caller checked is never
    /// replaced, and the call fails with `EEXIST`. Filesystems or kernels
    /// without the primitive fail with an error for which
    /// [`noreplace_unsupported`] is true; nothing was renamed then.
    pub fn rename_at_noreplace(dir: &File, old: &str, new: &str) -> io::Result<()> {
        let (o, n) = (cstr(old)?, cstr(new)?);
        let fd = dir.as_raw_fd();
        // SAFETY: valid C strings and an open directory descriptor.
        #[cfg(target_os = "macos")]
        let rc = unsafe { renameatx_np(fd, o.as_ptr(), fd, n.as_ptr(), RENAME_NOREPLACE) };
        // SAFETY: as above.
        #[cfg(target_os = "linux")]
        let rc = unsafe { renameat2(fd, o.as_ptr(), fd, n.as_ptr(), RENAME_NOREPLACE) };
        if rc != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Whether a [`rename_at_noreplace`] failure means the primitive is not
    /// available here (rather than a real rename failure).
    pub fn noreplace_unsupported(err: &io::Error) -> bool {
        [EINVAL, ENOSYS, ENOTSUP]
            .iter()
            .any(|&c| err.raw_os_error() == Some(c))
    }

    /// Names of the entries of the open directory `dir`, excluding `.` and
    /// `..`, as raw bytes.
    ///
    /// The directory is read with `fdopendir` on a fresh descriptor opened as
    /// `openat(dir, ".")`: the same directory object `dir` refers to, with
    /// its own read offset, never re-resolved from a path string. Entries
    /// are only named, not followed or stat'ed.
    pub fn list_dir(dir: &File) -> io::Result<Vec<Vec<u8>>> {
        let fresh = open_at_bytes(dir, b".", O_RDONLY | O_DIRECTORY, 0)?;
        let fd = fresh.into_raw_fd();
        // SAFETY: `fd` is an open directory descriptor; on success the DIR
        // stream owns it and `closedir` closes it.
        let dirp = unsafe { fdopendir(fd) };
        if dirp.is_null() {
            let err = io::Error::last_os_error();
            // SAFETY: fdopendir failed, so `fd` is still ours to close.
            drop(unsafe { File::from_raw_fd(fd) });
            return Err(err);
        }
        let mut names = Vec::new();
        let result = loop {
            // SAFETY: `errno_ptr` is the thread's errno; clearing it lets a
            // NULL return be told apart as end-of-directory or error.
            unsafe { *errno_ptr() = 0 };
            // SAFETY: `dirp` is a live DIR stream.
            let ent = unsafe { readdir(dirp) };
            if ent.is_null() {
                // SAFETY: as above.
                let errno = unsafe { *errno_ptr() };
                break if errno == 0 {
                    Ok(())
                } else {
                    Err(io::Error::from_raw_os_error(errno))
                };
            }
            // SAFETY: `ent` points at a `struct dirent` whose NUL-terminated
            // `d_name` starts at `DIRENT_NAME_OFFSET`; it stays valid until
            // the next `readdir` on this stream, and is copied before that.
            let name = unsafe { CStr::from_ptr(ent.add(DIRENT_NAME_OFFSET).cast::<c_char>()) };
            let name = name.to_bytes();
            if name != b"." && name != b".." {
                names.push(name.to_vec());
            }
        };
        // SAFETY: `dirp` is a live DIR stream, closed exactly once.
        unsafe { closedir(dirp) };
        result.map(|()| names)
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

    /// `fstatat(dirfd, name, AT_SYMLINK_NOFOLLOW)`: classifies the entry
    /// `name` of the open directory `dir` without following a symlink and
    /// without opening it (so a device's open side effects never run).
    pub fn stat_at_nofollow(dir: &File, name: &[u8]) -> io::Result<super::EntryStat> {
        let c = cstr(name)?;
        // Zeroed and larger than `struct stat` on every supported target
        // (144 bytes on macOS and Linux x86_64, 128 on Linux aarch64), and
        // 8-byte aligned like the struct.
        let mut buf = [0u64; 32];
        // SAFETY: valid C string, an open directory descriptor, and a
        // writable buffer at least as large as `struct stat`.
        let rc = unsafe {
            fstatat(
                dir.as_raw_fd(),
                c.as_ptr(),
                buf.as_mut_ptr().cast::<u8>(),
                AT_SYMLINK_NOFOLLOW,
            )
        };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        let bytes: Vec<u8> = buf.iter().flat_map(|w| w.to_ne_bytes()).collect();
        Ok(flags::decode_stat(&bytes))
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
    pub fn open_at_bytes(_: &File, _: &[u8], _: c_int, _: u32) -> io::Result<File> {
        Err(unsupported())
    }
    pub fn stat_at_nofollow(_: &File, _: &[u8]) -> io::Result<super::EntryStat> {
        Err(unsupported())
    }
    pub fn rename_at(_: &File, _: &str, _: &str) -> io::Result<()> {
        Err(unsupported())
    }
    pub fn rename_at_noreplace(_: &File, _: &str, _: &str) -> io::Result<()> {
        Err(unsupported())
    }
    pub fn noreplace_unsupported(_: &io::Error) -> bool {
        false
    }
    pub fn list_dir(_: &File) -> io::Result<Vec<Vec<u8>>> {
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
    // with `openat`. The name is passed as raw bytes: a root whose final
    // component is not valid UTF-8 gets the same `O_NOFOLLOW` open, never a
    // symlink-following fallback.
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

/// File type bits of `st_mode` (identical on every supported target).
pub const S_IFMT: u32 = 0o170000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFLNK: u32 = 0o120000;

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

    /// The hand-decoded `struct stat` fields must agree with `std`'s own
    /// `lstat` for every file type this crate distinguishes.
    #[test]
    fn stat_list_and_noreplace_match_std() {
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

        // `list_dir` agrees with `std::fs::read_dir`, and a second listing
        // of the same handle is complete (its own read offset each time).
        let mut expected: Vec<Vec<u8>> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| {
                use std::os::unix::ffi::OsStrExt;
                e.unwrap().file_name().as_bytes().to_vec()
            })
            .collect();
        expected.sort();
        for _ in 0..2 {
            let mut names = list_dir(&handle).unwrap();
            names.sort();
            assert_eq!(names, expected);
        }

        // `rename_at_noreplace` never replaces an existing entry.
        std::fs::write(dir.join("other"), "y").unwrap();
        match rename_at_noreplace(&handle, "other", "file") {
            Err(e) if noreplace_unsupported(&e) => {}
            Err(e) => assert!(errno_is(&e, EEXIST), "{e:?}"),
            Ok(()) => panic!("replaced an existing entry"),
        }
        assert_eq!(std::fs::read_to_string(dir.join("file")).unwrap(), "x");
        rename_at_noreplace(&handle, "other", "fresh")
            .or_else(|e| {
                if noreplace_unsupported(&e) {
                    rename_at(&handle, "other", "fresh")
                } else {
                    Err(e)
                }
            })
            .unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("fresh")).unwrap(), "y");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
