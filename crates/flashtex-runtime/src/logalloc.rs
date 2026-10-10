//! The host's global allocator on Linux and macOS (lane P4-MEMORY-BUDGET;
//! DESIGN.md §5.2): [`HEAP`] (on Linux jemalloc, or glibc's malloc without
//! the feature `jemalloc`; on macOS libmalloc), except that a large block
//! allocated for a checkpoint's undo log gets a mapping of its own, which
//! `free` unmaps.
//!
//! Most of the host's heap is undo logs (`flashtex_engine::arena`), sealed one per
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
//! **macOS: every large block, with spares** (lane MEM-BASELINE,
//! docs/evidence/mem-baseline-2026-10-09/). On macOS the logs are not the
//! only problem: a keystroke allocates and frees tens of megabytes of
//! transient blocks of 64 KB to 4 MB (the convergence test's `diff_branch`
//! and `ChunkDiff::table`, `rewound_until`, the prepared restore,
//! `free_cells`: 132 of the 150 MB a one-line document allocates in nine
//! compiles, `malloc_history -allEvents`), and xzone keeps every freed one in
//! the footprint (that document's host held 57-61 MB with 8 MB in use);
//! `malloc_zone_pressure_relief` returns none of it (0 bytes). So on macOS
//! every block of at least [`BIG`] bytes gets a mapping of its own, in any
//! scope ([`set_large_blocks`]: not under Low Memory's
//! `MallocSpaceEfficient`, which returns freed pages itself). A freed one is
//! kept as a *spare* (at most [`SPARE_SLOTS`] of them and [`SPARE_MAX`]
//! bytes, the least recently freed going first, joined to a spare right next
//! to it) for the next large blocks: the smallest spare that is large enough
//! serves a block, cut to its size when the rest is itself a large block (the
//! rest stays a spare). So a keystroke reuses the last one's pages, as xzone
//! did, without page faults; a block asked for zeroed is cleared there (the
//! zeroed blocks are mostly written: zeroing a spare took fewer instructions
//! than fresh pages' faults, measured). The idle trim ([`give_back`]) unmaps
//! the spares. (`MADV_FREE_REUSABLE`, which keeps the pages mapped, does not
//! take a spare out of the footprint once its mapping has been cut, nor
//! pages `mach_vm_copy` put there: measured. Growths copy: moving the pages
//! by `mach_vm_copy` cost 1-1.5 % more instructions a keystroke in copy-on-
//! write faults.) A block can get a little more than it asked for (a
//! spare's slack under `BIG`), so the table records each block's mapped
//! length.
//!
//! The mapped blocks are recorded in a fixed table (no allocation inside the
//! allocator); `dealloc` and `realloc` of a block of at least `BIG` bytes
//! look it up there. When the table is full, or [`MAX_MAPS`] mappings are
//! live (Linux allows a process 65,530 by default, `vm.max_map_count`, which
//! everything else in the process shares), a block goes to [`HEAP`].

use std::alloc::{GlobalAlloc, Layout};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// Where every block that is not a mapped log comes from.
#[cfg(all(target_os = "linux", feature = "jemalloc"))]
pub const HEAP: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
/// Where every block that is not a mapped log comes from.
#[cfg(not(all(target_os = "linux", feature = "jemalloc")))]
pub const HEAP: std::alloc::System = std::alloc::System;

/// The smallest block that gets a mapping of its own (a log's anywhere; on
/// macOS any block's).
pub const BIG: usize = 64 << 10;

/// Whether every block of at least [`BIG`] bytes gets a mapping, not only
/// the logs' (macOS, unless [`set_large_blocks`] turned it off).
static ANY_BIG: AtomicBool = AtomicBool::new(cfg!(target_os = "macos"));

fn any_big() -> bool {
    ANY_BIG.load(Ordering::Relaxed)
}

/// Whether every large block gets a mapping of its own (macOS only; never
/// elsewhere). The host turns it off when the system allocator already
/// returns freed pages (Low Memory's `MallocSpaceEfficient`): measured, the
/// mappings then cost footprint (full-100 at 106-118 MB against 82-86 MB).
pub fn set_large_blocks(on: bool) {
    ANY_BIG.store(on && cfg!(target_os = "macos"), Ordering::Relaxed);
}

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
/// Each recorded block's mapped length, by its slot in `TABLE`.
static LENS: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];

/// Mappings this allocator holds at most: half of Linux's default
/// `vm.max_map_count` (65,530), the rest left to the process (the word
/// space, the slab, glibc's own, file mappings).
const MAX_MAPS: usize = 32 << 10;
/// Mappings this allocator holds now (spares included; a spare cut in two,
/// or two spares joined, count as what they now are).
static MAPS: AtomicUsize = AtomicUsize::new(0);
/// Bytes of the recorded blocks' mappings (not the spares').
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

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

fn mapped_len(size: usize) -> usize {
    (size + PAGE - 1) & !(PAGE - 1)
}

fn slot(p: usize) -> usize {
    ((p >> 12).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (usize::BITS - SLOT_BITS)) & (SLOTS - 1)
}

/// Record block `p` of mapped length `len`; false if no slot is free.
fn insert(p: usize, len: usize) -> bool {
    let s = slot(p);
    for k in 0..PROBES {
        let i = (s + k) & (SLOTS - 1);
        let e = &TABLE[i];
        let v = e.load(Ordering::Relaxed);
        if (v == 0 || v == GONE)
            && e.compare_exchange(v, p, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        {
            LENS[i].store(len, Ordering::Release);
            LIVE_BYTES.fetch_add(len, Ordering::Relaxed);
            return true;
        }
    }
    false
}

/// The slot recording block `p`.
fn find(p: usize) -> Option<usize> {
    let s = slot(p);
    for k in 0..PROBES {
        let i = (s + k) & (SLOTS - 1);
        match TABLE[i].load(Ordering::Acquire) {
            0 => return None,
            v if v == p => return Some(i),
            _ => {}
        }
    }
    None
}

/// Forget slot `i`'s block (before its pages go anywhere else): its mapped
/// length.
fn forget(i: usize) -> usize {
    let len = LENS[i].load(Ordering::Acquire);
    TABLE[i].store(GONE, Ordering::Release);
    LIVE_BYTES.fetch_sub(len, Ordering::Relaxed);
    len
}

// ---------------------------------------------------------------------------
// Spares (macOS): freed mappings kept for the next large blocks
// ---------------------------------------------------------------------------

/// Spares kept at most.
const SPARE_SLOTS: usize = 64;
/// Bytes of spares kept by default ([`set_spare_cap`]): 64 MB measured as
/// well as 128 MB on art4 and full-100, and better than 32 MB.
pub const SPARE_MAX: usize = 64 << 20;
/// Bytes of spares kept at most: the least recently freed go first.
static SPARE_CAP: AtomicUsize = AtomicUsize::new(SPARE_MAX);

static SPARE_LOCK: AtomicBool = AtomicBool::new(false);
/// Each spare's address (0: none), mapped length and when it was freed (a
/// counter). Read and written only under `SPARE_LOCK`.
static SPARE_AT: [AtomicUsize; SPARE_SLOTS] = [const { AtomicUsize::new(0) }; SPARE_SLOTS];
static SPARE_LEN: [AtomicUsize; SPARE_SLOTS] = [const { AtomicUsize::new(0) }; SPARE_SLOTS];
static SPARE_AGE: [AtomicUsize; SPARE_SLOTS] = [const { AtomicUsize::new(0) }; SPARE_SLOTS];
static SPARE_CLOCK: AtomicUsize = AtomicUsize::new(0);
/// Bytes of spares now.
static SPARE_BYTES: AtomicUsize = AtomicUsize::new(0);

struct SpareGuard;
impl Drop for SpareGuard {
    fn drop(&mut self) {
        SPARE_LOCK.store(false, Ordering::Release);
    }
}
fn spare_lock() -> SpareGuard {
    while SPARE_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::hint::spin_loop();
    }
    SpareGuard
}

/// Take spare `i` out (under the lock): (address, length).
fn spare_out(i: usize) -> (usize, usize) {
    let at = SPARE_AT[i].swap(0, Ordering::Relaxed);
    let l = SPARE_LEN[i].load(Ordering::Relaxed);
    SPARE_BYTES.fetch_sub(l, Ordering::Relaxed);
    (at, l)
}

/// Keep at most `bytes` of spares (0 keeps none). Spares beyond it go at
/// the next free.
pub fn set_spare_cap(bytes: usize) {
    SPARE_CAP.store(bytes, Ordering::Relaxed);
}

/// Pages for a block of mapped length `len`, from the best-fitting spare:
/// the smallest one that is large enough, cut to `len` when what is left is
/// itself a large block (the rest stays a spare, with its age): (address,
/// mapped length).
fn take_spare(len: usize) -> Option<(usize, usize)> {
    if !any_big() || SPARE_BYTES.load(Ordering::Relaxed) < len {
        return None;
    }
    let _g = spare_lock();
    let mut best: Option<(usize, usize)> = None;
    for i in 0..SPARE_SLOTS {
        let l = SPARE_LEN[i].load(Ordering::Relaxed);
        if SPARE_AT[i].load(Ordering::Relaxed) != 0 && l >= len && best.is_none_or(|(_, b)| l < b) {
            best = Some((i, l));
        }
    }
    let (i, l) = best?;
    if l - len >= BIG {
        // two pieces of one mapping now, each unmapped on its own
        let p = SPARE_AT[i].load(Ordering::Relaxed);
        SPARE_AT[i].store(p + len, Ordering::Relaxed);
        SPARE_LEN[i].store(l - len, Ordering::Relaxed);
        SPARE_BYTES.fetch_sub(len, Ordering::Relaxed);
        MAPS.fetch_add(1, Ordering::Relaxed);
        Some((p, len))
    } else {
        Some(spare_out(i))
    }
}

/// Keep mapping `p` (mapped length `len`, no longer recorded) as a spare:
/// joined to any spare right next to it, room made by unmapping the least
/// recently freed; false if it is larger than all the room there is (the
/// caller unmaps it).
fn put_spare(p: usize, len: usize) -> bool {
    let cap = SPARE_CAP.load(Ordering::Relaxed);
    if !any_big() || len > cap {
        return false;
    }
    let mut gone: [(usize, usize); SPARE_SLOTS + 1] = [(0, 0); SPARE_SLOTS + 1];
    let mut ngone = 0;
    {
        let _g = spare_lock();
        let (mut at, mut l) = (p, len);
        for i in 0..SPARE_SLOTS {
            let a = SPARE_AT[i].load(Ordering::Relaxed);
            let li = SPARE_LEN[i].load(Ordering::Relaxed);
            if a != 0 && (a + li == at || at + l == a) {
                spare_out(i);
                at = at.min(a);
                l += li;
                MAPS.fetch_sub(1, Ordering::Relaxed);
            }
        }
        if l > cap {
            gone[ngone] = (at, l);
            ngone += 1;
        } else {
            loop {
                let mut free = None;
                let mut oldest: Option<(usize, usize)> = None;
                for i in 0..SPARE_SLOTS {
                    if SPARE_AT[i].load(Ordering::Relaxed) == 0 {
                        free = free.or(Some(i));
                    } else {
                        let a = SPARE_AGE[i].load(Ordering::Relaxed);
                        if oldest.is_none_or(|(_, o)| a < o) {
                            oldest = Some((i, a));
                        }
                    }
                }
                if let (Some(i), true) = (free, SPARE_BYTES.load(Ordering::Relaxed) + l <= cap) {
                    SPARE_AT[i].store(at, Ordering::Relaxed);
                    SPARE_LEN[i].store(l, Ordering::Relaxed);
                    SPARE_AGE[i].store(
                        SPARE_CLOCK.fetch_add(1, Ordering::Relaxed),
                        Ordering::Relaxed,
                    );
                    SPARE_BYTES.fetch_add(l, Ordering::Relaxed);
                    break;
                }
                // (`l <= cap`: some spare is there to go)
                let (i, _) = oldest.expect("a spare to make room");
                gone[ngone] = spare_out(i);
                ngone += 1;
            }
        }
    }
    for &(at, l) in &gone[..ngone] {
        // SAFETY: spares just taken out: our own mappings, unused.
        unsafe { munmap(at as *mut c_void, l) };
        MAPS.fetch_sub(1, Ordering::Relaxed);
        count(UNMAP_N, 1);
    }
    true
}

/// Unmap every spare (the idle trim): the bytes given back.
fn release_spares() -> usize {
    if SPARE_BYTES.load(Ordering::Relaxed) == 0 {
        return 0;
    }
    let mut gone: [(usize, usize); SPARE_SLOTS] = [(0, 0); SPARE_SLOTS];
    let mut ngone = 0;
    {
        let _g = spare_lock();
        for i in 0..SPARE_SLOTS {
            if SPARE_AT[i].load(Ordering::Relaxed) != 0 {
                gone[ngone] = spare_out(i);
                ngone += 1;
            }
        }
    }
    let mut n = 0;
    for &(at, l) in &gone[..ngone] {
        // SAFETY: spares just taken out: our own mappings, unused.
        unsafe { munmap(at as *mut c_void, l) };
        MAPS.fetch_sub(1, Ordering::Relaxed);
        count(UNMAP_N, 1);
        n += l;
    }
    n
}

/// Bytes in the recorded blocks' mappings and in the spares, for
/// `DONE.mem`.
pub fn mapped_stats() -> (u64, u64) {
    (
        LIVE_BYTES.load(Ordering::Relaxed) as u64,
        SPARE_BYTES.load(Ordering::Relaxed) as u64,
    )
}

const FRESH_N: usize = 0;
const FRESH_B: usize = 1;
const ZEROED_B: usize = 2;
const COPY_B: usize = 3;
const UNMAP_N: usize = 4;
static COUNTS: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];

fn count(i: usize, n: usize) {
    COUNTS[i].fetch_add(n as u64, Ordering::Relaxed);
}

/// What the mapped blocks cost since the start, for `DONE.mem`: fresh
/// mappings made (count, bytes), bytes of reused pages cleared for blocks
/// asked for zeroed, bytes copied by growths, mappings unmapped.
pub fn counters() -> [(&'static str, u64); 5] {
    let c = |i: usize| COUNTS[i].load(Ordering::Relaxed);
    [
        ("map_fresh", c(FRESH_N)),
        ("map_fresh_bytes", c(FRESH_B)),
        ("map_zeroed_bytes", c(ZEROED_B)),
        ("map_copied_bytes", c(COPY_B)),
        ("map_unmapped", c(UNMAP_N)),
    ]
}

/// A mapping of at least `size` bytes (a spare, or a fresh one), recorded;
/// `None` if there is none (then the caller uses [`HEAP`]). The flag says
/// whether it is a spare's (its pages hold old contents).
fn map(size: usize) -> Option<(*mut u8, bool)> {
    let want = mapped_len(size);
    if let Some((p, len)) = take_spare(want) {
        if insert(p, len) {
            return Some((p as *mut u8, true));
        }
        // SAFETY: the spare just taken, unused.
        unsafe { munmap(p as *mut c_void, len) };
        MAPS.fetch_sub(1, Ordering::Relaxed);
        return None;
    }
    // (a reservation: given back below when no mapping is made)
    if MAPS.fetch_add(1, Ordering::Relaxed) >= MAX_MAPS {
        MAPS.fetch_sub(1, Ordering::Relaxed);
        return None;
    }
    // SAFETY: an anonymous private mapping has no preconditions.
    let p = unsafe {
        mmap(
            std::ptr::null_mut(),
            want,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if p as isize == -1 {
        MAPS.fetch_sub(1, Ordering::Relaxed);
        return None;
    }
    count(FRESH_N, 1);
    count(FRESH_B, want);
    if insert(p as usize, want) {
        Some((p as *mut u8, false))
    } else {
        // SAFETY: the mapping just made, unused.
        unsafe { munmap(p, want) };
        MAPS.fetch_sub(1, Ordering::Relaxed);
        None
    }
}

/// Give back a recorded block (`i` its slot): the record first, so that no
/// lookup finds the address once another mapping may reuse it; then a spare
/// if there is room, else unmapped.
///
/// # Safety
/// `p` is the block slot `i` records, unused from now on.
unsafe fn unmap(i: usize, p: *mut u8) {
    let len = forget(i);
    if put_spare(p as usize, len) {
        return;
    }
    munmap(p as *mut c_void, len);
    MAPS.fetch_sub(1, Ordering::Relaxed);
    count(UNMAP_N, 1);
}

/// Whether this block is one of the mapped ones (only blocks of at least
/// `BIG` bytes can be): its slot.
fn is_mapped(p: *mut u8, size: usize) -> Option<usize> {
    if size < BIG {
        return None;
    }
    find(p as usize)
}

static ON: AtomicBool = AtomicBool::new(true);
/// Whether [`HostAlloc`] is the program's allocator (the host says so with
/// [`set_enabled`]); [`give_back`] does nothing otherwise.
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Turn the mapping of large blocks off or on (the host reads
/// `FLASHTEX_NO_LOG_MAPS` at start-up, for A/B). Blocks already mapped stay
/// recorded and are given back as usual. Only a program whose global
/// allocator is [`HostAlloc`] calls it.
pub fn set_enabled(on: bool) {
    ON.store(on, Ordering::Relaxed);
    INSTALLED.store(true, Ordering::Relaxed);
}

/// The host is idle (`host::resident`): give the pages [`HEAP`] holds free
/// back to the system now. jemalloc returns its free pages after a decay
/// time (10 s) but only while the program allocates, so an idle host would
/// keep its peak; this flushes the calling thread's cache and purges every
/// arena (`arena.<all>.purge`). On macOS the spares are unmapped. Whether
/// it purged or unmapped anything (not without the feature `jemalloc` on
/// Linux, nor in a program [`HostAlloc`] does not serve).
pub fn give_back() -> bool {
    if !INSTALLED.load(Ordering::Relaxed) {
        return false;
    }
    let spares = release_spares() > 0;
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
        call(c"arena.4096.purge") || spares
    }
    #[cfg(not(all(target_os = "linux", feature = "jemalloc")))]
    {
        spares
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
        && (any_big() || crate::memstat::in_log_scope())
        && ON.load(Ordering::Relaxed)
}

/// See the module's documentation.
pub struct HostAlloc;

// SAFETY: every block is either [`HEAP`]'s, passed to it unchanged, or a
// mapping of at least its size, page-aligned (so any alignment up to a
// page), recorded in `TABLE` (its mapped length in `LENS`) until it is given
// back.
unsafe impl GlobalAlloc for HostAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if wanted(&l) {
            if let Some((p, _)) = map(l.size()) {
                return p;
            }
        }
        HEAP.alloc(l)
    }

    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if wanted(&l) {
            if let Some((p, reused)) = map(l.size()) {
                // (a fresh anonymous mapping is zero; a spare is not)
                if reused {
                    count(ZEROED_B, l.size());
                    p.write_bytes(0, l.size());
                }
                return p;
            }
        }
        HEAP.alloc_zeroed(l)
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if let Some(i) = is_mapped(p, l.size()) {
            unmap(i, p);
            return;
        }
        HEAP.dealloc(p, l)
    }

    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        if let Some(i) = is_mapped(p, l.size()) {
            let (old_len, new_len) = (LENS[i].load(Ordering::Acquire), mapped_len(new));
            // In place, where the mapping allows (never moved by the kernel:
            // a moved block would leave its old address recorded for a
            // moment, and could not be given back if the new one found no
            // slot).
            if new >= BIG && new_len <= old_len {
                // The tail goes now, unless it is small (a spare's slack,
                // which a growth may use again).
                if new_len < old_len && (!any_big() || old_len - new_len >= BIG) {
                    munmap(p.add(new_len) as *mut c_void, old_len - new_len);
                    LENS[i].store(new_len, Ordering::Release);
                    LIVE_BYTES.fetch_sub(old_len - new_len, Ordering::Relaxed);
                }
                return p;
            }
            // (Linux only: macOS has no `mremap`, so a growth is a copy)
            #[cfg(target_os = "linux")]
            if new >= BIG && mremap(p as *mut c_void, old_len, new_len, 0) as *mut u8 == p {
                LENS[i].store(new_len, Ordering::Release);
                LIVE_BYTES.fetch_add(new_len - old_len, Ordering::Relaxed);
                return p;
            }
            // Moved by hand: a new block, recorded or [`HEAP`]'s, and the old
            // one given back only once the copy is made. A null leaves the old
            // block as it was, as `realloc` must.
            let q = match (new >= BIG).then(|| map(new)).flatten() {
                Some((q, _)) => q,
                None => HEAP.alloc(Layout::from_size_align_unchecked(new, l.align())),
            };
            if !q.is_null() {
                count(COPY_B, l.size().min(new));
                std::ptr::copy_nonoverlapping(p, q, l.size().min(new));
                unmap(i, p);
            }
            return q;
        }
        let nl = Layout::from_size_align_unchecked(new, l.align());
        if wanted(&nl) {
            if let Some((q, _)) = map(new) {
                count(COPY_B, l.size().min(new));
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

    /// The tests share the allocator's tables and spares.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Unmap every spare (a test starts from none).
    fn drop_spares() {
        release_spares();
        assert_eq!(SPARE_BYTES.load(Ordering::Relaxed), 0);
    }

    fn spares() -> Vec<(usize, usize)> {
        let _g = spare_lock();
        (0..SPARE_SLOTS)
            .filter(|&i| SPARE_AT[i].load(Ordering::Relaxed) != 0)
            .map(|i| {
                (
                    SPARE_AT[i].load(Ordering::Relaxed),
                    SPARE_LEN[i].load(Ordering::Relaxed),
                )
            })
            .collect()
    }

    #[test]
    fn log_blocks_are_mapped_and_given_back() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        drop_spares();
        let a = HostAlloc;
        let l = Layout::from_size_align(BIG * 3, 8).unwrap();
        // outside a log scope: HEAP's (on macOS, mapped: every large block is)
        let p = unsafe { a.alloc(l) };
        assert_eq!(find(p as usize).is_some(), any_big());
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
        // a growth nothing can give: null, and the block stays as it was
        // (recorded, its contents kept), as `realloc` must leave it
        let l4 = Layout::from_size_align(BIG * 4, 8).unwrap();
        let u = unsafe { a.alloc(l4) };
        assert!(find(u as usize).is_some());
        unsafe { u.write_bytes(5, l4.size()) };
        let huge = (isize::MAX as usize / 2) & !(PAGE - 1);
        assert!(unsafe { a.realloc(u, l4, huge) }.is_null());
        assert!(find(u as usize).is_some(), "still recorded");
        assert_eq!(unsafe { *u.add(l4.size() - 1) }, 5, "still there");
        // shrunk within its mapping: in place, the tail given back
        let v = unsafe { a.realloc(u, l4, BIG * 2) };
        assert_eq!(v, u);
        assert_eq!(unsafe { *v.add(BIG * 2 - 1) }, 5);
        let maps = MAPS.load(Ordering::Relaxed);
        let spare = SPARE_BYTES.load(Ordering::Relaxed);
        unsafe { a.dealloc(v, Layout::from_size_align(BIG * 2, 8).unwrap()) };
        assert!(find(v as usize).is_none());
        if any_big() {
            // kept as a spare: still a mapping (or joined to a spare next
            // to it)
            assert_eq!(SPARE_BYTES.load(Ordering::Relaxed), spare + BIG * 2);
            assert!(MAPS.load(Ordering::Relaxed) <= maps);
        } else {
            assert_eq!(
                MAPS.load(Ordering::Relaxed),
                maps - 1,
                "the mapping count follows"
            );
        }
    }

    #[test]
    fn idle_give_back_purges_the_heap() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        // (a test binary's own allocator is `System`: only after the host's
        // `set_enabled` does `give_back` touch HEAP)
        set_enabled(true);
        let a = HostAlloc;
        let l = Layout::from_size_align(8 << 20, 8).unwrap();
        let p = unsafe { a.alloc(l) };
        unsafe { p.write_bytes(1, l.size()) };
        unsafe { a.dealloc(p, l) };
        // (macOS: the block became a spare, which the trim unmaps)
        assert_eq!(
            give_back(),
            cfg!(any(
                all(target_os = "linux", feature = "jemalloc"),
                target_os = "macos"
            ))
        );
        assert_eq!(SPARE_BYTES.load(Ordering::Relaxed), 0);
    }

    /// macOS: a freed large block is kept and serves the next large blocks,
    /// whole or cut, cleared when asked for zeroed; the idle trim unmaps the
    /// spares; spares are bounded.
    #[test]
    #[cfg(target_os = "macos")]
    fn spares_serve_the_next_large_blocks() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        drop_spares();
        let a = HostAlloc;
        let mb = Layout::from_size_align(1 << 20, 8).unwrap();
        let p = unsafe { a.alloc(mb) };
        assert!(find(p as usize).is_some());
        unsafe { p.write_bytes(0xAB, mb.size()) };
        unsafe { a.dealloc(p, mb) };
        assert!(find(p as usize).is_none(), "a spare is not a live block");
        assert_eq!(spares(), vec![(p as usize, 1 << 20)]);
        // a block of about its size takes it whole, cleared when asked
        let near = Layout::from_size_align((1 << 20) - 20_000, 8).unwrap();
        let q = unsafe { a.alloc_zeroed(near) };
        assert_eq!(q, p, "the spare serves the next block of about its size");
        assert!(unsafe { std::slice::from_raw_parts(q, near.size()) }
            .iter()
            .all(|&b| b == 0));
        // its whole mapping is recorded: a growth into it stays in place
        let r = unsafe { a.realloc(q, near, 1 << 20) };
        assert_eq!(r, q);
        unsafe { r.write_bytes(0xCD, 1 << 20) };
        unsafe { a.dealloc(r, mb) };
        // a smaller block takes its head; the rest stays a spare
        let small = Layout::from_size_align(BIG, 8).unwrap();
        let s = unsafe { a.alloc(small) };
        assert_eq!(s, p);
        assert_eq!(spares(), vec![(p as usize + BIG, (1 << 20) - BIG)]);
        // given back, it joins the rest again
        unsafe { a.dealloc(s, small) };
        assert_eq!(spares(), vec![(p as usize, 1 << 20)]);
        // a growth moves by copy, the old block becoming a spare
        let half = Layout::from_size_align(512 << 10, 8).unwrap();
        let g = unsafe { a.alloc(half) };
        for k in 0..half.size() {
            unsafe { *g.add(k) = (k % 251) as u8 };
        }
        let h = unsafe { a.realloc(g, half, 2 << 20) };
        assert_ne!(h, g);
        assert!(find(g as usize).is_none() && find(h as usize).is_some());
        assert!((0..half.size()).all(|k| unsafe { *h.add(k) } == (k % 251) as u8));
        unsafe { a.dealloc(h, Layout::from_size_align(2 << 20, 8).unwrap()) };
        // the idle trim unmaps them all
        set_enabled(true);
        let maps = MAPS.load(Ordering::Relaxed);
        let n = spares().len();
        assert!(n > 0 && give_back());
        assert!(spares().is_empty());
        assert_eq!(MAPS.load(Ordering::Relaxed), maps - n);
        // spares are bounded: a block beyond the cap is unmapped
        let big = Layout::from_size_align(SPARE_CAP.load(Ordering::Relaxed) + PAGE, 8).unwrap();
        let u = unsafe { a.alloc(big) };
        let maps = MAPS.load(Ordering::Relaxed);
        unsafe { a.dealloc(u, big) };
        assert_eq!(MAPS.load(Ordering::Relaxed), maps - 1);
        assert!(spares().is_empty());
        // and they stay within the cap
        set_spare_cap(3 << 20);
        let blocks: Vec<*mut u8> = (0..4).map(|_| unsafe { a.alloc(mb) }).collect();
        for &b in &blocks {
            unsafe { a.dealloc(b, mb) };
        }
        assert!(SPARE_BYTES.load(Ordering::Relaxed) <= 3 << 20);
        set_spare_cap(SPARE_MAX);
        drop_spares();
    }
}
