//! Mechanism (b3): flat state, dirty bitmap, undo log.
//!
//! The interesting result from (b1) and (b2) is that their standing barrier is almost all
//! *read* cost, not write cost: both reach a word through a chunk table, so every load
//! becomes two dependent loads, and that is paid by the 75% of accesses that are reads and
//! never touch the barrier at all.
//!
//! Shadow paging forces that, because privatising a chunk moves it. Undo logging does not.
//! Here the live state stays one flat `Vec<u64>` — reads are exactly `plain`'s single load
//! — and the barrier saves a chunk's **previous** contents into a log before the first
//! write to it since the last checkpoint. Restoring replays that log backwards.
//!
//! The trade is restore cost. Shadow paging restores in O(1): swap the chunk table. Undo
//! logging restores in O(chunks dirtied since the target checkpoint), so restoring to the
//! newest checkpoint is cheap and restoring to an old one walks every log in between.
//!
//! **This implementation keeps one level**: it restores to the newest checkpoint only, so
//! `snapshot()` drops the previous log. A real engine would seal one log per retained
//! checkpoint instead. The memory is the same either way — the same dirty chunks at the
//! same 16 KB granularity — and the extra cost is that a restore walks several logs rather
//! than one. One level is enough to price the barrier and the restore-to-newest path.

use crate::backend::{Backend, Snapshot};
use crate::layout::{Layout, CHUNK_SHIFT, CHUNK_WORDS};

type PlainChunk = [u64; CHUNK_WORDS];

pub struct UndoCow {
    words: Vec<u64>,
    /// One bit per chunk: set means "the original is already in `log`", so a write may go
    /// straight through.
    saved: Vec<u64>,
    log: Vec<(u32, Box<PlainChunk>)>,
    pub slow_path: u64,
}

pub struct UndoSnap {
    /// Bytes the log held at the moment this snapshot was superseded.
    bytes: usize,
}

impl Snapshot for UndoSnap {
    fn nominal_bytes(&self) -> usize {
        self.bytes
    }
}

impl UndoCow {
    #[inline(never)]
    #[cold]
    fn save(&mut self, c: usize) {
        let start = c * CHUNK_WORDS;
        let mut boxed: Box<std::mem::MaybeUninit<PlainChunk>> = Box::new_uninit();
        // SAFETY: the box is freshly allocated and uniquely owned; the source is
        // CHUNK_WORDS initialised words inside `words` (c is a valid chunk index, so
        // start + CHUNK_WORDS <= words.len()); the two allocations cannot overlap.
        let chunk = unsafe {
            std::ptr::copy_nonoverlapping(
                self.words.as_ptr().add(start),
                boxed.as_mut_ptr() as *mut u64,
                CHUNK_WORDS,
            );
            boxed.assume_init()
        };
        self.log.push((c as u32, chunk));
        self.saved[c >> 6] |= 1u64 << (c & 63);
        self.slow_path += 1;
    }

    pub fn log_bytes(&self) -> usize {
        self.log.len() * CHUNK_WORDS * 8 + self.log.capacity() * std::mem::size_of::<(u32, usize)>()
    }
}

impl Backend for UndoCow {
    type Snap = UndoSnap;

    fn name(&self) -> &'static str {
        "flat-undo-log"
    }

    fn new(layout: &Layout) -> UndoCow {
        let n = layout.chunks();
        UndoCow {
            words: vec![0u64; layout.total_words],
            // Nothing needs saving until a checkpoint exists.
            saved: vec![u64::MAX; n.div_ceil(64)],
            log: Vec::new(),
            slow_path: 0,
        }
    }

    #[inline(always)]
    fn get(&self, i: usize) -> u64 {
        // Exactly `plain`: one load, one bounds check, no chunk table on the read path.
        self.words[i]
    }

    #[inline(always)]
    fn set(&mut self, i: usize, v: u64) {
        let c = i >> CHUNK_SHIFT;
        if self.saved[c >> 6] & (1u64 << (c & 63)) == 0 {
            self.save(c);
        }
        self.words[i] = v;
    }

    fn snapshot(&mut self) -> UndoSnap {
        let bytes = self.log_bytes();
        // One level: the previous checkpoint's log is dropped because only the newest
        // checkpoint stays restorable. A chained implementation would seal it instead.
        self.log.clear();
        self.saved.iter_mut().for_each(|w| *w = 0);
        UndoSnap { bytes }
    }

    fn restore(&mut self, _snap: &UndoSnap) {
        // The bitmap guarantees each chunk appears at most once per log, so the order
        // within one log does not matter; popping keeps it obviously correct anyway.
        while let Some((c, chunk)) = self.log.pop() {
            let start = c as usize * CHUNK_WORDS;
            self.words[start..start + CHUNK_WORDS].copy_from_slice(&chunk[..]);
        }
        self.saved.iter_mut().for_each(|w| *w = 0);
    }

    fn words(&self) -> usize {
        self.words.len()
    }
}

// ================================================================================
// (b3, chained) sealed undo logs, redo capture and the convergence jump
// ================================================================================
//
// `UndoCow` above keeps one level. DESIGN §5.3 needs more: an edit near the top of a long
// document restarts from an OLD checkpoint, re-executes, and when the new run's state hash
// matches the old run's at a later checkpoint j, it jumps back to the old run's final state
// and keeps the old checkpoints from j onward. `UndoChain` does that:
//
// * `checkpoint()` seals the open log and starts a new one. `logs[i]` holds, for every
//   chunk first written after checkpoint `ids[i]` and before the next one, that chunk's
//   contents *at* checkpoint `ids[i]`. The newest log is the open one.
// * `restore_branch(k)` walks the logs from k to the newest and, for each chunk, restores
//   the value from the OLDEST log that holds it — that is the chunk's value at k, because
//   nothing touched it between k and that log's checkpoint. Before overwriting a live chunk
//   it copies it into a **redo log**: the old run's final contents of every chunk the
//   restore changed. The old logs from k onward are detached into the returned `Branch`,
//   not destroyed.
// * `converge(branch, j)` is the jump. Precondition: the live state equals the old run's
//   state at j (the §5.3 hash check) and a checkpoint was just taken there. A chunk the old
//   run never wrote after j already holds its final value, so only chunks in the old logs
//   from j onward need the redo value. The old checkpoints j.. are re-attached with their
//   logs intact, so every one of them is restorable again.
// * `retain(keep)` drops checkpoints and merges each dropped log into its predecessor,
//   keeping the older copy of a chunk both hold. Entries move; no chunk is copied.
//
// The write barrier is byte-for-byte `UndoCow`'s: one bitmap test and, once per chunk per
// checkpoint interval, one 16 KB copy. Only `checkpoint` (seal instead of free) and restore
// differ.

use crate::layout::CHUNK_BYTES;

pub type CheckpointId = u64;

type Saved = (u32, Box<PlainChunk>);

/// A fresh heap copy of one chunk's worth of words: exactly one 16 KB `memcpy`.
fn boxed_copy(src: &[u64]) -> Box<PlainChunk> {
    assert_eq!(src.len(), CHUNK_WORDS);
    let mut boxed: Box<std::mem::MaybeUninit<PlainChunk>> = Box::new_uninit();
    // SAFETY: the box is freshly allocated and uniquely owned, src has exactly CHUNK_WORDS
    // initialised words, and the two allocations cannot overlap.
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), boxed.as_mut_ptr() as *mut u64, CHUNK_WORDS);
        boxed.assume_init()
    }
}

#[derive(Default)]
pub struct Log {
    entries: Vec<Saved>,
}

impl Log {
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// How a restore walks the logs. `Dedup` is the implementation; `Naive` is measured only to
/// show what the dedup buys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Walk {
    /// Oldest log first, each chunk restored once: cost ∝ distinct chunks.
    Dedup,
    /// Newest log first, every entry applied, older values overwriting newer:
    /// cost ∝ total entries.
    Naive,
}

/// The old run's future, detached by `restore_branch` until the new run converges.
pub struct Branch {
    /// Old checkpoints from the restore target (first) to the old newest.
    ids: Vec<CheckpointId>,
    /// Their logs, same indexing.
    logs: Vec<Log>,
    /// The live contents, at the moment of the restore, of every chunk it overwrote.
    redo: Vec<Saved>,
    /// Log entries walked, for the report.
    pub entries_walked: usize,
}

impl Branch {
    pub fn redo_chunks(&self) -> usize {
        self.redo.len()
    }
    pub fn logs_walked(&self) -> usize {
        self.logs.len()
    }
}

pub struct UndoChain {
    words: Vec<u64>,
    /// Set: this chunk's pre-image is already in the open log.
    saved: Vec<u64>,
    ids: Vec<CheckpointId>,
    logs: Vec<Log>,
    next_id: CheckpointId,
    /// Scratch bitmap for dedup and merges.
    mark: Vec<u64>,
    pub slow_path: u64,
    /// Worker threads for deep restores and jumps. A restore is independent per chunk and
    /// bound by memory bandwidth, so it parallelises; 1 keeps it serial.
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

impl UndoChain {
    fn chunk(&self, c: usize) -> &[u64] {
        &self.words[c * CHUNK_WORDS..(c + 1) * CHUNK_WORDS]
    }

    #[inline(never)]
    #[cold]
    fn save(&mut self, c: usize) {
        let chunk = boxed_copy(self.chunk(c));
        self.logs
            .last_mut()
            .expect("save only happens once a checkpoint exists")
            .entries
            .push((c as u32, chunk));
        set_bit(&mut self.saved, c);
        self.slow_path += 1;
    }

    fn clear(bits: &mut [u64]) {
        bits.iter_mut().for_each(|w| *w = 0);
    }

    pub fn state(&self) -> &[u64] {
        &self.words
    }

    pub fn checkpoint_ids(&self) -> &[CheckpointId] {
        &self.ids
    }

    /// Chunk bytes held by the chain, plus the entry tables.
    pub fn chain_bytes(&self) -> usize {
        self.logs
            .iter()
            .map(|l| l.entries.len() * CHUNK_BYTES + l.entries.capacity() * 16)
            .sum()
    }

    pub fn log_entries(&self) -> usize {
        self.logs.iter().map(|l| l.len()).sum()
    }

    /// Seal the open log and start a new checkpoint. O(bitmap): 1.6 KB at 200 MB.
    pub fn checkpoint(&mut self) -> CheckpointId {
        let id = self.next_id;
        self.next_id += 1;
        self.ids.push(id);
        self.logs.push(Log::default());
        Self::clear(&mut self.saved);
        id
    }

    fn index_of(&self, id: CheckpointId) -> usize {
        // Linear, because a converged chain is not sorted by id: the new run's checkpoints
        // sit between old ones. A chain is at most a few thousand entries.
        self.ids
            .iter()
            .rposition(|&x| x == id)
            .unwrap_or_else(|| panic!("checkpoint {id} is not in the chain"))
    }

    /// Put the logs' pre-images back into the live state. With `capture`, first copy each
    /// overwritten live chunk into the returned redo log.
    fn apply(&mut self, logs: &[Log], walk: Walk, capture: bool) -> (Vec<Saved>, usize) {
        let mut redo = Vec::new();
        let mut walked = 0usize;
        let mut mark = std::mem::take(&mut self.mark);
        Self::clear(&mut mark);
        match walk {
            Walk::Dedup => {
                for log in logs {
                    for (c, data) in &log.entries {
                        walked += 1;
                        let c = *c as usize;
                        if bit(&mark, c) {
                            continue;
                        }
                        set_bit(&mut mark, c);
                        if capture {
                            redo.push((c as u32, boxed_copy(self.chunk(c))));
                        }
                        self.words[c * CHUNK_WORDS..(c + 1) * CHUNK_WORDS]
                            .copy_from_slice(&data[..]);
                    }
                }
            }
            Walk::Naive => {
                for log in logs.iter().rev() {
                    for (c, data) in &log.entries {
                        walked += 1;
                        let c = *c as usize;
                        if capture && !bit(&mark, c) {
                            set_bit(&mut mark, c);
                            redo.push((c as u32, boxed_copy(self.chunk(c))));
                        }
                        self.words[c * CHUNK_WORDS..(c + 1) * CHUNK_WORDS]
                            .copy_from_slice(&data[..]);
                    }
                }
            }
        }
        self.mark = mark;
        (redo, walked)
    }

    /// `apply(.., Walk::Dedup, capture = true)` on `threads` workers: plan the distinct
    /// chunks serially (a pointer walk), then capture and copy them in parallel.
    fn apply_parallel(&mut self, logs: &[Log]) -> (Vec<Saved>, usize) {
        let mut mark = std::mem::take(&mut self.mark);
        Self::clear(&mut mark);
        let mut walked = 0usize;
        let mut plan: Vec<(u32, &PlainChunk)> = Vec::new();
        for log in logs {
            for (c, data) in &log.entries {
                walked += 1;
                if !bit(&mark, *c as usize) {
                    set_bit(&mut mark, *c as usize);
                    plan.push((*c, &**data));
                }
            }
        }
        self.mark = mark;
        let redo = copy_into(&mut self.words, plan, self.threads, true);
        (redo, walked)
    }

    /// Restore to `id` and discard every later checkpoint: the plain restart.
    pub fn restore_discard(&mut self, id: CheckpointId) {
        let k = self.index_of(id);
        let later = self.logs.split_off(k);
        self.ids.truncate(k + 1);
        let _ = self.apply(&later, Walk::Dedup, false);
        self.logs.push(Log::default());
        Self::clear(&mut self.saved);
    }

    /// Restore to `id`, keeping the old run's later checkpoints and a redo log in the
    /// returned branch so that `converge` can jump back to the old run's end.
    pub fn restore_branch(&mut self, id: CheckpointId, walk: Walk) -> Branch {
        let k = self.index_of(id);
        let old_logs = self.logs.split_off(k);
        let old_ids = self.ids.split_off(k);
        let (redo, walked) = if walk == Walk::Dedup && self.threads > 1 {
            self.apply_parallel(&old_logs)
        } else {
            self.apply(&old_logs, walk, true)
        };
        // Checkpoint k itself stays in the chain, with a fresh open log.
        self.ids.push(old_ids[0]);
        self.logs.push(Log::default());
        Self::clear(&mut self.saved);
        Branch {
            ids: old_ids,
            logs: old_logs,
            redo,
            entries_walked: walked,
        }
    }

    /// The convergence jump. The live state must equal the old run's state at checkpoint
    /// `old` (§5.3's hash check has said so), and the newest checkpoint must have been taken
    /// at that point, with nothing written since.
    pub fn converge(&mut self, branch: Branch, old: CheckpointId) {
        let j = branch
            .ids
            .iter()
            .position(|&x| x == old)
            .unwrap_or_else(|| panic!("checkpoint {old} is not in this branch"));
        assert!(
            self.logs.last().is_some_and(|l| l.is_empty()),
            "take a checkpoint at the convergence point, and write nothing, before converging"
        );
        // The new checkpoint at the convergence point is the old checkpoint j: same state.
        self.ids.pop();
        self.logs.pop();

        let Branch {
            mut ids,
            mut logs,
            redo,
            ..
        } = branch;
        let keep_ids = ids.split_off(j);
        let keep_logs = logs.split_off(j);
        drop(logs); // the old run's logs between the restore point and j are superseded

        // Only chunks the old run wrote after j differ between old@j and the old end.
        let mut mark = std::mem::take(&mut self.mark);
        Self::clear(&mut mark);
        for log in &keep_logs {
            for (c, _) in &log.entries {
                set_bit(&mut mark, *c as usize);
            }
        }
        let jobs: Vec<(u32, &PlainChunk)> = redo
            .iter()
            .filter(|(c, _)| bit(&mark, *c as usize))
            .map(|(c, d)| (*c, &**d))
            .collect();
        copy_into(&mut self.words, jobs, self.threads, false);
        self.mark = mark;
        drop(redo);

        self.ids.extend(keep_ids);
        self.logs.extend(keep_logs);
        // The open log is the old run's; its chunks are already saved.
        Self::clear(&mut self.saved);
        let open: Vec<usize> = self
            .logs
            .last()
            .map(|l| l.entries.iter().map(|(c, _)| *c as usize).collect())
            .unwrap_or_default();
        for c in open {
            set_bit(&mut self.saved, c);
        }
    }

    /// Drop every checkpoint `keep` rejects, except the newest, merging each dropped log
    /// into its predecessor (the older pre-image of a chunk wins). Dropping the oldest
    /// checkpoint discards its log. Entries move; nothing is copied.
    pub fn retain(&mut self, keep: impl Fn(CheckpointId) -> bool) {
        let n = self.ids.len();
        let ids = std::mem::take(&mut self.ids);
        let logs = std::mem::take(&mut self.logs);
        let mut mark = std::mem::take(&mut self.mark);
        let mut out_ids: Vec<CheckpointId> = Vec::with_capacity(n);
        let mut out_logs: Vec<Log> = Vec::with_capacity(n);
        for (i, (id, log)) in ids.into_iter().zip(logs).enumerate() {
            if i + 1 == n || keep(id) {
                out_ids.push(id);
                out_logs.push(log);
            } else if let Some(dst) = out_logs.last_mut() {
                Self::clear(&mut mark);
                for (c, _) in &dst.entries {
                    set_bit(&mut mark, *c as usize);
                }
                for (c, data) in log.entries {
                    if !bit(&mark, c as usize) {
                        dst.entries.push((c, data));
                    }
                }
            }
            // else: the oldest checkpoint is being dropped, and its log with it.
        }
        self.ids = out_ids;
        self.logs = out_logs;
        self.mark = mark;
    }
}

/// Copy each `(chunk, source)` into the live words, optionally capturing the live contents
/// first, split across `threads` scoped workers. Chunks are distinct, so the workers write
/// disjoint slices: each job takes its chunk's `&mut` out of a slot table, which is what lets
/// this stay safe Rust.
fn copy_into(
    words: &mut [u64],
    jobs: Vec<(u32, &PlainChunk)>,
    threads: usize,
    capture: bool,
) -> Vec<Saved> {
    // Below this, spawning costs more than it saves (tens of microseconds per thread).
    // Under test the layouts are tiny, so drop the threshold or the workers never run.
    const PARALLEL_MIN: usize = if cfg!(test) { 8 } else { 256 };
    let mut slots: Vec<Option<&mut [u64]>> = words.chunks_mut(CHUNK_WORDS).map(Some).collect();
    let mut work: Vec<(u32, &mut [u64], &PlainChunk)> = jobs
        .into_iter()
        .map(|(c, src)| {
            let live = slots[c as usize]
                .take()
                .expect("each chunk appears once in a restore plan");
            (c, live, src)
        })
        .collect();
    let run = |part: &mut [(u32, &mut [u64], &PlainChunk)]| -> Vec<Saved> {
        let mut out = Vec::with_capacity(if capture { part.len() } else { 0 });
        for (c, live, src) in part.iter_mut() {
            if capture {
                out.push((*c, boxed_copy(live)));
            }
            live.copy_from_slice(&src[..]);
        }
        out
    };
    if threads <= 1 || work.len() < PARALLEL_MIN {
        return run(&mut work);
    }
    let per = work.len().div_ceil(threads);
    std::thread::scope(|s| {
        let handles: Vec<_> = work
            .chunks_mut(per)
            .map(|part| s.spawn(move || run(part)))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("restore worker panicked"))
            .collect()
    })
}

pub struct ChainSnap {
    pub id: CheckpointId,
}

impl Snapshot for ChainSnap {
    /// A chained checkpoint's cost is its log, which lives in the chain, not the handle;
    /// `UndoChain::chain_bytes` reports it.
    fn nominal_bytes(&self) -> usize {
        0
    }
}

impl Backend for UndoChain {
    type Snap = ChainSnap;

    fn name(&self) -> &'static str {
        "flat-undo-chain"
    }

    fn new(layout: &Layout) -> UndoChain {
        let n = layout.chunks();
        UndoChain {
            words: vec![0u64; layout.total_words],
            // Nothing needs saving until a checkpoint exists.
            saved: vec![u64::MAX; n.div_ceil(64)],
            ids: Vec::new(),
            logs: Vec::new(),
            next_id: 0,
            mark: vec![0u64; n.div_ceil(64)],
            slow_path: 0,
            threads: 1,
        }
    }

    #[inline(always)]
    fn get(&self, i: usize) -> u64 {
        self.words[i]
    }

    #[inline(always)]
    fn set(&mut self, i: usize, v: u64) {
        let c = i >> CHUNK_SHIFT;
        if self.saved[c >> 6] & (1u64 << (c & 63)) == 0 {
            self.save(c);
        }
        self.words[i] = v;
    }

    fn snapshot(&mut self) -> ChainSnap {
        ChainSnap {
            id: self.checkpoint(),
        }
    }

    fn restore(&mut self, snap: &ChainSnap) {
        self.restore_discard(snap.id);
    }

    fn words(&self) -> usize {
        self.words.len()
    }
}

#[cfg(test)]
mod chain_tests {
    use super::*;
    use crate::harness::run_page;
    use crate::workload::{Generator, Locality, PageShape};

    struct Run {
        chain: UndoChain,
        ids: Vec<CheckpointId>,
        copies: Vec<Vec<u64>>,
        end: Vec<u64>,
    }

    /// `n` checkpoints, one page of writes after each, with a full copy of the state at
    /// every checkpoint to check restores against bit for bit.
    fn run(n: usize, seed: u64) -> Run {
        let l = Layout::with_mem_words("chain-test", 200_000);
        let mut chain = UndoChain::new(&l);
        let mut g = Generator::new(l, Locality::tex_freelist(), PageShape::touch(2), seed);
        let mut buf = Vec::new();
        g.page(&mut buf);
        let mut acc = run_page::<UndoChain, 0>(&mut chain, &buf, 1);
        let mut ids = Vec::new();
        let mut copies = Vec::new();
        for _ in 0..n {
            ids.push(chain.checkpoint());
            copies.push(chain.state().to_vec());
            g.page(&mut buf);
            acc = run_page::<UndoChain, 0>(&mut chain, &buf, acc);
        }
        std::hint::black_box(acc);
        let end = chain.state().to_vec();
        Run {
            chain,
            ids,
            copies,
            end,
        }
    }

    #[test]
    fn round_trip_to_every_checkpoint_both_walks() {
        let _ledger = crate::ledger_lock();
        let mut r = run(30, 11);
        // Out of order on purpose: every restore must start from the converged end state.
        let mut order: Vec<usize> = (0..r.ids.len()).collect();
        order.reverse();
        order.rotate_left(7);
        for walk in [Walk::Dedup, Walk::Naive] {
            for &i in &order {
                let br = r.chain.restore_branch(r.ids[i], walk);
                assert!(
                    r.chain.state() == &r.copies[i][..],
                    "{walk:?} restore to {i}"
                );
                r.chain.converge(br, r.ids[i]);
                assert!(r.chain.state() == &r.end[..], "{walk:?} jump back from {i}");
                assert_eq!(r.chain.checkpoint_ids(), &r.ids[..]);
            }
        }
    }

    #[test]
    fn compaction_keeps_every_retained_checkpoint_exact() {
        let _ledger = crate::ledger_lock();
        let mut r = run(40, 23);
        let before = r.chain.chain_bytes();
        let last = *r.ids.last().unwrap();
        r.chain.retain(|id| id % 5 == 0 || id == last);
        let kept: Vec<CheckpointId> = r
            .ids
            .iter()
            .copied()
            .filter(|id| id % 5 == 0 || *id == last)
            .collect();
        assert_eq!(r.chain.checkpoint_ids(), &kept[..]);
        assert!(r.chain.chain_bytes() < before, "merging must deduplicate");
        for &id in &kept {
            let i = r.ids.iter().position(|&x| x == id).unwrap();
            let br = r.chain.restore_branch(id, Walk::Dedup);
            assert!(r.chain.state() == &r.copies[i][..], "restore to kept {id}");
            r.chain.converge(br, id);
            assert!(r.chain.state() == &r.end[..]);
        }
        // Dropping the oldest discards its log and nothing else.
        let first = kept[0];
        r.chain.retain(|id| id != first);
        assert_eq!(r.chain.checkpoint_ids(), &kept[1..]);
        let i = r.ids.iter().position(|&x| x == kept[1]).unwrap();
        let br = r.chain.restore_branch(kept[1], Walk::Dedup);
        assert!(r.chain.state() == &r.copies[i][..]);
        r.chain.converge(br, kept[1]);
    }

    /// The §5.3 scenario end to end: restart from an old checkpoint, run a *different*
    /// edit that dirties other chunks, reach the old run's state at a later checkpoint by
    /// another path, converge, and the jump must reproduce the old end bit for bit — with
    /// the old later checkpoints and the new run's own checkpoints all still restorable.
    #[test]
    fn restore_old_mutate_converge_and_redo_jump() {
        let _ledger = crate::ledger_lock();
        let mut r = run(30, 37);
        let (k, j) = (5usize, 20usize);
        let br = r.chain.restore_branch(r.ids[k], Walk::Dedup);
        assert!(r.chain.state() == &r.copies[k][..]);

        let words = r.chain.words();
        // The edit: scribble over every 7th chunk, checkpoint, scribble some more.
        for c in (0..words / CHUNK_WORDS).step_by(7) {
            r.chain.set(c * CHUNK_WORDS + 3, 0xBAD0_0000 + c as u64);
        }
        let n1 = r.chain.checkpoint();
        let at_n1 = r.chain.state().to_vec();
        for c in (0..words / CHUNK_WORDS).step_by(5) {
            r.chain.set(c * CHUNK_WORDS + 11, 0xF00D_0000 + c as u64);
        }
        // Reach old@j by a different path: rewrite exactly the words that differ.
        for w in 0..words {
            let want = r.copies[j][w];
            if r.chain.state()[w] != want {
                r.chain.set(w, want);
            }
        }
        assert!(r.chain.state() == &r.copies[j][..], "converged state");
        let _nj = r.chain.checkpoint();
        r.chain.converge(br, r.ids[j]);

        assert!(
            r.chain.state() == &r.end[..],
            "redo jump must reproduce the old end"
        );
        let mut expect: Vec<CheckpointId> = r.ids[..=k].to_vec();
        expect.push(n1);
        expect.extend_from_slice(&r.ids[j..]);
        assert_eq!(r.chain.checkpoint_ids(), &expect[..]);

        // Old later checkpoints survived the jump intact.
        for m in j..r.ids.len() {
            let br = r.chain.restore_branch(r.ids[m], Walk::Dedup);
            assert!(r.chain.state() == &r.copies[m][..], "old checkpoint {m}");
            r.chain.converge(br, r.ids[m]);
            assert!(r.chain.state() == &r.end[..]);
        }
        // So did the new run's own checkpoint, and the one it started from.
        let br = r.chain.restore_branch(n1, Walk::Dedup);
        assert!(r.chain.state() == &at_n1[..], "new-run checkpoint");
        r.chain.converge(br, n1);
        let br = r.chain.restore_branch(r.ids[k], Walk::Naive);
        assert!(r.chain.state() == &r.copies[k][..]);
        r.chain.converge(br, r.ids[k]);
        assert!(r.chain.state() == &r.end[..]);

        // Writing after the jump must keep logging correctly into the old open log: the
        // saved bitmap was rebuilt from it, so restoring to the old newest still works.
        for c in (0..words / CHUNK_WORDS).step_by(3) {
            r.chain.set(c * CHUNK_WORDS, 0x5EED_0000 + c as u64);
        }
        let last = *r.ids.last().unwrap();
        r.chain.restore_discard(last);
        assert!(
            r.chain.state() == &r.copies[r.ids.len() - 1][..],
            "after post-jump writes"
        );
    }

    #[test]
    fn parallel_restore_and_jump_are_exact() {
        let _ledger = crate::ledger_lock();
        let mut r = run(30, 41);
        r.chain.threads = 4;
        for i in [0usize, 3, 15, 29] {
            let br = r.chain.restore_branch(r.ids[i], Walk::Dedup);
            assert!(
                r.chain.state() == &r.copies[i][..],
                "parallel restore to {i}"
            );
            r.chain.converge(br, r.ids[i]);
            assert!(r.chain.state() == &r.end[..], "parallel jump from {i}");
        }
    }

    #[test]
    fn discard_restore_then_continue() {
        let _ledger = crate::ledger_lock();
        let mut r = run(12, 5);
        r.chain.restore_discard(r.ids[4]);
        assert!(r.chain.state() == &r.copies[4][..]);
        assert_eq!(r.chain.checkpoint_ids(), &r.ids[..=4]);
        r.chain.set(0, 42);
        r.chain.restore_discard(r.ids[4]);
        assert!(r.chain.state() == &r.copies[4][..]);
    }
}
