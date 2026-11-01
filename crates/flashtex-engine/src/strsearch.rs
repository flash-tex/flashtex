//! `changes/throughput.ch` [8]: the scan of tex.ch's `search_string`
//! (`changes/filenames.ch`), which `end_name` runs for every file name it
//! scans (each `\input`, `\openin`, `\openout` and `\font`) to give an
//! equal string already in the pool back instead of a new one.
//!
//! The loop it replaces tests the strings `search - 1`, `search - 2`, ...,
//! 256 in that order and returns the first whose length is `len` and whose
//! characters are `search`'s (`str_eq_str`), else 0. Nearly every string
//! fails the length test, which is all the loop does for a LaTeX format's
//! tens of thousands of strings: 13.6 M instructions per keystroke on
//! full-10 letter@start (T7-CONV, `docs/evidence/t7-conv-2026-10-20/`).
//!
//! Here the length test runs over a block of [`BLOCK`] strings at a time
//! with no early exit, which the compiler vectorises, into a mask of the
//! strings of length `len`; those are then compared character by character
//! newest first, the original order. So the strings compared are the
//! strings the loop compares, in its order, and the result (which string,
//! or 0) is the same. Neither version writes anything.

use crate::generated::types::str_number;
use crate::generated::Globals;

/// Strings whose lengths are compared at once.
const BLOCK: usize = 32;

impl Globals {
    /// The newest string below `search` and above 255 that equals it, or
    /// 0; `len = length(search) > 0`.
    pub fn flashtex_search_pool(&mut self, search: str_number, len: i32) -> str_number {
        let start: &[i32] = &self.str_start[..];
        let pool: &[i32] = &self.str_pool[..];
        let a = start[search as usize] as usize;
        let want = &pool[a..a + len as usize];
        // `str_eq_str(s, search)` for a string `s` of length `len` (a loop,
        // not a slice comparison: names are short, and a call of `memcmp`
        // costs more than comparing them)
        let equal = |s: usize| {
            let b = start[s] as usize;
            pool[b..b + len as usize]
                .iter()
                .zip(want)
                .all(|(x, y)| x == y)
        };
        // the strings below `hi` are still to be tested
        let mut hi = search as usize;
        while hi >= 256 + BLOCK {
            let lo = hi - BLOCK;
            let w: &[i32; BLOCK + 1] = start[lo..=hi].try_into().unwrap();
            // bit i: string lo + i has length len
            let mut mask = 0u32;
            for i in 0..BLOCK {
                mask |= ((w[i + 1].wrapping_sub(w[i]) == len) as u32) << i;
            }
            while mask != 0 {
                let i = 31 - mask.leading_zeros() as usize;
                if equal(lo + i) {
                    return (lo + i) as str_number;
                }
                mask &= !(1 << i);
            }
            hi = lo;
        }
        for s in (256..hi).rev() {
            if start[s + 1].wrapping_sub(start[s]) == len && equal(s) {
                return s as str_number;
            }
        }
        0
    }
}
