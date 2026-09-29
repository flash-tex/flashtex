//! The interface every candidate mechanism implements, plus the two non-copy-on-write
//! reference points: `plain` (no snapshot support at all, the slowdown denominator) and
//! `memcpy` (mechanism (c), a full copy of all arenas).

use crate::layout::Layout;

/// A snapshot handle. Mechanisms differ wildly in what this owns, so the only thing the
/// harness needs from it is how much memory it costs.
pub trait Snapshot {
    /// Bytes this snapshot is responsible for, over and above the live state.
    ///
    /// * `memcpy`: the whole state.
    /// * chunk CoW: 16 KB per chunk version this snapshot is the sole owner of.
    /// * kernel CoW: 16 KB per page the kernel has had to copy on this snapshot's
    ///   behalf, which is what the harness measures out of the task's resident size
    ///   rather than guessing.
    fn nominal_bytes(&self) -> usize;
}

// `name`, `words` and `checksum` are exercised by the correctness self-tests rather than
// by the measured phases, which know their own mechanism statically.
#[allow(dead_code)]
pub trait Backend: Sized {
    type Snap: Snapshot;

    fn name(&self) -> &'static str;
    fn new(layout: &Layout) -> Self;

    fn get(&self, i: usize) -> u64;
    fn set(&mut self, i: usize, v: u64);

    /// Take a checkpoint of the whole state.
    fn snapshot(&mut self) -> Self::Snap;

    /// Restore the live state from `snap`. `snap` stays valid: L3 restarts from the
    /// same checkpoint repeatedly while converging.
    fn restore(&mut self, snap: &Self::Snap);

    /// Checksum of the whole state, for the correctness self-tests.
    fn checksum(&self) -> u64 {
        // Overridden where a cheaper path exists; the default is mechanism-agnostic.
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        let n = self.words();
        let mut i = 0;
        while i < n {
            h ^= self.get(i);
            h = h.wrapping_mul(0x100_0000_01b3);
            i += 1;
        }
        h
    }

    fn words(&self) -> usize;
}

// --------------------------------------------------------------------------------
// plain: Vec<u64>. Snapshot unsupported; this is only the hot-loop denominator.
// --------------------------------------------------------------------------------

pub struct NoSnap;
impl Snapshot for NoSnap {
    fn nominal_bytes(&self) -> usize {
        0
    }
}

pub struct Plain {
    words: Vec<u64>,
}

impl Backend for Plain {
    type Snap = NoSnap;
    fn name(&self) -> &'static str {
        "plain"
    }
    fn new(layout: &Layout) -> Plain {
        Plain {
            words: vec![0u64; layout.total_words],
        }
    }
    #[inline(always)]
    fn get(&self, i: usize) -> u64 {
        self.words[i]
    }
    #[inline(always)]
    fn set(&mut self, i: usize, v: u64) {
        self.words[i] = v;
    }
    fn snapshot(&mut self) -> NoSnap {
        NoSnap
    }
    fn restore(&mut self, _snap: &NoSnap) {}
    fn words(&self) -> usize {
        self.words.len()
    }
}

// --------------------------------------------------------------------------------
// (c) memcpy: a full copy of all arenas per checkpoint.
// --------------------------------------------------------------------------------

pub struct FullCopySnap {
    words: Vec<u64>,
}
impl Snapshot for FullCopySnap {
    fn nominal_bytes(&self) -> usize {
        self.words.len() * 8
    }
}

pub struct FullCopy {
    words: Vec<u64>,
}

impl Backend for FullCopy {
    type Snap = FullCopySnap;
    fn name(&self) -> &'static str {
        "memcpy"
    }
    fn new(layout: &Layout) -> FullCopy {
        FullCopy {
            words: vec![0u64; layout.total_words],
        }
    }
    #[inline(always)]
    fn get(&self, i: usize) -> u64 {
        self.words[i]
    }
    #[inline(always)]
    fn set(&mut self, i: usize, v: u64) {
        self.words[i] = v;
    }
    fn snapshot(&mut self) -> FullCopySnap {
        // Vec::clone is one `memcpy` plus one allocation. An engine would keep a pool
        // of snapshot buffers; `reuse_into` below measures that variant.
        FullCopySnap {
            words: self.words.clone(),
        }
    }
    fn restore(&mut self, snap: &FullCopySnap) {
        self.words.copy_from_slice(&snap.words);
    }
    fn words(&self) -> usize {
        self.words.len()
    }
}

impl FullCopy {
    /// Snapshot into an already-allocated buffer: the pooled variant, which separates
    /// the `memcpy` cost from the allocator's.
    pub fn snapshot_into(&self, dst: &mut [u64]) {
        dst.copy_from_slice(&self.words);
    }
    pub fn buffer(&self) -> Vec<u64> {
        vec![0u64; self.words.len()]
    }
}
