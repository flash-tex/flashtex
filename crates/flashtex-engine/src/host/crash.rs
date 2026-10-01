//! Why a host ended (review 2026-09-30, track 1, finding 10b: three
//! `flashtex-host iserve` sessions ended with nothing on stderr). Every way
//! out leaves one line on stderr, and on `FLASHTEX_HOST_CRASH_LOG` (a file
//! appended to) when that is set:
//!
//! * a panic: its message and location, the request being served, and a
//!   backtrace (the Rust default hook runs too);
//! * `SIGABRT` (an `abort`, which is also how Rust ends a thread that
//!   overflowed its stack, after saying so), `SIGTERM`, `SIGINT`, `SIGHUP`:
//!   the signal and the request being served, written from the handler with
//!   `write(2)` only; then the signal's default action;
//! * a normal end ([`exit`]): the reason (end of input, `quit`, the socket
//!   closed) and how many requests were served.
//!
//! A `SIGSEGV` or `SIGBUS` other than a stack overflow still ends the
//! process silently: Rust's own handler for those restores the default
//! action. `SIGKILL` (the kernel's out-of-memory killer, a supervisor) cannot
//! be caught at all.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Once;

const CAP: usize = 480;
static mut LAST: [u8; CAP] = [0; CAP];
static LAST_LEN: AtomicUsize = AtomicUsize::new(0);
static REQUESTS: AtomicU64 = AtomicU64::new(0);
static mut LOG_PATH: [u8; 512] = [0; 512];
static LOG_LEN: AtomicUsize = AtomicUsize::new(0);
static INSTALL: Once = Once::new();

extern "C" {
    fn signal(sig: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
    fn write(fd: i32, buf: *const u8, n: usize) -> isize;
    fn open(path: *const u8, flags: i32, mode: u32) -> i32;
    fn close(fd: i32) -> i32;
}

const SIGHUP: i32 = 1;
const SIGINT: i32 = 2;
const SIGABRT: i32 = 6;
const SIGTERM: i32 = 15;
const SIG_DFL: usize = 0;
#[cfg(target_os = "macos")]
const O_WRONLY_CREAT_APPEND: i32 = 0x0001 | 0x0200 | 0x0008;
#[cfg(not(target_os = "macos"))]
const O_WRONLY_CREAT_APPEND: i32 = 0o1 | 0o100 | 0o2000;

/// Note the request now being served (shortened), for the lines above.
pub fn serving(what: &str) {
    REQUESTS.fetch_add(1, Ordering::Relaxed);
    let b = what.as_bytes();
    let n = b.len().min(CAP);
    // SAFETY: the buffer is only read (best effort) by the lines above; a
    // torn read there shows a mixed request, never touches other memory.
    unsafe {
        let p = std::ptr::addr_of_mut!(LAST) as *mut u8;
        std::ptr::copy_nonoverlapping(b.as_ptr(), p, n);
    }
    LAST_LEN.store(n, Ordering::Release);
}

fn emit(line: &[u8]) {
    // SAFETY: write(2) of a valid buffer; open(2)/close(2) of a
    // NUL-terminated path kept in a static buffer.
    unsafe {
        write(2, line.as_ptr(), line.len());
        let n = LOG_LEN.load(Ordering::Acquire);
        if n > 0 {
            let fd = open(
                std::ptr::addr_of!(LOG_PATH) as *const u8,
                O_WRONLY_CREAT_APPEND,
                0o644,
            );
            if fd >= 0 {
                write(fd, line.as_ptr(), line.len());
                close(fd);
            }
        }
    }
}

extern "C" fn on_signal(sig: i32) {
    // Only async-signal-safe calls from here: fixed buffers and write(2).
    let mut buf = [0u8; 700];
    let mut k = 0;
    let mut put = |s: &[u8], k: &mut usize| {
        for &c in s {
            if *k < buf.len() {
                buf[*k] = c;
                *k += 1;
            }
        }
    };
    put(b"flashtex-host: ended by signal ", &mut k);
    let mut d = [0u8; 4];
    let mut v = sig as u32;
    let mut nd = 0;
    loop {
        d[nd] = b'0' + (v % 10) as u8;
        nd += 1;
        v /= 10;
        if v == 0 || nd == d.len() {
            break;
        }
    }
    while nd > 0 {
        nd -= 1;
        put(&d[nd..nd + 1], &mut k);
    }
    put(b" while serving: ", &mut k);
    let n = LAST_LEN.load(Ordering::Acquire).min(CAP);
    // SAFETY: see `serving`.
    let last = unsafe { std::slice::from_raw_parts(std::ptr::addr_of!(LAST) as *const u8, n) };
    put(last, &mut k);
    put(b"\n", &mut k);
    emit(&buf[..k]);
    // SAFETY: back to the default action, then the signal again.
    unsafe {
        signal(sig, SIG_DFL);
        raise(sig);
    }
}

/// Install the panic hook and the signal handlers (once per process).
pub fn install() {
    INSTALL.call_once(|| {
        if let Ok(p) = std::env::var("FLASHTEX_HOST_CRASH_LOG") {
            let b = p.as_bytes();
            if !b.is_empty() && b.len() < 511 {
                // SAFETY: written once, before any reader (the handlers are
                // installed below).
                unsafe {
                    let dst = std::ptr::addr_of_mut!(LOG_PATH) as *mut u8;
                    std::ptr::copy_nonoverlapping(b.as_ptr(), dst, b.len());
                    *dst.add(b.len()) = 0;
                }
                LOG_LEN.store(b.len(), Ordering::Release);
            }
        }
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let n = LAST_LEN.load(Ordering::Acquire).min(CAP);
            // SAFETY: see `serving`.
            let last =
                unsafe { std::slice::from_raw_parts(std::ptr::addr_of!(LAST) as *const u8, n) };
            let line = format!(
                "flashtex-host: panic in thread {:?}: {info}; while serving: {}\n{}\n",
                std::thread::current().name().unwrap_or("?"),
                String::from_utf8_lossy(last),
                std::backtrace::Backtrace::force_capture()
            );
            emit(line.as_bytes());
            prev(info);
        }));
        // SAFETY: installs a handler that only uses async-signal-safe calls.
        unsafe {
            for s in [SIGABRT, SIGTERM, SIGINT, SIGHUP] {
                signal(s, on_signal as *const () as usize);
            }
        }
    });
}

/// A normal end: say why, and how many requests were served.
pub fn exit(reason: &str) {
    emit(
        format!(
            "flashtex-host: exiting: {reason} ({} requests served)\n",
            REQUESTS.load(Ordering::Relaxed)
        )
        .as_bytes(),
    );
}
