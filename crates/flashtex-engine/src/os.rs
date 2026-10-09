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
///
/// Windows stays at reserve-and-commit (issue #1418, reviewed 2026-10-04):
/// committing lazily needs a vectored exception handler that commits the
/// page an access faults on, and that does not cover the kernel. A
/// `WriteFile`/`send` whose buffer reaches an uncommitted page fails with
/// `ERROR_NOACCESS` instead of faulting, and the word space goes to the
/// kernel straight from the mapping (`Arena::bytes`, the persisted S₀ and
/// checkpoint files, `host/mod.rs`, `checkpoint.rs`). Each such path would
/// need committing first. Not simple, so not done; the cost is commit
/// charge, not memory.
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
/// `None` where the platform does not give them.
pub fn thread_counts() -> Option<(u64, u64)> {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            // xnu `thread_selfcounts` (libsystem_kernel), type 1: the
            // thread's instructions and cycles.
            fn thread_selfcounts(kind: i32, buf: *mut u64, nbytes: usize) -> i32;
        }
        let mut c = [0u64; 2];
        // SAFETY: the kernel writes at most `nbytes` into `c`.
        let r = unsafe { thread_selfcounts(1, c.as_mut_ptr(), std::mem::size_of_val(&c)) };
        (r == 0).then_some((c[0], c[1]))
    }
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    {
        linux_pmu::thread_counts()
    }
    #[cfg(not(any(
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    {
        None
    }
}

/// Profiling one interval (measurement only): with `FLASHTEX_PERF_MARKS` set
/// to a file, the host appends `b NS` when an edit's engine resumes after the
/// restore and `e NS` at the edited page's shipout (NS: CLOCK_MONOTONIC), so
/// the samples of `perf record -k CLOCK_MONOTONIC` between them are the
/// typesetting to the edited page alone.
pub fn perf_mark(on: bool) {
    use std::io::Write;
    static MARKS: std::sync::OnceLock<Option<std::sync::Mutex<std::fs::File>>> =
        std::sync::OnceLock::new();
    let marks = MARKS.get_or_init(|| {
        let p = std::env::var_os("FLASHTEX_PERF_MARKS")?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .ok()
            .map(std::sync::Mutex::new)
    });
    if let Some(f) = marks {
        #[cfg(unix)]
        let ns = imp::monotonic_ns();
        #[cfg(not(unix))]
        let ns = 0u64;
        if let Ok(mut f) = f.lock() {
            let _ = writeln!(f, "{} {ns}", if on { 'b' } else { 'e' });
        }
    }
}

/// Linux: the thread's user-space instructions and cycles from
/// `perf_event_open` (one counter group per thread, opened on the first call;
/// `perf_event_paranoid` ≤ 2 allows a process to count itself). Kernel work
/// is excluded, unlike macOS's counts, so the two are compared only with
/// themselves.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux_pmu {
    extern "C" {
        fn syscall(n: i64, ...) -> i64;
        fn read(fd: i32, buf: *mut std::ffi::c_void, n: usize) -> isize;
        fn close(fd: i32) -> i32;
    }
    #[cfg(target_arch = "x86_64")]
    const SYS_PERF_EVENT_OPEN: i64 = 298;
    #[cfg(target_arch = "aarch64")]
    const SYS_PERF_EVENT_OPEN: i64 = 241;
    const PERF_TYPE_HARDWARE: u32 = 0;
    const PERF_COUNT_HW_CPU_CYCLES: u64 = 0;
    const PERF_COUNT_HW_INSTRUCTIONS: u64 = 1;
    const PERF_FORMAT_TOTAL_TIME_ENABLED: u64 = 1;
    const PERF_FORMAT_TOTAL_TIME_RUNNING: u64 = 2;
    const PERF_FORMAT_GROUP: u64 = 8;
    // perf_event_open's flags: the fds are not inherited by programs the
    // host runs (the external tools, kpathsea's mktex scripts)
    const PERF_FLAG_FD_CLOEXEC: u64 = 8;
    // attr.flags bits: exclude_kernel (5), exclude_hv (6)
    const EXCLUDE_KERNEL_HV: u64 = (1 << 5) | (1 << 6);

    /// `struct perf_event_attr` up to PERF_ATTR_SIZE_VER5 (112 bytes); the
    /// fields after `read_format` are zero.
    #[repr(C)]
    struct Attr {
        kind: u32,
        size: u32,
        config: u64,
        sample_period: u64,
        sample_type: u64,
        read_format: u64,
        flags: u64,
        rest: [u64; 8],
    }

    struct Group(i32, i32);
    impl Drop for Group {
        fn drop(&mut self) {
            // SAFETY: fds this thread opened and owns.
            unsafe {
                if self.1 >= 0 {
                    close(self.1);
                }
                close(self.0);
            }
        }
    }

    fn open(config: u64, group: i32) -> i32 {
        let a = Attr {
            kind: PERF_TYPE_HARDWARE,
            size: std::mem::size_of::<Attr>() as u32,
            config,
            sample_period: 0,
            sample_type: 0,
            read_format: PERF_FORMAT_GROUP
                | PERF_FORMAT_TOTAL_TIME_ENABLED
                | PERF_FORMAT_TOTAL_TIME_RUNNING,
            flags: EXCLUDE_KERNEL_HV,
            rest: [0; 8],
        };
        // SAFETY: perf_event_open(attr, pid 0 = this thread, cpu -1 = any,
        // group_fd, flags) reads `a` only.
        unsafe {
            syscall(
                SYS_PERF_EVENT_OPEN,
                &a as *const Attr,
                0i32,
                -1i32,
                group,
                PERF_FLAG_FD_CLOEXEC,
            ) as i32
        }
    }

    thread_local! {
        static GROUP: Option<Group> = {
            let lead = open(PERF_COUNT_HW_INSTRUCTIONS, -1);
            (lead >= 0).then(|| Group(lead, open(PERF_COUNT_HW_CPU_CYCLES, lead)))
        };
    }

    /// The group's reading: (instructions, cycles, time enabled, time
    /// running).
    fn reading() -> Option<(u64, u64, u64, u64)> {
        GROUP
            .try_with(|g| {
                let g = g.as_ref()?;
                // nr, time enabled, time running, then one value per member
                let mut v = [0u64; 5];
                // SAFETY: at most `size_of_val(&v)` bytes into `v`.
                let n = unsafe { read(g.0, v.as_mut_ptr().cast(), std::mem::size_of_val(&v)) };
                if n < 32 {
                    return None;
                }
                Some((v[3], if v[0] >= 2 { v[4] } else { 0 }, v[1], v[2]))
            })
            .ok()
            .flatten()
    }

    /// The group's counts. Raw, so that they never decrease: while other
    /// counters share the PMU (`perf stat` on the host, say), the group
    /// counts only part of the time and these undercount
    /// ([`counted_throughout`]).
    pub fn thread_counts() -> Option<(u64, u64)> {
        reading().map(|r| (r.0, r.1))
    }

    /// Whether the group has counted all the time it was enabled (no other
    /// counters took the PMU from it).
    #[cfg(test)]
    pub fn counted_throughout() -> bool {
        reading().is_some_and(|r| r.3 >= r.2)
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

/// The buffer C's stdio gives a `FILE` that pdfTeX opens for output on a
/// file with metadata `m` (`None`: `fstat` failed), so that a `\write`
/// stream reaches the disk where pdfTeX's does (issue #1557: a file read
/// back while still open for output holds only what stdio has flushed).
/// web2c's `open_output` is a plain `fopen` with no `setvbuf`, and pdfTeX
/// writes a text file one `putc` at a time; every C library below writes
/// out its full buffer when the byte after it comes, as `BufWriter` does
/// with one-byte writes.
/// - macOS (FreeBSD's `__swhatbuf`): `st_blksize`, or `BUFSIZ` (1024) when
///   it is 0 (4096 on APFS, measured on macOS 26).
/// - Linux (glibc's `_IO_file_doallocate`): `st_blksize` when it is below
///   `BUFSIZ` (8192), else `BUFSIZ`. (musl's buffer is `BUFSIZ`, 1024,
///   and its overflow writes the buffer and the new byte together; TeX
///   Live's Linux binaries are glibc's, so that is not mirrored.)
/// - Windows (the UCRT's `_getbuf`): `_INTERNAL_BUFSIZ`, 4096.
/// - WASI (wasi-libc, musl's stdio): `BUFSIZ`, 1024.
pub fn stdio_buffer_size(m: Option<&std::fs::Metadata>) -> usize {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        match m.map_or(0, |m| m.blksize()) {
            0 => 1024,
            b => b as usize,
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        match m.map_or(0, |m| m.blksize()) {
            b @ 1..=8191 => b as usize,
            _ => 8192,
        }
    }
    #[cfg(windows)]
    {
        let _ = m;
        4096
    }
    #[cfg(target_os = "wasi")]
    {
        let _ = m;
        1024
    }
}

/// The metadata of a pipe to a child's stdin (`\openout|`), for
/// `stdio_buffer_size` (pdfTeX's `popen` stream `fstat`s the pipe).
pub fn pipe_metadata(p: &std::process::ChildStdin) -> Option<std::fs::Metadata> {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        let fd = p.as_fd().try_clone_to_owned().ok()?;
        std::fs::File::from(fd).metadata().ok()
    }
    #[cfg(not(unix))]
    {
        let _ = p;
        None
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

/// The arguments of `cmd` after the program name, if `cmd` runs the tool
/// `prog` (makeindex, bibtex) and nothing else: the words `/bin/sh -c cmd`
/// would pass it.
///
/// `restricted` (allow 2): `cmd` is texmfmp.c's re-quoted command, the
/// program name followed by words that are runs of `QUOTE`-quoted segments
/// separated by blanks, never containing the quote (`shell_cmd_is_allowed`
/// refuses that). Otherwise (`-shell-escape`) only a plain command is
/// taken: the name `prog`, then words of characters the shell treats
/// literally, or single-quoted; anything else (`$`, `;`, `|`, `>`, a glob,
/// ...) is left to the shell.
pub fn tool_command_args(cmd: &[u8], restricted: bool, prog: &[u8]) -> Option<Vec<Vec<u8>>> {
    let is_blank = |c: u8| c == b' ' || c == b'\t';
    let quote = SHELL_QUOTE;
    let mut words: Vec<Vec<u8>> = vec![];
    let mut cur: Option<Vec<u8>> = None;
    let mut i = 0;
    while i < cmd.len() {
        let c = cmd[i];
        if is_blank(c) {
            if let Some(w) = cur.take() {
                words.push(w);
            }
            i += 1;
        } else if c == quote || (!restricted && c == b'\'') {
            let end = cmd[i + 1..].iter().position(|&b| b == c)? + i + 1;
            cur.get_or_insert_with(Vec::new)
                .extend_from_slice(&cmd[i + 1..end]);
            i = end + 1;
        } else {
            let literal = c.is_ascii_alphanumeric() || b"._-/+=,:@%".contains(&c);
            if !restricted && !literal {
                return None;
            }
            cur.get_or_insert_with(Vec::new).push(c);
            i += 1;
        }
    }
    if let Some(w) = cur.take() {
        words.push(w);
    }
    if words.first().map(|w| &w[..]) != Some(prog) {
        return None;
    }
    Some(words.split_off(1))
}

/// Whether texmfmp.c's `runpopen` turns each `'` of a pipe's command into
/// `"` before checking it (its `#ifdef WIN32` block; `system.rs`
/// `popen_command`).
pub const POPEN_QUOTES_TO_DOUBLE: bool = cfg!(windows);

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
    if let Some(v) = take_invoked_as() {
        return v;
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

/// The name [`engine_command`] gave this process through the environment
/// (Windows), removed from the environment so that the engine's own
/// children (`\write18`, pipes, the editor) do not inherit it. Every
/// program [`engine_command`] may start calls this first thing in `main`:
/// `flashtex-host` through [`invoked_as`], and `flashtex-initex` (a host's
/// `--engine`) directly. None on Unix, where `argv[0]` says it.
pub fn take_invoked_as() -> Option<String> {
    #[cfg(not(unix))]
    if let Some(v) = std::env::var_os(INVOKED_AS_ENV) {
        // Called at the start of `main`, before any other thread exists.
        remove_env(INVOKED_AS_ENV);
        return Some(v.to_string_lossy().into_owned());
    }
    None
}

/// Remove an environment variable from both of Windows' copies (see
/// [`set_env`]): the process's, which `std::process::Command` children
/// inherit, and the C runtime's, which the C parts' `getenv` and `spawn`
/// read (`_putenv_s` with an empty value removes it).
pub fn remove_env(var: &str) {
    std::env::remove_var(var);
    #[cfg(windows)]
    {
        extern "C" {
            fn _putenv_s(name: *const std::ffi::c_char, value: *const std::ffi::c_char) -> i32;
        }
        if let Ok(k) = std::ffi::CString::new(var) {
            // SAFETY: two NUL-terminated strings; the CRT copies them.
            unsafe { _putenv_s(k.as_ptr(), c"".as_ptr()) };
        }
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

/// Make a socket file (the engine host's, an export channel's) reachable by
/// its owner only: mode 0600 on Unix. On Windows a protected DACL with one
/// entry, full access for the user this process runs as, replaces the ACL
/// inherited from the directory (`%TEMP%`'s admits Administrators and
/// SYSTEM too); an `AF_UNIX` `connect` needs write access to the file, so
/// no other account can connect. The file is opened as the reparse point
/// it is, not followed. On WASI there are no other users to keep out.
pub fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
    }
    #[cfg(windows)]
    {
        imp::restrict_to_owner(path)
    }
    #[cfg(target_os = "wasi")]
    {
        let _ = path;
        Ok(())
    }
}

/// A listening socket at `path` that only its owner can connect to, or an
/// error and no socket: the engine host's and an export channel's. The
/// caller removes a stale file at `path` first.
///
/// * Windows: no other account can connect at any moment, not even between
///   `bind` and setting the DACL (Winsock's `bind` takes no security
///   descriptor). The socket is bound inside a fresh directory beside
///   `path` whose owner-only DACL is set as it is created
///   (`imp::create_private_dir`), so the socket file inherits that DACL
///   from its first instant; it is then given its own protected DACL
///   ([`restrict_to_owner`]), renamed to `path`, and the directory removed.
/// * Unix: `bind`, then mode 0600. Between the two the socket has the mode
///   the umask leaves, which under the usual 022 already denies other
///   users the write permission `connect` needs; the Mac app puts it in
///   the per-user, mode-0700 `NSTemporaryDirectory()` besides.
///
/// Either way a failure to restrict removes the socket: the caller never
/// listens on one it could not restrict.
#[cfg(not(feature = "tex82"))]
pub fn bind_owner_only(path: &Path) -> std::io::Result<flashtex_display_list::transport::Listener> {
    use flashtex_display_list::transport::Listener;
    #[cfg(windows)]
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let dir = parent.join(format!(
            ".flashtex-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        imp::create_private_dir(&dir)?;
        let tmp = dir.join("s");
        let r = Listener::bind(&tmp).and_then(|l| {
            restrict_to_owner(&tmp)?;
            std::fs::rename(&tmp, path)?;
            Ok(l)
        });
        if r.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        let _ = std::fs::remove_dir(&dir);
        r
    }
    #[cfg(not(windows))]
    {
        let l = Listener::bind(path)?;
        if let Err(e) = restrict_to_owner(path) {
            drop(l);
            let _ = std::fs::remove_file(path);
            return Err(e);
        }
        Ok(l)
    }
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
///   once. The socket file admits its owner only ([`bind_owner_only`]),
///   and the connection accepted is the child's: one from any other
///   process (`Stream::peer_pid`, `SIO_AF_UNIX_GETPEERPID`) is closed and
///   the wait goes on. Where Windows cannot name the peer (before Windows
///   10 1803), the owner-only DACL is the only check. Should the child end
///   without connecting, the read ends as soon as the caller says the
///   child has exited.
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
        #[cfg(unix)]
        {
            use flashtex_display_list::transport::*;
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
            let listener = bind_owner_only(&path)?;
            let ch = ExportChannel { listener, path };
            ch.listener.set_nonblocking(true)?;
            Ok(ch)
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

    /// After the child (process `pid`) was spawned: our end, to read
    /// frames from until the child closes its end. `exited` turns true when
    /// the child has ended.
    pub fn reader(
        self,
        pid: u32,
        exited: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Box<dyn std::io::Read + Send> {
        #[cfg(unix)]
        {
            let _ = (pid, exited);
            drop(self.theirs);
            Box::new(self.ours)
        }
        #[cfg(not(unix))]
        {
            Box::new(Accepting {
                ch: self,
                pid,
                stream: None,
                exited,
            })
        }
    }
}

#[cfg(all(not(feature = "tex82"), not(unix)))]
struct Accepting {
    ch: ExportChannel,
    pid: u32,
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
                        if !peer_is(&s, self.pid) {
                            continue;
                        }
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

/// Whether the export channel's connection `s` comes from process `pid`
/// (the engine child). Only a Windows too old to name a peer at all
/// (`ErrorKind::Unsupported`, before Windows 10 1803) is taken on trust,
/// the owner-only DACL having kept other accounts out; any other failure
/// to name the peer closes the connection.
#[cfg(all(not(feature = "tex82"), windows))]
fn peer_is(s: &flashtex_display_list::transport::Stream, pid: u32) -> bool {
    peer_verdict(s.peer_pid(), pid)
}

#[cfg(all(not(feature = "tex82"), windows))]
fn peer_verdict(peer: std::io::Result<u32>, pid: u32) -> bool {
    match peer {
        Ok(p) if p == pid => true,
        Ok(p) => {
            eprintln!("flashtex-host: export channel: closed a connection from process {p}, not the engine ({pid})");
            false
        }
        Err(e) if e.kind() == std::io::ErrorKind::Unsupported => true,
        Err(e) => {
            eprintln!(
                "flashtex-host: export channel: closed a connection whose process is unknown: {e}"
            );
            false
        }
    }
}

#[cfg(all(not(feature = "tex82"), target_os = "wasi"))]
fn peer_is(_: &flashtex_display_list::transport::Stream, _: u32) -> bool {
    true
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

    pub fn monotonic_ns() -> u64 {
        #[cfg(target_os = "macos")]
        const CLOCK_MONOTONIC: i32 = 6;
        #[cfg(not(target_os = "macos"))]
        const CLOCK_MONOTONIC: i32 = 1;
        let mut t = Timespec { sec: 0, nsec: 0 };
        // SAFETY: an out-parameter of the right layout.
        unsafe { clock_gettime(CLOCK_MONOTONIC, &mut t) };
        t.sec as u64 * 1_000_000_000 + t.nsec as u64
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
                f.as_raw_handle(),
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

    // ----- Owner-only DACL (`super::restrict_to_owner`) -----

    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_USER_CLASS: u32 = 1; // TOKEN_INFORMATION_CLASS::TokenUser
    const ACL_REVISION: u32 = 2;
    const FILE_ALL_ACCESS: u32 = 0x001F_01FF;
    const SE_FILE_OBJECT: u32 = 1;
    const DACL_SECURITY_INFORMATION: u32 = 0x0000_0004;
    const PROTECTED_DACL_SECURITY_INFORMATION: u32 = 0x8000_0000;
    const READ_CONTROL: u32 = 0x0002_0000;
    const WRITE_DAC: u32 = 0x0004_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const OBJECT_INHERIT_ACE: u32 = 0x1;
    const CONTAINER_INHERIT_ACE: u32 = 0x2;
    const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
    const SE_DACL_PROTECTED: u16 = 0x1000;

    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
        fn GetTokenInformation(
            token: *mut c_void,
            class: u32,
            info: *mut c_void,
            len: u32,
            ret: *mut u32,
        ) -> i32;
        fn GetLengthSid(sid: *mut c_void) -> u32;
        fn InitializeAcl(acl: *mut c_void, len: u32, revision: u32) -> i32;
        fn AddAccessAllowedAceEx(
            acl: *mut c_void,
            revision: u32,
            flags: u32,
            mask: u32,
            sid: *mut c_void,
        ) -> i32;
        fn InitializeSecurityDescriptor(sd: *mut c_void, revision: u32) -> i32;
        fn SetSecurityDescriptorDacl(
            sd: *mut c_void,
            present: i32,
            dacl: *mut c_void,
            defaulted: i32,
        ) -> i32;
        fn SetSecurityDescriptorControl(sd: *mut c_void, mask: u16, bits: u16) -> i32;
        fn SetSecurityInfo(
            handle: *mut c_void,
            object_type: u32,
            info: u32,
            owner: *mut c_void,
            group: *mut c_void,
            dacl: *mut c_void,
            sacl: *mut c_void,
        ) -> u32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn CreateDirectoryW(path: *const u16, attrs: *const c_void) -> i32;
    }

    /// The process token's user (`TOKEN_USER`, whose first field is the
    /// SID's pointer into the same buffer), as u64 words for alignment.
    pub(super) fn token_user() -> std::io::Result<Vec<u64>> {
        let err = std::io::Error::last_os_error;
        // SAFETY: out-parameters of the documented sizes; the token handle
        // is closed before returning.
        unsafe {
            let mut token = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return Err(err());
            }
            let mut len = 0u32;
            GetTokenInformation(token, TOKEN_USER_CLASS, std::ptr::null_mut(), 0, &mut len);
            let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
            let ok = GetTokenInformation(
                token,
                TOKEN_USER_CLASS,
                buf.as_mut_ptr() as *mut c_void,
                (buf.len() * 8) as u32,
                &mut len,
            );
            let e = err();
            CloseHandle(token);
            if ok == 0 {
                return Err(e);
            }
            Ok(buf)
        }
    }

    /// An ACL with one entry: full access for `user`'s SID (from
    /// [`token_user`]), with the ACE flags `inherit` (0, or object and
    /// container inheritance for a directory's children). u32 words, as
    /// InitializeAcl's documentation sizes it: the ACL header, then one
    /// ACCESS_ALLOWED_ACE whose SidStart DWORD the SID overlays.
    fn owner_acl(user: &[u64], inherit: u32) -> std::io::Result<Vec<u32>> {
        let sid = user[0] as usize as *mut c_void;
        // SAFETY: `sid` points into `user`, which outlives the calls; the
        // buffer has the length passed.
        unsafe {
            let ace = 8 + GetLengthSid(sid) as usize;
            let mut acl = vec![0u32; (8 + ace).div_ceil(4)];
            let p = acl.as_mut_ptr() as *mut c_void;
            if InitializeAcl(p, (acl.len() * 4) as u32, ACL_REVISION) == 0
                || AddAccessAllowedAceEx(p, ACL_REVISION, inherit, FILE_ALL_ACCESS, sid) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(acl)
        }
    }

    pub fn restrict_to_owner(path: &std::path::Path) -> std::io::Result<()> {
        use std::os::windows::fs::OpenOptionsExt;
        use std::os::windows::io::AsRawHandle;
        // The file itself, never what a reparse point (an AF_UNIX socket
        // is one) would lead to; a directory would open too.
        let f = std::fs::OpenOptions::new()
            .access_mode(READ_CONTROL | WRITE_DAC)
            .share_mode(7)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)?;
        let mut acl = owner_acl(&token_user()?, 0)?;
        // SAFETY: a handle opened with WRITE_DAC and a valid ACL.
        let r = unsafe {
            SetSecurityInfo(
                f.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl.as_mut_ptr() as *mut c_void,
                std::ptr::null_mut(),
            )
        };
        if r != 0 {
            return Err(std::io::Error::from_raw_os_error(r as i32));
        }
        Ok(())
    }

    /// A new directory whose DACL, set as it is created (no window), is
    /// protected and gives this process's user alone full access, inherited
    /// by everything made in it.
    pub fn create_private_dir(path: &std::path::Path) -> std::io::Result<()> {
        use std::os::windows::ffi::OsStrExt;
        #[repr(C)]
        struct SecurityAttributes {
            len: u32,
            sd: *mut c_void,
            inherit: i32,
        }
        let mut acl = owner_acl(&token_user()?, OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE)?;
        // An absolute SECURITY_DESCRIPTOR: 40 bytes on 64-bit Windows
        // (SECURITY_DESCRIPTOR_MIN_LENGTH), 20 on 32-bit.
        let mut sd = [0u64; 8];
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: `sd` and `acl` outlive CreateDirectoryW, which copies
        // the descriptor; `wide` is NUL-terminated.
        unsafe {
            let psd = sd.as_mut_ptr() as *mut c_void;
            if InitializeSecurityDescriptor(psd, SECURITY_DESCRIPTOR_REVISION) == 0
                || SetSecurityDescriptorDacl(psd, 1, acl.as_mut_ptr() as *mut c_void, 0) == 0
                || SetSecurityDescriptorControl(psd, SE_DACL_PROTECTED, SE_DACL_PROTECTED) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            let sa = SecurityAttributes {
                len: std::mem::size_of::<SecurityAttributes>() as u32,
                sd: psd,
                inherit: 0,
            };
            if CreateDirectoryW(
                wide.as_ptr(),
                &sa as *const SecurityAttributes as *const c_void,
            ) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
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

// ---------------------------------------------------------------------------
// Tests (the Windows ones run in .github/workflows/portability.yml)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// Where the system gives them, the thread's counts grow with its work
    /// and another thread's work does not move them.
    #[test]
    fn thread_counts_count_this_thread() {
        let Some(a) = super::thread_counts() else {
            return;
        };
        std::thread::spawn(|| {
            let mut x = 0u64;
            for i in 0..2_000_000u64 {
                x = std::hint::black_box(x.wrapping_mul(31).wrapping_add(i));
            }
        })
        .join()
        .unwrap();
        let b = super::thread_counts().unwrap();
        let mut x = 0u64;
        for i in 0..2_000_000u64 {
            x = std::hint::black_box(x.wrapping_mul(31).wrapping_add(i));
        }
        let c = super::thread_counts().unwrap();
        // (another counter user, `perf` or the CI host's, may have taken the
        // PMU part of the time: then the counts fall short, and prove less)
        #[cfg(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ))]
        if !super::linux_pmu::counted_throughout() {
            return;
        }
        assert!(c.0 - b.0 >= 2_000_000, "{b:?} -> {c:?}");
        assert!(
            b.0 - a.0 < 2_000_000,
            "{a:?} -> {b:?}: another thread's work"
        );
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("flashtex-os-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    /// The access-control entries of `path` (the file itself, not what a
    /// reparse point leads to): (type, mask, is this process's user).
    #[cfg(windows)]
    fn dacl(path: &std::path::Path) -> Vec<(u8, u32, bool)> {
        use std::ffi::c_void;
        use std::os::windows::fs::OpenOptionsExt;
        use std::os::windows::io::AsRawHandle;
        #[link(name = "advapi32")]
        extern "system" {
            fn GetSecurityInfo(
                handle: *mut c_void,
                object_type: u32,
                info: u32,
                owner: *mut *mut c_void,
                group: *mut *mut c_void,
                dacl: *mut *mut c_void,
                sacl: *mut *mut c_void,
                sd: *mut *mut c_void,
            ) -> u32;
            fn GetAce(acl: *mut c_void, i: u32, ace: *mut *mut c_void) -> i32;
            fn EqualSid(a: *mut c_void, b: *mut c_void) -> i32;
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn LocalFree(p: *mut c_void) -> *mut c_void;
        }
        let f = std::fs::OpenOptions::new()
            .access_mode(0x0002_0000) // READ_CONTROL
            .share_mode(7)
            .custom_flags(0x0020_0000 | 0x0200_0000)
            .open(path)
            .unwrap();
        let user = super::imp::token_user().unwrap();
        let me = user[0] as usize as *mut c_void;
        let mut out = vec![];
        // SAFETY: out-parameters of GetSecurityInfo; the ACL lives in the
        // security descriptor, freed last.
        unsafe {
            let (mut acl, mut sd) = (std::ptr::null_mut(), std::ptr::null_mut());
            let null = std::ptr::null_mut();
            let h = f.as_raw_handle();
            let r = GetSecurityInfo(h, 1, 4, null, null, &mut acl, null, &mut sd);
            assert_eq!(r, 0, "GetSecurityInfo");
            assert!(!acl.is_null(), "no DACL (everyone has access)");
            let count = *((acl as *const u8).add(4) as *const u16);
            for i in 0..count as u32 {
                let mut ace = std::ptr::null_mut();
                assert!(GetAce(acl, i, &mut ace) != 0);
                let b = ace as *const u8;
                let mask = *(b.add(4) as *const u32);
                out.push((*b, mask, EqualSid(b.add(8) as *mut c_void, me) != 0));
            }
            LocalFree(sd);
        }
        out
    }

    /// `take_invoked_as` empties both of Windows' environments, so neither
    /// a `Command` child (`\write18`) nor the C parts see the variable.
    #[cfg(windows)]
    #[test]
    fn invoked_as_is_taken_out_of_both_environments() {
        extern "C" {
            fn getenv(name: *const std::ffi::c_char) -> *const std::ffi::c_char;
        }
        let crt = || unsafe { !getenv(c"FLASHTEX_INVOKED_AS".as_ptr()).is_null() };
        super::set_env(super::INVOKED_AS_ENV, "pdftex");
        assert!(crt());
        assert_eq!(super::take_invoked_as().as_deref(), Some("pdftex"));
        assert_eq!(super::take_invoked_as(), None);
        assert!(!crt());
        let out = super::shell_command(
            b"if defined FLASHTEX_INVOKED_AS (echo inherited) else (echo clean)",
        )
        .output()
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim_end(), "clean");
    }

    /// A socket that cannot be made owner-only is not left listening (here
    /// it cannot even be made: its directory does not exist).
    #[cfg(not(feature = "tex82"))]
    #[test]
    fn bind_owner_only_fails_closed() {
        let p = scratch("no-such-dir").join("s.sock");
        assert!(super::bind_owner_only(&p).is_err());
        assert!(!p.exists());
    }

    #[cfg(all(unix, not(feature = "tex82")))]
    #[test]
    fn bind_owner_only_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let p = scratch("bind.sock");
        let l = super::bind_owner_only(&p).unwrap();
        let mode = std::fs::metadata(&p).unwrap().permissions().mode();
        drop(l);
        let _ = std::fs::remove_file(&p);
        assert_eq!(mode & 0o777, 0o600);
    }

    /// Owner-only at its final path, connectable there after the rename
    /// out of the private directory, which is gone.
    #[cfg(all(windows, not(feature = "tex82")))]
    #[test]
    fn bind_owner_only_on_windows() {
        use flashtex_display_list::transport::Stream;
        use std::io::{Read, Write};
        let p = scratch("bind.sock");
        let l = super::bind_owner_only(&p).unwrap();
        assert_eq!(dacl(&p), vec![(0, 0x001F_01FF, true)]);
        let mut c = Stream::connect(&p).unwrap();
        let (mut s, ()) = l.accept().unwrap();
        assert_eq!(s.peer_pid().unwrap(), std::process::id());
        c.write_all(b"ok").unwrap();
        let mut b = [0u8; 2];
        s.read_exact(&mut b).unwrap();
        assert_eq!(&b, b"ok");
        drop((c, s, l));
        let _ = std::fs::remove_file(&p);
        let me = format!(".flashtex-{}-", std::process::id());
        let left: Vec<_> = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(&me))
            .collect();
        // Another test of this process may be binding at this moment;
        // none of its directories outlives its bind.
        assert!(left.len() <= 1, "{left:?}");
    }

    /// The private directory is owner-only from its creation, and what is
    /// made in it inherits that.
    #[cfg(windows)]
    #[test]
    fn private_dir_is_owner_only_and_inherited() {
        let d = scratch("private");
        let _ = std::fs::remove_dir_all(&d);
        super::imp::create_private_dir(&d).unwrap();
        let f = d.join("f");
        std::fs::write(&f, b"x").unwrap();
        let (dd, fd) = (dacl(&d), dacl(&f));
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(dd, vec![(0, 0x001F_01FF, true)]);
        assert_eq!(fd, vec![(0, 0x001F_01FF, true)]);
    }

    /// The export channel's peer check refuses every failure to name the
    /// peer except Windows being too old to (`Unsupported`).
    #[cfg(all(windows, not(feature = "tex82")))]
    #[test]
    fn export_peer_verdicts() {
        use std::io::{Error, ErrorKind};
        assert!(super::peer_verdict(Ok(7), 7));
        assert!(!super::peer_verdict(Ok(8), 7));
        assert!(super::peer_verdict(
            Err(Error::new(ErrorKind::Unsupported, "old")),
            7
        ));
        assert!(!super::peer_verdict(
            Err(Error::from_raw_os_error(10054)),
            7
        ));
        assert!(!super::peer_verdict(Err(Error::other("x")), 7));
    }

    #[cfg(unix)]
    #[test]
    fn restrict_to_owner_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let p = scratch("mode");
        std::fs::write(&p, b"x").unwrap();
        super::restrict_to_owner(&p).unwrap();
        let mode = std::fs::metadata(&p).unwrap().permissions().mode();
        let _ = std::fs::remove_file(&p);
        assert_eq!(mode & 0o777, 0o600);
    }

    /// One entry: full access for this process's user, nothing inherited.
    #[cfg(windows)]
    #[test]
    fn restrict_to_owner_leaves_one_entry_for_the_user() {
        let p = scratch("dacl");
        std::fs::write(&p, b"x").unwrap();
        let before = dacl(&p);
        super::restrict_to_owner(&p).unwrap();
        let after = dacl(&p);
        let _ = std::fs::remove_file(&p);
        assert!(before.len() > 1, "{before:?}");
        assert_eq!(after, vec![(0, 0x001F_01FF, true)]);
    }

    /// The same on an AF_UNIX socket file (a reparse point), which its
    /// owner can still connect to.
    #[cfg(all(windows, not(feature = "tex82")))]
    #[test]
    fn restrict_to_owner_on_a_socket_keeps_the_owner_connecting() {
        use flashtex_display_list::transport::{Listener, Stream};
        let p = scratch("sock");
        let l = Listener::bind(&p).unwrap();
        super::restrict_to_owner(&p).unwrap();
        assert_eq!(dacl(&p), vec![(0, 0x001F_01FF, true)]);
        let c = Stream::connect(&p).unwrap();
        let (s, ()) = l.accept().unwrap();
        // Both ends are this process.
        assert_eq!(s.peer_pid().unwrap(), std::process::id());
        assert_eq!(c.peer_pid().unwrap(), std::process::id());
        drop((c, s, l));
        let _ = std::fs::remove_file(&p);
    }

    /// `FLASHTEX_DISPLAY_LIST=socket:PATH` as `attach` gives it.
    #[cfg(all(windows, not(feature = "tex82")))]
    fn channel_path(cmd: &std::process::Command) -> std::path::PathBuf {
        cmd.get_envs()
            .find(|(k, _)| *k == "FLASHTEX_DISPLAY_LIST")
            .and_then(|(_, v)| v)
            .and_then(|v| v.to_str())
            .and_then(|v| v.strip_prefix("socket:"))
            .map(std::path::PathBuf::from)
            .unwrap()
    }

    /// The export channel reads the engine child's connection and closes
    /// any other process's: first a `cmd.exe` child plays the engine (and
    /// never connects) while this process intrudes, then this process is
    /// the one named.
    #[cfg(all(windows, not(feature = "tex82")))]
    #[test]
    fn export_channel_reads_only_the_child() {
        use flashtex_display_list::transport::Stream;
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let ch = super::ExportChannel::open().unwrap();
        let mut cmd = std::process::Command::new("cmd.exe");
        ch.attach(&mut cmd);
        let path = channel_path(&cmd);
        assert_eq!(dacl(&path), vec![(0, 0x001F_01FF, true)]);
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/c", "exit 0"])
            .spawn()
            .unwrap();
        let mut intruder = Stream::connect(&path).unwrap();
        intruder.write_all(b"forged").unwrap();
        let exited = Arc::new(AtomicBool::new(false));
        let mut r = ch.reader(child.id(), exited.clone());
        child.wait().unwrap();
        exited.store(true, Ordering::Release);
        let mut got = vec![];
        r.read_to_end(&mut got).unwrap();
        assert!(got.is_empty(), "read another process's bytes: {got:?}");
        drop(intruder);

        let ch = super::ExportChannel::open().unwrap();
        let mut cmd = std::process::Command::new("cmd.exe");
        ch.attach(&mut cmd);
        let path = channel_path(&cmd);
        let mut ours = Stream::connect(&path).unwrap();
        ours.write_all(b"frames").unwrap();
        drop(ours);
        let mut r = ch.reader(std::process::id(), Arc::new(AtomicBool::new(false)));
        let mut got = vec![];
        r.read_to_end(&mut got).unwrap();
        assert_eq!(got, b"frames");
    }
}
