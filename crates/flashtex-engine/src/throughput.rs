//! Routines that compute exactly what pdftex.web's sections compute, faster
//! (changes/throughput.ch; DESIGN.md §4.2's allowed optimisations, each
//! carrying its proof of identity in the change file).

use crate::generated::consts::hash_prime;
use crate::generated::Globals;

/// Characters added to the 64-bit hash between two reductions: from a
/// value below `hash_prime` (< 2^20), 40 doublings and additions of a
/// character (< 2^8) stay below 2^61.
const HASH_RUN: usize = 40;

impl Globals {
    /// §261's hash code of `buffer[j..j+l]` (changes/throughput.ch [5]):
    /// `buffer[j]` doubled and added to, character by character, modulo
    /// `hash_prime`, reduced every `HASH_RUN` characters instead of after
    /// each one.
    #[inline]
    pub fn tp_hash_code(&self, j: i32, l: i32) -> i32 {
        let (j, l) = (j as usize, l.max(1) as usize);
        let b = &self.buffer[j..j + l];
        let p = hash_prime as u64;
        let mut h: u64 = 0;
        for run in b.chunks(HASH_RUN) {
            for &c in run {
                h = 2 * h + c as u64;
            }
            h %= p;
        }
        h as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 64-bit hash equals §261's step-by-step one for every length up
    /// to 300 and characters across 0..=255 (the buffer's range).
    #[test]
    fn the_hash_code_equals_sections_261() {
        let mut g = Globals::new();
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        for l in 1..=300usize {
            for _ in 0..20 {
                for k in 0..l {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    g.buffer[k] = (x % 256) as _;
                }
                let mut h = g.buffer[0] as i32;
                for k in 1..l {
                    h = h + h + g.buffer[k] as i32;
                    while h >= hash_prime {
                        h -= hash_prime;
                    }
                }
                assert_eq!(g.tp_hash_code(0, l as i32), h, "length {l}");
            }
        }
        for k in 0..300 {
            g.buffer[k] = 255;
        }
        let mut h = 255;
        for _ in 1..300 {
            h = h + h + 255;
            while h >= hash_prime {
                h -= hash_prime;
            }
        }
        assert_eq!(g.tp_hash_code(0, 300), h);
    }
}
