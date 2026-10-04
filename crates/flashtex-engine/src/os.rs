//! Every operating-system call of the engine, in one place (DESIGN.md §16
//! rule 2): Unix (macOS and Linux), Windows, and WASI (the batch path only;
//! docs/evidence/portability-2026-10-03/). Any other target is a compile
//! error, never a silent Linux-constant fallback.
//!
//! The rest of the crate calls these functions and keeps no `cfg` on the
//! OS of its own, except where a whole feature is Unix-only (the engine
//! host's socket pair for exports, `src/host/server.rs`).
//!
//! On macOS and Linux every function here is the code the call sites had
//! before this module existed; moving it changed nothing they do.

#[cfg(not(any(unix, windows, target_os = "wasi")))]
compile_error!("flashtex-engine: add this OS to src/os.rs");

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
compile_error!(
    "flashtex-engine: add this Unix's constants to src/os.rs (macOS and Linux are known)"
);

use std::path::Path;
use std::process::Command;

// MSVC's Universal CRT defines `snprintf` and `sscanf` inline in <stdio.h>
// and exports no symbols of those names; the C-formatted numbers of
// `pdftex/cfmt.rs` link them from the library Microsoft provides for that.
// (MinGW-w64 exports them.)
#[cfg(all(windows, target_env = "msvc"))]
#[link(name = "legacy_stdio_definitions")]
extern "C" {}

// ---------------------------------------------------------------------------
// The word space's anonymous memory (src/arena.rs)
// ---------------------------------------------------------------------------

/// `len` zero bytes, page aligned, for the engine's word space. On Unix an
/// anonymous private `mmap`, which the kernel commits lazily (the hundreds
/// of megabytes texmf.cnf's sizes reserve cost address space, not
/// memory). On Windows `VirtualAlloc(MEM_RESERVE | MEM_COMMIT)`: pages are
/// still zero-filled on first touch and take no RAM before it, but the
/// whole length counts against the system commit limit (RAM plus page
/// file) at once. On WASI the global allocator's zeroed memory.
pub fn alloc_zeroed(len: usize) -> *mut u8 {
    let p = imp::alloc_zeroed(len);
    if p.is_null() {
        panic!("flashtex: cannot map {len} bytes for the engine's word space");
    }
    p
}

/// # Safety
/// `p`/`len` must come from [`alloc_zeroed`] and not be used afterwards.
pub unsafe fn free(p: *mut u8, len: usize) {
    imp::free(p, len)
}

// ---------------------------------------------------------------------------
// A file mapped read-only (the persisted S0, src/host/mod.rs)
// ---------------------------------------------------------------------------

/// A whole file, read-only: mapped on Unix and Windows, read into memory on
/// WASI.
pub struct MappedFile {
    ptr: *mut u8,
    len: usize,
    #[cfg(windows)]
    mapping: usize,
    #[cfg(target_os = "wasi")]
    _owned: Vec<u8>,
}

// SAFETY: the mapping is read-only and owned by the value.
unsafe impl Send for MappedFile {}
unsafe impl Sync for MappedFile {}

impl MappedFile {
    pub fn open(path: &str) -> Result<MappedFile, String> {
        let f = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
        let len = f.metadata().map_err(|e| format!("{path}: {e}"))?.len() as usize;
        if len == 0 {
            return Err(format!("{path} is empty"));
        }
        imp::map_file(&f, len).map_err(|()| format!("{path}: mmap failed"))
    }

    pub fn bytes(&self) -> &[u8] {
        // SAFETY: the mapping is `len` bytes and lives as long as `self`.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for MappedFile {
    fn drop(&mut self) {
        imp::unmap_file(self)
    }
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

/// `struct tm` as far as POSIX fixes it, followed by the BSD/glibc/musl
/// extensions macOS, Linux and wasi-libc have. Windows' `struct tm` is the
/// first nine fields; the C library writes only those.
#[repr(C)]
pub struct Tm {
    pub tm_sec: i32,
    pub tm_min: i32,
    pub tm_hour: i32,
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
    pub tm_gmtoff: std::ffi::c_long,
    pub tm_zone: *const std::ffi::c_char,
}

/// The C library's broken-down time of `t` (seconds since the epoch), in
/// UTC or the local time zone: `gmtime_r`/`localtime_r`, or on Windows
/// `_gmtime64_s`/`_localtime64_s`.
pub fn broken_down(t: i64, utc: bool) -> Tm {
    let mut tm = Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };
    imp::broken_down(t, utc, &mut tm);
    tm
}

/// CPU time of this thread, in seconds (the machine is shared: wall time
/// includes other processes' load, this does not). On WASI, which has no
/// per-thread CPU clock, the process's monotonic time since the first call.
pub fn thread_cpu_s() -> f64 {
    imp::thread_cpu_s()
}

/// Instructions retired and cycles elapsed by this thread so far, from the
/// CPU's fixed counters (macOS: the kernel's per-thread counts, which do not
/// move with other processes' load; for measurement only, DESIGN.md §5.6).
/// `None` where the platform does not give them. The macOS call is a private
/// symbol of libsystem_kernel, so it is looked up when first needed rather
/// than linked: a system without it gives `None` instead of failing to load.
pub fn thread_counts() -> Option<(u64, u64)> {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::{c_char, c_void};
        use std::sync::OnceLock;
        // xnu `thread_selfcounts`, type 1: the thread's instructions and
        // cycles.
        type SelfCounts = unsafe extern "C" fn(i32, *mut u64, usize) -> i32;
        extern "C" {
            fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        }
        // <dlfcn.h>: RTLD_DEFAULT is ((void *) -2) on macOS.
        const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
        static F: OnceLock<Option<SelfCounts>> = OnceLock::new();
        let f = (*F.get_or_init(|| {
            // SAFETY: dlsym with a NUL-terminated name only looks it up.
            let p = unsafe { dlsym(RTLD_DEFAULT, c"thread_selfcounts".as_ptr()) };
            // SAFETY: the symbol, when present, is a function of this type.
            (!p.is_null()).then(|| unsafe { std::mem::transmute::<*mut c_void, SelfCounts>(p) })
        }))?;
        let mut c = [0u64; 2];
        // SAFETY: the kernel writes at most `nbytes` into `c`.
        let r = unsafe { f(1, c.as_mut_ptr(), std::mem::size_of_val(&c)) };
        (r == 0).then_some((c[0], c[1]))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

/// The file's inode number, for staleness stamps; 0 where the standard
/// library does not expose one (Windows' file index is unstable in std),
/// so the stamp there rests on size and modification time alone.
pub fn file_id(m: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        std::os::unix::fs::MetadataExt::ino(m)
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        0
    }
}

/// A file's stat fields, for the format cache's signatures: size,
/// modification and status-change times (ns), inode and device. Windows has
/// no status-change time, inode or device in std: there `ctime_ns` is the
/// creation time and `ino`/`dev` are 0.
pub struct FileStat {
    pub size: u64,
    pub mtime_ns: i128,
    pub ctime_ns: i128,
    pub ino: u64,
    pub dev: u64,
}

pub fn file_stat(m: &std::fs::Metadata) -> FileStat {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        FileStat {
            size: m.size(),
            mtime_ns: m.mtime() as i128 * 1_000_000_000 + m.mtime_nsec() as i128,
            ctime_ns: m.ctime() as i128 * 1_000_000_000 + m.ctime_nsec() as i128,
            ino: m.ino(),
            dev: m.dev(),
        }
    }
    #[cfg(not(unix))]
    {
        let ns = |t: std::io::Result<std::time::SystemTime>| {
            t.ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos() as i128)
        };
        FileStat {
            size: m.len(),
            mtime_ns: ns(m.modified()),
            ctime_ns: ns(m.created()),
            ino: 0,
            dev: 0,
        }
    }
}

/// Write all of `buf` at byte `offset` of `f` (`pwrite`; on Windows
/// `seek_write`, which moves the file position, which no caller relies on).
pub fn write_all_at(f: &std::fs::File, buf: &[u8], offset: u64) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::FileExt::write_all_at(f, buf, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let (mut buf, mut offset) = (buf, offset);
        while !buf.is_empty() {
            match f.seek_write(buf, offset) {
                Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
                Ok(n) => {
                    buf = &buf[n..];
                    offset += n as u64;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    // std's WASI `FileExt` is unstable: a seek, then the write.
    #[cfg(target_os = "wasi")]
    {
        use std::io::{Seek, Write};
        let mut f = f;
        f.seek(std::io::SeekFrom::Start(offset))?;
        f.write_all(buf)
    }
}

/// Make `dst` a copy of `src` sharing its blocks (APFS `clonefile`, O(1));
/// false where the file system cannot (the caller copies instead).
pub fn clone_file(src: &str, dst: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn clonefile(
                src: *const std::ffi::c_char,
                dst: *const std::ffi::c_char,
                flags: u32,
            ) -> i32;
        }
        let (Ok(a), Ok(b)) = (std::ffi::CString::new(src), std::ffi::CString::new(dst)) else {
            return false;
        };
        let _ = std::fs::remove_file(dst);
        // SAFETY: two NUL-terminated paths.
        unsafe { clonefile(a.as_ptr(), b.as_ptr(), 0) == 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (src, dst);
        false
    }
}

/// A file name as the C code sees it (bytes) as a path: the bytes
/// themselves on Unix, UTF-8 (lossily) elsewhere.
pub fn path_from_bytes(name: &[u8]) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        std::path::PathBuf::from(std::ffi::OsStr::from_bytes(name))
    }
    #[cfg(not(unix))]
    {
        std::path::PathBuf::from(String::from_utf8_lossy(name).into_owned())
    }
}

/// `name` as an executable's file name on this OS (`kpsewhich.exe` on
/// Windows).
pub fn exe_name(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

/// Set an environment variable that the C parts (kpathsea's `getenv`) must
/// see too. On Unix that is the one environment `std::env::set_var` writes.
/// Windows' C runtime keeps its own copy, taken at start-up, which
/// `SetEnvironmentVariableW` (std) does not update: there the value goes
/// to both, through the CRT's `_putenv_s` as well.
pub fn set_env(var: &str, value: &str) {
    std::env::set_var(var, value);
    #[cfg(windows)]
    {
        extern "C" {
            fn _putenv_s(name: *const std::ffi::c_char, value: *const std::ffi::c_char) -> i32;
        }
        if let (Ok(k), Ok(v)) = (std::ffi::CString::new(var), std::ffi::CString::new(value)) {
            // SAFETY: two NUL-terminated strings; the CRT copies them.
            unsafe { _putenv_s(k.as_ptr(), v.as_ptr()) };
        }
    }
}

/// The shell web2c's `runsystem` and `runpopen` use: `/bin/sh -c CMD`, or
/// on Windows `%COMSPEC% /c CMD` (`cmd.exe`), as `_wsystem` runs it, with
/// the command line passed through unquoted. On Unix the shell's `argv[0]`
/// is `sh`, as the C library's `system()` and `popen()` give it, so the
/// shell's own messages start `sh: `, not `/bin/sh: `.
pub fn shell_command(cmd: &[u8]) -> Command {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::process::CommandExt;
        let mut c = Command::new("/bin/sh");
        c.arg0("sh").arg("-c").arg(std::ffi::OsStr::from_bytes(cmd));
        c
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
        let mut c = Command::new(shell);
        c.raw_arg("/c")
            .raw_arg(String::from_utf8_lossy(cmd).as_ref());
        c
    }
    #[cfg(target_os = "wasi")]
    {
        // WASI cannot start processes; spawning this fails with
        // `Unsupported`, which the callers report like a failed fork.
        let mut c = Command::new("/bin/sh");
        c.arg("-c").arg(String::from_utf8_lossy(cmd).as_ref());
        c
    }
}

/// The quote character web2c's restricted shell escape wraps arguments in
/// (texmfmp.c `QUOTE`): `"` on Windows, `'` elsewhere.
pub const SHELL_QUOTE: u8 = if cfg!(windows) { b'"' } else { b'\'' };

/// The environment variable that tells `flashtex-host` to run as the
/// engine where `argv[0]` cannot be set (Windows): see [`engine_command`].
pub const INVOKED_AS_ENV: &str = "FLASHTEX_INVOKED_AS";

/// `exe` (an engine binary) to be run as `pdftex`, as fmtutil runs INITEX
/// and pdflatex runs the engine: `argv[0]` is `pdftex` on Unix. Windows
/// cannot set `argv[0]` apart from the program, so there the child gets
/// `FLASHTEX_INVOKED_AS=pdftex` instead, which [`invoked_as`] reads.
pub fn engine_command(exe: &Path) -> Command {
    let mut c = Command::new(exe);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        c.arg0("pdftex");
    }
    #[cfg(not(unix))]
    c.env(INVOKED_AS_ENV, "pdftex");
    c
}

/// The name this process was invoked under: the file name of `argv[0]`
/// without `.exe`, unless [`engine_command`] said otherwise through the
/// environment (which is then removed, so that the engine's own children
/// do not inherit it).
pub fn invoked_as(argv0: &str) -> String {
    #[cfg(not(unix))]
    if let Some(v) = std::env::var_os(INVOKED_AS_ENV) {
        // SAFETY (edition 2024 makes this `unsafe`): called at the start of
        // `main`, before any other thread exists.
        std::env::remove_var(INVOKED_AS_ENV);
        return v.to_string_lossy().into_owned();
    }
    let base = Path::new(argv0)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match base.len().checked_sub(4) {
        Some(k) if cfg!(windows) && base[k..].eq_ignore_ascii_case(".exe") => base[..k].to_string(),
        _ => base,
    }
}

/// Make `link` run `target` (the distribution tool's `bin/pdftex`): a
/// symbolic link on Unix, a hard link (else a copy) elsewhere.
pub fn link_executable(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(not(unix))]
    {
        std::fs::hard_link(target, link).or_else(|_| std::fs::copy(target, link).map(|_| ()))
    }
}

/// Make the engine host's socket file reachable by its owner only (mode
/// 0600). On Windows the socket file keeps the ACL it inherits from its
/// directory (a per-user directory such as `%TEMP%` is private already).
pub fn restrict_to_owner(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// The display-list channel of an export child (`src/host/server.rs`
/// `start_export`): the engine writes its frames into it, the host reads
/// them.
///
/// * Unix: a socket pair whose other end the child inherits as descriptor
///   3, `FLASHTEX_DISPLAY_LIST=fd:3`.
/// * Windows, which has neither `socketpair` nor numbered-descriptor
///   inheritance (DESIGN.md §16 rule 1): a listener on a fresh path in the
///   temporary directory, `FLASHTEX_DISPLAY_LIST=socket:PATH`, accepted
///   once. Should the child end without connecting, the read ends as
///   soon as the caller says the child has exited.
#[cfg(not(feature = "tex82"))]
pub struct ExportChannel {
    #[cfg(unix)]
    ours: flashtex_display_list::transport::Stream,
    #[cfg(unix)]
    theirs: flashtex_display_list::transport::Stream,
    #[cfg(not(unix))]
    listener: flashtex_display_list::transport::Listener,
    #[cfg(not(unix))]
    path: std::path::PathBuf,
}

#[cfg(not(feature = "tex82"))]
impl ExportChannel {
    pub fn open() -> std::io::Result<ExportChannel> {
        use flashtex_display_list::transport::*;
        #[cfg(unix)]
        {
            let (ours, theirs) = Stream::pair()?;
            widen_socket_buffers(&ours);
            widen_socket_buffers(&theirs);
            Ok(ExportChannel { ours, theirs })
        }
        #[cfg(not(unix))]
        {
            use std::sync::atomic::{AtomicU64, Ordering};
            static N: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "flashtex-export-{}-{}.sock",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::remove_file(&path);
            let listener = Listener::bind(&path)?;
            listener.set_nonblocking(true)?;
            Ok(ExportChannel { listener, path })
        }
    }

    /// Give `cmd` (the engine child) its end of the channel.
    pub fn attach(&self, cmd: &mut Command) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            use std::os::unix::process::CommandExt;
            extern "C" {
                fn dup2(oldfd: i32, newfd: i32) -> i32;
            }
            let fd = self.theirs.as_raw_fd();
            cmd.env("FLASHTEX_DISPLAY_LIST", "fd:3");
            // Descriptor 3 in the child is our socket pair's other end.
            // SAFETY: dup2 is async-signal-safe and touches no memory.
            unsafe {
                cmd.pre_exec(move || {
                    if dup2(fd, 3) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        #[cfg(not(unix))]
        {
            let mut v = std::ffi::OsString::from("socket:");
            v.push(&self.path);
            cmd.env("FLASHTEX_DISPLAY_LIST", v);
        }
    }

    /// After the child was spawned: our end, to read frames from until the
    /// child closes its end. `exited` turns true when the child has ended.
    pub fn reader(
        self,
        exited: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Box<dyn std::io::Read + Send> {
        #[cfg(unix)]
        {
            let _ = exited;
            drop(self.theirs);
            Box::new(self.ours)
        }
        #[cfg(not(unix))]
        {
            Box::new(Accepting {
                ch: self,
                stream: None,
                exited,
            })
        }
    }
}

#[cfg(all(not(feature = "tex82"), not(unix)))]
struct Accepting {
    ch: ExportChannel,
    stream: Option<flashtex_display_list::transport::Stream>,
    exited: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(all(not(feature = "tex82"), not(unix)))]
impl std::io::Read for Accepting {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use std::sync::atomic::Ordering;
        if self.stream.is_none() {
            loop {
                // Read before the attempt: a child that had exited by then
                // and connected at all is in the backlog for this accept.
                let gone = self.exited.load(Ordering::Acquire);
                match self.ch.listener.accept() {
                    Ok((s, _)) => {
                        s.set_nonblocking(false)?;
                        flashtex_display_list::transport::widen_socket_buffers(&s);
                        self.stream = Some(s);
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if gone {
                            return Ok(0);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        self.stream.as_mut().unwrap().read(buf)
    }
}

#[cfg(all(not(feature = "tex82"), not(unix)))]
impl Drop for ExportChannel {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Unix
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod imp {
    use super::{MappedFile, Tm};
    use std::ffi::c_void;

    extern "C" {
        fn mmap(
            addr: *mut c_void,
            len: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            off: i64,
        ) -> *mut c_void;
        fn munmap(addr: *mut c_void, len: usize) -> i32;
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
        fn gmtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
        fn clock_gettime(clk: i32, tp: *mut Timespec) -> i32;
    }
    const PROT_READ: i32 = 1;
    const PROT_WRITE: i32 = 2;
    const MAP_PRIVATE: i32 = 2;
    #[cfg(target_os = "macos")]
    const MAP_ANON: i32 = 0x1000;
    #[cfg(target_os = "linux")]
    const MAP_ANON: i32 = 0x20;
    #[cfg(target_os = "macos")]
    const CLOCK_THREAD_CPUTIME_ID: i32 = 16;
    #[cfg(target_os = "linux")]
    const CLOCK_THREAD_CPUTIME_ID: i32 = 3;

    #[repr(C)]
    struct Timespec {
        sec: i64,
        nsec: i64,
    }

    pub fn alloc_zeroed(len: usize) -> *mut u8 {
        // SAFETY: an anonymous private mapping has no preconditions.
        let p = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANON,
                -1,
                0,
            )
        };
        if p as isize == -1 {
            std::ptr::null_mut()
        } else {
            p as *mut u8
        }
    }

    pub unsafe fn free(p: *mut u8, len: usize) {
        munmap(p as *mut c_void, len);
    }

    pub fn map_file(f: &std::fs::File, len: usize) -> Result<MappedFile, ()> {
        use std::os::fd::AsRawFd;
        // SAFETY: a private read-only mapping of an open file.
        let p = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                PROT_READ,
                MAP_PRIVATE,
                f.as_raw_fd(),
                0,
            )
        };
        if p as isize == -1 {
            return Err(());
        }
        Ok(MappedFile {
            ptr: p as *mut u8,
            len,
        })
    }

    pub fn unmap_file(m: &mut MappedFile) {
        // SAFETY: from `map_file`.
        unsafe { munmap(m.ptr as *mut c_void, m.len) };
    }

    pub fn broken_down(t: i64, utc: bool, tm: &mut Tm) {
        // SAFETY: both functions only write the `struct tm` they are given.
        unsafe {
            if utc {
                gmtime_r(&t, tm);
            } else {
                localtime_r(&t, tm);
            }
        }
    }

    pub fn thread_cpu_s() -> f64 {
        let mut t = Timespec { sec: 0, nsec: 0 };
        // SAFETY: an out-parameter of the right layout.
        unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut t) };
        t.sec as f64 + t.nsec as f64 * 1e-9
    }
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::{MappedFile, Tm};
    use std::ffi::c_void;

    const MEM_COMMIT: u32 = 0x1000;
    const MEM_RESERVE: u32 = 0x2000;
    const MEM_RELEASE: u32 = 0x8000;
    const PAGE_READONLY: u32 = 0x02;
    const PAGE_READWRITE: u32 = 0x04;
    const FILE_MAP_READ: u32 = 0x0004;

    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn VirtualAlloc(addr: *mut c_void, size: usize, ty: u32, protect: u32) -> *mut c_void;
        fn VirtualFree(addr: *mut c_void, size: usize, ty: u32) -> i32;
        fn CreateFileMappingW(
            file: *mut c_void,
            attrs: *mut c_void,
            protect: u32,
            size_high: u32,
            size_low: u32,
            name: *const u16,
        ) -> *mut c_void;
        fn MapViewOfFile(
            mapping: *mut c_void,
            access: u32,
            off_high: u32,
            off_low: u32,
            len: usize,
        ) -> *mut c_void;
        fn UnmapViewOfFile(base: *const c_void) -> i32;
        fn CloseHandle(h: *mut c_void) -> i32;
        fn GetCurrentThread() -> *mut c_void;
        fn GetThreadTimes(
            thread: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    // The Universal (and MinGW's msvcrt) C runtime: errno_t f(struct tm *,
    // const __time64_t *).
    extern "C" {
        fn _localtime64_s(out: *mut Tm, t: *const i64) -> i32;
        fn _gmtime64_s(out: *mut Tm, t: *const i64) -> i32;
    }

    pub fn alloc_zeroed(len: usize) -> *mut u8 {
        // SAFETY: a fresh private allocation has no preconditions.
        unsafe {
            VirtualAlloc(
                std::ptr::null_mut(),
                len,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            ) as *mut u8
        }
    }

    pub unsafe fn free(p: *mut u8, _len: usize) {
        VirtualFree(p as *mut c_void, 0, MEM_RELEASE);
    }

    pub fn map_file(f: &std::fs::File, len: usize) -> Result<MappedFile, ()> {
        use std::os::windows::io::AsRawHandle;
        // SAFETY: a read-only mapping of an open file handle, closed below
        // on failure or in `unmap_file`.
        unsafe {
            let h = CreateFileMappingW(
                f.as_raw_handle() as *mut c_void,
                std::ptr::null_mut(),
                PAGE_READONLY,
                0,
                0,
                std::ptr::null(),
            );
            if h.is_null() {
                return Err(());
            }
            let p = MapViewOfFile(h, FILE_MAP_READ, 0, 0, len);
            if p.is_null() {
                CloseHandle(h);
                return Err(());
            }
            Ok(MappedFile {
                ptr: p as *mut u8,
                len,
                mapping: h as usize,
            })
        }
    }

    pub fn unmap_file(m: &mut MappedFile) {
        // SAFETY: from `map_file`.
        unsafe {
            UnmapViewOfFile(m.ptr as *const c_void);
            CloseHandle(m.mapping as *mut c_void);
        }
    }

    pub fn broken_down(t: i64, utc: bool, tm: &mut Tm) {
        // SAFETY: both functions only write the nine-int `struct tm` they
        // are given, which `Tm` begins with.
        unsafe {
            if utc {
                _gmtime64_s(tm, &t);
            } else {
                _localtime64_s(tm, &t);
            }
        }
    }

    pub fn thread_cpu_s() -> f64 {
        let (mut c, mut e, mut k, mut u) = Default::default();
        // SAFETY: four FILETIME out-parameters of the current thread.
        unsafe { GetThreadTimes(GetCurrentThread(), &mut c, &mut e, &mut k, &mut u) };
        let t = |f: &FileTime| ((f.high as u64) << 32 | f.low as u64) as f64 * 1e-7;
        t(&k) + t(&u)
    }
}

// ---------------------------------------------------------------------------
// WASI (preview 1): no mmap, no per-thread CPU clock, no processes
// ---------------------------------------------------------------------------

#[cfg(target_os = "wasi")]
mod imp {
    use super::{MappedFile, Tm};
    use std::alloc::Layout;

    extern "C" {
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
        fn gmtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    }

    const PAGE: usize = 65536;

    pub fn alloc_zeroed(len: usize) -> *mut u8 {
        match Layout::from_size_align(len, PAGE) {
            // SAFETY: a non-zero size (the arena never asks for 0 bytes).
            Ok(l) if len > 0 => unsafe { std::alloc::alloc_zeroed(l) },
            _ => std::ptr::null_mut(),
        }
    }

    pub unsafe fn free(p: *mut u8, len: usize) {
        std::alloc::dealloc(p, Layout::from_size_align_unchecked(len, PAGE));
    }

    pub fn map_file(f: &std::fs::File, len: usize) -> Result<MappedFile, ()> {
        use std::io::Read;
        let mut v = Vec::with_capacity(len);
        (&*f).read_to_end(&mut v).map_err(|_| ())?;
        Ok(MappedFile {
            ptr: v.as_mut_ptr(),
            len: v.len(),
            _owned: v,
        })
    }

    pub fn unmap_file(_m: &mut MappedFile) {}

    pub fn broken_down(t: i64, utc: bool, tm: &mut Tm) {
        // SAFETY: both functions only write the `struct tm` they are given.
        unsafe {
            if utc {
                gmtime_r(&t, tm);
            } else {
                localtime_r(&t, tm);
            }
        }
    }

    pub fn thread_cpu_s() -> f64 {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
    }
}
