//! `changes/throughput.ch` [9]: a direct-mapped index in front of
//! `id_lookup`'s chain walk (DESIGN.md §5.6 item 3, "a fast `\csname`
//! lookup"; lane L6, `docs/design/engine-v2/L6.md` §4, E2).
//!
//! `id_lookup` (tex.web §259) hashes a name into one of `hash_prime` = 8,501
//! chains and walks it. A LaTeX format holds tens of thousands of names, so
//! the walk passes several entries, and each costs two dependent loads
//! (`hash`, then `str_start`) before the one whose text is the name.
//!
//! [`Host`] is the engine's state outside the word space (web2rust's
//! `--host-state`): a table of [`SLOTS`] slots, each holding a location the
//! walk returned and a tag, both taken from a 64-bit fingerprint of the name.
//! - [`Globals::flashtex_csx_find`] answers a location only when its slot's
//!   tag matches and the location's text in the current state is the name,
//!   character for character.
//! - Otherwise it answers 0. The walk then runs as before, and
//!   [`Globals::flashtex_csx_note`] stores where it ended.
//!
//! The proof that an answer is the walk's is in `changes/throughput.ch` [9].
//! In short, at most one location in the hash proper holds a given name, and
//! the walk from that name's chain finds it. The index stores only locations
//! the walk returned. And the check reads the state as it is now, so restores,
//! convergence jumps, loaded formats and names that `readset.rs` takes out
//! cannot make it answer wrongly. A collision or a stale slot only costs a
//! walk.
//!
//! `FLASHTEX_CSX=off` turns the index off (the walk alone, the ablation's
//! other side). `FLASHTEX_CSX=verify` also runs the walk for every answer
//! the index gives, and panics if they differ.

use crate::generated::Globals;

/// log2 of the number of slots: 64 Ki slots of 8 bytes, 512 KiB, about the
/// number of names a large LaTeX document's format and packages hold.
const BITS: u32 = 16;
pub const SLOTS: usize = 1 << BITS;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    On,
    Off,
    Verify,
}

/// The engine's state outside the word space (`Globals::host`).
#[derive(Debug)]
pub struct Host {
    /// slot -> (tag << 32) | location; 0 is empty (no location is 0)
    csx: Vec<u64>,
    /// the last miss (`flashtex_csx_find`): j, l, slot, tag
    miss: (i32, i32, usize, u32),
    mode: Option<Mode>,
    /// answers given by the index, misses, entries stored
    pub hits: u64,
    pub misses: u64,
}

impl Default for Host {
    fn default() -> Self {
        Host {
            csx: Vec::new(),
            miss: (-1, 0, 0, 0),
            mode: None,
            hits: 0,
            misses: 0,
        }
    }
}

impl Host {
    /// The ablation's switch for one engine, whatever `FLASHTEX_CSX` says
    /// (tests).
    pub fn set_enabled(&mut self, on: bool) {
        self.mode = Some(if on { Mode::On } else { Mode::Off });
    }

    /// Verify mode for one engine (tests).
    pub fn set_verify(&mut self) {
        self.mode = Some(Mode::Verify);
    }

    fn mode(&mut self) -> Mode {
        *self
            .mode
            .get_or_insert_with(|| match std::env::var("FLASHTEX_CSX").as_deref() {
                Ok("off") | Ok("0") => Mode::Off,
                Ok("verify") => Mode::Verify,
                _ => Mode::On,
            })
    }
}

/// A 64-bit fingerprint of a name: eight characters (each below 256) packed
/// into a word, then one multiply per word.
#[inline]
fn fingerprint(name: &[i32]) -> u64 {
    const K: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut h = (name.len() as u64).wrapping_mul(K);
    let (words, rest) = name.as_chunks::<8>();
    for w in words {
        let mut x = 0u64;
        for (i, &c) in w.iter().enumerate() {
            x |= (c as u8 as u64) << (8 * i);
        }
        h = (h.rotate_left(23) ^ x).wrapping_mul(K);
    }
    if !rest.is_empty() {
        let mut x = 0u64;
        for (i, &c) in rest.iter().enumerate() {
            x |= (c as u8 as u64) << (8 * i);
        }
        h = (h.rotate_left(23) ^ x).wrapping_mul(K);
    }
    h ^ (h >> 29)
}

impl Globals {
    /// `id_lookup`'s answer for the name `buffer[j..j+l-1]` if the index
    /// holds it, else 0 (`changes/throughput.ch` [9]).
    #[inline]
    pub fn flashtex_csx_find(&mut self, j: i32, l: i32) -> i32 {
        let mode = self.host.mode();
        if mode == Mode::Off {
            return 0;
        }
        let (a, n) = (j as usize, l as usize);
        let f = fingerprint(&self.buffer[a..a + n]);
        let slot = (f >> (64 - BITS)) as usize;
        let tag = (f >> 8) as u32;
        if self.host.csx.is_empty() {
            self.host.csx = vec![0; SLOTS];
        }
        let e = self.host.csx[slot];
        if e != 0 && (e >> 32) as u32 == tag {
            let p = e as u32 as i32;
            if self.csx_holds(p, a, n) {
                self.host.hits += 1;
                if mode == Mode::Verify {
                    let w = self.csx_walk(a, n);
                    assert_eq!(
                        w, p,
                        "FLASHTEX_CSX=verify: the index answered {p} for a name the walk finds at {w}"
                    );
                }
                return p;
            }
        }
        self.host.misses += 1;
        self.host.miss = (j, l, slot, tag);
        0
    }

    /// The walk of `id_lookup` for `buffer[j..j+l-1]` ended at `p`, which
    /// now holds that name (found, or entered).
    #[inline]
    pub fn flashtex_csx_note(&mut self, j: i32, l: i32, p: i32) {
        let (mj, ml, slot, tag) = self.host.miss;
        if self.host.csx.is_empty() || p <= 0 {
            return;
        }
        let (slot, tag) = if (mj, ml) == (j, l) {
            (slot, tag)
        } else {
            // (not the last miss: every hook between them left it alone so
            // far, but a stale slot must not be used)
            let f = fingerprint(&self.buffer[j as usize..(j + l) as usize]);
            ((f >> (64 - BITS)) as usize, (f >> 8) as u32)
        };
        self.host.csx[slot] = ((tag as u64) << 32) | p as u32 as u64;
        self.host.miss = (-1, 0, 0, 0);
    }

    /// Whether location `p` of the hash holds the name `buffer[a..a+n]` in
    /// the current state.
    #[inline]
    fn csx_holds(&self, p: i32, a: usize, n: usize) -> bool {
        use crate::generated::consts::hash_base;
        let Some(w) = self.hash.get((p - hash_base) as usize) else {
            return false;
        };
        let t = w.rh();
        if t <= 0 || t >= self.str_ptr {
            return false;
        }
        let (s0, s1) = (
            self.str_start[t as usize] as usize,
            self.str_start[t as usize + 1] as usize,
        );
        // (a loop, not a slice comparison: names are short, and a call of
        // `memcmp` costs more than comparing them; strsearch.rs)
        s1 - s0 == n
            && self.str_pool[s0..s1]
                .iter()
                .zip(&self.buffer[a..a + n])
                .all(|(x, y)| x == y)
    }

    /// tex.web §259's search alone, without its insertion: the location of
    /// the name `buffer[a..a+n]`, or 0 if no chain holds it (verify mode and
    /// tests).
    pub fn csx_walk(&self, a: usize, n: usize) -> i32 {
        use crate::generated::consts::{hash_base, hash_prime};
        let b = &self.buffer[a..a + n];
        let mut h = b[0];
        for &c in &b[1..] {
            h = h + h + c;
            while h >= hash_prime {
                h -= hash_prime;
            }
        }
        let mut p = h + hash_base;
        loop {
            if self.csx_holds(p, a, n) {
                return p;
            }
            let next = self.hash[(p - hash_base) as usize].lh();
            if next == 0 {
                return 0;
            }
            p = next;
        }
    }
}
