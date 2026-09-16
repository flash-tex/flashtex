//! An inline formula cut at its break points shares its `\text` runs among
//! the pieces. `Context::math_pieces` cloned the whole `MathRec`, runs
//! included, for every piece, so memory grew as break points x runs: the
//! fuzzer's `$x+x+...\textbf{\textbf{...}}...$` passed the 4 GB watchdog
//! (`examples/fuzz_render.rs`, "memory over 4096 MiB in render"). The peak
//! heap is measured with a counting global allocator (std only).

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK.fetch_max(live, Ordering::SeqCst);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[test]
fn formula_pieces_share_their_text_runs() {
    let text = format!(
        "\\documentclass{{article}}\\begin{{document}}\n${}{}x{}{}$\n\\end{{document}}\n",
        "x+".repeat(1000),
        "\\textbf{".repeat(300),
        "}".repeat(300),
        "x+".repeat(1000)
    );
    let before = LIVE.load(Ordering::SeqCst);
    PEAK.store(before, Ordering::SeqCst);
    let rendered = common::render_one(&text);
    let peak = PEAK.load(Ordering::SeqCst) - before;
    assert!(!rendered.v2.pages.is_empty());
    // Before: 1.5 GB resident in release for this input. After: tens of MB.
    assert!(peak < 400 << 20, "rendering peaked at {} MiB of heap", peak >> 20);
}
