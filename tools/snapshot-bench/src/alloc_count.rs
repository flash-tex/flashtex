//! A counting global allocator, so the software mechanisms' memory can be reported as
//! bytes actually held rather than bytes inferred from a model.
//!
//! Kernel copy-on-write is invisible to this: its cost is in the task's resident size,
//! which `crate::mach::resident_bytes` reads instead.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

pub struct Counting;

// SAFETY: every method forwards to `System`, which is a correct allocator, and only adds
// bookkeeping that does not touch the returned memory.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            bump(layout.size() as isize);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        bump(-(layout.size() as isize));
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, layout, new_size);
        if !p.is_null() {
            bump(new_size as isize - layout.size() as isize);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            bump(layout.size() as isize);
        }
        p
    }
}

#[inline]
fn bump(delta: isize) {
    let prev = if delta >= 0 {
        LIVE.fetch_add(delta as usize, Ordering::Relaxed) + delta as usize
    } else {
        LIVE.fetch_sub((-delta) as usize, Ordering::Relaxed)
    };
    // Relaxed max: exact enough for a report, and never in the hot loop.
    PEAK.fetch_max(prev, Ordering::Relaxed);
}

pub fn live_bytes() -> usize {
    LIVE.load(Ordering::Relaxed)
}
/// High-water mark of live heap bytes. Reported next to a retention figure so the
/// transient the measurement passed through is visible, not just the settled total.
pub fn peak_bytes() -> usize {
    PEAK.load(Ordering::Relaxed)
}
