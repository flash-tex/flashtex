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
//! The space is `mmap`ed anonymous memory (`crate::os::alloc_zeroed`), so the hundreds of megabytes of
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

// The mapping itself is `crate::os`'s (DESIGN.md §16 rule 2).
use crate::os;

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
            let block = os::alloc_zeroed(SLAB_BLOCK_CHUNKS * CHUNK_BYTES);
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
            // SAFETY: each block came from os::alloc_zeroed with this length.
            unsafe { os::free(b, SLAB_BLOCK_CHUNKS * CHUNK_BYTES) };
        }
    }
}

const MASK_WORDS: usize = CHUNK_WORDS / 64;

// ---------------------------------------------------------------------------
// The words of a sealed log, packed
// ---------------------------------------------------------------------------
//
// A sealed log's words are the pre-images of TeX's memory words: two 32-bit
// halves each (`link`/`info`, a `scaled`, character and font codes), most of
// them small. Each half is stored in 0, 1, 2 or 4 bytes (zero, then the
// smallest sign-extended width that holds it), with a 2-bit tag; a word's two
// tags are one nibble. A delta's words are its nibbles (two to a byte), then
// the halves' bytes in the same order. On the documents measured this takes
// 0.50–0.58 of the 8 bytes a word took (docs/evidence/mem-footprint-2026-10-04/).
// Decoding a word needs the sizes of the words before it in its delta, which
// the nibbles give without touching their bytes.

/// Bytes a half takes, by its tag.
const HALF_LEN: [usize; 4] = [0, 1, 2, 4];

/// Bytes a word takes, by its nibble (the low half's tag, then the high half's).
const NIBBLE_LEN: [usize; 16] = {
    let mut t = [0usize; 16];
    let mut i = 0;
    while i < 16 {
        t[i] = HALF_LEN[i & 3] + HALF_LEN[i >> 2];
        i += 1;
    }
    t
};

#[inline]
fn half_tag(x: u32) -> usize {
    let v = x as i32;
    if v == 0 {
        0
    } else if v as i8 as i32 == v {
        1
    } else if v as i16 as i32 == v {
        2
    } else {
        3
    }
}

#[inline]
fn put_half(out: &mut Vec<u8>, x: u32, t: usize) {
    match t {
        0 => {}
        1 => out.push(x as u8),
        2 => out.extend_from_slice(&(x as u16).to_le_bytes()),
        _ => out.extend_from_slice(&x.to_le_bytes()),
    }
}

/// The half at `b[p..]` with tag `t`.
#[inline]
fn get_half(b: &[u8], p: usize, t: usize) -> u32 {
    match t {
        0 => 0,
        1 => b[p] as i8 as i32 as u32,
        2 => i16::from_le_bytes([b[p], b[p + 1]]) as i32 as u32,
        _ => u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]]),
    }
}

/// Append `vals`, packed, to `out`.
fn pack_words(vals: &[u64], out: &mut Vec<u8>) {
    let t0 = out.len();
    out.resize(t0 + vals.len().div_ceil(2), 0);
    for (i, &x) in vals.iter().enumerate() {
        let (lo, hi) = (x as u32, (x >> 32) as u32);
        let (tl, th) = (half_tag(lo), half_tag(hi));
        out[t0 + i / 2] |= ((tl | th << 2) as u8) << ((i & 1) * 4);
        put_half(out, lo, tl);
        put_half(out, hi, th);
    }
}

/// The word at byte `p` of `b` with nibble `t`.
#[inline]
fn get_word(b: &[u8], p: usize, t: usize) -> u64 {
    let lo = get_half(b, p, t & 3);
    let hi = get_half(b, p + HALF_LEN[t & 3], t >> 2);
    lo as u64 | (hi as u64) << 32
}

/// One chunk of a sealed log: the words that differ between the state at
/// the log's checkpoint and the state at the next, as a bit mask; their
/// values at the log's checkpoint are packed at `Log::bytes[at..]`, in bit
/// order (`pack_words`).
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

    /// The values of the words in `mask`, in bit order, into `out[..len]`.
    fn unpack(&self, bytes: &[u8], out: &mut [u64; CHUNK_WORDS]) {
        let n = self.len();
        let tags = self.at as usize;
        let mut p = tags + n.div_ceil(2);
        for (i, v) in out.iter_mut().enumerate().take(n) {
            let t = (bytes[tags + i / 2] >> ((i & 1) * 4)) as usize & 15;
            *v = get_word(bytes, p, t);
            p += NIBBLE_LEN[t];
        }
    }

    /// Bytes the packed words take.
    fn packed_len(&self, bytes: &[u8]) -> usize {
        let n = self.len();
        let tags = self.at as usize;
        let mut p = n.div_ceil(2);
        for i in 0..n {
            p += NIBBLE_LEN[(bytes[tags + i / 2] >> ((i & 1) * 4)) as usize & 15];
        }
        p
    }

    /// Rewind a chunk by this delta where an older entry has not already:
    /// write the words it holds that are not in `done` into `dst`, and add
    /// them to `done`. Applied from the oldest log on, this leaves each word
    /// at its value at the oldest log's checkpoint.
    ///
    /// # Safety
    /// `dst` points at CHUNK_WORDS writable words.
    #[inline]
    unsafe fn apply_under(&self, bytes: &[u8], dst: *mut u64, done: &mut [u64; MASK_WORDS]) {
        let need = [self.mask[0] & !done[0], self.mask[1] & !done[1]];
        if need[0] | need[1] != 0 {
            // Walk the words in order up to the last one needed, decoding
            // only those (the others' sizes come from their nibbles).
            let n = self.len();
            let tags = self.at as usize;
            let mut p = tags + n.div_ceil(2);
            let mut i = 0usize;
            for mw in 0..MASK_WORDS {
                let mut m = self.mask[mw];
                let mut left = need[mw];
                while left != 0 {
                    let b = m.trailing_zeros();
                    let t = (bytes[tags + i / 2] >> ((i & 1) * 4)) as usize & 15;
                    if left >> b & 1 == 1 {
                        *dst.add(mw * 64 + b as usize) = get_word(bytes, p, t);
                        left &= left - 1;
                    }
                    p += NIBBLE_LEN[t];
                    i += 1;
                    m &= m - 1;
                }
                i += m.count_ones() as usize;
                if mw + 1 < MASK_WORDS && need[mw + 1] != 0 {
                    // the skipped rest of this mask word: their sizes
                    let skip = m.count_ones() as usize;
                    for k in i - skip..i {
                        p += NIBBLE_LEN[(bytes[tags + k / 2] >> ((k & 1) * 4)) as usize & 15];
                    }
                }
            }
        }
        done[0] |= self.mask[0];
        done[1] |= self.mask[1];
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
    /// The deltas' words, packed (`pack_words`).
    bytes: Vec<u8>,
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
        self.deltas.capacity() * std::mem::size_of::<Delta>() + self.bytes.capacity()
    }

    /// The words the deltas hold.
    fn words(&self) -> usize {
        self.deltas.iter().map(|d| d.len()).sum()
    }

    /// Rewrite the sealed log through `f`, which sees each delta as its
    /// chunk, mask and values (in bit order) and may change them or add
    /// deltas (kept sorted by chunk). For the rare edits of a sealed log
    /// (`Arena::or_from`), not for restores.
    fn rewrite(&mut self, f: impl FnOnce(&mut Vec<(u32, [u64; MASK_WORDS], Vec<u64>)>)) {
        let mut buf = [0u64; CHUNK_WORDS];
        let mut v: Vec<(u32, [u64; MASK_WORDS], Vec<u64>)> = self
            .deltas
            .iter()
            .map(|d| {
                d.unpack(&self.bytes, &mut buf);
                (d.c, d.mask, buf[..d.len()].to_vec())
            })
            .collect();
        f(&mut v);
        v.sort_by_key(|e| e.0);
        let mut deltas = Vec::with_capacity(v.len());
        let mut bytes = Vec::with_capacity(self.bytes.len() + 16);
        for (c, mask, vals) in &v {
            debug_assert_eq!(
                vals.len(),
                mask.iter().map(|m| m.count_ones() as usize).sum::<usize>()
            );
            let at = bytes.len() as u32;
            pack_words(vals, &mut bytes);
            deltas.push(Delta {
                c: *c,
                at,
                mask: *mask,
            });
        }
        bytes.shrink_to_fit();
        self.deltas = deltas;
        self.bytes = bytes;
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
        // (the copy is tens of MB on a 1,000-page document: asked here too)
        if i % STOP_CHUNKS == STOP_CHUNKS - 1 && stop() {
            return None;
        }
        // SAFETY: `start` gives a whole chunk.
        let src = unsafe { std::slice::from_raw_parts(start(c), CHUNK_WORDS) };
        buf[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS].copy_from_slice(src);
    }
    // the wanted chunks' bits (a test per entry), and each one's index in
    // `cs` (a lookup per match, not a binary search)
    let mut want = vec![0u64; nchunks.div_ceil(64)];
    let mut slot = vec![0u32; nchunks];
    for (i, &c) in cs.iter().enumerate() {
        set_bit(&mut want, c as usize);
        slot[c as usize] = i as u32;
    }
    let mut done = vec![[0u64; MASK_WORDS]; cs.len()];
    for (k, log) in logs.iter().enumerate() {
        if k % STOP_LOGS == STOP_LOGS - 1 && stop() {
            return None;
        }
        for &(c, p) in &log.entries {
            if bit(&want, c as usize) {
                let i = slot[c as usize] as usize;
                // SAFETY: a slab chunk; a chunk of `buf`.
                unsafe {
                    apply_whole_under(p, buf.as_mut_ptr().add(i * CHUNK_WORDS), &mut done[i])
                };
            }
        }
        for d in &log.deltas {
            if bit(&want, d.c as usize) {
                let i = slot[d.c as usize] as usize;
                // SAFETY: a chunk of `buf`.
                unsafe {
                    d.apply_under(
                        &log.bytes,
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

/// Chunks copied between two questions to a `stop` (256 KB, tens of µs;
/// lane P4-TYPING-200WPM: `prepare_restore` held a keystroke 42 M cycles in
/// its copies, which asked nothing).
const STOP_CHUNKS: usize = 256;

/// `older` then `newer`, two adjacent sealed logs, as one: the state at
/// `older`'s checkpoint from the state after `newer`'s. Where both hold a
/// word, `older`'s value wins.
fn merge_sealed(older: &Log, newer: &Log) -> Log {
    let mut deltas = Vec::with_capacity(older.deltas.len() + newer.deltas.len());
    let mut bytes = Vec::with_capacity(older.bytes.len() + newer.bytes.len());
    // a delta of one log alone keeps its packed bytes as they are
    let copy = |d: &Delta, from: &[u8], bytes: &mut Vec<u8>| -> Delta {
        let at = bytes.len();
        let n = d.packed_len(from);
        bytes.extend_from_slice(&from[d.at as usize..d.at as usize + n]);
        Delta {
            c: d.c,
            at: at as u32,
            mask: d.mask,
        }
    };
    let (mut xv, mut yv) = ([0u64; CHUNK_WORDS], [0u64; CHUNK_WORDS]);
    let mut vals: Vec<u64> = Vec::with_capacity(CHUNK_WORDS);
    let (mut i, mut j) = (0, 0);
    let (a, b) = (&older.deltas, &newer.deltas);
    while i < a.len() || j < b.len() {
        if j == b.len() || (i < a.len() && a[i].c < b[j].c) {
            deltas.push(copy(&a[i], &older.bytes, &mut bytes));
            i += 1;
        } else if i == a.len() || b[j].c < a[i].c {
            deltas.push(copy(&b[j], &newer.bytes, &mut bytes));
            j += 1;
        } else {
            let (x, y) = (&a[i], &b[j]);
            x.unpack(&older.bytes, &mut xv);
            y.unpack(&newer.bytes, &mut yv);
            let at = bytes.len();
            let (mut kx, mut ky) = (0usize, 0usize);
            let mut mask = [0u64; MASK_WORDS];
            vals.clear();
            for (mw, m) in mask.iter_mut().enumerate() {
                let (mx, my) = (x.mask[mw], y.mask[mw]);
                *m = mx | my;
                let mut all = mx | my;
                while all != 0 {
                    let bitv = all.isolate_lowest_one();
                    let (in_x, in_y) = (mx & bitv != 0, my & bitv != 0);
                    if in_x {
                        vals.push(xv[kx]);
                        kx += 1;
                        if in_y {
                            ky += 1;
                        }
                    } else {
                        vals.push(yv[ky]);
                        ky += 1;
                    }
                    all &= all - 1;
                }
            }
            pack_words(&vals, &mut bytes);
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
    bytes.shrink_to_fit();
    Log {
        entries: Vec::new(),
        deltas,
        bytes,
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
    /// A restore worked out ahead of time (`prepare_restore`, `reattach`).
    prepared: Option<Prepared>,
    /// One `prepare_restore` stopped before it was done (`PartPrep`).
    part: Option<PartPrep>,
    /// `reattach` leaves a prepared restore to its target (default on;
    /// `FLASHTEX_NO_PREPARE=1` turns it off, for A/B).
    prepare_on_reattach: bool,
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
    /// The redo made ahead too (lane P4-PAGE-COST): for each of `cs` not
    /// written since the last checkpoint when it was prepared, a slab chunk
    /// holding the live chunk then (else null). A chunk the barrier has not
    /// saved since is still that at the restore; one it has is copied again.
    /// Empty: none made.
    pre: Vec<ChunkPtr>,
}

/// A `prepare_restore` that newer work stopped (a keystroke): how far it
/// got, kept so that the next `prepare_restore` to the same target, or the
/// restore itself, goes on from there instead of starting again (lane
/// P4-TYPING-200WPM (a): on *Infinite Descent* a preparation takes 300 to
/// 600 ms and the next keystroke came first, every time). Valid while
/// `Prepared` would be: the same target, the checkpoint list `ids`, no log
/// changed in place (`history_gen`); the open log may grow meanwhile, as
/// it may after a whole preparation, and is rewound through last (its
/// entries hold the chunks' values at the newest checkpoint, which a chunk
/// copied before or after its first write since then both end with).
/// Anything else discards it.
struct PartPrep {
    id: CheckpointId,
    ids: Vec<CheckpointId>,
    history_gen: u64,
    /// Logs from the target's on looked through for their chunks.
    collected: usize,
    /// The chunks found (sorted once all are found), and their bits.
    cs: Vec<u32>,
    seen: Vec<u64>,
    sorted: bool,
    /// The copies: how many taken from the live space, then the logs
    /// rewound through (oldest first), with each word's done bits.
    copied: usize,
    buf: Vec<u64>,
    done: Vec<[u64; MASK_WORDS]>,
    rewound: usize,
    /// `Prepared::pre`, as far as it got.
    pre: Vec<ChunkPtr>,
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

    fn saved_ref(&self) -> &[u8] {
        // SAFETY: as `saved`.
        unsafe { std::slice::from_raw_parts(self.saved, self.nchunks) }
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
        let _m = crate::memstat::scope(crate::memstat::tag::LOG);
        let mut entries = std::mem::take(&mut self.logs[i].entries);
        if entries.is_empty() {
            return;
        }
        debug_assert!(self.logs[i].deltas.is_empty(), "sealing a sealed log");
        entries.sort_unstable_by_key(|e| e.0);
        let mut deltas = Vec::with_capacity(entries.len());
        let mut bytes = Vec::with_capacity(entries.len() * CHUNK_WORDS / 2);
        let mut vals: Vec<u64> = Vec::with_capacity(CHUNK_WORDS);
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
                let at = bytes.len();
                vals.clear();
                for (mw, &m0) in mask.iter().enumerate() {
                    let mut m = m0;
                    while m != 0 {
                        vals.push(old[mw * 64 + m.trailing_zeros() as usize]);
                        m &= m - 1;
                    }
                }
                pack_words(&vals, &mut bytes);
                deltas.push(Delta {
                    c,
                    at: at as u32,
                    mask,
                });
            }
            self.slab.give(p);
        }
        deltas.shrink_to_fit();
        bytes.shrink_to_fit();
        let log = &mut self.logs[i];
        log.deltas = deltas;
        log.bytes = bytes;
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
                    unsafe { d.apply_under(&log.bytes, live, dn) };
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
    /// is not in the live chain. Stopped, what it did is kept (`PartPrep`):
    /// the next call for the same target, or the restore, goes on from
    /// there.
    fn prepare_restore(&mut self, id: CheckpointId, stop: &mut dyn FnMut() -> bool) -> bool {
        let _m = crate::memstat::scope(crate::memstat::tag::PREPARE);
        self.set_prepared(None);
        let part = self.take_part(id);
        let Some(k) = self.index_of(id) else {
            if let Some(p) = part {
                self.give_pre(p.pre);
            }
            return false;
        };
        let mut p = part.unwrap_or_else(|| PartPrep {
            id,
            ids: self.ids.clone(),
            history_gen: self.history_gen,
            collected: 0,
            cs: Vec::new(),
            seen: vec![0u64; self.nchunks.div_ceil(64)],
            sorted: false,
            copied: 0,
            buf: Vec::new(),
            done: Vec::new(),
            rewound: 0,
            pre: Vec::new(),
        });
        if self.prepare_go_on(&mut p, k, stop) {
            self.prepared = Some(Prepared {
                id,
                ids: p.ids,
                history_gen: p.history_gen,
                cs: p.cs,
                buf: p.buf,
                pre: p.pre,
            });
            true
        } else {
            self.part = Some(p);
            false
        }
    }

    /// Give back a kept `PartPrep`, if any: its copies were made from logs
    /// that are being replaced. (`take_part`'s check of the checkpoint list
    /// would not always catch it: a `reattach` of a branch holding only its
    /// target, with no checkpoint taken while it was detached, leaves the
    /// same list over other logs and another live state.)
    fn drop_part(&mut self) {
        if let Some(p) = self.part.take() {
            self.give_pre(p.pre);
        }
    }

    /// The kept `PartPrep` if it is for `id` and still valid; any other is
    /// given back.
    fn take_part(&mut self, id: CheckpointId) -> Option<PartPrep> {
        let p = self.part.take()?;
        if p.id == id && p.ids == self.ids && p.history_gen == self.history_gen {
            return Some(p);
        }
        self.give_pre(p.pre);
        None
    }

    /// `prepare_restore`'s work on `p` (the logs from index `k` on), from
    /// where it stopped last: true when done, false when `stop` said to
    /// stop (`p` holds how far it got).
    fn prepare_go_on(
        &mut self,
        p: &mut PartPrep,
        k: usize,
        stop: &mut dyn FnMut() -> bool,
    ) -> bool {
        // the chunks the logs from the target's on hold
        let n = self.logs.len() - k;
        while p.collected < n {
            if p.collected % STOP_LOGS == STOP_LOGS - 1 && stop() {
                return false;
            }
            for c in self.logs[k + p.collected].chunk_ids() {
                if !bit(&p.seen, c as usize) {
                    set_bit(&mut p.seen, c as usize);
                    p.cs.push(c);
                }
            }
            p.collected += 1;
        }
        if !p.sorted {
            p.cs.sort_unstable();
            p.buf = vec![0u64; p.cs.len() * CHUNK_WORDS];
            p.done = vec![[0u64; MASK_WORDS]; p.cs.len()];
            p.sorted = true;
        }
        // their live copies
        let base = self.base as usize;
        while p.copied < p.cs.len() {
            if p.copied % STOP_CHUNKS == STOP_CHUNKS - 1 && stop() {
                return false;
            }
            let (i, c) = (p.copied, p.cs[p.copied]);
            let live = (base + ((c as usize) << CHUNK_SHIFT)) as *const u64;
            // SAFETY: a whole live chunk.
            let src = unsafe { std::slice::from_raw_parts(live, CHUNK_WORDS) };
            p.buf[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS].copy_from_slice(src);
            p.copied += 1;
        }
        // rewound through the logs, oldest first, each word once (as
        // `rewound_until`; the open log last, whole, as it is now)
        while p.rewound < n {
            if p.rewound % STOP_LOGS == STOP_LOGS - 1 && stop() {
                return false;
            }
            let log = &self.logs[k + p.rewound];
            for &(c, q) in &log.entries {
                if bit(&p.seen, c as usize) {
                    let i = p.cs.binary_search(&c).unwrap();
                    // SAFETY: a slab chunk; a chunk of `buf`.
                    unsafe {
                        apply_whole_under(
                            q,
                            p.buf.as_mut_ptr().add(i * CHUNK_WORDS),
                            &mut p.done[i],
                        )
                    };
                }
            }
            for d in &log.deltas {
                if bit(&p.seen, d.c as usize) {
                    let i = p.cs.binary_search(&d.c).unwrap();
                    // SAFETY: a chunk of `buf`.
                    unsafe {
                        d.apply_under(
                            &log.bytes,
                            p.buf.as_mut_ptr().add(i * CHUNK_WORDS),
                            &mut p.done[i],
                        )
                    };
                }
            }
            p.rewound += 1;
        }
        // the redo made ahead
        while p.pre.len() < p.cs.len() {
            if !p.pre.is_empty() && stop() {
                return false;
            }
            let from = p.pre.len();
            let to = (from + STOP_CHUNKS).min(p.cs.len());
            let more = self.pre_redo(&p.cs[from..to]);
            p.pre.extend(more);
        }
        p.done = Vec::new();
        p.seen = Vec::new();
        true
    }

    /// `Prepared::pre` for chunks `cs`: a copy of each live chunk the
    /// barrier has not saved since the last checkpoint.
    fn pre_redo(&mut self, cs: &[u32]) -> Vec<ChunkPtr> {
        let mut pre = Vec::with_capacity(cs.len());
        for &c in cs {
            if self.saved()[c as usize] != 0 {
                pre.push(std::ptr::null_mut());
                continue;
            }
            let d = self.slab.take();
            // SAFETY: a live chunk and a slab chunk, CHUNK_WORDS words each.
            unsafe { std::ptr::copy_nonoverlapping(self.chunk_ptr(c as usize), d, CHUNK_WORDS) };
            pre.push(d);
        }
        pre
    }

    /// Replace the prepared restore, giving the old one's redo chunks back.
    fn set_prepared(&mut self, p: Option<Prepared>) {
        if let Some(old) = std::mem::replace(&mut self.prepared, p) {
            self.give_pre(old.pre);
        }
    }

    fn give_pre(&mut self, pre: Vec<ChunkPtr>) {
        for d in pre {
            if !d.is_null() {
                self.slab.give(d);
            }
        }
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
        // whether each redo chunk still needs the live chunk copied in (not
        // when `Prepared::pre` made it and the chunk is unchanged since)
        let mut fresh: Vec<bool> = Vec::with_capacity(p.cs.len() + extra.len());
        for (i, &c) in p.cs.iter().enumerate() {
            match p.pre.get(i).copied().filter(|d| !d.is_null()) {
                Some(d) => {
                    redo.push((c, d));
                    fresh.push(self.saved()[c as usize] != 0);
                }
                None => {
                    redo.push((c, self.slab.take()));
                    fresh.push(true);
                }
            }
        }
        for &(c, _) in &extra {
            redo.push((c, self.slab.take()));
            fresh.push(true);
        }
        let work: Vec<(usize, usize, usize, bool)> = redo
            .iter()
            .enumerate()
            .map(|(i, &(c, r))| {
                let src = if i < p.cs.len() {
                    p.buf[i * CHUNK_WORDS..].as_ptr() as usize
                } else {
                    extra[i - p.cs.len()].1 as usize
                };
                (
                    base + ((c as usize) << CHUNK_SHIFT),
                    r as usize,
                    src,
                    fresh[i],
                )
            })
            .collect();
        let job = |k: usize| {
            let (live, r, src, fresh) = work[k];
            // SAFETY: distinct live chunks, their own redo chunks, and
            // sources nobody writes meanwhile (the prepared buffer, the open
            // log's slab chunks).
            unsafe {
                if fresh {
                    std::ptr::copy_nonoverlapping(live as *const u64, r as *mut u64, CHUNK_WORDS);
                }
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
        self.drop_part();
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
        let mut prepared = self.prepared.take();
        // a preparation newer work stopped: finished here, from where it got
        // (a whole one, when there is one, is newer: `prepare_restore` drops
        // the other kind) -- once its rewind is done. Before that, going on
        // costs more than the plain restore: the rest of a serial rewind
        // into a copy of every chunk, then the copies in, against `rewind`'s
        // one parallel pass over the live space (measured on Infinite
        // Descent x2 with 60 ms between keys: the restore's engine-thread
        // instructions 82 M plain, 358 M going on from early parts).
        match self.take_part(id) {
            Some(p) if prepared.is_some() => self.give_pre(p.pre),
            Some(p) if p.rewound < self.logs.len() - k => {
                if std::env::var_os("FLASHTEX_INCR_DEBUG").is_some() {
                    eprintln!(
                        "[arena] preparation for {id} dropped: {}/{} logs rewound",
                        p.rewound,
                        self.logs.len() - k
                    );
                }
                self.give_pre(p.pre);
            }
            Some(mut p) => {
                if std::env::var_os("FLASHTEX_INCR_DEBUG").is_some() {
                    eprintln!(
                        "[arena] preparation for {id} goes on (logs {}/{} looked through, \
                         {}/{} copied, {} rewound, {} redo)",
                        p.collected,
                        self.logs.len() - k,
                        p.copied,
                        p.cs.len(),
                        p.rewound,
                        p.pre.len()
                    );
                }
                if self.prepare_go_on(&mut p, k, &mut || false) {
                    prepared = Some(Prepared {
                        id,
                        ids: p.ids,
                        history_gen: p.history_gen,
                        cs: p.cs,
                        buf: p.buf,
                        pre: p.pre,
                    });
                } else {
                    // (not reached: nothing asks it to stop)
                    self.give_pre(p.pre);
                }
            }
            None => {}
        }
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
        let prepared = match prepared {
            Some(p) if p.id == id && p.history_gen == self.history_gen && p.ids == self.ids => {
                Some(p)
            }
            Some(p) => {
                self.give_pre(p.pre);
                None
            }
            None => None,
        };
        if let Some(p) = &prepared {
            if std::env::var_os("FLASHTEX_VERIFY_PREPARED").is_some() {
                if let Err(e) = self.verify_prepared_use(p, k) {
                    // (a verify mode: loud, so that a sweep fails)
                    eprintln!("{e}");
                    std::process::abort();
                }
            }
        }
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
        self.drop_part();
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
        self.drop_part();
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
        // The live state is the target's now. The chunks the branch's logs
        // hold are the redo's (the restore that detached the branch saved
        // each chunk those logs hold, and nothing else): their values here
        // are a prepared restore to the target (`Prepared`, as
        // `prepare_restore` would work it out from the reattached logs), so
        // that the next restore there -- the next keystroke in the same
        // paragraph, after a preempted compile was abandoned -- copies them
        // in instead of rewinding the logs to the document's end.
        let prepared = if self.prepare_on_reattach {
            let mut cs: Vec<u32> = redo.iter().map(|&(c, _)| c).collect();
            cs.sort_unstable();
            let mut buf = vec![0u64; cs.len() * CHUNK_WORDS];
            for (i, &c) in cs.iter().enumerate() {
                // SAFETY: a live chunk of CHUNK_WORDS words.
                let src =
                    unsafe { std::slice::from_raw_parts(self.chunk_ptr(c as usize), CHUNK_WORDS) };
                buf[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS].copy_from_slice(src);
            }
            Some((cs, buf))
        } else {
            None
        };
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
        let prepared = prepared.map(|(cs, buf)| {
            let pre = self.pre_redo(&cs);
            Prepared {
                id: target,
                ids: self.ids.clone(),
                history_gen: self.history_gen,
                cs,
                buf,
                pre,
            }
        });
        self.set_prepared(prepared);
        if std::env::var_os("FLASHTEX_VERIFY_PREPARED").is_some() {
            if let Err(e) = self.verify_prepared() {
                // (a verify mode: loud, so that a sweep fails)
                eprintln!("{e}");
                std::process::abort();
            }
        }
        Ok(())
    }

    /// `FLASHTEX_VERIFY_PREPARED`, where `restore_branch` uses `p` (the
    /// logs from index `k` on): it holds every chunk the sealed logs from
    /// its checkpoint on hold, and only chunks those logs or the open one
    /// hold (the open log may have taken new chunks since: their pre-images
    /// there are their values at the checkpoint, `rewind_prepared`); each
    /// chunk is what rewinding the logs as they are now gives; and each
    /// redo made ahead of a chunk the barrier has not saved since is the
    /// live chunk.
    fn verify_prepared_use(&self, p: &Prepared, k: usize) -> Result<(), String> {
        let logs = &self.logs[k..];
        let (sealed, open) = logs.split_at(logs.len().saturating_sub(1));
        let mut must: Vec<u32> = sealed.iter().flat_map(|l| l.chunk_ids()).collect();
        must.sort_unstable();
        must.dedup();
        if let Some(c) = must.iter().find(|c| p.cs.binary_search(c).is_err()) {
            return Err(format!(
                "FLASHTEX_VERIFY_PREPARED: chunk {c} of a sealed log is not prepared"
            ));
        }
        let mut open_ids: Vec<u32> = open.iter().flat_map(|l| l.chunk_ids()).collect();
        open_ids.sort_unstable();
        let held = |c: &u32| must.binary_search(c).is_ok() || open_ids.binary_search(c).is_ok();
        if let Some(c) = p.cs.iter().find(|c| !held(c)) {
            return Err(format!(
                "FLASHTEX_VERIFY_PREPARED: chunk {c} is prepared, no log holds it"
            ));
        }
        let live = |c: u32| self.chunk_ptr(c as usize) as *const u64;
        if rewound(self.nchunks, &p.cs, &live, logs) != p.buf {
            return Err(
                "FLASHTEX_VERIFY_PREPARED: a prepared chunk differs from the rewound one".into(),
            );
        }
        let saved = self.saved_ref();
        for (i, &d) in p.pre.iter().enumerate() {
            let c = p.cs[i] as usize;
            if d.is_null() || saved[c] != 0 {
                continue;
            }
            // SAFETY: a slab chunk and a live chunk, CHUNK_WORDS words each.
            let (a, b) = unsafe {
                (
                    std::slice::from_raw_parts(d as *const u64, CHUNK_WORDS),
                    std::slice::from_raw_parts(self.chunk_ptr(c), CHUNK_WORDS),
                )
            };
            if a != b {
                return Err(format!(
                    "FLASHTEX_VERIFY_PREPARED: the redo made ahead of chunk {c} is stale"
                ));
            }
        }
        Ok(())
    }

    /// `FLASHTEX_VERIFY_PREPARED`: the prepared restore holds exactly the
    /// chunks the logs from its checkpoint on hold, each as rewinding those
    /// logs gives it.
    fn verify_prepared(&self) -> Result<(), String> {
        let Some(p) = &self.prepared else {
            return Ok(());
        };
        let k = self
            .index_of(p.id)
            .ok_or("verify: the prepared checkpoint is gone")?;
        let mut cs: Vec<u32> = self.logs[k..].iter().flat_map(|l| l.chunk_ids()).collect();
        cs.sort_unstable();
        cs.dedup();
        if cs != p.cs {
            return Err(format!(
                "FLASHTEX_VERIFY_PREPARED: {} chunks prepared, the logs hold {}",
                p.cs.len(),
                cs.len()
            ));
        }
        let live = |c: u32| self.chunk_ptr(c as usize) as *const u64;
        if rewound(self.nchunks, &cs, &live, &self.logs[k..]) != p.buf {
            return Err(
                "FLASHTEX_VERIFY_PREPARED: a prepared chunk differs from the rewound one".into(),
            );
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

    /// `retain` for a detached branch (the old run's future): drop every
    /// checkpoint of `b` that `keep` rejects, except its first (the restore
    /// target, in the live chain too) and its newest, merging each dropped
    /// log into its predecessor. What the branch is for stays exact: the
    /// old run's state at every checkpoint kept (rewound from its end
    /// through the logs after it, which are untouched or merged), the
    /// chunks it wrote after the target (a merged log holds the union of
    /// its parts'), and `reattach`, which puts back a chain with fewer
    /// checkpoints.
    fn retain_branch(&mut self, b: &mut Branch, keep: &dyn Fn(CheckpointId) -> bool) {
        let _m = crate::memstat::scope(crate::memstat::tag::LOG);
        let n = b.ids.len();
        let ids = std::mem::take(&mut b.ids);
        let logs = std::mem::take(&mut b.logs);
        let mut out_ids = Vec::with_capacity(n);
        let mut out_logs: Vec<Log> = Vec::with_capacity(n);
        for (i, (id, log)) in ids.into_iter().zip(logs).enumerate() {
            let sealed = |l: &Log| l.entries.is_empty();
            let dst = out_logs.last_mut();
            match dst {
                Some(dst) if i + 1 < n && !keep(id) && sealed(dst) && sealed(&log) => {
                    let merged = merge_sealed(dst, &log);
                    self.sealed_bytes += merged.sealed_bytes();
                    self.sealed_bytes -= dst.sealed_bytes() + log.sealed_bytes();
                    *dst = merged;
                }
                _ => {
                    out_ids.push(id);
                    out_logs.push(log);
                }
            }
        }
        b.ids = out_ids;
        b.logs = out_logs;
    }

    /// Drop every checkpoint `keep` rejects, except the newest, merging each
    /// dropped log into its predecessor (the older value of a word wins).
    fn retain(&mut self, keep: &dyn Fn(CheckpointId) -> bool) {
        let _m = crate::memstat::scope(crate::memstat::tag::LOG);
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

/// Arrays that are not the engine's state, which `Arena::diff_branch`
/// leaves out: the display list's side table (changes/displaylist.ch),
/// source positions that nothing TeX computes reads. Only its chunks that
/// hold nothing else are left out. The convergence test compares it node
/// by node instead (`crate::iso`: a node still to be shipped keeps its
/// position from the jump), rewinding only the chunks of live nodes
/// ([`Arena::branch_old_chunks`]). The convergence
/// jump adopts them like every other array ([`Arena::diff_branch_all`]):
/// the side table must describe the nodes of the `mem` it adopts (left out,
/// nodes live at the jump kept the new run's positions for the old run's
/// addresses: 1,252 glyphs with wrong source lines on plain-10, review of
/// #1300).
const UNSTATED: &[&str] = &["dl_side"];

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
    /// Old checkpoints' chunk values, as the convergence tests rewound
    /// them (`OldCache`).
    old_cache: std::cell::RefCell<OldCache>,
}

/// The value of chunks at a few checkpoints, as `diff_branch_inner` rewound
/// them from the old run's end (lane LIVE-30MS). A retained checkpoint's
/// state never changes: a restore only moves it between the live chain and a
/// detached branch, the convergence jump adopts the old run's state at its
/// checkpoint whole (`Globals::redo_to_remapped`), a merge drops the
/// checkpoint itself, and the only change in place, `or_from`, bumps
/// `history_gen`, which every entry carries. Ids are never reused. So a
/// value kept here is the one a rewind would give again, and typing in one
/// place, which tests against the same old checkpoints keystroke after
/// keystroke, rewinds the old future (O(pages after the edit)) only for
/// chunks it has not seen there. `FLASHTEX_VERIFY_OLDCACHE=1` rewinds
/// every chunk anyway and aborts the process on any difference.
#[derive(Default)]
struct OldCache {
    /// (checkpoint, history_gen, chunk -> index into `words` / CHUNK_WORDS),
    /// most recently used last.
    at: Vec<OldAt>,
    /// Chunks `diff_branch_inner` took from here, and rewound, over this
    /// space's life (for DONE's stages: cumulative). Per space, not
    /// process-wide: tests running in parallel each read their own.
    hits: u64,
    misses: u64,
}

/// One checkpoint's kept chunks: (checkpoint, history_gen, chunk -> index
/// into the words / CHUNK_WORDS, the words).
type OldAt = (CheckpointId, u64, HashMap<u32, usize>, Vec<u64>);

/// Checkpoints `OldCache` keeps, and the chunks it keeps in all (1 KB each).
const OLD_CACHE_CHECKPOINTS: usize = 4;
const OLD_CACHE_CHUNKS: usize = 16 * 1024;

impl OldCache {
    fn get(&self, id: CheckpointId, gen: u64, c: u32) -> Option<&[u64]> {
        let (_, _, ix, w) = self.at.iter().find(|e| e.0 == id && e.1 == gen)?;
        ix.get(&c)
            .map(|&i| &w[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS])
    }

    /// Keep chunks `cs` (their values in `buf`, CHUNK_WORDS each) at `id`.
    fn put(&mut self, id: CheckpointId, gen: u64, cs: &[u32], buf: &[u64]) {
        self.at.retain(|e| e.1 == gen);
        match self.at.iter().position(|e| e.0 == id) {
            Some(k) => {
                let e = self.at.remove(k);
                self.at.push(e);
            }
            None => self.at.push((id, gen, HashMap::new(), vec![])),
        }
        // Room first, least recently used checkpoints out (never the one
        // being filled, now the last): the cache never holds more than
        // OLD_CACHE_CHUNKS chunks, also while filling.
        let new = cs.len().min(OLD_CACHE_CHUNKS);
        while self.at.len() > 1
            && (self.at.len() > OLD_CACHE_CHECKPOINTS
                || self.at.iter().map(|e| e.2.len()).sum::<usize>() + new > OLD_CACHE_CHUNKS)
        {
            self.at.remove(0);
        }
        let room = OLD_CACHE_CHUNKS.saturating_sub(self.at.iter().map(|e| e.2.len()).sum());
        let e = self.at.last_mut().expect("the entry being filled");
        let mut added = 0;
        for (i, &c) in cs.iter().enumerate() {
            if added == room {
                break;
            }
            if e.2.contains_key(&c) {
                continue;
            }
            e.2.insert(c, e.3.len() / CHUNK_WORDS);
            e.3.extend_from_slice(&buf[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS]);
            added += 1;
        }
    }
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
        let map = os::alloc_zeroed(map_len);
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
            part: None,
            history_gen: 0,
            prepare_on_reattach: std::env::var_os("FLASHTEX_NO_PREPARE").is_none(),
        });
        Arena {
            core: Box::into_raw(core),
            scalar_bytes,
            extra: None,
            regions: vec![],
            old_cache: Default::default(),
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
            let held = prev
                .deltas
                .iter()
                .any(|d| d.c == c && d.mask[mw] >> b & 1 == 1);
            if !held {
                let before = prev.sealed_bytes();
                prev.rewrite(|ds| match ds.iter_mut().find(|e| e.0 == c) {
                    Some((_, mask, vals)) => {
                        // the word's place among the delta's words, in bit order
                        let idx = mask[..mw]
                            .iter()
                            .map(|m| m.count_ones() as usize)
                            .sum::<usize>()
                            + (mask[mw] & ((1u64 << b) - 1)).count_ones() as usize;
                        vals.insert(idx, v);
                        mask[mw] |= 1u64 << b;
                    }
                    None => {
                        let mut mask = [0u64; MASK_WORDS];
                        mask[mw] = 1u64 << b;
                        ds.push((c, mask, vec![v]));
                    }
                });
                core.sealed_bytes = core.sealed_bytes + prev.sealed_bytes() - before;
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
            if log
                .deltas
                .iter()
                .any(|d| d.c == c && d.mask[mw] >> b & 1 == 1)
            {
                let before = log.sealed_bytes();
                log.rewrite(|ds| {
                    for (dc, mask, vals) in ds.iter_mut() {
                        if *dc == c && mask[mw] >> b & 1 == 1 {
                            let idx = mask[..mw]
                                .iter()
                                .map(|m| m.count_ones() as usize)
                                .sum::<usize>()
                                + (mask[mw] & ((1u64 << b) - 1)).count_ones() as usize;
                            vals[idx] |= bits;
                            n += 1;
                        }
                    }
                });
                core.sealed_bytes = core.sealed_bytes + log.sealed_bytes() - before;
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

    pub fn retain_branch(&mut self, b: &mut Branch, keep: &dyn Fn(CheckpointId) -> bool) {
        self.core_mut().retain_branch(b, keep)
    }

    pub fn reattach(&mut self, b: Branch) -> Result<(), String> {
        self.core_mut().reattach(b)
    }

    pub fn retain(&mut self, keep: &dyn Fn(CheckpointId) -> bool) {
        self.core_mut().retain(keep)
    }

    /// Drop every checkpoint and log: the space becomes plain memory again.
    /// Drop the old checkpoints' kept chunks (`OldCache`): the host does
    /// when it has been idle a while (its memory goes back to the system).
    pub fn drop_old_cache(&self) {
        self.old_cache.borrow_mut().at = Vec::new();
    }

    /// The convergence tests' old chunks taken from `OldCache` and rewound,
    /// over this space's life: (kept, rewound).
    pub fn old_cache_counts(&self) -> (u64, u64) {
        let c = self.old_cache.borrow();
        (c.hits, c.misses)
    }

    /// Bytes the old checkpoints' kept chunks hold.
    pub fn old_cache_bytes(&self) -> usize {
        self.old_cache
            .borrow()
            .at
            .iter()
            .map(|e| e.3.capacity() * 8)
            .sum()
    }

    pub fn forget_checkpoints(&mut self) {
        self.old_cache.borrow_mut().at.clear();
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
        Ok(View {
            arena: self,
            cs,
            bufs,
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
        self.diff_branch_inner(b, old, stop, true)
    }

    /// [`diff_branch`](Self::diff_branch) including the `UNSTATED` arrays
    /// (the convergence jump adopts them too).
    pub fn diff_branch_all(&self, b: &Branch, old: CheckpointId) -> Result<ChunkDiff, String> {
        self.diff_branch_inner(b, old, &mut || false, false)?
            .ok_or_else(|| "stopped".to_string())
    }

    /// [`diff_branch_all`](Self::diff_branch_all), asking `stop` as it
    /// rewinds: `Ok(None)` when it said to stop.
    pub fn diff_branch_all_until(
        &self,
        b: &Branch,
        old: CheckpointId,
        stop: &mut dyn FnMut() -> bool,
    ) -> Result<Option<ChunkDiff>, String> {
        self.diff_branch_inner(b, old, stop, false)
    }

    /// The chunks of region `name` that either run wrote since the restore
    /// target of `b` (the old run up to its checkpoint `old`, the live run
    /// since the restore), as a bitmap by chunk: every other chunk of the
    /// region holds the same words in the old run's state at `old` as now.
    pub fn branch_written(
        &self,
        b: &Branch,
        old: CheckpointId,
        name: &str,
    ) -> Result<Vec<u64>, String> {
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
        let Some(reg) = self.regions.iter().find(|r| r.name == name) else {
            return Ok(vec![]);
        };
        let (lo, hi) = (
            reg.off / CHUNK_BYTES,
            (reg.off + reg.bytes).div_ceil(CHUNK_BYTES),
        );
        let mut seen = vec![0u64; core.nchunks.div_ceil(64)];
        for log in b.logs[..jj].iter().chain(&core.logs[kr..]) {
            for c in log.chunk_ids() {
                if (lo..hi).contains(&(c as usize)) {
                    set_bit(&mut seen, c as usize);
                }
            }
        }
        Ok(seen)
    }

    /// The old run's value at its checkpoint `old` of each chunk of `cs`
    /// (sorted, each written since the restore target: `branch_written`),
    /// rewound only for those chunks: [`diff_branch`](Self::diff_branch)'s
    /// rule for one chunk. `Ok(None)` when `stop` said to stop.
    pub fn branch_old_chunks(
        &self,
        b: &Branch,
        old: CheckpointId,
        cs: &[u32],
        stop: &mut dyn FnMut() -> bool,
    ) -> Result<Option<HashMap<u32, Vec<u64>>>, String> {
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
        let want: std::collections::HashSet<u32> = cs.iter().copied().collect();
        let mut redo_of: HashMap<u32, *const u64> = HashMap::new();
        for &(c, p) in &b.redo {
            if want.contains(&c) {
                redo_of.insert(c, p as *const u64);
            }
        }
        let (in_old, at_r): (Vec<u32>, Vec<u32>) = cs.iter().partition(|c| redo_of.contains_key(c));
        let n = core.nchunks;
        let Some(o) = rewound_until(n, &in_old, &|c| redo_of[&c], &b.logs[jj..], stop) else {
            return Ok(None);
        };
        let live = |c: u32| core.chunk_ptr(c as usize) as *const u64;
        let Some(a) = rewound_until(n, &at_r, &live, &core.logs[kr..], stop) else {
            return Ok(None);
        };
        let mut out = HashMap::with_capacity(cs.len());
        for (i, &c) in in_old.iter().enumerate() {
            out.insert(c, o[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS].to_vec());
        }
        for (i, &c) in at_r.iter().enumerate() {
            out.insert(c, a[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS].to_vec());
        }
        Ok(Some(out))
    }

    fn diff_branch_inner(
        &self,
        b: &Branch,
        old: CheckpointId,
        stop: &mut dyn FnMut() -> bool,
        skip_unstated: bool,
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
        // Chunks wholly inside an array the comparison leaves out
        // (`UNSTATED`) are not candidates. Their old values are rewound
        // anyway, in the same pass, and kept (`OldCache`): a test that
        // passes is followed by the jump's comparison at the same
        // checkpoint, which reads them (`diff_branch_all`), and it then
        // rewinds nothing (lane P4-TYPING-200WPM (b): on *Infinite Descent*
        // each rewind is a pass over 3,100 logs, 4.5 M entries, ~140 M
        // instructions).
        let mut unstated = vec![0u64; n.div_ceil(64)];
        for r in self
            .regions
            .iter()
            .filter(|r| skip_unstated && UNSTATED.contains(&r.name))
        {
            for c in r.off.div_ceil(CHUNK_BYTES)..(r.off + r.bytes) / CHUNK_BYTES {
                set_bit(&mut unstated, c);
            }
        }
        let mut extra: Vec<u32> = Vec::new();
        for log in b.logs[..jj].iter().chain(&core.logs[kr..]) {
            for c in log.chunk_ids() {
                if !bit(&seen, c as usize) {
                    set_bit(&mut seen, c as usize);
                    if bit(&unstated, c as usize) {
                        extra.push(c);
                    } else {
                        cand.push(c);
                    }
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
        extra.retain(|c| redo_of.contains_key(c));
        let t = std::time::Instant::now();
        // The old run's values at `old`: those `OldCache` holds, the rest
        // rewound from the old run's end (every one with
        // FLASHTEX_VERIFY_OLDCACHE, which compares).
        let gen = core.history_gen;
        let verify = std::env::var_os("FLASHTEX_VERIFY_OLDCACHE").is_some();
        let (cached, misses): (Vec<u32>, Vec<u32>) = {
            let cache = self.old_cache.borrow();
            in_old
                .iter()
                .partition(|&&c| cache.get(old, gen, c).is_some())
        };
        // (the unstated ones not kept yet, with them)
        let extra_misses: Vec<u32> = {
            let cache = self.old_cache.borrow();
            extra
                .iter()
                .copied()
                .filter(|&c| cache.get(old, gen, c).is_none())
                .collect()
        };
        let mut rewind: Vec<u32> = if verify {
            in_old.clone()
        } else {
            misses.clone()
        };
        if !misses.is_empty() || verify {
            rewind.extend(&extra_misses);
            rewind.sort_unstable();
        }
        // (nothing to rewind: no pass over the logs)
        let rew = if rewind.is_empty() {
            Some(Vec::new())
        } else {
            rewound_until(n, &rewind, &|c| redo_of[&c], &b.logs[jj..], stop)
        };
        let Some(rew) = rew else {
            return Ok(None);
        };
        let mut old_buf = vec![0u64; in_old.len() * CHUNK_WORDS];
        {
            let cache = self.old_cache.borrow();
            for (i, &c) in in_old.iter().enumerate() {
                let dst = &mut old_buf[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS];
                let mine = rewind.binary_search(&c).ok();
                if let Some(r) = mine {
                    dst.copy_from_slice(&rew[r * CHUNK_WORDS..(r + 1) * CHUNK_WORDS]);
                }
                let mine = mine.is_some();
                if let Some(v) = cache.get(old, gen, c) {
                    if mine && v != &dst[..] {
                        // (a verify mode: loud, so that a sweep fails)
                        eprintln!(
                            "FLASHTEX_VERIFY_OLDCACHE: chunk {c} at checkpoint {old} differs from its rewind"
                        );
                        std::process::abort();
                    }
                    dst.copy_from_slice(v);
                }
            }
        }
        {
            let mut cache = self.old_cache.borrow_mut();
            cache.hits += cached.len() as u64;
            cache.misses += misses.len() as u64;
        }
        // (kept: every one rewound that was not, in `rewind`'s order)
        let keep: Vec<u32> = {
            let cache = self.old_cache.borrow();
            rewind
                .iter()
                .copied()
                .filter(|&c| cache.get(old, gen, c).is_none())
                .collect()
        };
        if !keep.is_empty() {
            let mut mb = Vec::with_capacity(keep.len() * CHUNK_WORDS);
            for &c in &keep {
                let i = rewind.binary_search(&c).expect("a rewound chunk");
                mb.extend_from_slice(&rew[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS]);
            }
            self.old_cache.borrow_mut().put(old, gen, &keep, &mb);
        }
        let t_old = t.elapsed();
        let live = |c: u32| core.chunk_ptr(c as usize) as *const u64;
        let r_buf = rewound(n, &at_r, &live, &core.logs[kr..]);
        if std::env::var_os("FLASHTEX_INCR_DEBUG").is_some() {
            let entries: usize = b.logs[jj..]
                .iter()
                .map(|l| l.entries.len() + l.deltas.len())
                .sum();
            eprintln!(
                "[arena] diff: {} candidates ({} in the old run, {} of them kept, rewound through {} logs, {} entries: {:.2} ms; {} at the restart point through {} logs: {:.2} ms)",
                cand.len(),
                in_old.len(),
                cached.len(),
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

    /// Memory accounting (`crate::memstat`, lane P4-MEMORY): the word
    /// space (reserved, ever written, resident), the slab (mapped, in use,
    /// resident), the sealed logs of the live chain and of `branch` (their
    /// heap bytes and entries), the open log, and the prepared restore.
    pub fn mem_stats(&self, branch: Option<&Branch>) -> Vec<(&'static str, i64)> {
        let c = self.core();
        let touched = c.touched.iter().filter(|&&t| t != 0).count();
        let space_res = crate::memstat::resident(c.base, c.bytes).unwrap_or(0);
        let slab_res: usize = c
            .slab
            .blocks
            .iter()
            .map(|&b| crate::memstat::resident(b, SLAB_BLOCK_CHUNKS * CHUNK_BYTES).unwrap_or(0))
            .sum();
        let sum = |logs: &[Log]| -> (usize, usize, usize) {
            logs.iter().fold((0, 0, 0), |(b, d, w), l| {
                (b + l.sealed_bytes(), d + l.deltas.len(), w + l.words())
            })
        };
        let (live_b, live_d, live_w) = sum(&c.logs);
        let (br_b, br_d, br_w) = branch.map_or((0, 0, 0), |b| sum(&b.logs));
        let open = c.logs.last().map_or(0, |l| l.entries.len());
        let prepared = c.prepared.as_ref().map_or(0, |p| {
            p.buf.capacity() * 8
                + p.cs.capacity() * 4
                + p.ids.capacity() * 8
                + p.pre.iter().filter(|d| !d.is_null()).count() * CHUNK_BYTES
        }) + c.part.as_ref().map_or(0, |p| {
            // (a stopped one, `PartPrep`, kept to go on)
            p.buf.capacity() * 8
                + p.cs.capacity() * 4
                + p.ids.capacity() * 8
                + p.seen.capacity() * 8
                + p.done.capacity() * MASK_WORDS * 8
                + p.pre.iter().filter(|d| !d.is_null()).count() * CHUNK_BYTES
        });
        vec![
            ("space_reserved", c.bytes as i64),
            ("space_touched", (touched * CHUNK_BYTES) as i64),
            ("space_resident", space_res as i64),
            (
                "slab_mapped",
                (c.slab.blocks.len() * SLAB_BLOCK_CHUNKS * CHUNK_BYTES) as i64,
            ),
            ("slab_live", (c.slab.live * CHUNK_BYTES) as i64),
            ("slab_resident", slab_res as i64),
            ("open_chunks", open as i64),
            (
                "branch_redo_chunks",
                branch.map_or(0, |b| b.redo.len()) as i64,
            ),
            ("checkpoints", c.ids.len() as i64),
            ("sealed_bytes", c.sealed_bytes as i64),
            ("chain_sealed", live_b as i64),
            ("chain_deltas", live_d as i64),
            ("chain_words", live_w as i64),
            (
                "branch_checkpoints",
                branch.map_or(0, |b| b.ids.len()) as i64,
            ),
            ("branch_sealed", br_b as i64),
            ("branch_deltas", br_d as i64),
            ("branch_words", br_w as i64),
            ("prepared", prepared as i64),
            (
                "bookkeeping",
                (c.touched.len() + c.nchunks + c.mark.len() * 8 + c.slot.len() * 4) as i64,
            ),
        ]
    }

    /// The resident bytes of each region of the word space (`mincore`),
    /// largest first, for the regions with any (lane MEM-FOOTPRINT: which of
    /// TeX's fixed-size arrays the run actually touched).
    pub fn region_residency(&self) -> Vec<(&'static str, usize)> {
        let c = self.core();
        let mut v: Vec<(&'static str, usize)> = self
            .regions
            .iter()
            .filter(|r| r.bytes > 0)
            .filter_map(|r| {
                // SAFETY: inside the mapping (regions lie within `c.bytes`).
                let p = unsafe { c.base.add(r.off) };
                let n = crate::memstat::resident(p, r.bytes)?;
                (n > 0).then_some((r.name, n))
            })
            .collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.1));
        v
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
    /// The chunks that differ from the live space, sorted; chunk `cs[i]`'s
    /// words are `bufs[i * CHUNK_WORDS..]`. (Not a pointer per chunk of
    /// the space: that was a 4 MB vector, freed after each S₀ save and
    /// then kept by the macOS allocator; lane MEM-MODES.)
    cs: Vec<u32>,
    bufs: Vec<u64>,
}

impl View<'_> {
    pub fn chunk(&self, c: usize) -> &[u8] {
        let w: &[u64] = match self.cs.binary_search(&(c as u32)) {
            Ok(i) => &self.bufs[i * CHUNK_WORDS..(i + 1) * CHUNK_WORDS],
            Err(_) => self.arena.chunk(c),
        };
        // SAFETY: a chunk of CHUNK_WORDS words viewed as bytes.
        unsafe { std::slice::from_raw_parts(w.as_ptr() as *const u8, CHUNK_BYTES) }
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
        // Measurement only: the branchless barrier, an unconditional store
        // of the chunk's flag (what a dirty map without pre-images would
        // need). Checkpoints are wrong with it; never ship it.
        if cfg!(feature = "bench-store-barrier") {
            // SAFETY: as below; the map is owned by the core and writable.
            unsafe { *(flag as *mut u8) = 1 };
            return;
        }
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

// ---------------------------------------------------------------------------
// ArrView: an array global's address, length and barrier in a local
// ---------------------------------------------------------------------------

/// A copy of a **fixed-length** array global's header (`Arr::view`), which
/// `tools/web2rust --array-view NAME` puts in a local at the start of each
/// routine that indexes the array. The generated code then indexes the view
/// instead of `self.NAME`. Only `mem` and `eqtb` are viewed
/// (`web2rust-default.args`): a view costs its loads at every call of the
/// routine, and for the other arrays that cost more than it saved (measured,
/// P6).
///
/// Why: every element write is a store through a raw pointer, which LLVM
/// must assume can change any field of `Globals` once `&mut self` has been
/// passed to a call. So after each write it reloads the array's address,
/// length and flag pointer from `Globals` before the next access. A local is
/// not reachable through any pointer, so it stays in a register. Nothing
/// else changes: reads and writes are bounds-checked against the same length
/// (views are made only of arrays created at their full length and never
/// resized: web2c's fixed arrays, never `xmalloc_array`'d), and writes pass
/// the same barrier as `Arr`'s.
pub struct ArrView<T> {
    ptr: *mut T,
    len: usize,
    flags: *const u8,
    core: *mut Core,
}

impl<T> Clone for ArrView<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ArrView<T> {}

impl<T> Arr<T> {
    /// This array's view (see [`ArrView`]). Only for an array whose length
    /// never changes: a view keeps the length it was made with.
    #[inline(always)]
    pub fn view(&self) -> ArrView<T> {
        ArrView {
            ptr: self.ptr,
            len: self.len,
            flags: self.flags,
            core: self.core,
        }
    }
}

impl<T> ArrView<T> {
    const SIZE: usize = std::mem::size_of::<T>();
    const STRADDLES: bool = !Self::SIZE.is_power_of_two();

    /// `Arr::touch_addr`.
    #[inline(always)]
    fn touch_addr(&self, a: usize) {
        let flag = self.flags.wrapping_add(a >> CHUNK_SHIFT);
        if cfg!(feature = "bench-store-barrier") {
            // SAFETY: as in `Arr::touch_addr`.
            unsafe { *(flag as *mut u8) = 1 };
            return;
        }
        // SAFETY: as in `Arr::touch_addr`: `a` lies in the array's region.
        if unsafe { *flag } == 0 {
            // SAFETY: the core outlives every array and view.
            let base = unsafe { (*self.core).base } as usize;
            save_cold(self.core, (a - base) >> CHUNK_SHIFT);
        }
    }

    /// The write barrier for element `i` (already bounds-checked): exactly
    /// `Arr::touch`'s.
    #[inline(always)]
    fn touch(&self, i: usize) {
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

    /// Element `i` for reading, bounds-checked.
    #[inline(always)]
    pub fn get(&self, i: usize) -> &T {
        if i >= self.len {
            out_of_bounds(i, self.len);
        }
        // SAFETY: in bounds; the elements live as long as the arena, which
        // outlives the routine holding the view.
        unsafe { &*self.ptr.add(i) }
    }

    /// Element `i` for reading without the check: only for the
    /// `unchecked-reads` measurement feature (`crate::ix`).
    ///
    /// # Safety
    /// `i` must be below the view's length.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, i: usize) -> &T {
        // SAFETY: the caller's.
        unsafe { &*self.ptr.add(i) }
    }

    /// Element `i` for writing, bounds-checked and through the barrier.
    #[inline(always)]
    pub fn get_mut(&mut self, i: usize) -> &mut T {
        if i >= self.len {
            out_of_bounds(i, self.len);
        }
        self.touch(i);
        // SAFETY: in bounds; the generated code holds no other reference to
        // the element while it writes (it reads elements by value).
        unsafe { &mut *self.ptr.add(i) }
    }
}

impl<T> Index<usize> for ArrView<T> {
    type Output = T;
    #[inline(always)]
    fn index(&self, i: usize) -> &T {
        self.get(i)
    }
}

impl<T> IndexMut<usize> for ArrView<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut T {
        self.get_mut(i)
    }
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
            arr[i] = shaped(x);
        }
    }

    /// A word whose halves take every packed width (`pack_words`): zero,
    /// one, two or four bytes, positive and negative.
    fn shaped(x: u64) -> u64 {
        let half = |y: u64| -> u32 {
            match (y >> 61) & 7 {
                0 => 0,
                1 => (y as i8) as i32 as u32,
                2 => (y as i16) as i32 as u32,
                3 => y as u32,
                4 => 1,
                5 => u32::MAX,
                6 => (y >> 8) as u16 as u32,
                _ => 0x8000_0000 | y as u32,
            }
        };
        half(x) as u64 | (half(x.rotate_left(29)) as u64) << 32
    }

    #[test]
    fn packed_words_round_trip() {
        let mut x = 7u64;
        let mut vals = vec![];
        for _ in 0..10_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            vals.push(shaped(x));
        }
        vals.extend([
            0,
            1,
            u64::MAX,
            0x7f,
            0x80,
            0xff,
            0x7fff,
            0x8000,
            0xffff_8000,
            1 << 32,
            0x8000_0000_0000_0000,
        ]);
        for n in [0usize, 1, 2, 3, 127, 128] {
            for start in [0usize, 5, 333] {
                let chunk = &vals[start..start + n];
                let mut bytes = vec![9u8; 3];
                pack_words(chunk, &mut bytes);
                let mut mask = [0u64; MASK_WORDS];
                // the first n bits set, any n words do
                for k in 0..n {
                    mask[k / 64] |= 1 << (k % 64);
                }
                let d = Delta { c: 0, at: 3, mask };
                let mut out = [0u64; CHUNK_WORDS];
                d.unpack(&bytes, &mut out);
                assert_eq!(&out[..n], chunk);
                assert_eq!(d.packed_len(&bytes), bytes.len() - 3);
                // apply_under with every other word already done
                let mut dst = [0u64; CHUNK_WORDS];
                let mut done = [0xaaaa_aaaa_aaaa_aaaau64; MASK_WORDS];
                // SAFETY: `dst` is CHUNK_WORDS words.
                unsafe { d.apply_under(&bytes, dst.as_mut_ptr(), &mut done) };
                for k in 0..n {
                    let want = if (0xaaaa_aaaa_aaaa_aaaau64 >> (k % 64)) & 1 == 1 {
                        0
                    } else {
                        chunk[k]
                    };
                    assert_eq!(dst[k], want, "n {n} start {start} word {k}");
                }
            }
        }
        // scattered masks, both mask words, only the second needed
        let mask = [0x8000_0000_0000_0011u64, 0x0100_0000_0000_8001];
        let chunk = &vals[40..46];
        let mut bytes = vec![];
        pack_words(chunk, &mut bytes);
        let d = Delta { c: 0, at: 0, mask };
        let mut dst = [0u64; CHUNK_WORDS];
        let mut done = [u64::MAX, 0];
        // SAFETY: as above.
        unsafe { d.apply_under(&bytes, dst.as_mut_ptr(), &mut done) };
        assert_eq!([dst[64], dst[79], dst[120]], [chunk[3], chunk[4], chunk[5]]);
        assert_eq!(dst[0] | dst[4] | dst[63], 0);
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

    /// P4-TYPING-200WPM (a): a preparation stopped part way (at every kind
    /// of step: looking through the logs, copying, rewinding, the redo) is
    /// kept and goes on from there, in the next `prepare_restore` to the
    /// same target or in the restore itself, also when the live state was
    /// written in between; the restore equals a plain one and the jump back
    /// gives the old state. One for another target, or from before a new
    /// checkpoint, is not used.
    #[test]
    fn stopped_preparations_go_on_where_they_stopped() {
        // (more than STOP_LOGS logs and STOP_CHUNKS chunks: every step can
        // be stopped)
        let (mut a, mut arr) = space(400_000);
        scribble(&mut arr, 3, 20_000);
        let mut ids = vec![];
        let mut copies = std::collections::HashMap::new();
        for k in 0..150u64 {
            ids.push(a.checkpoint());
            if [0, 4, 5, 6, 9, 13, 70].contains(&k) {
                copies.insert(k as usize, arr.to_vec());
            }
            scribble(&mut arr, 300 + k, 400);
        }
        // a stop after `n` questions
        let after = |n: usize| {
            let mut k = 0usize;
            move || {
                k += 1;
                k > n
            }
        };
        let mut round = 0u64;
        for i in [0usize, 5, 13, 70] {
            for n in [0usize, 1, 2, 3, 5, 8, 40] {
                round += 1;
                let _ = a.prepare_restore(ids[i], &mut after(n));
                // written in between, then stopped again further on
                scribble(&mut arr, 900 + round, 150);
                let _ = a.prepare_restore(ids[i], &mut after(n + 1));
                scribble(&mut arr, 1900 + round, 150);
                let end = arr.to_vec();
                let br = a.restore_branch(ids[i]).unwrap();
                assert!(
                    arr[..] == copies[&i][..],
                    "restore to {i}, stopped after {n}"
                );
                a.converge(br, ids[i]).unwrap();
                assert!(arr[..] == end[..], "jump back from {i}, stopped after {n}");
            }
        }
        // stopped for one target, restored to another: not used
        let _ = a.prepare_restore(ids[4], &mut after(2));
        let end = arr.to_vec();
        let br = a.restore_branch(ids[9]).unwrap();
        assert!(arr[..] == copies[&9][..], "restore to another target");
        a.converge(br, ids[9]).unwrap();
        assert!(arr[..] == end[..]);
        // stopped, then a checkpoint: not used
        let _ = a.prepare_restore(ids[6], &mut after(2));
        scribble(&mut arr, 4000, 100);
        let last = a.checkpoint();
        let _ = last;
        let br = a.restore_branch(ids[6]).unwrap();
        assert!(arr[..] == copies[&6][..], "restore after a checkpoint");
        a.drop_branch(br);
    }

    /// A preparation stopped while a branch was detached, then the branch
    /// reattached: the part was copied from the abandoned run's state and
    /// rewound through its logs, so it is not used. (When the branch holds
    /// only its target and no checkpoint was taken while it was detached,
    /// the checkpoint list after the reattach is the one the part saw:
    /// `reattach` drops the part itself.)
    #[test]
    fn a_part_from_before_a_reattach_is_not_used() {
        let after = |n: usize| {
            let mut k = 0usize;
            move || {
                k += 1;
                k > n
            }
        };
        for n in [1usize, 2, 3, 5] {
            let (mut a, mut arr) = space(400_000);
            scribble(&mut arr, 3, 20_000);
            for k in 0..8u64 {
                a.checkpoint();
                scribble(&mut arr, 300 + k, 400);
            }
            let target = a.checkpoint();
            let at_target = arr.to_vec();
            // the old run writes after its newest checkpoint
            scribble(&mut arr, 77 + n as u64, 3000);
            let old_end = arr.to_vec();
            let br = a.restore_branch(target).unwrap();
            assert_eq!(br.ids, vec![target]);
            assert!(arr[..] == at_target[..]);
            // the new run writes elsewhere, takes no checkpoint, and a
            // preparation to the same target stops in its copies
            scribble(&mut arr, 880 + n as u64, 3000);
            assert!(!a.prepare_restore(target, &mut after(n)));
            assert!(a.core().part.is_some(), "stopped part way ({n})");
            let ids = a.checkpoint_ids().to_vec();
            a.reattach(br).unwrap();
            assert!(arr[..] == old_end[..]);
            assert_eq!(a.checkpoint_ids(), &ids[..], "the same checkpoint list");
            assert!(
                a.core().part.is_none(),
                "the reattach dropped the part ({n})"
            );
            // the next preparation (`prepare_next`) starts afresh
            assert!(a.prepare_restore(target, &mut || false));
            let br = a.restore_branch(target).unwrap();
            assert!(arr[..] == at_target[..], "restore after the reattach ({n})");
            a.converge(br, target).unwrap_or_else(|e| panic!("{e}"));
            assert!(arr[..] == old_end[..], "jump back ({n})");
        }
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
        // the redo made ahead (`Prepared::pre`): written after the
        // preparation heavily, so that most prepared chunks the barrier had
        // not saved are written again before the restore, and some are not
        for (n, i) in [2usize, 15, 9].into_iter().enumerate() {
            assert!(a.prepare_restore(ids[i], &mut || false));
            let p = a.core().prepared.as_ref().unwrap();
            assert!(p.pre.iter().any(|d| !d.is_null()), "a redo made ahead");
            scribble(&mut arr, 1000 + n as u64, 2_000);
            let end = arr.to_vec();
            let br = a.restore_branch(ids[i]).unwrap();
            assert!(
                arr[..] == copies[i][..],
                "prepared restore to {i}, written since"
            );
            a.converge(br, ids[i]).unwrap();
            assert!(arr[..] == end[..], "jump back from {i}, written since");
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
        // stopped inside the copies (fewer logs than one `STOP_LOGS` but more
        // chunks than one `STOP_CHUNKS`: P4-TYPING-200WPM), at each question
        // in turn: nothing prepared, no slab chunk in use but the kept
        // part's redo made ahead (`PartPrep`), and the restore after the
        // stops, which takes the part, equals a plain one
        let live0 = a.core().slab.live;
        for n in 1..6 {
            let mut asked = 0;
            assert!(!a.prepare_restore(ids[0], &mut || {
                asked += 1;
                asked >= n
            }));
            assert!(a.core().prepared.is_none());
            let held = a
                .core()
                .part
                .as_ref()
                .map_or(0, |p| p.pre.iter().filter(|d| !d.is_null()).count());
            assert_eq!(
                a.core().slab.live,
                live0 + held,
                "only the kept part holds slab chunks ({n})"
            );
        }
        let end = arr.to_vec();
        let br = a.restore_branch(ids[0]).unwrap();
        assert!(a.core().part.is_none(), "the restore took the kept part");
        assert!(arr[..] == copies[0][..], "a restore after the stops");
        a.converge(br, ids[0]).unwrap();
        assert!(arr[..] == end[..]);
    }

    /// A reattach leaves a prepared restore to its target (LIVE-30MS): the
    /// next restore there equals a plain one, the jump back from it too,
    /// and the preparation is exactly what the reattached logs rewind to.
    #[test]
    fn a_reattach_prepares_the_restore_to_its_target() {
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
        for (n, i) in [4usize, 4, 11, 0].into_iter().enumerate() {
            let br = a.restore_branch(ids[i]).unwrap();
            assert!(arr[..] == copies[i][..], "restore to {i}");
            // a paused run: a few pages of its own, then abandoned
            for m in 0..3 {
                scribble(&mut arr, 900 + 10 * n as u64 + m, 1500);
                a.checkpoint();
            }
            a.reattach(br).unwrap();
            assert!(arr[..] == end[..], "reattached at {i}");
            a.core().verify_prepared().unwrap();
            let p = a.core().prepared.as_ref().expect("prepared");
            assert_eq!(p.id, ids[i]);
            let br = a.restore_branch(ids[i]).unwrap();
            assert!(arr[..] == copies[i][..], "prepared restore to {i}");
            a.converge(br, ids[i]).unwrap();
            assert!(arr[..] == end[..], "jump back from {i}");
        }
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

    /// LIVE-30MS: an old checkpoint's chunks kept by one comparison
    /// (`OldCache`) serve the next, across a jump back and a new restore
    /// (typing in one place), and give exactly the old states; an `or_from`
    /// since makes them stale.
    #[test]
    fn kept_old_chunks_never_exceed_the_cap() {
        let mut c = OldCache::default();
        let n = OLD_CACHE_CHUNKS / 3 + 7;
        let buf = vec![7u64; n * CHUNK_WORDS];
        for id in 0..10u64 {
            let cs: Vec<u32> = (0..n as u32).map(|x| x + id as u32).collect();
            c.put(id, 0, &cs, &buf);
            let total: usize = c.at.iter().map(|e| e.2.len()).sum();
            assert!(total <= OLD_CACHE_CHUNKS, "{total} chunks after {id}");
            assert!(c.at.len() <= OLD_CACHE_CHECKPOINTS);
            assert!(c.get(id, 0, id as u32).is_some(), "the newest is kept");
        }
        // one bigger than the cap: as much as fits
        let big: Vec<u32> = (0..(OLD_CACHE_CHUNKS + 5) as u32).collect();
        c.put(99, 0, &big, &vec![1u64; big.len() * CHUNK_WORDS]);
        assert_eq!(
            c.at.iter().map(|e| e.2.len()).sum::<usize>(),
            OLD_CACHE_CHUNKS
        );
        // another history: everything older goes
        c.put(100, 1, &[3], &[2u64; CHUNK_WORDS]);
        assert_eq!(c.at.len(), 1);
    }

    /// P4-TYPING-200WPM (b): a convergence test rewinds the `UNSTATED`
    /// arrays' chunks too, in its one pass, and keeps them: the jump's
    /// comparison at the same checkpoint (`diff_branch_all`) then rewinds
    /// nothing, and its old values equal a fresh rewind's (nothing kept)
    /// word for word, the unstated array's too.
    #[test]
    fn a_test_keeps_what_the_jump_compares() {
        let mut p = Plan::new(64);
        let rt = p.reserve::<u64>("t", 150_000);
        let ru = p.reserve::<u64>("dl_side", 50_000);
        let mut a = p.build();
        let mut t = a.arr(rt, 150_000);
        let mut u = a.arr(ru, 50_000);
        scribble(&mut t, 41, 5000);
        scribble(&mut u, 42, 2000);
        let mut ids = vec![];
        for k in 0..12 {
            ids.push(a.checkpoint());
            scribble(&mut t, 400 + k, 2000);
            scribble(&mut u, 500 + k, 800);
        }
        let br = a.restore_branch(ids[3]).unwrap();
        scribble(&mut t, 990, 1500);
        scribble(&mut u, 991, 600);
        a.checkpoint();
        let all_words = |a: &Arena, d: &ChunkDiff| -> Vec<u64> {
            (0..a.bytes().len() / 8)
                .filter(|w| d.old_at.contains_key(&(((w * 8) >> CHUNK_SHIFT) as u32)))
                .map(|w| d.old_word(a, w * 8))
                .collect()
        };
        for j in [4usize, 6] {
            a.drop_old_cache();
            drop(a.diff_branch(&br, ids[j]).unwrap());
            let misses0 = a.old_cache_counts().1;
            let kept = all_words(&a, &a.diff_branch_all(&br, ids[j]).unwrap());
            let misses = a.old_cache_counts().1 - misses0;
            assert_eq!(misses, 0, "the jump's comparison at {j} rewound again");
            a.drop_old_cache();
            let fresh = all_words(&a, &a.diff_branch_all(&br, ids[j]).unwrap());
            assert!(
                kept == fresh,
                "at {j}: the kept values differ from a fresh rewind"
            );
        }
        a.drop_branch(br);
    }

    /// P4-CONVERGE-REWIND: the `UNSTATED` chunks a convergence test kept
    /// (`OldCache`) are the old run's values at that checkpoint, whatever
    /// the new run writes after the test: written on (the chunks the test
    /// saw again, and chunks it never saw, in both arrays), the jump's
    /// comparison at the same checkpoint gives a fresh rewind's old values
    /// word for word, and adopting what differs, then converging, gives
    /// exactly the old run's latest state.
    #[test]
    fn values_a_test_kept_hold_for_chunks_written_after_it() {
        let mut p = Plan::new(64);
        let rt = p.reserve::<u64>("t", 150_000);
        let ru = p.reserve::<u64>("dl_side", 50_000);
        let mut a = p.build();
        let mut t = a.arr(rt, 150_000);
        let mut u = a.arr(ru, 50_000);
        scribble(&mut t, 61, 5000);
        scribble(&mut u, 62, 2000);
        let mut ids = vec![];
        for k in 0..12 {
            ids.push(a.checkpoint());
            scribble(&mut t, 600 + k, 2000);
            scribble(&mut u, 700 + k, 800);
        }
        let (t_end, u_end) = (t.to_vec(), u.to_vec());
        let j = 6;
        let br = a.restore_branch(ids[3]).unwrap();
        scribble(&mut t, 1990, 1500);
        scribble(&mut u, 1991, 600);
        a.checkpoint();
        let all_words = |a: &Arena, d: &ChunkDiff| -> Vec<(usize, u64)> {
            (0..a.bytes().len() / 8)
                .filter(|w| d.old_at.contains_key(&(((w * 8) >> CHUNK_SHIFT) as u32)))
                .map(|w| (w, d.old_word(a, w * 8)))
                .collect()
        };
        a.drop_old_cache();
        // the test, which keeps the unstated chunks it rewinds
        let seen_by_test = a.diff_branch(&br, ids[j]).unwrap().compared;
        // the new run writes on: again where it wrote, and elsewhere
        scribble(&mut t, 2990, 3000);
        scribble(&mut u, 2991, 3000);
        a.checkpoint();
        let kept = all_words(&a, &a.diff_branch_all(&br, ids[j]).unwrap());
        a.drop_old_cache();
        let fresh_d = a.diff_branch_all(&br, ids[j]).unwrap();
        assert!(
            fresh_d.compared > seen_by_test,
            "the new run wrote new chunks"
        );
        let fresh = all_words(&a, &fresh_d);
        assert!(kept == fresh, "kept values differ from a fresh rewind");
        drop(fresh_d);
        // the jump: the test kept them again, then the comparison adopts
        // what differs from the old run's state at `j`
        drop(a.diff_branch(&br, ids[j]).unwrap());
        let d = a.diff_branch_all(&br, ids[j]).unwrap();
        let adopt: Vec<(usize, Vec<u8>)> = d
            .differing
            .iter()
            .map(|&(c, old_p, _)| {
                // SAFETY: a chunk of the comparison's own buffers or the
                // arena, unchanged while `d` lives.
                let b = unsafe { std::slice::from_raw_parts(old_p as *const u8, CHUNK_BYTES) };
                ((c as usize) << CHUNK_SHIFT, b.to_vec())
            })
            .collect();
        drop(d);
        for (off, b) in &adopt {
            a.write_through(*off, b);
        }
        a.checkpoint();
        a.converge(br, ids[j]).unwrap();
        assert!(
            t[..] == t_end[..],
            "the jump gives the old run's latest state"
        );
        assert!(
            u[..] == u_end[..],
            "the jump gives the old run's latest side table"
        );
    }

    #[test]
    fn kept_old_chunks_equal_rewound_ones() {
        let (mut a, mut arr) = space(200_000);
        scribble(&mut arr, 21, 5000);
        let mut ids = vec![];
        let mut copies = vec![];
        for k in 0..12 {
            ids.push(a.checkpoint());
            copies.push(arr.to_vec());
            scribble(&mut arr, 70 + k, 2000);
        }
        let start = arr.as_ptr() as usize - a.bytes().as_ptr() as usize;
        let len = arr.len();
        let check = |a: &Arena, d: &ChunkDiff, j: usize, what: &str| {
            for e in (0..len).step_by(41) {
                assert_eq!(
                    d.old_word(a, start + e * 8),
                    copies[j][e],
                    "{what}: old {j}, element {e}"
                );
            }
        };
        let end = arr.to_vec();
        for round in 0..4u64 {
            let hits0 = a.old_cache_counts().0;
            let br = a.restore_branch(ids[3]).unwrap();
            scribble(&mut arr, 900 + round, 1500);
            a.checkpoint();
            for j in [4usize, 5] {
                let d = a.diff_branch(&br, ids[j]).unwrap();
                check(&a, &d, j, &format!("round {round}"));
            }
            if round > 0 {
                assert!(
                    a.old_cache_counts().0 > hits0,
                    "round {round}: nothing kept was used"
                );
            }
            // back to the old run's end, as an abandoned compile does
            a.reattach(br).unwrap();
            assert!(arr[..] == end[..]);
        }
        // a history rewrite since: the kept values are not used
        let off = start + 8 * 1000;
        a.or_from(ids[2], off, 1 << 40).unwrap();
        let mut c4 = copies[4].clone();
        c4[1000] |= 1 << 40;
        let br = a.restore_branch(ids[3]).unwrap();
        a.checkpoint();
        let d = a.diff_branch(&br, ids[4]).unwrap();
        assert_eq!(d.old_word(&a, off), c4[1000]);
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

    /// The cost of a restore through many sealed logs (the tail of a long
    /// document after an edit near its start), and of the logs' bytes:
    /// `cargo test --release -p flashtex-engine --lib restore_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn restore_cost() {
        let (mut a, mut arr) = space(8_000_000);
        scribble(&mut arr, 5, 1_000_000);
        let first = a.checkpoint();
        let mut x = 99u64;
        // 400 pages, each rewriting 13 words in 2000 of the same 6000 chunks
        for round in 0..400u64 {
            for k in 0..2000usize {
                let c = (k * 3 + round as usize % 3) % 6000;
                for w in 0..13usize {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    arr[c * CHUNK_WORDS + w * 9] = shaped(x);
                }
            }
            a.checkpoint();
        }
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = std::time::Instant::now();
            let br = a.restore_branch(first).unwrap();
            best = best.min(t.elapsed().as_secs_f64());
            a.converge(br, first).unwrap();
        }
        eprintln!(
            "restore through 400 logs: best {:.3} ms; logs {:.1} MB",
            best * 1000.0,
            a.log_bytes() as f64 / 1e6
        );
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
