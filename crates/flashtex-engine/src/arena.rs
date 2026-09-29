//! One flat word space for all of the engine's mutable state (DESIGN.md §4.2,
//! §5.2 mechanism (b3)).
//!
//! Every array global of the translated engine (`mem`, `eqtb`, `hash`,
//! `save_stack`, the input and nest stacks, `str_pool`/`str_start`,
//! `font_info` and the font arrays, the trie, pdfTeX's object and destination
//! tables, ...) is an [`Arr`], a view into one contiguous, zero-initialised
//! allocation. The scalar globals are spilled into a region at the start of
//! the same space at every checkpoint (see `checkpoint.rs`), so one 16 KB
//! dirty bitmap covers everything a checkpoint has to capture.
//!
//! * **Reads** are plain loads: `Index` and `Deref` go straight to the
//!   element, exactly like the `Vec` they replace.
//! * **Writes** go through `IndexMut` (or `slice_mut`), which is the write
//!   barrier: one byte test in the `saved` map (one flag per 16 KB chunk,
//!   addressed straight from the element's address) and, the first time a
//!   chunk is written after a checkpoint, a copy of its previous contents
//!   into the open undo log (a cold call).
//! * A **checkpoint** seals the open log and clears the bitmap.
//! * A **restore** walks the logs from the newest back to the target, putting
//!   each chunk's oldest pre-image back, and captures the chunks it overwrites
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

use std::marker::PhantomData;
use std::ops::{Deref, Index, IndexMut};

pub const CHUNK_SHIFT: usize = 14;
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

/// A 16 KB chunk's saved contents, a slot of the slab.
type ChunkPtr = *mut u64;

/// Chunks are carved out of 1 MiB blocks and recycled through a free list:
/// the snapshot benchmark measured 1.24x overhead for 16 KB `malloc`s.
struct Slab {
    blocks: Vec<*mut u8>,
    free: Vec<ChunkPtr>,
    live: usize,
}

const SLAB_BLOCK_CHUNKS: usize = 64;

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

#[derive(Default)]
struct Log {
    entries: Vec<(u32, ChunkPtr)>,
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
    slab: Slab,
    /// Chunks copied by the barrier since the space was made.
    pub slow_path: u64,
    /// Workers for deep restores (0 = one per available core, up to 8).
    pub threads: usize,
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
        let id = self.next_id;
        self.next_id += 1;
        self.ids.push(id);
        self.logs.push(Log::default());
        self.clear_saved();
        id
    }

    fn index_of(&self, id: CheckpointId) -> Option<usize> {
        self.ids.iter().rposition(|&x| x == id)
    }

    /// The distinct chunks of `logs`, each with its oldest pre-image: that is
    /// the chunk's value at the first log's checkpoint.
    fn plan(&mut self, logs: &[Log]) -> Vec<(u32, ChunkPtr)> {
        let mut mark = std::mem::take(&mut self.mark);
        mark.fill(0);
        let mut plan = Vec::new();
        for log in logs {
            for &(c, data) in &log.entries {
                if !bit(&mark, c as usize) {
                    set_bit(&mut mark, c as usize);
                    plan.push((c, data));
                }
            }
        }
        self.mark = mark;
        plan
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

    /// Copy each planned pre-image into the live space; with `capture`,
    /// first copy the live chunk into a fresh slab chunk (the redo log).
    fn apply(&mut self, plan: &[(u32, ChunkPtr)], capture: bool) -> Vec<(u32, ChunkPtr)> {
        let redo: Vec<(u32, ChunkPtr)> = if capture {
            plan.iter().map(|&(c, _)| (c, self.slab.take())).collect()
        } else {
            Vec::new()
        };
        let base = self.base as usize;
        // Addresses as integers so that the job is `Sync`: (live, source,
        // redo destination or 0).
        let work: Vec<(usize, usize, usize)> = plan
            .iter()
            .enumerate()
            .map(|(k, &(c, src))| {
                let live = base + ((c as usize) << CHUNK_SHIFT);
                (
                    live,
                    src as usize,
                    if capture { redo[k].1 as usize } else { 0 },
                )
            })
            .collect();
        let job = |k: usize| {
            let (live, src, dst) = work[k];
            // SAFETY: every chunk appears once in a plan, so the workers write
            // disjoint live chunks and disjoint redo chunks; sources are
            // log chunks nobody writes during a restore.
            unsafe {
                if dst != 0 {
                    std::ptr::copy_nonoverlapping(live as *const u64, dst as *mut u64, CHUNK_WORDS);
                }
                std::ptr::copy_nonoverlapping(src as *const u64, live as *mut u64, CHUNK_WORDS);
            }
        };
        run_jobs(work.len(), self.workers(), &job);
        redo
    }

    fn free_log(&mut self, log: Log) {
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
        let plan = self.plan(&later);
        self.apply(&plan, false);
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
        let old_logs = self.logs.split_off(k);
        let old_ids = self.ids.split_off(k);
        let plan = self.plan(&old_logs);
        let redo = self.apply(&plan, true);
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
        self.logs.pop();
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
            for &(c, _) in &log.entries {
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
        self.apply(&jobs, false);
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

    fn drop_branch(&mut self, b: Branch) {
        for log in b.logs {
            self.free_log(log);
        }
        for (_, p) in b.redo {
            self.slab.give(p);
        }
    }

    /// Drop every checkpoint `keep` rejects, except the newest, merging each
    /// dropped log into its predecessor (the older pre-image of a chunk wins).
    fn retain(&mut self, keep: &dyn Fn(CheckpointId) -> bool) {
        let n = self.ids.len();
        let ids = std::mem::take(&mut self.ids);
        let logs = std::mem::take(&mut self.logs);
        let mut mark = std::mem::take(&mut self.mark);
        let mut out_ids = Vec::with_capacity(n);
        let mut out_logs: Vec<Log> = Vec::with_capacity(n);
        let mut freed = Vec::new();
        for (i, (id, log)) in ids.into_iter().zip(logs).enumerate() {
            if i + 1 == n || keep(id) {
                out_ids.push(id);
                out_logs.push(log);
            } else if let Some(dst) = out_logs.last_mut() {
                mark.fill(0);
                for &(c, _) in &dst.entries {
                    set_bit(&mut mark, c as usize);
                }
                for (c, data) in log.entries {
                    if bit(&mark, c as usize) {
                        freed.push(data);
                    } else {
                        dst.entries.push((c, data));
                    }
                }
            } else {
                freed.extend(log.entries.into_iter().map(|(_, p)| p));
            }
        }
        for p in freed {
            self.slab.give(p);
        }
        self.ids = out_ids;
        self.logs = out_logs;
        self.mark = mark;
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

/// Run `job(0..n)` on up to `workers` scoped threads. A restore is bound by
/// memory bandwidth and independent per chunk; below the threshold spawning
/// costs more than it saves.
fn run_jobs(n: usize, workers: usize, job: &(dyn Fn(usize) + Sync)) {
    const PARALLEL_MIN: usize = 256;
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
            slab: Slab {
                blocks: Vec::new(),
                free: Vec::new(),
                live: 0,
            },
            slow_path: 0,
            threads: 0,
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

    pub fn converge(&mut self, b: Branch, old: CheckpointId) -> Result<(), String> {
        self.core_mut().converge(b, old)
    }

    pub fn drop_branch(&mut self, b: Branch) {
        self.core_mut().drop_branch(b)
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
    /// chunk's oldest pre-image logged since `id`, else the live chunk.
    pub fn view_at(&self, id: CheckpointId) -> Result<View<'_>, String> {
        let core = self.core();
        let k = core
            .index_of(id)
            .ok_or_else(|| format!("checkpoint {id} is not retained"))?;
        let mut over: Vec<*const u64> = vec![std::ptr::null(); core.nchunks];
        for log in &core.logs[k..] {
            for &(c, p) in &log.entries {
                if over[c as usize].is_null() {
                    over[c as usize] = p;
                }
            }
        }
        Ok(View { arena: self, over })
    }

    /// The chunks the open log holds (written since the newest checkpoint),
    /// counted by the array each starts in, most first.
    pub fn open_log_by_region(&self) -> Vec<(&'static str, usize)> {
        let core = self.core();
        let mut counts: std::collections::BTreeMap<&'static str, usize> = Default::default();
        if let Some(log) = core.logs.last() {
            for &(c, _) in &log.entries {
                let at = (c as usize) << CHUNK_SHIFT;
                let r = match self.regions.binary_search_by(|r| r.off.cmp(&at)) {
                    Ok(i) => i,
                    Err(i) => i.saturating_sub(1),
                };
                *counts.entry(self.regions[r].name).or_default() += 1;
            }
        }
        let mut v: Vec<_> = counts.into_iter().collect();
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

    /// Entries in the open log (chunks written since the newest checkpoint).
    pub fn open_log_len(&self) -> usize {
        self.core().logs.last().map_or(0, |l| l.entries.len())
    }

    /// Chunks copied by the barrier so far.
    pub fn slow_path(&self) -> u64 {
        self.core().slow_path
    }

    /// Bytes held by undo and redo chunks (the slab's live slots).
    pub fn log_bytes(&self) -> usize {
        self.core().slab.live * CHUNK_BYTES
    }

    /// Workers for restores; 0 picks one per core, up to 8.
    pub fn set_threads(&mut self, n: usize) {
        self.core_mut().threads = n;
    }
}

/// The space at a checkpoint (`Arena::view_at`).
pub struct View<'a> {
    arena: &'a Arena,
    over: Vec<*const u64>,
}

impl View<'_> {
    pub fn chunk(&self, c: usize) -> &[u8] {
        let p = self.over[c];
        if p.is_null() {
            let w = self.arena.chunk(c);
            // SAFETY: a chunk of CHUNK_WORDS words viewed as bytes.
            unsafe { std::slice::from_raw_parts(w.as_ptr() as *const u8, CHUNK_BYTES) }
        } else {
            // SAFETY: a slab chunk owned by the arena's logs, which `&Arena`
            // keeps alive and unchanged.
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
