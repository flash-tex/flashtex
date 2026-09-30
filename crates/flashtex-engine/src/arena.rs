//! One flat word space for all of the engine's mutable state (DESIGN.md §4.2,
//! §5.2 mechanism (b3)).
//!
//! Every array global of the translated engine (`mem`, `eqtb`, `hash`,
//! `save_stack`, the input and nest stacks, `str_pool`/`str_start`,
//! `font_info` and the font arrays, the trie, pdfTeX's object and destination
//! tables, ...) is an [`Arr`], a view into one contiguous, zero-initialised
//! allocation. The scalar globals are spilled into a region at the start of
//! the same space at every checkpoint (see `checkpoint.rs`), so one
//! dirty map covers everything a checkpoint has to capture.
//!
//! * **Reads** are plain loads: `Index` and `Deref` go straight to the
//!   element, exactly like the `Vec` they replace.
//! * **Writes** go through `IndexMut` (or `slice_mut`), which is the write
//!   barrier: one byte test in the `saved` map (one flag per 1 KB chunk,
//!   addressed straight from the element's address) and, the first time a
//!   chunk is written after a checkpoint, a copy of its previous contents
//!   into the open undo log (a cold call).
//! * A **checkpoint** seals the open log and clears the bitmap. Sealing
//!   keeps, of each whole pre-image, only the words that differ from the
//!   chunk now (DESIGN.md §5.2's per-page deltas: a tenth of the words of a
//!   written chunk, measured on the benchmark documents).
//! * A **restore** applies the logs from the target on, each word taking
//!   the value of its oldest entry, and captures the chunks it overwrites
//!   into a redo log, so that the old run's later state and checkpoints can be
//!   re-attached (`converge`, DESIGN.md §5.3).
//!
//! This is `tools/snapshot-bench/src/undo_cow.rs`'s `UndoChain`, measured in
//! `docs/evidence/snapshot-bench-2026-09-29/`, with the two changes that
//! report asks for: chunks come from a slab, not `malloc`, and the restore is
//! parallel above a threshold.
//!
//! The space is `mmap`ed anonymous memory, so the hundreds of megabytes of
//! capacity that web2c's `texmf.cnf` sizes reserve (most of it never touched)
//! cost address space, not memory.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Deref, Index, IndexMut};

pub const CHUNK_SHIFT: usize = 10;
pub const CHUNK_BYTES: usize = 1 << CHUNK_SHIFT;
pub const CHUNK_WORDS: usize = CHUNK_BYTES / 8;

/// Bytes a scalar global of type `T` takes in the scalar region.
pub const fn slot<T>() -> usize {
    (std::mem::size_of::<T>() + 7) & !7
}

// ---------------------------------------------------------------------------
// Anonymous memory
// ---------------------------------------------------------------------------

mod os {
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
    }
    const PROT_READ: i32 = 1;
    const PROT_WRITE: i32 = 2;
    const MAP_PRIVATE: i32 = 2;
    #[cfg(target_os = "macos")]
    const MAP_ANON: i32 = 0x1000;
    #[cfg(not(target_os = "macos"))]
    const MAP_ANON: i32 = 0x20;

    /// `len` zero bytes, page aligned, committed lazily by the kernel.
    pub fn alloc(len: usize) -> *mut u8 {
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
        if p as isize == -1 || p.is_null() {
            panic!("flashtex: cannot map {len} bytes for the engine's word space");
        }
        p as *mut u8
    }

    /// # Safety
    /// `p`/`len` must come from `alloc` and not be used afterwards.
    pub unsafe fn free(p: *mut u8, len: usize) {
        munmap(p as *mut c_void, len);
    }
}

// ---------------------------------------------------------------------------
// The plan: regions reserved in declaration order
// ---------------------------------------------------------------------------

/// A region of the word space holding up to `cap` elements of `T`.
pub struct Region<T> {
    off: usize,
    cap: usize,
    _t: PhantomData<T>,
}

impl<T> Clone for Region<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Region<T> {}

/// The layout of the space: the scalar region at offset 0, then each array
/// region in the order the generated `Globals::new` reserves them.
pub struct Plan {
    scalar_bytes: usize,
    bytes: usize,
    regions: Vec<RegionInfo>,
}

/// Where an array lives in the space (for diagnostics: which arrays a run
/// writes).
#[derive(Clone, Debug)]
pub struct RegionInfo {
    pub name: &'static str,
    pub off: usize,
    pub bytes: usize,
    /// Bytes per element (1 for the scalar region).
    pub elem: usize,
}

impl Plan {
    pub fn new(scalar_bytes: usize) -> Plan {
        Plan {
            scalar_bytes,
            bytes: scalar_bytes.next_multiple_of(64),
            regions: vec![RegionInfo {
                name: "(scalars)",
                off: 0,
                bytes: scalar_bytes,
                elem: 1,
            }],
        }
    }

    /// Room for `cap` elements of `T`. A region is aligned to its element
    /// size rounded up to a power of two (at least 64 bytes, at most one
    /// chunk), so an element whose size is a power of two never straddles
    /// two chunks.
    pub fn reserve<T>(&mut self, name: &'static str, cap: usize) -> Region<T> {
        let size = std::mem::size_of::<T>();
        assert!(size > 0 && size <= CHUNK_BYTES);
        let align = size.next_power_of_two().clamp(64, CHUNK_BYTES);
        let off = self.bytes.next_multiple_of(align);
        self.bytes = off + cap * size;
        self.regions.push(RegionInfo {
            name,
            off,
            bytes: cap * size,
            elem: size,
        });
        Region {
            off,
            cap,
            _t: PhantomData,
        }
    }

    pub fn build(self) -> Arena {
        let mut a = Arena::new(self.scalar_bytes, self.bytes);
        a.regions = self.regions;
        a
    }
}

// ---------------------------------------------------------------------------
// The core: live words, bitmap, undo-log chain
// ---------------------------------------------------------------------------

pub type CheckpointId = u64;

/// A chunk's saved contents, a slot of the slab.
type ChunkPtr = *mut u64;

/// Chunks are carved out of 1 MiB blocks and recycled through a free list:
/// the snapshot benchmark measured 1.24x overhead for 16 KB `malloc`s.
struct Slab {
    blocks: Vec<*mut u8>,
    free: Vec<ChunkPtr>,
    live: usize,
}

const SLAB_BLOCK_CHUNKS: usize = (1 << 20) / CHUNK_BYTES;

impl Slab {
    fn take(&mut self) -> ChunkPtr {
        if self.free.is_empty() {
            let block = os::alloc(SLAB_BLOCK_CHUNKS * CHUNK_BYTES);
            self.blocks.push(block);
            for i in (0..SLAB_BLOCK_CHUNKS).rev() {
                // SAFETY: inside the block just mapped.
                self.free
                    .push(unsafe { block.add(i * CHUNK_BYTES) } as ChunkPtr);
            }
        }
        self.live += 1;
        self.free.pop().unwrap()
    }
    fn give(&mut self, p: ChunkPtr) {
        self.live -= 1;
        self.free.push(p);
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        for &b in &self.blocks {
            // SAFETY: each block came from os::alloc with this length.
            unsafe { os::free(b, SLAB_BLOCK_CHUNKS * CHUNK_BYTES) };
        }
    }
}

const MASK_WORDS: usize = CHUNK_WORDS / 64;

/// One chunk of a sealed log: the words that differ between the state at
/// the log's checkpoint and the state at the next, as a bit mask; their
/// values at the log's checkpoint are `Log::words[at..]`, in bit order.
#[derive(Clone, Copy)]
struct Delta {
    c: u32,
    at: u32,
    mask: [u64; MASK_WORDS],
}

impl Delta {
    fn len(&self) -> usize {
        self.mask.iter().map(|m| m.count_ones() as usize).sum()
    }

    /// Rewind a chunk by this delta where an older entry has not already:
    /// write the words it holds that are not in `done` into `dst`, and add
    /// them to `done`. Applied from the oldest log on, this leaves each word
    /// at its value at the oldest log's checkpoint.
    ///
    /// # Safety
    /// `dst` points at CHUNK_WORDS writable words.
    #[inline]
    unsafe fn apply_under(&self, words: &[u64], dst: *mut u64, done: &mut [u64; MASK_WORDS]) {
        let mut k = self.at as usize;
        for (mw, dmw) in done.iter_mut().enumerate() {
            let m = self.mask[mw];
            let mut need = m & !*dmw;
            while need != 0 {
                let b = need.trailing_zeros();
                let idx = k + (m & ((1u64 << b) - 1)).count_ones() as usize;
                *dst.add(mw * 64 + b as usize) = words[idx];
                need &= need - 1;
            }
            *dmw |= m;
            k += m.count_ones() as usize;
        }
    }
}

/// A hint to bring the chunk at address `a` into the cache (a no-op where
/// there is no prefetch instruction to hand).
#[inline(always)]
fn prefetch_chunk(a: usize) {
    for line in (0..CHUNK_BYTES).step_by(64) {
        #[cfg(target_arch = "aarch64")]
        // SAFETY: a prefetch never faults and has no other effect.
        unsafe {
            std::arch::asm!("prfm pldl1keep, [{0}]", in(reg) a + line, options(nostack, readonly, preserves_flags));
        }
        #[cfg(target_arch = "x86_64")]
        // SAFETY: as above.
        unsafe {
            std::arch::x86_64::_mm_prefetch::<{ std::arch::x86_64::_MM_HINT_T0 }>(
                (a + line) as *const i8,
            );
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        let _ = a + line;
    }
}

/// `Delta::apply_under` for a whole pre-image `src`.
///
/// # Safety
/// `dst` and `src` point at CHUNK_WORDS words each.
#[inline]
unsafe fn apply_whole_under(src: *const u64, dst: *mut u64, done: &mut [u64; MASK_WORDS]) {
    for (mw, d) in done.iter_mut().enumerate() {
        let mut need = !*d;
        while need != 0 {
            let b = mw * 64 + need.trailing_zeros() as usize;
            *dst.add(b) = *src.add(b);
            need &= need - 1;
        }
        *d = u64::MAX;
    }
}

/// The undo log of one checkpoint: how to get the state at the checkpoint
/// back from the state after it.
///
/// While the checkpoint is the newest the log is *open*: the barrier puts
/// each chunk's whole pre-image in `entries` the first time it is written.
/// The next checkpoint *seals* it: each pre-image becomes a [`Delta`]
/// against the chunk then (DESIGN.md §5.2's per-page deltas: most words of
/// a written chunk are unchanged at the next checkpoint, so a sealed log
/// holds a fraction of the bytes), sorted by chunk. A log is open (only
/// `entries`) or sealed (only `deltas`), never both.
#[derive(Default)]
struct Log {
    entries: Vec<(u32, ChunkPtr)>,
    deltas: Vec<Delta>,
    words: Vec<u64>,
}

impl Log {
    /// The chunks the log holds.
    fn chunk_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries
            .iter()
            .map(|e| e.0)
            .chain(self.deltas.iter().map(|d| d.c))
    }

    /// Heap bytes of the sealed part.
    fn sealed_bytes(&self) -> usize {
        self.deltas.capacity() * std::mem::size_of::<Delta>() + self.words.capacity() * 8
    }
}

/// Copies of chunks `cs` (sorted, distinct), each starting from `start(c)`
/// and rewound through `logs`: their values at the first log's checkpoint.
/// CHUNK_WORDS words per chunk, in `cs` order. (Oldest log first, each word
/// written once: see `Core::rewind`.)
fn rewound(
    nchunks: usize,
    cs: &[u32],
    start: &dyn Fn(u32) -> *const u64,
    logs: &[Log],
) -> Vec<u64> {
    rewound_until(nchunks, cs, start, logs, &mut || false).expect("never stopped")
}

/// [`rewound`], asking `stop` every [`STOP_LOGS`] logs (the convergence
/// test rewinds through the whole old future: 3,400 logs, 2.6 M entries,
/// ~17 ms on a 1,072-page document); `None` when it said to stop.
fn rewound_until(
    nchunks: usize,
    cs: &[u32],
    start: &dyn Fn(u32) -> *const u64,
    logs: &[Log],
    stop: &mut dyn FnMut() -> bool,
) -> Option<Vec<u64>> {
    let mut buf = vec![0u64; cs.len() * CHUNK_WORDS];
    for (i, &c) in cs.iter().enumerate() {
        // SAFETY: `start` gives a whole chunk.
        let src = unsafe { std::slice::from_raw_parts(start(c), CHUNK_WORDS) };
        buf[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS].copy_from_slice(src);
    }
    let mut want = vec![0u64; nchunks.div_ceil(64)];
    for &c in cs {
        set_bit(&mut want, c as usize);
    }
    let mut done = vec![[0u64; MASK_WORDS]; cs.len()];
    for (k, log) in logs.iter().enumerate() {
        if k % STOP_LOGS == STOP_LOGS - 1 && stop() {
            return None;
        }
        for &(c, p) in &log.entries {
            if bit(&want, c as usize) {
                let i = cs.binary_search(&c).unwrap();
                // SAFETY: a slab chunk; a chunk of `buf`.
                unsafe {
                    apply_whole_under(p, buf.as_mut_ptr().add(i * CHUNK_WORDS), &mut done[i])
                };
            }
        }
        for d in &log.deltas {
            if bit(&want, d.c as usize) {
                let i = cs.binary_search(&d.c).unwrap();
                // SAFETY: a chunk of `buf`.
                unsafe {
                    d.apply_under(
                        &log.words,
                        buf.as_mut_ptr().add(i * CHUNK_WORDS),
                        &mut done[i],
                    )
                };
            }
        }
    }
    Some(buf)
}

/// Logs between two questions to `rewound_until`'s `stop` (about 0.3 ms).
const STOP_LOGS: usize = 64;

/// `older` then `newer`, two adjacent sealed logs, as one: the state at
/// `older`'s checkpoint from the state after `newer`'s. Where both hold a
/// word, `older`'s value wins.
fn merge_sealed(older: &Log, newer: &Log) -> Log {
    let mut deltas = Vec::with_capacity(older.deltas.len() + newer.deltas.len());
    let mut words = Vec::with_capacity(older.words.len() + newer.words.len());
    let copy = |d: &Delta, from: &[u64], words: &mut Vec<u64>| -> Delta {
        let at = words.len();
        words.extend_from_slice(&from[d.at as usize..d.at as usize + d.len()]);
        Delta {
            c: d.c,
            at: at as u32,
            mask: d.mask,
        }
    };
    let (mut i, mut j) = (0, 0);
    let (a, b) = (&older.deltas, &newer.deltas);
    while i < a.len() || j < b.len() {
        if j == b.len() || (i < a.len() && a[i].c < b[j].c) {
            deltas.push(copy(&a[i], &older.words, &mut words));
            i += 1;
        } else if i == a.len() || b[j].c < a[i].c {
            deltas.push(copy(&b[j], &newer.words, &mut words));
            j += 1;
        } else {
            let (x, y) = (&a[i], &b[j]);
            let at = words.len();
            let (mut kx, mut ky) = (x.at as usize, y.at as usize);
            let mut mask = [0u64; MASK_WORDS];
            for (mw, m) in mask.iter_mut().enumerate() {
                let (mx, my) = (x.mask[mw], y.mask[mw]);
                *m = mx | my;
                for bpos in 0..64 {
                    let bitv = 1u64 << bpos;
                    let (in_x, in_y) = (mx & bitv != 0, my & bitv != 0);
                    if in_x {
                        words.push(older.words[kx]);
                        kx += 1;
                        if in_y {
                            ky += 1;
                        }
                    } else if in_y {
                        words.push(newer.words[ky]);
                        ky += 1;
                    }
                }
            }
            deltas.push(Delta {
                c: x.c,
                at: at as u32,
                mask,
            });
            i += 1;
            j += 1;
        }
    }
    deltas.shrink_to_fit();
    words.shrink_to_fit();
    Log {
        entries: Vec::new(),
        deltas,
        words,
    }
}

/// The old run's future, detached by a branching restore until the new run
/// converges (`converge`) or is abandoned (`drop_branch`).
pub struct Branch {
    /// Old checkpoints from the restore target (first) to the old newest.
    ids: Vec<CheckpointId>,
    logs: Vec<Log>,
    /// The live contents, at the moment of the restore, of every chunk the
    /// restore overwrote: the old run's latest state.
    redo: Vec<(u32, ChunkPtr)>,
}

// SAFETY: a branch owns its slab chunks; nothing else points at them.
unsafe impl Send for Branch {}

impl Branch {
    pub fn ids(&self) -> &[CheckpointId] {
        &self.ids
    }
    pub fn redo_chunks(&self) -> usize {
        self.redo.len()
    }
}

pub(crate) struct Core {
    /// The mapping as `mmap` returned it, and its length.
    map: *mut u8,
    map_len: usize,
    /// The space: `map` rounded up to a chunk boundary.
    base: *mut u8,
    bytes: usize,
    nchunks: usize,
    /// One flag byte per chunk: nonzero means its pre-image is already in
    /// the open log (or there is no checkpoint yet), so writes may go
    /// straight through. A byte rather than a bit, so that the barrier is a
    /// single load: see `Arr::touch`.
    saved: *mut u8,
    /// One byte per chunk: ever written. A chunk never written is still
    /// all zero, so hashing and persisting the space skip it.
    touched: Vec<u8>,
    ids: Vec<CheckpointId>,
    logs: Vec<Log>,
    next_id: CheckpointId,
    mark: Vec<u64>,
    /// Scratch for `rewind`: a chunk's index among the chunks it restores.
    slot: Vec<u32>,
    slab: Slab,
    /// Chunks copied by the barrier since the space was made.
    pub slow_path: u64,
    /// Workers for deep restores (0 = one per available core, up to 8).
    pub threads: usize,
    /// Heap bytes of every sealed log, the core's and detached branches'.
    sealed_bytes: usize,
    /// A restore worked out ahead of time (`prepare_restore`).
    prepared: Option<Prepared>,
    /// Bumped by every change to the logs' contents that keeps the
    /// checkpoint list (`or_from`): a `Prepared` from before it is stale.
    history_gen: u64,
}

/// The state at checkpoint `id` of the chunks the logs from `id` on hold,
/// worked out while the engine was idle: a later `restore_branch(id)` of
/// the same history copies these in instead of rewinding the logs (on a
/// 1,000-page document, thousands of logs and millions of entries after an
/// early page). Valid while the checkpoint list is `ids` and no log was
/// changed in place (`history_gen`); the open log may have grown since
/// (its new chunks had no history after `id`: their pre-images are their
/// values there).
struct Prepared {
    id: CheckpointId,
    ids: Vec<CheckpointId>,
    history_gen: u64,
    cs: Vec<u32>,
    buf: Vec<u64>,
}

#[inline(always)]
fn bit(bits: &[u64], c: usize) -> bool {
    bits[c >> 6] & (1u64 << (c & 63)) != 0
}
#[inline(always)]
fn set_bit(bits: &mut [u64], c: usize) {
    bits[c >> 6] |= 1u64 << (c & 63);
}

impl Core {
    fn saved(&mut self) -> &mut [u8] {
        // SAFETY: `saved` is an allocation of `nchunks` bytes owned by the core.
        unsafe { std::slice::from_raw_parts_mut(self.saved, self.nchunks) }
    }

    fn chunk_ptr(&self, c: usize) -> *mut u64 {
        debug_assert!(c < self.nchunks);
        // SAFETY: c < nchunks, so the chunk lies inside the mapping.
        unsafe { self.base.add(c << CHUNK_SHIFT) as *mut u64 }
    }

    #[inline(never)]
    #[cold]
    fn save(&mut self, c: usize) {
        self.touched[c] = 1;
        self.saved()[c] = 1;
        if self.logs.is_empty() {
            // No checkpoint yet: the chunk is only noted as written.
            return;
        }
        let dst = self.slab.take();
        // SAFETY: both are CHUNK_WORDS-word ranges of distinct allocations.
        unsafe { std::ptr::copy_nonoverlapping(self.chunk_ptr(c), dst, CHUNK_WORDS) };
        self.logs.last_mut().unwrap().entries.push((c as u32, dst));
        self.slow_path += 1;
    }

    fn clear_saved(&mut self) {
        self.saved().fill(0);
    }

    fn checkpoint(&mut self) -> CheckpointId {
        if let Some(i) = self.logs.len().checked_sub(1) {
            self.seal(i);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.ids.push(id);
        self.logs.push(Log::default());
        self.clear_saved();
        id
    }

    /// Seal open log `i` against the live space, which is the state at the
    /// next checkpoint: each whole pre-image becomes the words that differ.
    fn seal(&mut self, i: usize) {
        let mut entries = std::mem::take(&mut self.logs[i].entries);
        if entries.is_empty() {
            return;
        }
        debug_assert!(self.logs[i].deltas.is_empty(), "sealing a sealed log");
        entries.sort_unstable_by_key(|e| e.0);
        let mut deltas = Vec::with_capacity(entries.len());
        let mut words = Vec::with_capacity(entries.len() * (CHUNK_WORDS / 8));
        // The two chunks of each entry are mostly not in the cache any more
        // (the page wrote them long ago): ask for the next ones early. On the
        // 953-page benchmark this cut the seal from 0.31 to 0.20 ms a page.
        let base = self.base as usize;
        let prefetch = |k: usize| {
            if let Some(&(c, p)) = entries.get(k) {
                prefetch_chunk(p as usize);
                prefetch_chunk(base + ((c as usize) << CHUNK_SHIFT));
            }
        };
        prefetch(0);
        prefetch(1);
        for (k, &(c, p)) in entries.iter().enumerate() {
            prefetch(k + 2);
            // SAFETY: a slab chunk and a live chunk, CHUNK_WORDS words each.
            let (old, now) = unsafe {
                (
                    std::slice::from_raw_parts(p as *const u64, CHUNK_WORDS),
                    std::slice::from_raw_parts(
                        self.chunk_ptr(c as usize) as *const u64,
                        CHUNK_WORDS,
                    ),
                )
            };
            // Blocks of 8 words: most are unchanged, and the test for that
            // vectorises; only the others are compared word by word.
            let mut mask = [0u64; MASK_WORDS];
            for blk in 0..CHUNK_WORDS / 8 {
                let (o, n) = (&old[blk * 8..blk * 8 + 8], &now[blk * 8..blk * 8 + 8]);
                let mut acc = 0u64;
                for k in 0..8 {
                    acc |= o[k] ^ n[k];
                }
                if acc != 0 {
                    let mut bits = 0u64;
                    for k in 0..8 {
                        bits |= ((o[k] != n[k]) as u64) << k;
                    }
                    mask[blk / 8] |= bits << ((blk % 8) * 8);
                }
            }
            if mask.iter().any(|&m| m != 0) {
                let at = words.len();
                for (mw, &m0) in mask.iter().enumerate() {
                    let mut m = m0;
                    while m != 0 {
                        words.push(old[mw * 64 + m.trailing_zeros() as usize]);
                        m &= m - 1;
                    }
                }
                deltas.push(Delta {
                    c,
                    at: at as u32,
                    mask,
                });
            }
            self.slab.give(p);
        }
        deltas.shrink_to_fit();
        words.shrink_to_fit();
        let log = &mut self.logs[i];
        log.deltas = deltas;
        log.words = words;
        self.sealed_bytes += log.sealed_bytes();
    }

    fn index_of(&self, id: CheckpointId) -> Option<usize> {
        self.ids.iter().rposition(|&x| x == id)
    }

    fn workers(&self) -> usize {
        if self.threads > 0 {
            self.threads
        } else {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .min(8)
        }
    }

    /// Put the state at the first of `logs`' checkpoints into the live
    /// space. With `capture`, first copy every chunk that changes into a
    /// fresh slab chunk (the redo log: the state being left).
    ///
    /// A word's value at the first checkpoint is the one its oldest entry
    /// in `logs` holds (a whole pre-image or a delta), else the live one.
    /// So the logs are applied from the oldest on, each word written once
    /// and every later entry for it skipped: a restore across hundreds of
    /// pages that keep rewriting the same two thousand chunks writes each
    /// word once instead of once per page.
    fn rewind(&mut self, logs: &[Log], capture: bool) -> Vec<(u32, ChunkPtr)> {
        let mut mark = std::mem::take(&mut self.mark);
        mark.fill(0);
        let mut cs: Vec<u32> = Vec::new();
        let mut entries = 0usize;
        for log in logs {
            entries += log.entries.len() + log.deltas.len();
            for c in log.chunk_ids() {
                if !bit(&mark, c as usize) {
                    set_bit(&mut mark, c as usize);
                    cs.push(c);
                }
            }
        }
        self.mark = mark;
        cs.sort_unstable();
        let mut slot = std::mem::take(&mut self.slot);
        for (i, &c) in cs.iter().enumerate() {
            slot[c as usize] = i as u32;
        }
        let redo: Vec<(u32, ChunkPtr)> = if capture {
            cs.iter().map(|&c| (c, self.slab.take())).collect()
        } else {
            Vec::new()
        };
        let base = self.base as usize;
        let redo_at: Vec<usize> = redo.iter().map(|&(_, p)| p as usize).collect();
        let n = cs.len();
        // Contiguous chunk ranges, one per worker, each with its own words
        // done so far.
        let workers = if entries < PARALLEL_MIN && n < PARALLEL_MIN {
            1
        } else {
            self.workers().max(1)
        };
        let mut done = vec![[0u64; MASK_WORDS]; n];
        let mut parts: Vec<&mut [[u64; MASK_WORDS]]> = Vec::with_capacity(workers);
        {
            let mut rest: &mut [[u64; MASK_WORDS]] = &mut done;
            for w in 0..workers {
                let len = (w + 1) * n / workers - w * n / workers;
                let (a, b) = rest.split_at_mut(len);
                parts.push(a);
                rest = b;
            }
        }
        /// The logs, shared read-only by the workers.
        struct Shared<'a>(&'a [Log]);
        // SAFETY: nobody writes the logs (or the slab chunks their entries
        // point at) during a restore.
        unsafe impl Sync for Shared<'_> {}
        impl<'a> Shared<'a> {
            fn get(&self) -> &'a [Log] {
                self.0
            }
        }
        let shared = Shared(logs);
        let (cs, slot_r) = (&cs, &slot);
        let job = |w: usize, done: &mut [[u64; MASK_WORDS]]| {
            let logs = shared.get();
            let (i0, i1) = (w * n / workers, (w + 1) * n / workers);
            if i0 == i1 {
                return;
            }
            let lo = cs[i0];
            let hi = if i1 == n { u32::MAX } else { cs[i1] };
            if capture {
                for i in i0..i1 {
                    let live = base + ((cs[i] as usize) << CHUNK_SHIFT);
                    // SAFETY: a live chunk and its own redo chunk.
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            live as *const u64,
                            redo_at[i] as *mut u64,
                            CHUNK_WORDS,
                        )
                    };
                }
            }
            for log in logs {
                for &(c, p) in &log.entries {
                    if c < lo || c >= hi {
                        continue;
                    }
                    let dn = &mut done[slot_r[c as usize] as usize - i0];
                    let live = (base + ((c as usize) << CHUNK_SHIFT)) as *mut u64;
                    // SAFETY: this worker alone writes chunks in [lo, hi);
                    // `p` is a whole slab chunk.
                    unsafe { apply_whole_under(p, live, dn) };
                }
                let from = log.deltas.partition_point(|d| d.c < lo);
                for d in &log.deltas[from..] {
                    if d.c >= hi {
                        break;
                    }
                    let dn = &mut done[slot_r[d.c as usize] as usize - i0];
                    let live = (base + ((d.c as usize) << CHUNK_SHIFT)) as *mut u64;
                    // SAFETY: as above.
                    unsafe { d.apply_under(&log.words, live, dn) };
                }
            }
        };
        if workers == 1 {
            job(0, parts.pop().unwrap());
        } else {
            std::thread::scope(|s| {
                for (w, part) in parts.into_iter().enumerate() {
                    let job = &job;
                    s.spawn(move || job(w, part));
                }
            });
        }
        self.slot = slot;
        redo
    }

    /// Work out `restore_branch(id)`'s rewind ahead of time
    /// (`Prepared`), asking `stop` as it goes: false if it stopped or `id`
    /// is not in the live chain.
    fn prepare_restore(&mut self, id: CheckpointId, stop: &mut dyn FnMut() -> bool) -> bool {
        self.prepared = None;
        let Some(k) = self.index_of(id) else {
            return false;
        };
        let mut mark = std::mem::take(&mut self.mark);
        mark.fill(0);
        let mut cs: Vec<u32> = Vec::new();
        for log in &self.logs[k..] {
            for c in log.chunk_ids() {
                if !bit(&mark, c as usize) {
                    set_bit(&mut mark, c as usize);
                    cs.push(c);
                }
            }
        }
        self.mark = mark;
        cs.sort_unstable();
        let base = self.base as usize;
        let live = |c: u32| (base + ((c as usize) << CHUNK_SHIFT)) as *const u64;
        let Some(buf) = rewound_until(self.nchunks, &cs, &live, &self.logs[k..], stop) else {
            return false;
        };
        self.prepared = Some(Prepared {
            id,
            ids: self.ids.clone(),
            history_gen: self.history_gen,
            cs,
            buf,
        });
        true
    }

    /// `rewind(logs from p.id on, true)` from a `Prepared`: save the live
    /// chunks it will change (the redo), copy in the prepared ones and, for
    /// chunks the open log `open` took since, their pre-images there.
    fn rewind_prepared(&mut self, p: &Prepared, open: Option<&Log>) -> Vec<(u32, ChunkPtr)> {
        let base = self.base as usize;
        let mut extra: Vec<(u32, ChunkPtr)> = vec![];
        if let Some(o) = open {
            for &(c, q) in &o.entries {
                if p.cs.binary_search(&c).is_err() {
                    extra.push((c, q));
                }
            }
        }
        let mut redo: Vec<(u32, ChunkPtr)> = Vec::with_capacity(p.cs.len() + extra.len());
        for &c in p.cs.iter().chain(extra.iter().map(|(c, _)| c)) {
            redo.push((c, self.slab.take()));
        }
        let work: Vec<(usize, usize, usize)> = redo
            .iter()
            .enumerate()
            .map(|(i, &(c, r))| {
                let src = if i < p.cs.len() {
                    p.buf[i * CHUNK_WORDS..].as_ptr() as usize
                } else {
                    extra[i - p.cs.len()].1 as usize
                };
                (base + ((c as usize) << CHUNK_SHIFT), r as usize, src)
            })
            .collect();
        let job = |k: usize| {
            let (live, r, src) = work[k];
            // SAFETY: distinct live chunks, their own redo chunks, and
            // sources nobody writes meanwhile (the prepared buffer, the open
            // log's slab chunks).
            unsafe {
                std::ptr::copy_nonoverlapping(live as *const u64, r as *mut u64, CHUNK_WORDS);
                std::ptr::copy_nonoverlapping(src as *const u64, live as *mut u64, CHUNK_WORDS);
            }
        };
        run_jobs(work.len(), self.workers(), &job);
        redo
    }

    /// Copy whole chunks into the live space.
    fn copy_in(&mut self, chunks: &[(u32, ChunkPtr)]) {
        let base = self.base as usize;
        let work: Vec<(usize, usize)> = chunks
            .iter()
            .map(|&(c, p)| (base + ((c as usize) << CHUNK_SHIFT), p as usize))
            .collect();
        let job = |k: usize| {
            let (live, src) = work[k];
            // SAFETY: distinct live chunks; sources are slab chunks nobody
            // writes meanwhile.
            unsafe {
                std::ptr::copy_nonoverlapping(src as *const u64, live as *mut u64, CHUNK_WORDS)
            };
        };
        run_jobs(work.len(), self.workers(), &job);
    }

    fn free_log(&mut self, log: Log) {
        self.sealed_bytes -= log.sealed_bytes();
        for (_, p) in log.entries {
            self.slab.give(p);
        }
    }

    /// Restore to `id` and discard every later checkpoint (the plain restart).
    fn restore_discard(&mut self, id: CheckpointId) -> Result<(), String> {
        let k = self
            .index_of(id)
            .ok_or_else(|| format!("checkpoint {id} is not retained"))?;
        let later = self.logs.split_off(k);
        self.ids.truncate(k + 1);
        self.rewind(&later, false);
        for log in later {
            self.free_log(log);
        }
        self.logs.push(Log::default());
        self.clear_saved();
        Ok(())
    }

    /// Restore to `id`, keeping the old run's later checkpoints and a redo
    /// log so that `converge` can jump back to the old run's latest state.
    fn restore_branch(&mut self, id: CheckpointId) -> Result<Branch, String> {
        let k = self
            .index_of(id)
            .ok_or_else(|| format!("checkpoint {id} is not retained"))?;
        let prepared = self.prepared.take();
        if std::env::var_os("FLASHTEX_INCR_DEBUG").is_some() {
            match &prepared {
                Some(p) => eprintln!(
                    "[arena] prepared for {} (gen {} vs {}, same ids {}: {} vs {})",
                    p.id,
                    p.history_gen,
                    self.history_gen,
                    p.ids == self.ids,
                    p.ids.len(),
                    self.ids.len()
                ),
                None => eprintln!("[arena] nothing prepared"),
            }
        }
        let prepared = prepared
            .filter(|p| p.id == id && p.history_gen == self.history_gen && p.ids == self.ids);
        let old_logs = self.logs.split_off(k);
        let old_ids = self.ids.split_off(k);
        if std::env::var_os("FLASHTEX_INCR_DEBUG").is_some() {
            eprintln!(
                "[arena] restore to {id}: {} ({} logs)",
                match &prepared {
                    Some(p) => format!("prepared, {} chunks", p.cs.len()),
                    None => "rewound".into(),
                },
                self.logs.len() - k
            );
        }
        let redo = match prepared {
            Some(p) => self.rewind_prepared(&p, old_logs.last()),
            None => self.rewind(&old_logs, true),
        };
        self.ids.push(old_ids[0]);
        self.logs.push(Log::default());
        self.clear_saved();
        Ok(Branch {
            ids: old_ids,
            logs: old_logs,
            redo,
        })
    }

    /// The convergence jump (DESIGN.md §5.3): the live state equals the old
    /// run's state at `old`, and the newest checkpoint was taken there with
    /// nothing written since. Jump to the old run's latest state and keep its
    /// checkpoints from `old` on.
    fn converge(&mut self, branch: Branch, old: CheckpointId) -> Result<(), String> {
        let Some(j) = branch.ids.iter().position(|&x| x == old) else {
            let b = branch;
            self.drop_branch(b);
            return Err(format!("checkpoint {old} is not in the detached branch"));
        };
        if !self.logs.last().is_some_and(|l| l.entries.is_empty()) {
            self.drop_branch(branch);
            return Err(
                "take a checkpoint at the convergence point, and write nothing, before converging"
                    .into(),
            );
        }
        self.ids.pop();
        if let Some(l) = self.logs.pop() {
            self.free_log(l);
        }
        let Branch {
            mut ids,
            mut logs,
            redo,
        } = branch;
        let keep_ids = ids.split_off(j);
        let keep_logs = logs.split_off(j);
        for log in logs {
            self.free_log(log);
        }
        // Only chunks the old run wrote after `old` differ between old@old
        // and the old latest state.
        let mut mark = std::mem::take(&mut self.mark);
        mark.fill(0);
        for log in &keep_logs {
            for c in log.chunk_ids() {
                set_bit(&mut mark, c as usize);
            }
        }
        let mut jobs = Vec::new();
        let mut unused = Vec::new();
        for (c, p) in redo {
            if bit(&mark, c as usize) {
                jobs.push((c, p));
            } else {
                unused.push(p);
            }
        }
        self.mark = mark;
        self.copy_in(&jobs);
        for (_, p) in jobs {
            self.slab.give(p);
        }
        for p in unused {
            self.slab.give(p);
        }
        self.ids.extend(keep_ids);
        self.logs.extend(keep_logs);
        self.clear_saved();
        let open: Vec<usize> = self
            .logs
            .last()
            .map(|l| l.entries.iter().map(|&(c, _)| c as usize).collect())
            .unwrap_or_default();
        let saved = self.saved();
        for c in open {
            saved[c] = 1;
        }
        Ok(())
    }

    /// Abandon the run since `restore_branch` detached `branch`: rewind
    /// the new run's writes back to the restore target, then take the old
    /// run's latest state (the redo log) and its checkpoints back -- as if
    /// the restore had not happened.
    fn reattach(&mut self, branch: Branch) -> Result<(), String> {
        let target = *branch.ids.first().ok_or("empty branch")?;
        let Some(k) = self.index_of(target) else {
            self.drop_branch(branch);
            return Err(format!("restore target {target} is not in the live chain"));
        };
        let later = self.logs.split_off(k);
        self.ids.truncate(k);
        self.rewind(&later, false);
        for log in later {
            self.free_log(log);
        }
        let Branch { ids, logs, redo } = branch;
        self.copy_in(&redo);
        for (_, p) in redo {
            self.slab.give(p);
        }
        self.ids.extend(ids);
        self.logs.extend(logs);
        self.clear_saved();
        let open: Vec<usize> = self
            .logs
            .last()
            .map(|l| l.entries.iter().map(|&(c, _)| c as usize).collect())
            .unwrap_or_default();
        let saved = self.saved();
        for c in open {
            saved[c] = 1;
        }
        Ok(())
    }

    fn drop_branch(&mut self, b: Branch) {
        for log in b.logs {
            self.free_log(log);
        }
        for (_, p) in b.redo {
            self.slab.give(p);
        }
    }

    /// Drop every checkpoint `keep` rejects, except the newest, merging each
    /// dropped log into its predecessor (the older value of a word wins).
    fn retain(&mut self, keep: &dyn Fn(CheckpointId) -> bool) {
        let n = self.ids.len();
        let ids = std::mem::take(&mut self.ids);
        let logs = std::mem::take(&mut self.logs);
        let mut out_ids = Vec::with_capacity(n);
        let mut out_logs: Vec<Log> = Vec::with_capacity(n);
        for (i, (id, log)) in ids.into_iter().zip(logs).enumerate() {
            let sealed = |l: &Log| l.entries.is_empty();
            if i + 1 == n || keep(id) {
                out_ids.push(id);
                out_logs.push(log);
            } else if let Some(dst) = out_logs.last_mut() {
                if !sealed(dst) || !sealed(&log) {
                    // (not reached: only the newest log is open)
                    out_ids.push(id);
                    out_logs.push(log);
                    continue;
                }
                let merged = merge_sealed(dst, &log);
                self.sealed_bytes += merged.sealed_bytes();
                self.sealed_bytes -= dst.sealed_bytes() + log.sealed_bytes();
                *dst = merged;
            } else {
                // the oldest checkpoint goes: its log with it
                self.free_log(log);
            }
        }
        self.ids = out_ids;
        self.logs = out_logs;
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        // SAFETY: map/map_len and saved/nchunks were allocated in Arena::new.
        unsafe {
            os::free(self.map, self.map_len);
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                self.saved,
                self.nchunks,
            )));
        }
    }
}

/// Chunks below which a restore runs on one thread.
const PARALLEL_MIN: usize = (4 << 20) / CHUNK_BYTES;

/// Run `job(0..n)` on up to `workers` scoped threads. A restore is bound by
/// memory bandwidth and independent per chunk; below the threshold spawning
/// costs more than it saves.
fn run_jobs(n: usize, workers: usize, job: &(dyn Fn(usize) + Sync)) {
    if workers <= 1 || n < PARALLEL_MIN {
        (0..n).for_each(job);
        return;
    }
    let per = n.div_ceil(workers);
    std::thread::scope(|s| {
        for w in 0..workers {
            let lo = w * per;
            let hi = ((w + 1) * per).min(n);
            if lo < hi {
                s.spawn(move || (lo..hi).for_each(job));
            }
        }
    });
}

// ---------------------------------------------------------------------------
// The arena: owner of the core, and what the generated code sees
// ---------------------------------------------------------------------------

/// Owns the word space. Lives in `Globals::arena`; every [`Arr`] points into
/// it, so it must outlive them (it does: they are fields of the same
/// struct, and the space never moves).
pub struct Arena {
    core: *mut Core,
    scalar_bytes: usize,
    /// Bookkeeping of the checkpoint layer (`checkpoint.rs`), which the
    /// trip-test build does not have. Owned by this engine alone, like the
    /// space (see the `Send` below).
    pub extra: Option<Box<dyn std::any::Any>>,
    /// The regions, in address order.
    pub regions: Vec<RegionInfo>,
}

// SAFETY: the arena and its arrays are owned by one `Globals`, which is
// moved between threads as a whole; nothing is shared.
unsafe impl Send for Arena {}

impl Drop for Arena {
    fn drop(&mut self) {
        // SAFETY: `core` came from Box::into_raw in `new`.
        unsafe { drop(Box::from_raw(self.core)) };
    }
}

impl Arena {
    fn new(scalar_bytes: usize, bytes: usize) -> Arena {
        let bytes = bytes.max(CHUNK_BYTES).next_multiple_of(CHUNK_BYTES);
        let nchunks = bytes >> CHUNK_SHIFT;
        let nwords = nchunks.div_ceil(64);
        // Until a checkpoint exists the first write to a chunk only notes it
        // in `touched` (Core::save).
        let saved = Box::into_raw(vec![0u8; nchunks].into_boxed_slice()) as *mut u8;
        // The space starts on a chunk boundary, so that an element's chunk is
        // its address shifted (Arr::touch); a 4 KB-page kernel may need the
        // extra chunk to get there.
        let map_len = bytes + CHUNK_BYTES;
        let map = os::alloc(map_len);
        let base = (map as usize).next_multiple_of(CHUNK_BYTES) as *mut u8;
        let core = Box::new(Core {
            map,
            map_len,
            base,
            bytes,
            nchunks,
            saved,
            touched: vec![0; nchunks],
            ids: Vec::new(),
            logs: Vec::new(),
            next_id: 0,
            mark: vec![0; nwords],
            slot: vec![0; nchunks],
            slab: Slab {
                blocks: Vec::new(),
                free: Vec::new(),
                live: 0,
            },
            slow_path: 0,
            threads: 0,
            sealed_bytes: 0,
            prepared: None,
            history_gen: 0,
        });
        Arena {
            core: Box::into_raw(core),
            scalar_bytes,
            extra: None,
            regions: vec![],
        }
    }

    /// The array over `r`, `len` elements long (at most `r`'s capacity).
    pub fn arr<T>(&self, r: Region<T>, len: usize) -> Arr<T> {
        assert!(len <= r.cap);
        // SAFETY: `core` is valid for the arena's lifetime.
        let core = unsafe { &*self.core };
        Arr {
            // SAFETY: the region lies inside the mapping (Plan::reserve).
            ptr: unsafe { core.base.add(r.off) } as *mut T,
            len,
            cap: r.cap,
            flags: core.saved.wrapping_sub(core.base as usize >> CHUNK_SHIFT),
            core: self.core,
            #[cfg(feature = "bench-count-writes")]
            base_chunk: core.base as usize >> CHUNK_SHIFT,
            #[cfg(feature = "bench-count-writes")]
            last: std::cell::Cell::new(usize::MAX),
        }
    }

    fn core(&self) -> &Core {
        // SAFETY: valid for the arena's lifetime; no `&mut Core` is live
        // while `&self` is.
        unsafe { &*self.core }
    }

    fn core_mut(&mut self) -> &mut Core {
        // SAFETY: as above, and `&mut self` excludes other access.
        unsafe { &mut *self.core }
    }

    /// The whole space, for hashing, persisting and comparing.
    pub fn bytes(&self) -> &[u8] {
        let c = self.core();
        // SAFETY: the mapping is `bytes` long and initialised (zero-filled).
        unsafe { std::slice::from_raw_parts(c.base, c.bytes) }
    }

    pub fn len_bytes(&self) -> usize {
        self.core().bytes
    }

    pub fn chunks(&self) -> usize {
        self.core().nchunks
    }

    pub fn scalar_bytes(&self) -> usize {
        self.scalar_bytes
    }

    /// Chunk `c` of the live space.
    pub fn chunk(&self, c: usize) -> &[u64] {
        assert!(c < self.chunks());
        // SAFETY: inside the mapping.
        unsafe { std::slice::from_raw_parts(self.core().chunk_ptr(c), CHUNK_WORDS) }
    }

    /// A hash of the whole space: every chunk that is not all zero, with its
    /// index. Equal spaces hash equal whatever their write histories.
    pub fn hash_nonzero(&self) -> [u64; 2] {
        let mut acc: Vec<u8> = Vec::with_capacity(4096);
        for c in 0..self.chunks() {
            if !self.touched(c) {
                continue;
            }
            let w = self.chunk(c);
            if w.iter().all(|&x| x == 0) {
                continue;
            }
            // SAFETY: a chunk viewed as bytes.
            let b = unsafe { std::slice::from_raw_parts(w.as_ptr() as *const u8, CHUNK_BYTES) };
            let h = crate::persist::hash128(b);
            acc.extend_from_slice(&(c as u64).to_le_bytes());
            acc.extend_from_slice(&h[0].to_le_bytes());
            acc.extend_from_slice(&h[1].to_le_bytes());
        }
        crate::persist::hash128(&acc)
    }

    /// Whether chunk `c` was ever written (else it is all zero).
    pub fn touched(&self, c: usize) -> bool {
        self.core().touched[c] != 0
    }

    /// Overwrite chunk `c` without the barrier: only for loading a persisted
    /// space into a fresh arena that has no checkpoints.
    pub fn load_chunk(&mut self, c: usize, data: &[u8]) {
        assert!(c < self.chunks() && data.len() == CHUNK_BYTES);
        assert!(self.core().ids.is_empty(), "load_chunk under a checkpoint");
        let core = self.core_mut();
        core.touched[c] = 1;
        core.saved()[c] = 1;
        // SAFETY: inside the mapping; data is CHUNK_BYTES long.
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.core().chunk_ptr(c) as *mut u8,
                CHUNK_BYTES,
            )
        };
    }

    /// Add `bits` to the 8-byte word at `off` in the state at checkpoint
    /// `from` and at every later checkpoint of the live chain, and in the
    /// live state: as if the word had always held them from `from` on. The
    /// logs from `from` on hold the word's earlier values (whole chunks or
    /// sealed deltas), so every later rewind or restore sees the bits; the
    /// live word is written without the barrier (the open log's copy is
    /// changed like the others). Returns how many copies were changed.
    pub fn or_from(&mut self, from: CheckpointId, off: usize, bits: u64) -> Result<usize, String> {
        let core = self.core_mut();
        if !off.is_multiple_of(8) || off + 8 > core.bytes {
            return Err(format!("or_from: word {off} outside the space"));
        }
        let k = core
            .index_of(from)
            .ok_or_else(|| format!("or_from: checkpoint {from} is not in the live chain"))?;
        core.history_gen += 1;
        let c = (off >> CHUNK_SHIFT) as u32;
        let w = (off & (CHUNK_BYTES - 1)) / 8;
        let (mw, b) = (w / 64, w % 64);
        // The checkpoints before `from` keep the word as it was: the log
        // into `from` (sealed) must hold its value there, which is its value
        // at `from` unless that interval wrote it (then it holds it already).
        if k > 0 {
            let live = |cc: u32| core.chunk_ptr(cc as usize) as *const u64;
            let v = rewound(core.nchunks, &[c], &live, &core.logs[k..])[w];
            let prev = &mut core.logs[k - 1];
            if !prev.entries.is_empty() {
                return Err("or_from: the log before the checkpoint is not sealed".into());
            }
            match prev.deltas.iter().position(|d| d.c == c) {
                Some(di) if prev.deltas[di].mask[mw] >> b & 1 == 1 => {}
                Some(di) => {
                    let d = prev.deltas[di];
                    let idx = d.at as usize
                        + d.mask[..mw]
                            .iter()
                            .map(|m| m.count_ones() as usize)
                            .sum::<usize>()
                        + (d.mask[mw] & ((1u64 << b) - 1)).count_ones() as usize;
                    prev.words.insert(idx, v);
                    prev.deltas[di].mask[mw] |= 1u64 << b;
                    for d2 in prev.deltas.iter_mut() {
                        if d2.at > d.at {
                            d2.at += 1;
                        }
                    }
                }
                None => {
                    let at = prev.words.len() as u32;
                    prev.words.push(v);
                    let mut mask = [0u64; MASK_WORDS];
                    mask[mw] = 1u64 << b;
                    let pos = prev.deltas.partition_point(|d| d.c < c);
                    prev.deltas.insert(pos, Delta { c, at, mask });
                }
            }
        }
        let mut n = 0;
        for log in &mut core.logs[k..] {
            for &(cc, p) in &log.entries {
                if cc == c {
                    // SAFETY: a slab chunk of CHUNK_WORDS words.
                    unsafe { *p.add(w) |= bits };
                    n += 1;
                }
            }
            for d in &log.deltas {
                if d.c == c && d.mask[mw] >> b & 1 == 1 {
                    let idx = d.at as usize
                        + d.mask[..mw]
                            .iter()
                            .map(|m| m.count_ones() as usize)
                            .sum::<usize>()
                        + (d.mask[mw] & ((1u64 << b) - 1)).count_ones() as usize;
                    log.words[idx] |= bits;
                    n += 1;
                }
            }
        }
        // SAFETY: inside the mapping, 8-byte aligned.
        unsafe { *(core.base.add(off) as *mut u64) |= bits };
        Ok(n + 1)
    }

    /// Write `src` at byte `off` of the space through the barrier, touching
    /// only the words that differ (the scalar spill).
    pub fn write_through(&mut self, off: usize, src: &[u8]) {
        let core = self.core_mut();
        assert!(off + src.len() <= core.bytes);
        // SAFETY: inside the mapping.
        let dst = unsafe { std::slice::from_raw_parts_mut(core.base.add(off), src.len()) };
        if dst == src {
            return;
        }
        let first = off >> CHUNK_SHIFT;
        let last = (off + src.len() - 1) >> CHUNK_SHIFT;
        for c in first..=last {
            let lo = (c << CHUNK_SHIFT).max(off) - off;
            let hi = ((c + 1) << CHUNK_SHIFT).min(off + src.len()) - off;
            if dst[lo..hi] != src[lo..hi] {
                if core.saved()[c] == 0 {
                    core.save(c);
                }
                dst[lo..hi].copy_from_slice(&src[lo..hi]);
            }
        }
    }

    /// Bytes `off..off+len` of the space.
    pub fn read(&self, off: usize, len: usize) -> &[u8] {
        &self.bytes()[off..off + len]
    }

    pub fn checkpoint(&mut self) -> CheckpointId {
        self.core_mut().checkpoint()
    }

    pub fn checkpoint_ids(&self) -> &[CheckpointId] {
        &self.core().ids
    }

    pub fn restore_discard(&mut self, id: CheckpointId) -> Result<(), String> {
        self.core_mut().restore_discard(id)
    }

    pub fn restore_branch(&mut self, id: CheckpointId) -> Result<Branch, String> {
        self.core_mut().restore_branch(id)
    }

    /// Work out a later `restore_branch(id)` now (`Prepared`), while the
    /// engine waits for the next edit; `stop` ends it early.
    pub fn prepare_restore(&mut self, id: CheckpointId, stop: &mut dyn FnMut() -> bool) -> bool {
        self.core_mut().prepare_restore(id, stop)
    }

    pub fn converge(&mut self, b: Branch, old: CheckpointId) -> Result<(), String> {
        self.core_mut().converge(b, old)
    }

    pub fn drop_branch(&mut self, b: Branch) {
        self.core_mut().drop_branch(b)
    }

    pub fn reattach(&mut self, b: Branch) -> Result<(), String> {
        self.core_mut().reattach(b)
    }

    pub fn retain(&mut self, keep: &dyn Fn(CheckpointId) -> bool) {
        self.core_mut().retain(keep)
    }

    /// Drop every checkpoint and log: the space becomes plain memory again.
    pub fn forget_checkpoints(&mut self) {
        let core = self.core_mut();
        let logs = std::mem::take(&mut core.logs);
        for log in logs {
            core.free_log(log);
        }
        core.ids.clear();
        let Core { touched, .. } = core;
        let t = touched.clone();
        core.saved().copy_from_slice(&t);
    }

    /// The space as it was at checkpoint `id`, without restoring it: each
    /// chunk the logs since `id` hold, rewound through them, else the live
    /// chunk.
    pub fn view_at(&self, id: CheckpointId) -> Result<View<'_>, String> {
        let core = self.core();
        let k = core
            .index_of(id)
            .ok_or_else(|| format!("checkpoint {id} is not retained"))?;
        let mut seen = vec![0u64; core.nchunks.div_ceil(64)];
        let mut cs: Vec<u32> = Vec::new();
        for log in &core.logs[k..] {
            for c in log.chunk_ids() {
                if !bit(&seen, c as usize) {
                    set_bit(&mut seen, c as usize);
                    cs.push(c);
                }
            }
        }
        cs.sort_unstable();
        let live = |c: u32| core.chunk_ptr(c as usize) as *const u64;
        let bufs = rewound(core.nchunks, &cs, &live, &core.logs[k..]);
        let mut over: Vec<*const u64> = vec![std::ptr::null(); core.nchunks];
        for (i, &c) in cs.iter().enumerate() {
            over[c as usize] = bufs[i * CHUNK_WORDS..].as_ptr();
        }
        Ok(View {
            arena: self,
            over,
            _bufs: bufs,
        })
    }

    /// The chunks the open log holds (written since the newest checkpoint),
    /// counted by the array each starts in, most first.
    pub fn open_log_by_region(&self) -> Vec<(&'static str, usize)> {
        let core = self.core();
        let mut counts = vec![0usize; self.regions.len()];
        if let Some(log) = core.logs.last() {
            for &(c, _) in &log.entries {
                let at = (c as usize) << CHUNK_SHIFT;
                let r = match self.regions.binary_search_by(|r| r.off.cmp(&at)) {
                    Ok(i) => i,
                    Err(i) => i.saturating_sub(1),
                };
                if let Some(n) = counts.get_mut(r) {
                    *n += 1;
                }
            }
        }
        let mut v: Vec<(&'static str, usize)> = counts
            .iter()
            .enumerate()
            .filter(|(_, &n)| n > 0)
            .map(|(r, &n)| (self.regions[r].name, n))
            .collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v
    }

    /// Measurement only: element writes so far, by the array each chunk
    /// starts in, most first.
    #[cfg(feature = "bench-count-writes")]
    pub fn write_counts_by_region(&self) -> Vec<(&'static str, u64)> {
        let mut counts: std::collections::BTreeMap<&'static str, u64> = Default::default();
        for (c, n) in WRITE_COUNTS.iter().enumerate().take(self.chunks()) {
            let n = n.load(std::sync::atomic::Ordering::Relaxed);
            if n == 0 {
                continue;
            }
            let at = c << CHUNK_SHIFT;
            let r = match self.regions.binary_search_by(|r| r.off.cmp(&at)) {
                Ok(i) => i,
                Err(i) => i.saturating_sub(1),
            };
            *counts.entry(self.regions[r].name).or_default() += n;
        }
        let mut v: Vec<_> = counts.into_iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v
    }

    /// Compare the live space with the old run's space at checkpoint `old`
    /// of the detached branch `b` (DESIGN.md §5.3, the convergence test).
    ///
    /// Both runs started from the branch's first checkpoint `r` (the restore
    /// target), which is also retained in the live chain. A chunk can differ
    /// only if one of the runs wrote it since `r`: the old run in the
    /// branch's logs up to `old`, the new run in the live chain's logs from
    /// `r` on. For each such chunk the old value at `old` is, if the old run
    /// wrote it at all, its redo copy (the old run's last value) rewound
    /// through the branch's logs from `old` on; else -- the old run never
    /// wrote it -- its value at `r`: the live chunk rewound through the live
    /// chain's logs since `r`. Returns the chunks that differ, with pointers
    /// to the two versions (valid while the arena, the branch and the
    /// `ChunkDiff` are unchanged).
    pub fn diff_branch(&self, b: &Branch, old: CheckpointId) -> Result<ChunkDiff, String> {
        self.diff_branch_until(b, old, &mut || false)?
            .ok_or_else(|| "stopped".to_string())
    }

    /// [`diff_branch`](Self::diff_branch), asking `stop` as it rewinds the
    /// old run's logs: `Ok(None)` when it said to stop.
    pub fn diff_branch_until(
        &self,
        b: &Branch,
        old: CheckpointId,
        stop: &mut dyn FnMut() -> bool,
    ) -> Result<Option<ChunkDiff>, String> {
        let core = self.core();
        let r = *b.ids.first().ok_or("empty branch")?;
        let kr = core
            .index_of(r)
            .ok_or_else(|| format!("restore target {r} is not in the live chain"))?;
        let jj = b
            .ids
            .iter()
            .position(|&x| x == old)
            .ok_or_else(|| format!("checkpoint {old} is not in the detached run"))?;
        let n = core.nchunks;
        let mut seen = vec![0u64; n.div_ceil(64)];
        let mut cand: Vec<u32> = Vec::new();
        for log in b.logs[..jj].iter().chain(&core.logs[kr..]) {
            for c in log.chunk_ids() {
                if !bit(&seen, c as usize) {
                    set_bit(&mut seen, c as usize);
                    cand.push(c);
                }
            }
        }
        // The old run's last value of the candidates it wrote at all.
        let mut redo_of: HashMap<u32, *const u64> = HashMap::new();
        for &(c, p) in &b.redo {
            if bit(&seen, c as usize) {
                redo_of.insert(c, p as *const u64);
            }
        }
        let (mut in_old, mut at_r): (Vec<u32>, Vec<u32>) =
            cand.iter().partition(|c| redo_of.contains_key(c));
        in_old.sort_unstable();
        at_r.sort_unstable();
        let t = std::time::Instant::now();
        let Some(old_buf) = rewound_until(n, &in_old, &|c| redo_of[&c], &b.logs[jj..], stop) else {
            return Ok(None);
        };
        let t_old = t.elapsed();
        let live = |c: u32| core.chunk_ptr(c as usize) as *const u64;
        let r_buf = rewound(n, &at_r, &live, &core.logs[kr..]);
        if std::env::var_os("FLASHTEX_INCR_DEBUG").is_some() {
            let entries: usize = b.logs[jj..]
                .iter()
                .map(|l| l.entries.len() + l.deltas.len())
                .sum();
            eprintln!(
                "[arena] diff: {} candidates ({} in the old run, rewound through {} logs, {} entries: {:.2} ms; {} at the restart point through {} logs: {:.2} ms)",
                cand.len(),
                in_old.len(),
                b.logs.len() - jj,
                entries,
                t_old.as_secs_f64() * 1e3,
                at_r.len(),
                core.logs.len() - kr,
                (t.elapsed() - t_old).as_secs_f64() * 1e3
            );
        }
        let mut old_at: HashMap<u32, *const u64> = HashMap::with_capacity(cand.len());
        for (i, &c) in in_old.iter().enumerate() {
            old_at.insert(c, old_buf[i * CHUNK_WORDS..].as_ptr());
        }
        for (i, &c) in at_r.iter().enumerate() {
            old_at.insert(c, r_buf[i * CHUNK_WORDS..].as_ptr());
        }
        cand.sort_unstable();
        let mut out = ChunkDiff {
            compared: cand.len(),
            differing: Vec::new(),
            old_at: HashMap::new(),
            _bufs: (old_buf, r_buf),
        };
        for c in cand {
            let live = core.chunk_ptr(c as usize) as *const u64;
            let old_p = old_at[&c];
            // SAFETY: both point at CHUNK_WORDS words owned by the arena or
            // `out`, neither written during this call.
            let (a, bb) = unsafe {
                (
                    std::slice::from_raw_parts(old_p, CHUNK_WORDS),
                    std::slice::from_raw_parts(live, CHUNK_WORDS),
                )
            };
            if a != bb {
                out.differing.push((c, old_p, live));
            }
        }
        out.old_at = old_at;
        Ok(Some(out))
    }

    /// The region (array) holding byte `off` of the space, and the offset
    /// into it.
    pub fn region_at(&self, off: usize) -> (&RegionInfo, usize) {
        let r = match self.regions.binary_search_by(|r| r.off.cmp(&off)) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        (&self.regions[r], off - self.regions[r].off)
    }

    /// The chunks in the open log (written since the newest checkpoint):
    /// what the intrinsics' both-paths verifier compares
    /// (`crate::intrinsics_verify`).
    pub fn open_log_chunks(&self) -> Vec<u32> {
        self.core()
            .logs
            .last()
            .map_or(vec![], |l| l.chunk_ids().collect())
    }

    /// Entries in the open log (chunks written since the newest checkpoint).
    pub fn open_log_len(&self) -> usize {
        self.core().logs.last().map_or(0, |l| l.entries.len())
    }

    /// Chunks copied by the barrier so far.
    pub fn slow_path(&self) -> u64 {
        self.core().slow_path
    }

    /// Bytes held by undo and redo logs: whole chunks (the open log, redo
    /// copies: the slab's live slots) and sealed logs.
    pub fn log_bytes(&self) -> usize {
        let c = self.core();
        c.slab.live * CHUNK_BYTES + c.sealed_bytes
    }

    /// Workers for restores; 0 picks one per core, up to 8.
    pub fn set_threads(&mut self, n: usize) {
        self.core_mut().threads = n;
    }
}

/// The chunks where the live space and an old run's checkpoint differ
/// (`Arena::diff_branch`): (chunk, old words, live words). It also reads
/// any word of the old run's space there ([`ChunkDiff::old_word`]), while
/// the arena and the branch are unchanged.
pub struct ChunkDiff {
    pub compared: usize,
    pub differing: Vec<(u32, *const u64, *const u64)>,
    /// The old run's value of every chunk either run wrote since the
    /// restore target (every other chunk is the live one).
    old_at: HashMap<u32, *const u64>,
    /// Where `old_at` points.
    _bufs: (Vec<u64>, Vec<u64>),
}

impl ChunkDiff {
    /// Where each chunk of the old run's space is (`old_word`'s answer for
    /// a whole chunk), indexed by chunk.
    pub fn table(&self, a: &Arena) -> Vec<*const u64> {
        let core = a.core();
        let mut t: Vec<*const u64> = (0..core.nchunks)
            .map(|c| core.chunk_ptr(c) as *const u64)
            .collect();
        for (&c, &p) in &self.old_at {
            t[c as usize] = p;
        }
        t
    }

    /// Every chunk either run wrote since the restore target (the chunks
    /// compared), in no order.
    pub fn written(&self) -> Vec<u32> {
        self.old_at.keys().copied().collect()
    }

    /// The old run's 8-byte word at byte `off` of the space.
    pub fn old_word(&self, a: &Arena, off: usize) -> u64 {
        let c = (off >> CHUNK_SHIFT) as u32;
        let w = (off & (CHUNK_BYTES - 1)) >> 3;
        let p = self
            .old_at
            .get(&c)
            .copied()
            .unwrap_or_else(|| a.core().chunk_ptr(c as usize) as *const u64);
        // SAFETY: a chunk of CHUNK_WORDS words; w < CHUNK_WORDS.
        unsafe { *p.add(w) }
    }
}

/// The space at a checkpoint (`Arena::view_at`).
pub struct View<'a> {
    arena: &'a Arena,
    over: Vec<*const u64>,
    /// Where `over` points.
    _bufs: Vec<u64>,
}

impl View<'_> {
    pub fn chunk(&self, c: usize) -> &[u8] {
        let p = self.over[c];
        if p.is_null() {
            let w = self.arena.chunk(c);
            // SAFETY: a chunk of CHUNK_WORDS words viewed as bytes.
            unsafe { std::slice::from_raw_parts(w.as_ptr() as *const u8, CHUNK_BYTES) }
        } else {
            // SAFETY: a chunk of `_bufs`, alive and unchanged with `self`.
            unsafe { std::slice::from_raw_parts(p as *const u8, CHUNK_BYTES) }
        }
    }
}

// ---------------------------------------------------------------------------
// Arr: an array global
// ---------------------------------------------------------------------------

/// An array global of the engine: `len` elements of `T` in the word space.
/// Reads are plain loads; every write passes the barrier.
pub struct Arr<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
    /// The core's `saved` flags, biased so that the flag of the chunk
    /// holding address `a` is at `flags + (a >> CHUNK_SHIFT)` (the space
    /// starts on a chunk boundary). Never dereferenced except so offset.
    flags: *const u8,
    core: *mut Core,
    /// Measurement only: the space's first chunk as an address >> 14.
    #[cfg(feature = "bench-count-writes")]
    base_chunk: usize,
    /// Measurement only: the chunk of this array's previous write.
    #[cfg(feature = "bench-count-writes")]
    last: std::cell::Cell<usize>,
}

// SAFETY: see `Arena`.
unsafe impl<T: Send> Send for Arr<T> {}

/// Measurement only: writes to the same chunk as the array's previous one.
#[cfg(feature = "bench-count-writes")]
pub static SAME_CHUNK_AS_LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Measurement only (feature `bench-count-writes`): writes per chunk.
#[cfg(feature = "bench-count-writes")]
pub static WRITE_COUNTS: [std::sync::atomic::AtomicU64; 32768] =
    [const { std::sync::atomic::AtomicU64::new(0) }; 32768];

#[cold]
#[inline(never)]
fn out_of_bounds(i: usize, len: usize) -> ! {
    panic!("index out of bounds: the len is {len} but the index is {i}")
}

#[cold]
#[inline(never)]
fn save_cold(core: *mut Core, c: usize) {
    // SAFETY: the core outlives every array; no other reference to it is
    // live while the generated code writes an element.
    unsafe { (*core).save(c) }
}

impl<T> Arr<T> {
    const SIZE: usize = std::mem::size_of::<T>();
    /// An element can straddle two chunks only if its size is not a power
    /// of two (Plan::reserve aligns the rest).
    const STRADDLES: bool = !Self::SIZE.is_power_of_two();

    /// The barrier for the chunk holding byte address `a` of the space.
    #[inline(always)]
    fn touch_addr(&self, a: usize) {
        let flag = self.flags.wrapping_add(a >> CHUNK_SHIFT);
        // SAFETY: `a` lies in this array's region, so `flag` is the saved
        // flag of its chunk, inside the core's map.
        if unsafe { *flag } == 0 {
            // SAFETY: the core outlives the array.
            let base = unsafe { (*self.core).base } as usize;
            save_cold(self.core, (a - base) >> CHUNK_SHIFT);
        }
    }

    /// The write barrier for element `i` (already bounds-checked).
    #[inline(always)]
    fn touch(&self, i: usize) {
        #[cfg(feature = "bench-count-writes")]
        {
            let c = ((self.ptr as usize + i * Self::SIZE) >> CHUNK_SHIFT) - self.base_chunk;
            WRITE_COUNTS[c.min(32767)].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if self.last.replace(c) == c {
                SAME_CHUNK_AS_LAST.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        // Measurement only (docs/evidence/p4-l1-2026-09-29): the same
        // layout without the barrier, which makes checkpoints wrong.
        if cfg!(feature = "bench-no-barrier") {
            return;
        }
        let a = self.ptr as usize + i * Self::SIZE;
        self.touch_addr(a);
        if Self::STRADDLES {
            let a2 = a + Self::SIZE - 1;
            if (a2 ^ a) >> CHUNK_SHIFT != 0 {
                self.touch_addr(a2);
            }
        }
    }

    /// The barrier for elements `lo..hi`.
    fn touch_range(&self, lo: usize, hi: usize) {
        if lo >= hi {
            return;
        }
        let first = (self.ptr as usize + lo * Self::SIZE) >> CHUNK_SHIFT;
        let last = (self.ptr as usize + hi * Self::SIZE - 1) >> CHUNK_SHIFT;
        for c in first..=last {
            self.touch_addr(c << CHUNK_SHIFT);
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Elements `lo..hi` for writing, through the barrier.
    pub fn slice_mut(&mut self, lo: usize, hi: usize) -> &mut [T] {
        if lo > hi || hi > self.len {
            out_of_bounds(hi, self.len);
        }
        self.touch_range(lo, hi);
        // SAFETY: in bounds; `&mut self` makes the slice unique.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.add(lo), hi - lo) }
    }

    /// web2c's `xmalloc_array(T, n-1)`: `n` fresh zero elements.
    pub fn alloc_len(&mut self, n: usize) {
        if n > self.cap {
            panic!(
                "flashtex: array of {n} elements exceeds its reserved {}",
                self.cap
            );
        }
        self.len = n;
        let s = self.slice_mut(0, n);
        // SAFETY: every element type of the space is plain old data whose
        // all-zero bit pattern is its default value.
        unsafe { std::ptr::write_bytes(s.as_mut_ptr(), 0, n) };
    }

    /// web2c's `xrealloc_array(p, T, n-1)`: keep the first elements, new
    /// ones are zero (as `Vec::resize` with the default value made them).
    pub fn resize_len(&mut self, n: usize) {
        if n > self.cap {
            panic!(
                "flashtex: array of {n} elements exceeds its reserved {}",
                self.cap
            );
        }
        let old = self.len;
        self.len = n;
        if n > old {
            let s = self.slice_mut(old, n);
            // SAFETY: as in alloc_len.
            unsafe { std::ptr::write_bytes(s.as_mut_ptr(), 0, n - old) };
        }
    }

    /// The logical length, spilled with the scalars at a checkpoint.
    pub fn len_mut(&mut self) -> &mut usize {
        &mut self.len
    }
}

impl<T> Deref for Arr<T> {
    type Target = [T];
    #[inline(always)]
    fn deref(&self) -> &[T] {
        // SAFETY: `len` initialised elements live at `ptr`.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T, I: std::slice::SliceIndex<[T]>> Index<I> for Arr<T> {
    type Output = I::Output;
    #[inline(always)]
    fn index(&self, i: I) -> &I::Output {
        &(**self)[i]
    }
}

impl<T> IndexMut<usize> for Arr<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut T {
        if i >= self.len {
            out_of_bounds(i, self.len);
        }
        self.touch(i);
        // SAFETY: in bounds; `&mut self` makes the reference unique.
        unsafe { &mut *self.ptr.add(i) }
    }
}

macro_rules! range_mut {
    ($($r:ty => |$s:ident, $x:ident| $lo:expr, $hi:expr;)*) => {$(
        impl<T> IndexMut<$r> for Arr<T> {
            #[allow(unused_variables)]
            fn index_mut(&mut self, $x: $r) -> &mut [T] {
                let $s = &*self;
                let (lo, hi) = ($lo, $hi);
                self.slice_mut(lo, hi)
            }
        }
    )*};
}

range_mut! {
    std::ops::Range<usize> => |s, r| r.start, r.end;
    std::ops::RangeFrom<usize> => |s, r| r.start, s.len;
    std::ops::RangeTo<usize> => |s, r| 0, r.end;
    std::ops::RangeInclusive<usize> => |s, r| *r.start(), *r.end() + 1;
    std::ops::RangeFull => |s, r| { let _ = r; 0 }, s.len;
}

impl<T: std::fmt::Debug> std::fmt::Debug for Arr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Arr[{} of {}]", self.len, self.cap)
    }
}

// ---------------------------------------------------------------------------
// The scalar globals
// ---------------------------------------------------------------------------

/// What the generated `Globals::visit_scalars` calls for each scalar global
/// (and each growable array's length), in declaration order.
pub trait Visit {
    fn pod<T: Copy>(&mut self, x: &mut T);
    fn arr_len<T>(&mut self, a: &mut Arr<T>) {
        self.pod(a.len_mut());
    }
}

/// Copies every scalar into a byte image (the spill).
pub struct Spill {
    pub buf: Vec<u8>,
}

impl Visit for Spill {
    fn pod<T: Copy>(&mut self, x: &mut T) {
        let n = std::mem::size_of::<T>();
        // SAFETY: T is one of the engine's plain-old-data types (integers,
        // reals, booleans, byte arrays and records of them, without padding).
        let b = unsafe { std::slice::from_raw_parts(x as *const T as *const u8, n) };
        self.buf.extend_from_slice(b);
        self.buf.resize(self.buf.len() + (slot::<T>() - n), 0);
    }
}

/// Reads every scalar back from a byte image (the fill).
pub struct Fill<'a> {
    pub src: &'a [u8],
    pub pos: usize,
}

impl Visit for Fill<'_> {
    fn pod<T: Copy>(&mut self, x: &mut T) {
        let n = std::mem::size_of::<T>();
        let b = &self.src[self.pos..self.pos + n];
        // SAFETY: the bytes were written by `Spill` from a value of the same
        // type in the same position, so they are a valid `T`.
        unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), x as *mut T as *mut u8, n) };
        self.pos += slot::<T>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space(words: usize) -> (Arena, Arr<u64>) {
        let mut p = Plan::new(64);
        let r = p.reserve::<u64>("t", words);
        let a = p.build();
        let arr = a.arr(r, words);
        (a, arr)
    }

    fn scribble(arr: &mut Arr<u64>, seed: u64, n: usize) {
        let mut x = seed;
        for _ in 0..n {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let i = (x as usize) % arr.len();
            arr[i] = x;
        }
    }

    #[test]
    fn restore_to_every_checkpoint_and_converge() {
        let (mut a, mut arr) = space(200_000);
        scribble(&mut arr, 1, 5000);
        let mut ids = vec![];
        let mut copies = vec![];
        for k in 0..20 {
            ids.push(a.checkpoint());
            copies.push(arr.to_vec());
            scribble(&mut arr, 100 + k, 3000);
        }
        let end = arr.to_vec();
        for i in [0usize, 7, 19, 3] {
            let br = a.restore_branch(ids[i]).unwrap();
            assert!(arr[..] == copies[i][..], "restore to {i}");
            a.converge(br, ids[i]).unwrap();
            assert!(arr[..] == end[..], "jump back from {i}");
        }
        a.restore_discard(ids[5]).unwrap();
        assert!(arr[..] == copies[5][..]);
        assert_eq!(a.checkpoint_ids(), &ids[..=5]);
    }

    /// A prepared restore equals a plain one, also when the live state was
    /// written after the preparation, and a stale one is not used.
    #[test]
    fn prepared_restores_equal_plain_ones() {
        let (mut a, mut arr) = space(200_000);
        scribble(&mut arr, 1, 5000);
        let mut ids = vec![];
        let mut copies = vec![];
        for k in 0..20 {
            ids.push(a.checkpoint());
            copies.push(arr.to_vec());
            scribble(&mut arr, 100 + k, 3000);
        }
        for (n, i) in [0usize, 7, 19, 3, 12].into_iter().enumerate() {
            assert!(a.prepare_restore(ids[i], &mut || false));
            // written after the preparation (the open log grows)
            scribble(&mut arr, 500 + n as u64, 200);
            let end = arr.to_vec();
            let br = a.restore_branch(ids[i]).unwrap();
            assert!(arr[..] == copies[i][..], "prepared restore to {i}");
            a.converge(br, ids[i]).unwrap();
            assert!(arr[..] == end[..], "jump back from {i}");
        }
        // stale: a checkpoint since the preparation
        assert!(a.prepare_restore(ids[5], &mut || false));
        let extra = a.checkpoint();
        scribble(&mut arr, 777, 300);
        let br = a.restore_branch(ids[5]).unwrap();
        assert!(arr[..] == copies[5][..], "stale preparation not used");
        a.drop_branch(br);
        let _ = extra;
        // stopped
        assert!(!a.prepare_restore(ids[2], &mut || true) || ids.len() < 64);
    }

    /// `or_from`: the bits are in the word at the checkpoint named and at
    /// every later one, and in the live state, and nowhere before.
    #[test]
    fn or_from_rewrites_the_later_history() {
        let (mut a, mut arr) = space(200_000);
        scribble(&mut arr, 1, 5000);
        let mut ids = vec![];
        let mut copies = vec![];
        for k in 0..20 {
            ids.push(a.checkpoint());
            copies.push(arr.to_vec());
            scribble(&mut arr, 100 + k, 3000);
        }
        let end = arr.to_vec();
        // a word some logs hold (written after checkpoint 7) and one none do
        let w = (0..arr.len())
            .find(|&i| copies[8][i] != copies[7][i])
            .unwrap();
        let quiet = (0..arr.len())
            .find(|&i| copies.iter().all(|c| c[i] == end[i]) && i != w)
            .unwrap();
        let bits = 1u64 << 63 | 1;
        for &i in &[w, quiet] {
            let off = (&arr[i] as *const u64 as usize) - (a.bytes().as_ptr() as usize);
            a.or_from(ids[7], off, bits).unwrap();
        }
        let expect = |c: &Vec<u64>, k: usize| {
            let mut c = c.clone();
            if k >= 7 {
                c[w] |= bits;
                c[quiet] |= bits;
            }
            c
        };
        assert!(arr[w] == end[w] | bits && arr[quiet] == end[quiet] | bits);
        for i in [3usize, 7, 12, 19, 0] {
            let br = a.restore_branch(ids[i]).unwrap();
            assert!(arr[..] == expect(&copies[i], i)[..], "restore to {i}");
            a.converge(br, ids[i]).unwrap();
        }
        a.restore_discard(ids[10]).unwrap();
        assert!(arr[..] == expect(&copies[10], 10)[..]);
    }

    #[test]
    fn reattach_abandons_the_new_run() {
        let (mut a, mut arr) = space(200_000);
        scribble(&mut arr, 1, 5000);
        let mut ids = vec![];
        let mut copies = vec![];
        for k in 0..12 {
            ids.push(a.checkpoint());
            copies.push(arr.to_vec());
            scribble(&mut arr, 100 + k, 3000);
        }
        let end = arr.to_vec();
        for i in [0usize, 5, 11] {
            let br = a.restore_branch(ids[i]).unwrap();
            // a new run that writes and takes checkpoints of its own
            scribble(&mut arr, 900 + i as u64, 4000);
            a.checkpoint();
            scribble(&mut arr, 950 + i as u64, 4000);
            a.checkpoint();
            scribble(&mut arr, 990 + i as u64, 100);
            a.reattach(br).unwrap();
            assert!(arr[..] == end[..], "back to the old run's end from {i}");
            assert_eq!(a.checkpoint_ids(), &ids[..], "the old checkpoints from {i}");
            // writes after the reattach are logged against the old newest
            scribble(&mut arr, 7, 10);
            a.restore_discard(*ids.last().unwrap()).unwrap();
            assert!(arr[..] == copies[11][..]);
            scribble(&mut arr, 111, 3000);
            assert!(arr[..] == end[..], "the old run's last interval replays");
        }
        for (i, &id) in ids.iter().enumerate().rev() {
            a.restore_discard(id).unwrap();
            assert!(arr[..] == copies[i][..], "restore {i} after reattaches");
        }
    }

    #[test]
    fn straddling_elements_are_saved_whole() {
        #[derive(Clone, Copy, PartialEq, Debug)]
        struct R([u32; 6]);
        let mut p = Plan::new(8);
        let r = p.reserve::<R>("t", 10_000);
        let mut a = p.build();
        let mut arr = a.arr(r, 10_000);
        for i in 0..10_000 {
            arr[i] = R([i as u32; 6]);
        }
        let id = a.checkpoint();
        let before = arr.to_vec();
        for i in 0..10_000 {
            arr[i] = R([7; 6]);
        }
        a.restore_discard(id).unwrap();
        assert!(arr[..] == before[..]);
    }

    #[test]
    fn scalar_spill_goes_through_the_barrier() {
        let (mut a, _arr) = space(10);
        a.write_through(0, &[1, 2, 3, 4]);
        let id = a.checkpoint();
        a.write_through(0, &[9, 9, 9, 9]);
        assert_eq!(a.read(0, 4), &[9, 9, 9, 9]);
        a.restore_discard(id).unwrap();
        assert_eq!(a.read(0, 4), &[1, 2, 3, 4]);
    }

    #[test]
    fn retain_merges_logs() {
        let (mut a, mut arr) = space(100_000);
        let mut ids = vec![];
        let mut copies = vec![];
        for k in 0..12 {
            ids.push(a.checkpoint());
            copies.push(arr.to_vec());
            scribble(&mut arr, 7 + k, 2000);
        }
        let before = a.log_bytes();
        a.retain(&|id| id % 3 == 0);
        assert!(a.log_bytes() < before);
        for (i, &id) in ids.iter().enumerate() {
            if id % 3 == 0 || i + 1 == ids.len() {
                let br = a.restore_branch(id).unwrap();
                assert!(arr[..] == copies[i][..], "kept {id}");
                a.converge(br, id).unwrap();
            }
        }
    }

    #[test]
    fn sealed_logs_hold_only_changed_words() {
        let (mut a, mut arr) = space(100_000);
        scribble(&mut arr, 3, 20_000);
        let id = a.checkpoint();
        let before = arr.to_vec();
        // one word in each of 500 chunks
        for k in 0..500 {
            arr[k * CHUNK_WORDS + 5] = 42 + k as u64;
        }
        a.checkpoint();
        assert!(a.log_bytes() < 500 * 64, "{} bytes", a.log_bytes());
        // written back to the same value: nothing to keep
        for k in 0..500 {
            arr[k * CHUNK_WORDS + 5] = 42 + k as u64;
        }
        a.checkpoint();
        a.restore_discard(id).unwrap();
        assert!(arr[..] == before[..]);
    }

    #[test]
    fn views_and_branch_diffs_see_old_states() {
        let (mut a, mut arr) = space(200_000);
        scribble(&mut arr, 11, 5000);
        let mut ids = vec![];
        let mut copies = vec![];
        for k in 0..10 {
            ids.push(a.checkpoint());
            copies.push(arr.to_vec());
            scribble(&mut arr, 50 + k, 2000);
        }
        let word = |a: &Arena, bytes: &[u8], i: usize| -> u64 {
            let _ = a;
            u64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap())
        };
        let start = arr.as_ptr() as usize - a.bytes().as_ptr() as usize;
        for i in [0usize, 4, 9] {
            let v = a.view_at(ids[i]).unwrap();
            for e in (0..arr.len()).step_by(97) {
                let off = start + e * 8;
                let c = off >> CHUNK_SHIFT;
                let got = word(&a, v.chunk(c), (off & (CHUNK_BYTES - 1)) / 8);
                assert_eq!(got, copies[i][e], "view at {i}, element {e}");
            }
        }
        // restore to 3, re-run differently for two checkpoints, compare with
        // the old run's checkpoints 4 and 5
        let br = a.restore_branch(ids[3]).unwrap();
        scribble(&mut arr, 900, 1500);
        let n1 = a.checkpoint();
        let now = arr.to_vec();
        for (j, old) in [(3usize, ids[3]), (4, ids[4]), (5, ids[5])] {
            let d = a.diff_branch(&br, old).unwrap();
            for e in (0..arr.len()).step_by(89) {
                let off = start + e * 8;
                assert_eq!(d.old_word(&a, off), copies[j][e], "old {j}, element {e}");
            }
            let differ = (0..arr.len()).any(|e| copies[j][e] != now[e]);
            assert_eq!(!d.differing.is_empty(), differ);
        }
        let _ = n1;
        a.drop_branch(br);
    }

    #[test]
    fn deep_parallel_rewind_through_merged_logs() {
        let (mut a, mut arr) = space(2_000_000);
        a.set_threads(4);
        let mut ids = vec![];
        let mut copies = vec![];
        for k in 0..40 {
            ids.push(a.checkpoint());
            copies.push(arr.to_vec());
            // wide enough to take the parallel path
            for i in (0..arr.len()).step_by(61 + k as usize) {
                arr[i] = arr[i].wrapping_mul(31).wrapping_add(k + i as u64);
            }
            if k % 8 == 7 {
                a.retain(&|id| id % 3 != 1);
            }
        }
        let end = arr.to_vec();
        let kept: Vec<CheckpointId> = a.checkpoint_ids().to_vec();
        for &id in kept.iter().rev().step_by(3) {
            let i = ids.iter().position(|&x| x == id).unwrap();
            let br = a.restore_branch(id).unwrap();
            assert!(arr[..] == copies[i][..], "restore to {id}");
            a.converge(br, id).unwrap();
            assert!(arr[..] == end[..], "back from {id}");
        }
    }

    /// `cargo test --release -p flashtex-engine --lib seal_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn seal_cost() {
        let (mut a, mut arr) = space(8_000_000);
        scribble(&mut arr, 5, 1_000_000);
        a.checkpoint();
        let mut best = f64::MAX;
        for round in 0..30u64 {
            // 2000 chunks, 13 words each (a tenth), as a page of long8
            for k in 0..2000usize {
                for w in 0..13usize {
                    arr[k * 3 * CHUNK_WORDS + w * 9] = round * 1000 + (k * w) as u64;
                }
            }
            let t = std::time::Instant::now();
            a.checkpoint();
            best = best.min(t.elapsed().as_secs_f64());
        }
        eprintln!("seal of 2000 chunks: best {:.3} ms", best * 1000.0);
    }

    #[test]
    fn parallel_restore_is_exact() {
        let (mut a, mut arr) = space(4_000_000);
        a.set_threads(4);
        let id = a.checkpoint();
        let before = arr.to_vec();
        for i in (0..arr.len()).step_by(512) {
            arr[i] = i as u64 + 1;
        }
        let br = a.restore_branch(id).unwrap();
        assert!(arr[..] == before[..]);
        a.converge(br, id).unwrap();
        assert_eq!(arr[512], 513);
    }
}
