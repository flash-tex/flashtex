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
        self.log.len() * CHUNK_WORDS * 8
            + self.log.capacity() * std::mem::size_of::<(u32, usize)>()
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
