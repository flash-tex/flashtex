//! Mechanism (b): software 16 KB-chunk copy-on-write.
//!
//! Two variants, because the assignment and DESIGN §5.2 name different ones and they do
//! not cost the same:
//!
//! * [`ArcCow`] (b1) — `Vec<Arc<[u64; CHUNK_WORDS]>>`, barrier = `Arc::make_mut`. This
//!   is the shape the assignment names. Its cost is an **atomic compare-exchange on the
//!   strong count for every write**, hit or miss: `Arc::make_mut` has to prove
//!   uniqueness, and it does so by `strong.compare_exchange(1, 0, Acquire, Relaxed)`.
//!   There is no fast path that avoids the RMW.
//!
//! * [`BitmapCow`] (b2) — `Vec<Arc<[Cell<u64>; CHUNK_WORDS]>>` plus a dirty bitmap, the
//!   shape DESIGN §5.2 names. The barrier is a plain non-atomic bit test against a
//!   bitmap that is tiny (one bit per 16 KB, so 1.6 KB for a 200 MB state, L1-resident)
//!   and the slow path runs once per chunk per checkpoint interval. `Cell<u64>` is
//!   `#[repr(transparent)]`, so reads and writes compile to a plain load and store; it
//!   only buys the ability to mutate through the shared `Arc` handle after the bitmap
//!   has already proved the chunk is private.
//!
//! Both snapshot by cloning the chunk table, which is a refcount bump per chunk.
//! `Arc` rather than `Rc` is deliberate: checkpoint eviction and the persistence of S₀
//! (DESIGN §5.1) want to drop and write snapshots off the engine thread. The atomic
//! increments land in the snapshot path, never in the barrier, so they cost one pass
//! over a 12,800-entry pointer table at 200 MB.

use std::cell::Cell;
use std::sync::Arc;

use crate::backend::{Backend, Snapshot};
use crate::layout::{Layout, CHUNK_MASK, CHUNK_SHIFT, CHUNK_WORDS};

// --------------------------------------------------------------------------------
// (b1) Arc::make_mut
// --------------------------------------------------------------------------------

type PlainChunk = [u64; CHUNK_WORDS];

/// A zeroed 16 KB chunk, allocated straight on the heap.
///
/// `Arc::new([0u64; CHUNK_WORDS])` would build a 16 KB temporary in the caller's frame
/// and then copy it; `Arc::new_uninit` (stable since 1.82) lets the single `memset` land
/// in the final allocation, which keeps the privatisation measurement honest.
fn new_plain_chunk() -> Arc<PlainChunk> {
    let mut a: Arc<std::mem::MaybeUninit<PlainChunk>> = Arc::new_uninit();
    // SAFETY: a is freshly allocated and uniquely owned, so get_mut succeeds; writing
    // CHUNK_WORDS zeroed u64 initialises every element of [u64; CHUNK_WORDS].
    unsafe {
        let dst = Arc::get_mut(&mut a).unwrap().as_mut_ptr() as *mut u64;
        std::ptr::write_bytes(dst, 0, CHUNK_WORDS);
        a.assume_init()
    }
}

pub struct ArcSnap {
    chunks: Vec<Arc<PlainChunk>>,
}

impl Snapshot for ArcSnap {
    fn nominal_bytes(&self) -> usize {
        let n = self
            .chunks
            .iter()
            .filter(|c| Arc::strong_count(c) == 1)
            .count();
        n * CHUNK_WORDS * 8 + self.chunks.len() * std::mem::size_of::<usize>()
    }
}

pub struct ArcCow {
    chunks: Vec<Arc<PlainChunk>>,
}

impl Backend for ArcCow {
    type Snap = ArcSnap;

    fn name(&self) -> &'static str {
        "arc-make-mut"
    }

    fn new(layout: &Layout) -> ArcCow {
        // Every slot gets its own chunk: the engine owns its post-preamble state
        // outright, so no page should pay privatisation for a shared zero template.
        let mut chunks = Vec::with_capacity(layout.chunks());
        for _ in 0..layout.chunks() {
            chunks.push(new_plain_chunk());
        }
        ArcCow { chunks }
    }

    #[inline(always)]
    fn get(&self, i: usize) -> u64 {
        self.chunks[i >> CHUNK_SHIFT][i & CHUNK_MASK]
    }

    #[inline(always)]
    fn set(&mut self, i: usize, v: u64) {
        // The barrier as the assignment specifies it: `&mut` access that checks and
        // clones. `Arc::make_mut` is the check; it is an atomic RMW on every call.
        Arc::make_mut(&mut self.chunks[i >> CHUNK_SHIFT])[i & CHUNK_MASK] = v;
    }

    fn snapshot(&mut self) -> ArcSnap {
        ArcSnap {
            chunks: self.chunks.clone(),
        }
    }

    fn restore(&mut self, snap: &ArcSnap) {
        // Re-share every chunk with the snapshot; the next write to each privatises it
        // again. This is the whole restore: no data moves.
        self.chunks.clear();
        self.chunks.extend(snap.chunks.iter().cloned());
    }

    fn words(&self) -> usize {
        self.chunks.len() * CHUNK_WORDS
    }
}

// --------------------------------------------------------------------------------
// (b2) dirty bitmap, the DESIGN §5.2 shape
// --------------------------------------------------------------------------------

type CellChunk = [Cell<u64>; CHUNK_WORDS];

fn zero_cell_chunk() -> Arc<CellChunk> {
    let mut a: Arc<std::mem::MaybeUninit<CellChunk>> = Arc::new_uninit();
    // SAFETY: a is freshly allocated and uniquely owned. `Cell<u64>` is
    // `#[repr(transparent)]` over `u64`, so writing CHUNK_WORDS zeroed u64 initialises
    // every element of [Cell<u64>; CHUNK_WORDS].
    unsafe {
        let dst = Arc::get_mut(&mut a).unwrap().as_mut_ptr() as *mut u64;
        std::ptr::write_bytes(dst, 0, CHUNK_WORDS);
        a.assume_init()
    }
}

/// A private copy of `src`: exactly one 16 KB `memcpy` into a fresh allocation. This is
/// the cost copy-on-write pays, once per chunk per checkpoint interval.
///
/// `Arc::new(src.clone())` would work — `[Cell<u64>; N]: Clone` via `Cell<T: Copy>` —
/// but it is allowed to stage the array in the caller's frame first, which would double
/// the bytes moved on the one path whose cost the benchmark is trying to attribute.
fn clone_cell_chunk(src: &CellChunk) -> Arc<CellChunk> {
    let mut a: Arc<std::mem::MaybeUninit<CellChunk>> = Arc::new_uninit();
    // SAFETY: as above, plus: src is fully initialised, `Cell<u64>` is transparent over
    // `u64`, the lengths match, and the two Arc allocations cannot overlap.
    unsafe {
        let dst = Arc::get_mut(&mut a).unwrap().as_mut_ptr() as *mut u64;
        std::ptr::copy_nonoverlapping(src.as_ptr() as *const u64, dst, CHUNK_WORDS);
        a.assume_init()
    }
}

pub struct BitmapSnap {
    chunks: Vec<Arc<CellChunk>>,
}

impl Snapshot for BitmapSnap {
    fn nominal_bytes(&self) -> usize {
        let n = self
            .chunks
            .iter()
            .filter(|c| Arc::strong_count(c) == 1)
            .count();
        n * CHUNK_WORDS * 8 + self.chunks.len() * std::mem::size_of::<usize>()
    }
}

impl BitmapSnap {
    /// Chunk versions this snapshot is the sole owner of, i.e. its marginal cost.
    pub fn unique_chunks(&self) -> usize {
        self.chunks
            .iter()
            .filter(|c| Arc::strong_count(c) == 1)
            .count()
    }
}

pub struct BitmapCow {
    chunks: Vec<Arc<CellChunk>>,
    /// One bit per chunk: set means "privatised since the last snapshot", so the live
    /// handle is the sole owner and a write may go straight through.
    private: Vec<u64>,
    /// Cumulative count of barrier slow-path entries, for the report.
    pub slow_path: u64,
}

impl BitmapCow {
    #[inline(never)]
    #[cold]
    fn privatise(&mut self, c: usize) {
        // Only really clone when somebody else still holds this version. Right after a
        // restore, or for a chunk no live snapshot references, `get_mut` succeeds and
        // the clone is skipped.
        if Arc::get_mut(&mut self.chunks[c]).is_none() {
            self.chunks[c] = clone_cell_chunk(&self.chunks[c]);
        }
        self.private[c >> 6] |= 1u64 << (c & 63);
        self.slow_path += 1;
    }

    /// Live heap held by the chunk table itself, excluding chunk contents.
    pub fn table_bytes(&self) -> usize {
        self.chunks.len() * std::mem::size_of::<usize>() + self.private.len() * 8
    }
}

impl Backend for BitmapCow {
    type Snap = BitmapSnap;

    fn name(&self) -> &'static str {
        "chunk-bitmap"
    }

    fn new(layout: &Layout) -> BitmapCow {
        let n = layout.chunks();
        let mut chunks = Vec::with_capacity(n);
        for _ in 0..n {
            chunks.push(zero_cell_chunk());
        }
        BitmapCow {
            chunks,
            // Every chunk starts private: the engine owns its post-preamble state.
            private: vec![u64::MAX; n.div_ceil(64)],
            slow_path: 0,
        }
    }

    #[inline(always)]
    fn get(&self, i: usize) -> u64 {
        self.chunks[i >> CHUNK_SHIFT][i & CHUNK_MASK].get()
    }

    #[inline(always)]
    fn set(&mut self, i: usize, v: u64) {
        let c = i >> CHUNK_SHIFT;
        // The barrier: one load from a bitmap that is 1.6 KB for a 200 MB state, one
        // test, one branch that is taken at most once per chunk per checkpoint.
        if self.private[c >> 6] & (1u64 << (c & 63)) == 0 {
            self.privatise(c);
        }
        self.chunks[c][i & CHUNK_MASK].set(v);
    }

    fn snapshot(&mut self) -> BitmapSnap {
        let snap = BitmapSnap {
            chunks: self.chunks.clone(),
        };
        // Everything is shared again; the next write to each chunk privatises it.
        self.private.iter_mut().for_each(|w| *w = 0);
        snap
    }

    fn restore(&mut self, snap: &BitmapSnap) {
        self.chunks.clear();
        self.chunks.extend(snap.chunks.iter().cloned());
        self.private.iter_mut().for_each(|w| *w = 0);
    }

    fn words(&self) -> usize {
        self.chunks.len() * CHUNK_WORDS
    }
}
