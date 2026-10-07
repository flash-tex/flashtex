//! The host's global allocator on Linux and macOS (lane P4-MEMORY-BUDGET;
//! DESIGN.md §5.2): [`HEAP`] (on Linux jemalloc, or glibc's malloc without
//! the feature `jemalloc`; on macOS libmalloc), except that a large block
//! allocated for a checkpoint's undo log gets a mapping of its own, which
//! `free` unmaps.
//!
//! Most of the host's heap is undo logs (`crate::arena`), sealed one per
//! checkpoint and merged, a few hundred kilobytes to megabytes at a time, by
//! retention and when a run thins the old run behind it; a run that ends
//! drops a whole branch of them. glibc gives a block above its mmap threshold
//! a mapping of its own, but the threshold rises to the largest such block
//! freed (up to 32 MB), so after the first merges every log came from the
//! arenas, and what the logs freed there stayed resident: on full-1000, 0.4
//! of the 0.9 GB the host held after its first compile was free (measured
//! 2026-10-06, main + #1573/#1575). A fixed threshold of 64 KB for every
//! block (`mallopt`) gave the memory back too, but it also mapped and
//! unmapped every large transient buffer of every keystroke (the convergence
//! test's, the restore's: about 100 MB per keystroke on full-1000), and
//! their page faults cost 5-10% more thread CPU time on cold compiles and
//! keystrokes. So only the logs go to their own mappings: a block of at
//! least [`BIG`] bytes allocated while the thread is in
//! `memstat::scope(tag::LOG)` (sealing and merging logs,
//! [`crate::memstat::in_log_scope`]). Everything else stays with [`HEAP`],
//! whose arenas keep reusing the transient buffers as before.
//!
//! [`HEAP`] is jemalloc (feature `jemalloc`, on by default): for the same
//! work glibc's arenas held more at the peak, 650 against 601-605 MB on
//! full-1000 and 536-538 against 507-512 MB on plain-1000, with the same
//! instructions (docs/evidence/mem-research-2026-10-06/ §2.4, under
//! `LD_PRELOAD`). The C parts (kpathsea, zlib, pdfTeX's C code) keep
//! calling the C library's `malloc`: the crate's symbols are prefixed.
//! [`give_back`] purges what jemalloc holds free when the host is idle.
//!
//! macOS: libmalloc's xzone allocator keeps freed spans in the process's
//! `phys_footprint` (what Activity Monitor shows) until the kernel reclaims
//! them, which it does only once they reach a tenth of the lifetime peak
//! (docs/evidence/mem-research-2026-10-06/ §2.7). A log block's own mapping
//! leaves the footprint at `munmap`. There is no `mremap`: a mapped block
//! shrinks in place (its tail unmapped) and grows into a new mapping, by
//! copy. (The logs are sized before they are filled and only shrink, by
//! `shrink_to_fit`, so the copy is the rare case.)
//!
//! The mapped blocks are recorded in a fixed table (no allocation inside the
//! allocator); `dealloc` and `realloc` of a block of at least `BIG` bytes
//! look it up there. When the table is full a block goes to [`HEAP`].

use std::alloc::{GlobalAlloc, Layout};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Where every block that is not a mapped log comes from.
#[cfg(all(target_os = "linux", feature = "jemalloc"))]
pub const HEAP: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
/// Where every block that is not a mapped log comes from.
#[cfg(not(all(target_os = "linux", feature = "jemalloc")))]
pub const HEAP: std::alloc::System = std::alloc::System;

/// The smallest log block that gets a mapping of its own.
pub const BIG: usize = 64 << 10;

/// The mappings' granularity: the page (16 KB on Apple silicon; also used
/// on Intel Macs, whose 4 KB pages divide it).
#[cfg(target_os = "macos")]
const PAGE: usize = 16384;
#[cfg(not(target_os = "macos"))]
const PAGE: usize = 4096;
const SLOT_BITS: u32 = 17;
const SLOTS: usize = 1 << SLOT_BITS;
/// A slot whose block was unmapped (a lookup goes past it, an insert
/// reuses it).
const GONE: usize = usize::MAX;
/// Slots an operation looks at before it gives up.
const PROBES: usize = 64;

static TABLE: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];

extern "C" {
    fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, off: i64)
        -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
    #[cfg(target_os = "linux")]
    fn mremap(old: *mut c_void, old_len: usize, new_len: usize, flags: i32, ...) -> *mut c_void;
}
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 2;
#[cfg(target_os = "linux")]
const MAP_ANONYMOUS: i32 = 0x20;
#[cfg(target_os = "macos")]
const MAP_ANONYMOUS: i32 = 0x1000;
#[cfg(target_os = "linux")]
const MREMAP_MAYMOVE: i32 = 1;

fn mapped_len(size: usize) -> usize {
    (size + PAGE - 1) & !(PAGE - 1)
}

fn slot(p: usize) -> usize {
    ((p >> 12).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (usize::BITS - SLOT_BITS)) & (SLOTS - 1)
}

fn insert(p: usize) -> bool {
    let s = slot(p);
    for k in 0..PROBES {
        let e = &TABLE[(s + k) & (SLOTS - 1)];
        let v = e.load(Ordering::Relaxed);
        if (v == 0 || v == GONE)
            && e.compare_exchange(v, p, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        {
            return true;
        }
    }
    false
}

fn find(p: usize) -> Option<&'static AtomicUsize> {
    let s = slot(p);
    for k in 0..PROBES {
        let e = &TABLE[(s + k) & (SLOTS - 1)];
        match e.load(Ordering::Acquire) {
            0 => return None,
            v if v == p => return Some(e),
            _ => {}
        }
    }
    None
}

/// A fresh mapping of at least `size` bytes, recorded; `None` if there is
/// none (then the caller uses [`HEAP`]).
fn map(size: usize) -> Option<*mut u8> {
    let len = mapped_len(size);
    // SAFETY: an anonymous private mapping has no preconditions.
    let p = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if p as isize == -1 {
        return None;
    }
    if insert(p as usize) {
        Some(p as *mut u8)
    } else {
        // SAFETY: the mapping just made, unused.
        unsafe { munmap(p, len) };
        None
    }
}

/// Whether this block is one of the mapped ones (only blocks of at least
/// `BIG` bytes can be).
fn is_mapped(p: *mut u8, size: usize) -> Option<&'static AtomicUsize> {
    if size < BIG {
        return None;
    }
    find(p as usize)
}

static ON: AtomicBool = AtomicBool::new(true);
/// Whether [`HostAlloc`] is the program's allocator (the host says so with
/// [`set_enabled`]); [`give_back`] does nothing otherwise.
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Turn the mapping of log blocks off or on (the host reads
/// `FLASHTEX_NO_LOG_MAPS` at start-up, for A/B). Blocks already mapped stay
/// recorded and are unmapped as usual. Only a program whose global
/// allocator is [`HostAlloc`] calls it.
pub fn set_enabled(on: bool) {
    ON.store(on, Ordering::Relaxed);
    INSTALLED.store(true, Ordering::Relaxed);
}

/// The host is idle (`host::resident`): give the pages [`HEAP`] holds free
/// back to the system now. jemalloc returns its free pages after a decay
/// time (10 s) but only while the program allocates, so an idle host would
/// keep its peak; this flushes the calling thread's cache and purges every
/// arena (`arena.<all>.purge`). Whether it purged (not without the feature
/// `jemalloc`, nor in a program [`HostAlloc`] does not serve).
pub fn give_back() -> bool {
    if !INSTALLED.load(Ordering::Relaxed) {
        return false;
    }
    #[cfg(all(target_os = "linux", feature = "jemalloc"))]
    {
        use tikv_jemalloc_sys::mallctl;
        let call = |name: &std::ffi::CStr| {
            // SAFETY: a control that takes and returns nothing.
            unsafe {
                mallctl(
                    name.as_ptr(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                ) == 0
            }
        };
        call(c"thread.tcache.flush");
        // (4096 is MALLCTL_ARENAS_ALL)
        call(c"arena.4096.purge")
    }
    #[cfg(not(all(target_os = "linux", feature = "jemalloc")))]
    {
        false
    }
}

/// jemalloc's bytes allocated and resident (`stats.allocated`,
/// `stats.resident`; the features `jemalloc` and `mem-stats`), for
/// `memstat::malloc_in_use`.
pub fn heap_stats() -> Option<(u64, u64)> {
    #[cfg(all(target_os = "linux", feature = "jemalloc", feature = "mem-stats"))]
    {
        use tikv_jemalloc_sys::mallctl;
        if !INSTALLED.load(Ordering::Relaxed) {
            return None;
        }
        let read = |name: &std::ffi::CStr| -> Option<u64> {
            let mut v: usize = 0;
            let mut len = std::mem::size_of::<usize>();
            // SAFETY: a `size_t` statistic read into a `size_t`.
            let r = unsafe {
                mallctl(
                    name.as_ptr(),
                    (&mut v as *mut usize).cast(),
                    &mut len,
                    std::ptr::null_mut(),
                    0,
                )
            };
            (r == 0).then_some(v as u64)
        };
        // the statistics are refreshed when the epoch advances
        let mut e: u64 = 1;
        let mut elen = std::mem::size_of::<u64>();
        // SAFETY: `epoch` reads and writes a `uint64_t`.
        unsafe {
            mallctl(
                c"epoch".as_ptr(),
                (&mut e as *mut u64).cast(),
                &mut elen,
                (&mut e as *mut u64).cast(),
                elen,
            )
        };
        Some((read(c"stats.allocated")?, read(c"stats.resident")?))
    }
    #[cfg(not(all(target_os = "linux", feature = "jemalloc", feature = "mem-stats")))]
    {
        None
    }
}

fn wanted(l: &Layout) -> bool {
    l.size() >= BIG
        && l.align() <= PAGE
        && crate::memstat::in_log_scope()
        && ON.load(Ordering::Relaxed)
}

/// See the module's documentation.
pub struct HostAlloc;

/// A mapped block `p` (recorded in `e`, laid out as `l`) resized to `new`
/// bytes, at least [`BIG`]: Linux moves its pages (`mremap`).
#[cfg(target_os = "linux")]
unsafe fn resize(p: *mut u8, e: &AtomicUsize, l: Layout, new: usize) -> *mut u8 {
    let q = mremap(
        p as *mut c_void,
        mapped_len(l.size()),
        mapped_len(new),
        MREMAP_MAYMOVE,
    );
    if q as isize == -1 {
        return std::ptr::null_mut();
    }
    if q as *mut u8 != p {
        e.store(GONE, Ordering::Release);
        if !insert(q as usize) {
            // (cannot keep it recorded: give it to HEAP)
            let s = HEAP.alloc(Layout::from_size_align_unchecked(new, l.align()));
            if !s.is_null() {
                std::ptr::copy_nonoverlapping(q as *const u8, s, new);
            }
            munmap(q, mapped_len(new));
            return s;
        }
    }
    q as *mut u8
}

/// A mapped block `p` (recorded in `e`, laid out as `l`) resized to `new`
/// bytes, at least [`BIG`]. macOS has no `mremap`: a block that shrinks
/// keeps its place and unmaps its tail; one that grows is copied into a new
/// mapping (or, if none can be recorded, into a block of [`HEAP`]).
#[cfg(target_os = "macos")]
unsafe fn resize(p: *mut u8, e: &AtomicUsize, l: Layout, new: usize) -> *mut u8 {
    let (had, want) = (mapped_len(l.size()), mapped_len(new));
    if want <= had {
        if want < had {
            munmap(p.add(want) as *mut c_void, had - want);
        }
        return p;
    }
    let q = match map(new) {
        Some(q) => q,
        None => {
            let s = HEAP.alloc(Layout::from_size_align_unchecked(new, l.align()));
            if s.is_null() {
                return s;
            }
            s
        }
    };
    std::ptr::copy_nonoverlapping(p, q, l.size());
    e.store(GONE, Ordering::Release);
    munmap(p as *mut c_void, had);
    q
}

// SAFETY: every block is either [`HEAP`]'s, passed to it unchanged, or a
// mapping of at least its size, page-aligned (so any alignment up to a
// page), recorded in `TABLE` until it is unmapped.
unsafe impl GlobalAlloc for HostAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if wanted(&l) {
            if let Some(p) = map(l.size()) {
                return p;
            }
        }
        HEAP.alloc(l)
    }

    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if wanted(&l) {
            if let Some(p) = map(l.size()) {
                return p; // (a fresh anonymous mapping is zero)
            }
        }
        HEAP.alloc_zeroed(l)
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if let Some(e) = is_mapped(p, l.size()) {
            e.store(GONE, Ordering::Release);
            munmap(p as *mut c_void, mapped_len(l.size()));
            return;
        }
        HEAP.dealloc(p, l)
    }

    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        if let Some(e) = is_mapped(p, l.size()) {
            if new >= BIG {
                return resize(p, e, l, new);
            }
            let s = HEAP.alloc(Layout::from_size_align_unchecked(new, l.align()));
            if !s.is_null() {
                std::ptr::copy_nonoverlapping(p, s, new);
                e.store(GONE, Ordering::Release);
                munmap(p as *mut c_void, mapped_len(l.size()));
            }
            return s;
        }
        let nl = Layout::from_size_align_unchecked(new, l.align());
        if wanted(&nl) {
            if let Some(q) = map(new) {
                std::ptr::copy_nonoverlapping(p, q, l.size().min(new));
                HEAP.dealloc(p, l);
                return q;
            }
        }
        HEAP.realloc(p, l, new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_blocks_are_mapped_and_given_back() {
        let a = HostAlloc;
        let l = Layout::from_size_align(BIG * 3, 8).unwrap();
        // outside a log scope: HEAP's
        let p = unsafe { a.alloc(l) };
        assert!(find(p as usize).is_none());
        unsafe { a.dealloc(p, l) };
        let _m = crate::memstat::scope(crate::memstat::tag::LOG);
        let p = unsafe { a.alloc(l) };
        assert!(find(p as usize).is_some(), "a log block is mapped");
        unsafe { p.write_bytes(7, l.size()) };
        // grown: still mapped, contents kept
        let q = unsafe { a.realloc(p, l, BIG * 40) };
        assert!(find(q as usize).is_some());
        assert_eq!(unsafe { *q.add(BIG * 3 - 1) }, 7);
        unsafe { q.write_bytes(8, BIG * 40) };
        // shrunk, still at least BIG: in place, mapped
        let l40 = Layout::from_size_align(BIG * 40, 8).unwrap();
        let q2 = unsafe { a.realloc(q, l40, BIG * 2 + 100) };
        assert_eq!(q2, q, "a log block shrinks in place");
        assert!(find(q as usize).is_some());
        assert_eq!(unsafe { *q.add(BIG * 2 + 99) }, 8);
        // grown again: mapped, contents kept
        let l2 = Layout::from_size_align(BIG * 2 + 100, 8).unwrap();
        let q = unsafe { a.realloc(q2, l2, BIG * 40) };
        assert!(find(q as usize).is_some());
        assert_eq!(unsafe { *q.add(BIG * 2 + 99) }, 8);
        let l2 = Layout::from_size_align(BIG * 40, 8).unwrap();
        // shrunk below BIG: back to HEAP, contents kept
        let r = unsafe { a.realloc(q, l2, 100) };
        assert!(find(q as usize).is_none() && find(r as usize).is_none());
        assert_eq!(unsafe { *r.add(99) }, 8);
        unsafe { a.dealloc(r, Layout::from_size_align(100, 8).unwrap()) };
        // a HEAP block grown past BIG in a log scope moves to a mapping
        let s = unsafe { a.alloc(Layout::from_size_align(1000, 8).unwrap()) };
        unsafe { s.write_bytes(9, 1000) };
        let t = unsafe { a.realloc(s, Layout::from_size_align(1000, 8).unwrap(), BIG * 2) };
        assert!(find(t as usize).is_some());
        assert_eq!(unsafe { *t.add(999) }, 9);
        unsafe { a.dealloc(t, Layout::from_size_align(BIG * 2, 8).unwrap()) };
        assert!(find(t as usize).is_none());
    }

    #[test]
    fn idle_give_back_purges_the_heap() {
        // (a test binary's own allocator is `System`: only after the host's
        // `set_enabled` does `give_back` touch HEAP)
        set_enabled(true);
        let a = HostAlloc;
        let l = Layout::from_size_align(8 << 20, 8).unwrap();
        let p = unsafe { a.alloc(l) };
        unsafe { p.write_bytes(1, l.size()) };
        unsafe { a.dealloc(p, l) };
        assert_eq!(
            give_back(),
            cfg!(all(target_os = "linux", feature = "jemalloc"))
        );
    }
}
