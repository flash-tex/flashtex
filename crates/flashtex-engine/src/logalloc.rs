//! The host's global allocator on Linux (lane P4-MEMORY-BUDGET; DESIGN.md
//! §5.2): `System` (glibc's malloc), except that a large block allocated for
//! a checkpoint's undo log gets a mapping of its own, which `free` unmaps.
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
//! [`crate::memstat::in_log_scope`]). Everything else stays with `System`,
//! whose arenas keep reusing the transient buffers as before.
//!
//! The mapped blocks are recorded in a fixed table (no allocation inside the
//! allocator); `dealloc` and `realloc` of a block of at least `BIG` bytes
//! look it up there. When the table is full a block goes to `System`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The smallest log block that gets a mapping of its own.
pub const BIG: usize = 64 << 10;

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
    fn mremap(old: *mut c_void, old_len: usize, new_len: usize, flags: i32, ...) -> *mut c_void;
}
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 2;
const MAP_ANONYMOUS: i32 = 0x20;
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
/// none (then the caller uses `System`).
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

static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Turn the mapping of log blocks off or on (the host reads
/// `FLASHTEX_NO_LOG_MAPS` at start-up, for A/B). Blocks already mapped stay
/// recorded and are unmapped as usual.
pub fn set_enabled(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

fn wanted(l: &Layout) -> bool {
    l.size() >= BIG
        && l.align() <= PAGE
        && crate::memstat::in_log_scope()
        && ON.load(Ordering::Relaxed)
}

/// See the module's documentation.
pub struct HostAlloc;

// SAFETY: every block is either `System`'s, passed to it unchanged, or a
// mapping of at least its size, page-aligned (so any alignment up to a
// page), recorded in `TABLE` until it is unmapped.
unsafe impl GlobalAlloc for HostAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if wanted(&l) {
            if let Some(p) = map(l.size()) {
                return p;
            }
        }
        System.alloc(l)
    }

    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if wanted(&l) {
            if let Some(p) = map(l.size()) {
                return p; // (a fresh anonymous mapping is zero)
            }
        }
        System.alloc_zeroed(l)
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if let Some(e) = is_mapped(p, l.size()) {
            e.store(GONE, Ordering::Release);
            munmap(p as *mut c_void, mapped_len(l.size()));
            return;
        }
        System.dealloc(p, l)
    }

    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        if let Some(e) = is_mapped(p, l.size()) {
            if new >= BIG {
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
                        // (cannot keep it recorded: give it to System)
                        let s = System.alloc(Layout::from_size_align_unchecked(new, l.align()));
                        if !s.is_null() {
                            std::ptr::copy_nonoverlapping(q as *const u8, s, new);
                        }
                        munmap(q, mapped_len(new));
                        return s;
                    }
                }
                return q as *mut u8;
            }
            let s = System.alloc(Layout::from_size_align_unchecked(new, l.align()));
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
                System.dealloc(p, l);
                return q;
            }
        }
        System.realloc(p, l, new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_blocks_are_mapped_and_given_back() {
        let a = HostAlloc;
        let l = Layout::from_size_align(BIG * 3, 8).unwrap();
        // outside a log scope: System's
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
        let l2 = Layout::from_size_align(BIG * 40, 8).unwrap();
        // shrunk below BIG: back to System, contents kept
        let r = unsafe { a.realloc(q, l2, 100) };
        assert!(find(q as usize).is_none() && find(r as usize).is_none());
        assert_eq!(unsafe { *r.add(99) }, 7);
        unsafe { a.dealloc(r, Layout::from_size_align(100, 8).unwrap()) };
        // a System block grown past BIG in a log scope moves to a mapping
        let s = unsafe { a.alloc(Layout::from_size_align(1000, 8).unwrap()) };
        unsafe { s.write_bytes(9, 1000) };
        let t = unsafe { a.realloc(s, Layout::from_size_align(1000, 8).unwrap(), BIG * 2) };
        assert!(find(t as usize).is_some());
        assert_eq!(unsafe { *t.add(999) }, 9);
        unsafe { a.dealloc(t, Layout::from_size_align(BIG * 2, 8).unwrap()) };
        assert!(find(t as usize).is_none());
    }
}
