//! Memory accounting of the resident engine (DESIGN.md §5.2's retention
//! budget, §7; lane P4-MEMORY).
//!
//! * [`rss`]: the process's resident memory now and at its peak, as the
//!   operating system counts it (Linux: `VmRSS`/`VmHWM`; macOS:
//!   `phys_footprint` and its lifetime maximum, what Activity Monitor calls
//!   "Memory").
//! * [`resident`]: how much of a mapping is resident (`mincore`), for the
//!   word space and the slab, which do not come from the allocator.
//! * With the feature `mem-stats`, a counting global allocator
//!   ([`Counting`]) that attributes every heap allocation to the *tag* in
//!   force on its thread when it was made ([`scope`]), and keeps the live
//!   bytes of each tag and their split at the peak. Measurement only: it
//!   adds a 16-byte header and two atomic adds to every allocation. Without
//!   the feature [`scope`] compiles to nothing.

#[cfg(feature = "mem-stats")]
use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(feature = "mem-stats")]
use std::sync::atomic::{AtomicI64, Ordering};

/// What an allocation is for, by the code that made it.
pub mod tag {
    /// Nothing more specific (the host's own bookkeeping, the socket).
    pub const OTHER: u8 = 0;
    /// The engine running (and the session's bookkeeping around it).
    pub const ENGINE: u8 = 1;
    /// A checkpoint's host record (`ExtRecord`: file positions, pdfTeX's
    /// C-part state as copied then).
    pub const RECORD: u8 = 2;
    /// Sealed undo logs, and their merges.
    pub const LOG: u8 = 3;
    /// A restore: the detached branch's records and output tails.
    pub const BRANCH: u8 = 4;
    /// The display list's side table (source spans of nodes): its chunks
    /// copied on write after a snapshot, and each snapshot's chunk list.
    pub const SIDE: u8 = 5;
    /// Copies made by the first write to a `Shared` part of pdfTeX's C
    /// state after a checkpoint shared it.
    pub const COW: u8 = 6;
    /// Display lists being built (and kept by the host's page cache).
    pub const DL: u8 = 7;
    /// The convergence test.
    pub const TEST: u8 = 8;
    /// The next restore, worked out while idle.
    pub const PREPARE: u8 = 9;
    pub const N: usize = 10;
    pub const NAMES: [&str; N] = [
        "other", "engine", "record", "log", "branch", "side", "cow", "dl", "test", "prepare",
    ];
}

thread_local! {
    static CUR: std::cell::Cell<u8> = const { std::cell::Cell::new(tag::OTHER) };
}

/// Restores the previous tag when dropped (see [`scope`]).
pub struct Scope(#[allow(dead_code)] u8);

/// Attribute this thread's allocations to `t` until the returned guard is
/// dropped. Free without the feature `mem-stats`.
#[inline(always)]
pub fn scope(t: u8) -> Scope {
    #[cfg(feature = "mem-stats")]
    {
        Scope(CUR.try_with(|c| c.replace(t)).unwrap_or(tag::OTHER))
    }
    #[cfg(not(feature = "mem-stats"))]
    {
        let _ = t;
        Scope(0)
    }
}

impl Drop for Scope {
    #[inline(always)]
    fn drop(&mut self) {
        #[cfg(feature = "mem-stats")]
        {
            let _ = CUR.try_with(|c| c.set(self.0));
        }
    }
}

/// Whether the counting allocator is in (the feature `mem-stats`).
pub const fn enabled() -> bool {
    cfg!(feature = "mem-stats")
}

#[cfg(feature = "mem-stats")]
#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "mem-stats")]
static LIVE: [AtomicI64; tag::N] = [ZERO; tag::N];
#[cfg(feature = "mem-stats")]
static TOTAL: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "mem-stats")]
static PEAK: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "mem-stats")]
static PEAK_BY: [AtomicI64; tag::N] = [ZERO; tag::N];
#[cfg(feature = "mem-stats")]
static PEAK_SNAP: AtomicI64 = AtomicI64::new(0);

/// The counting allocator (feature `mem-stats`): `System` with a header
/// holding the allocation's tag.
#[cfg(feature = "mem-stats")]
pub struct Counting;

#[cfg(feature = "mem-stats")]
const HDR: usize = 16;

#[cfg(feature = "mem-stats")]
fn account(t: u8, d: i64) {
    LIVE[t as usize % tag::N].fetch_add(d, Ordering::Relaxed);
    let total = TOTAL.fetch_add(d, Ordering::Relaxed) + d;
    if d > 0 && total > PEAK.load(Ordering::Relaxed) {
        PEAK.store(total, Ordering::Relaxed);
        // the split at the peak, refreshed every MiB of new peak
        if total > PEAK_SNAP.load(Ordering::Relaxed) + (1 << 20) {
            PEAK_SNAP.store(total, Ordering::Relaxed);
            for (p, l) in PEAK_BY.iter().zip(LIVE.iter()) {
                p.store(l.load(Ordering::Relaxed), Ordering::Relaxed);
            }
        }
    }
}

#[cfg(feature = "mem-stats")]
fn cur() -> u8 {
    CUR.try_with(|c| c.get()).unwrap_or(tag::OTHER)
}

#[cfg(feature = "mem-stats")]
fn outer(l: Layout) -> (Layout, usize) {
    let a = l.align().max(HDR);
    // SAFETY: `a` is a power of two; the size cannot overflow for any
    // allocation that could succeed.
    (
        unsafe { Layout::from_size_align_unchecked(l.size() + a, a) },
        a,
    )
}

#[cfg(feature = "mem-stats")]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let (o, a) = outer(l);
        let p = System.alloc(o);
        if p.is_null() {
            return p;
        }
        let t = cur();
        let q = p.add(a);
        *q.sub(1) = t;
        account(t, l.size() as i64);
        q
    }

    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let (o, a) = outer(l);
        let p = System.alloc_zeroed(o);
        if p.is_null() {
            return p;
        }
        let t = cur();
        let q = p.add(a);
        *q.sub(1) = t;
        account(t, l.size() as i64);
        q
    }

    unsafe fn dealloc(&self, q: *mut u8, l: Layout) {
        let (o, a) = outer(l);
        let t = *q.sub(1);
        account(t, -(l.size() as i64));
        System.dealloc(q.sub(a), o);
    }

    unsafe fn realloc(&self, q: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let (o, a) = outer(l);
        let t = *q.sub(1);
        let p = System.realloc(q.sub(a), o, new + a);
        if p.is_null() {
            return p;
        }
        account(t, new as i64 - l.size() as i64);
        p.add(a)
    }
}

/// Live heap bytes by tag (empty without `mem-stats`).
pub fn live_by_tag() -> Vec<(&'static str, i64)> {
    #[cfg(feature = "mem-stats")]
    {
        tag::NAMES
            .iter()
            .zip(LIVE.iter())
            .map(|(n, l)| (*n, l.load(Ordering::Relaxed)))
            .collect()
    }
    #[cfg(not(feature = "mem-stats"))]
    {
        vec![]
    }
}

/// The heap's live bytes now and at the peak, and the peak's split by tag
/// (as of the last MiB of growth); `None` without `mem-stats`.
pub fn heap() -> Option<(i64, i64, Vec<(&'static str, i64)>)> {
    #[cfg(feature = "mem-stats")]
    {
        Some((
            TOTAL.load(Ordering::Relaxed),
            PEAK.load(Ordering::Relaxed),
            tag::NAMES
                .iter()
                .zip(PEAK_BY.iter())
                .map(|(n, l)| (*n, l.load(Ordering::Relaxed)))
                .collect(),
        ))
    }
    #[cfg(not(feature = "mem-stats"))]
    {
        None
    }
}

/// The process's resident bytes now and at its peak (Linux `VmRSS` and
/// `VmHWM`; macOS `phys_footprint` and its lifetime maximum).
pub fn rss() -> Option<(u64, u64)> {
    #[cfg(target_os = "linux")]
    {
        let s = std::fs::read_to_string("/proc/self/status").ok()?;
        let kb = |key: &str| -> Option<u64> {
            let l = s.lines().find(|l| l.starts_with(key))?;
            l[key.len()..]
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()
                .map(|v| v * 1024)
        };
        Some((kb("VmRSS:")?, kb("VmHWM:")?))
    }
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
        }
        // rusage_info_v4 as u64s: a 16-byte uuid (0, 1), then the fields;
        // ri_phys_footprint is 9, ri_lifetime_max_phys_footprint 30.
        let mut b = [0u64; 64];
        // SAFETY: the buffer is larger than rusage_info_v4 (RUSAGE_INFO_V4 = 4).
        let r = unsafe { proc_pid_rusage(std::process::id() as i32, 4, b.as_mut_ptr()) };
        if r != 0 {
            return None;
        }
        Some((b[9], b[30]))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// How many bytes of `[p, p + len)` are resident (`mincore`); the range is
/// widened to whole pages.
pub fn resident(p: *const u8, len: usize) -> Option<usize> {
    #[cfg(unix)]
    {
        extern "C" {
            fn mincore(addr: *mut std::ffi::c_void, len: usize, vec: *mut u8) -> i32;
            fn getpagesize() -> i32;
        }
        if len == 0 {
            return Some(0);
        }
        // SAFETY: no preconditions.
        let page = unsafe { getpagesize() } as usize;
        let lo = p as usize / page * page;
        let hi = (p as usize + len).next_multiple_of(page);
        let n = (hi - lo) / page;
        let mut v = vec![0u8; n];
        // SAFETY: `v` has one byte per page of the range, which the caller
        // says is mapped.
        let r = unsafe { mincore(lo as *mut std::ffi::c_void, hi - lo, v.as_mut_ptr()) };
        if r != 0 {
            return None;
        }
        Some(v.iter().filter(|&&b| b & 1 != 0).count() * page)
    }
    #[cfg(not(unix))]
    {
        let _ = (p, len);
        None
    }
}

/// `k: v` pairs as a JSON object's members.
pub fn json_members(kv: &[(&str, i64)]) -> String {
    kv.iter()
        .map(|(k, v)| format!("\"{k}\":{v}"))
        .collect::<Vec<_>>()
        .join(",")
}
