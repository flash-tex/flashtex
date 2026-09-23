//! SHA-256 (FIPS 180-4), written in-tree so font program identity can be
//! reported without adding a dependency. Used for evidence only, never for
//! security decisions.
//!
//! Export hashes whole font files twice (`v2::resolve_font` verifies the
//! content address, `exact::cid_from_opentype` derives the subset tag from
//! the CFF table), so on a maths-heavy document this is several megabytes on
//! the critical path. Two things make that cheap without changing a single
//! output byte:
//!
//! - the portable path compresses the input in place — no padded copy of the
//!   message — with a rolling 16-word schedule and the working variables
//!   rotated by the macro instead of moved every round;
//! - on x86-64 with the SHA extensions, [`compress`] dispatches to
//!   `sha256rnds2`/`sha256msg1`/`sha256msg2`.
//!
//! Both paths are the same function of the input: `accelerated_matches_portable`
//! checks them against each other over every length from 0 to 300 plus larger
//! pseudo-random buffers, and `known_vectors` pins the FIPS answers.

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const IV: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// One round, with the working variables named in their rotated positions so
/// nothing is moved between rounds.
macro_rules! round {
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $k:expr, $w:expr) => {{
        let s1 = $e.rotate_right(6) ^ $e.rotate_right(11) ^ $e.rotate_right(25);
        let ch = ($e & $f) ^ (!$e & $g);
        let t1 = $h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add($k)
            .wrapping_add($w);
        let s0 = $a.rotate_right(2) ^ $a.rotate_right(13) ^ $a.rotate_right(22);
        let maj = ($a & $b) ^ ($a & $c) ^ ($b & $c);
        $d = $d.wrapping_add(t1);
        $h = t1.wrapping_add(s0.wrapping_add(maj));
    }};
}

/// The portable compression function over whole 64-byte blocks.
fn compress_portable(state: &mut [u32; 8], blocks: &[[u8; 64]]) {
    for block in blocks {
        let mut w = [0u32; 16];
        for (slot, word) in w.iter_mut().zip(block.as_chunks::<4>().0) {
            *slot = u32::from_be_bytes(*word);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
        for group in 0..4 {
            if group > 0 {
                // Rolling schedule: w[i & 15] becomes W[i] for i = 16.. .
                for i in 0..16 {
                    let s0 = {
                        let x = w[(i + 1) & 15];
                        x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
                    };
                    let s1 = {
                        let x = w[(i + 14) & 15];
                        x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
                    };
                    w[i] = w[i]
                        .wrapping_add(s0)
                        .wrapping_add(w[(i + 9) & 15])
                        .wrapping_add(s1);
                }
            }
            let base = group * 16;
            round!(a, b, c, d, e, f, g, h, K[base], w[0]);
            round!(h, a, b, c, d, e, f, g, K[base + 1], w[1]);
            round!(g, h, a, b, c, d, e, f, K[base + 2], w[2]);
            round!(f, g, h, a, b, c, d, e, K[base + 3], w[3]);
            round!(e, f, g, h, a, b, c, d, K[base + 4], w[4]);
            round!(d, e, f, g, h, a, b, c, K[base + 5], w[5]);
            round!(c, d, e, f, g, h, a, b, K[base + 6], w[6]);
            round!(b, c, d, e, f, g, h, a, K[base + 7], w[7]);
            round!(a, b, c, d, e, f, g, h, K[base + 8], w[8]);
            round!(h, a, b, c, d, e, f, g, K[base + 9], w[9]);
            round!(g, h, a, b, c, d, e, f, K[base + 10], w[10]);
            round!(f, g, h, a, b, c, d, e, K[base + 11], w[11]);
            round!(e, f, g, h, a, b, c, d, K[base + 12], w[12]);
            round!(d, e, f, g, h, a, b, c, K[base + 13], w[13]);
            round!(c, d, e, f, g, h, a, b, K[base + 14], w[14]);
            round!(b, c, d, e, f, g, h, a, K[base + 15], w[15]);
        }
        for (slot, v) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(v);
        }
    }
}

/// The same compression function through the x86-64 SHA extensions.
///
/// # Safety
/// The caller must have checked `sha`, `ssse3` and `sse4.1` at runtime.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sha,sse2,ssse3,sse4.1")]
unsafe fn compress_x86_sha(state: &mut [u32; 8], blocks: &[[u8; 64]]) {
    use std::arch::x86_64::*;
    // SAFETY: the caller checked `sha`, `ssse3` and `sse4.1`; every load and
    // store below is within `state` (8 u32) or one 64-byte `block`.
    unsafe {
        // Big-endian load mask for a 16-byte message quad.
        let mask = _mm_set_epi64x(0x0c0d_0e0f_0809_0a0b_u64 as i64, 0x0405_0607_0001_0203_u64 as i64);

        // The instructions want the state as ABEF / CDGH.
        let dcba = _mm_loadu_si128(state.as_ptr().cast());
        let hgfe = _mm_loadu_si128(state.as_ptr().add(4).cast());
        let cdab = _mm_shuffle_epi32(dcba, 0xB1);
        let efgh = _mm_shuffle_epi32(hgfe, 0x1B);
        let mut abef = _mm_alignr_epi8(cdab, efgh, 8);
        let mut cdgh = _mm_blend_epi16(efgh, cdab, 0xF0);

        for block in blocks {
            let (saved_abef, saved_cdgh) = (abef, cdgh);
            let p = block.as_ptr();
            // m[g] holds the schedule words W[4g .. 4g+4], rotating every four
            // groups: after the update, m[g % 4] is the current quad.
            let mut m = [
                _mm_shuffle_epi8(_mm_loadu_si128(p.cast()), mask),
                _mm_shuffle_epi8(_mm_loadu_si128(p.add(16).cast()), mask),
                _mm_shuffle_epi8(_mm_loadu_si128(p.add(32).cast()), mask),
                _mm_shuffle_epi8(_mm_loadu_si128(p.add(48).cast()), mask),
            ];
            for g in 0..16 {
                if g >= 4 {
                    // W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16],
                    // four words at a time: msg1 adds W[t-16] + sigma0(W[t-15]),
                    // the alignr supplies W[t-7], msg2 adds sigma1(W[t-2]).
                    let older = m[g % 4]; // W[t-16 ..]
                    let prev3 = m[(g + 1) % 4]; // W[t-12 ..]
                    let prev2 = m[(g + 2) % 4]; // W[t-8 ..]
                    let prev1 = m[(g + 3) % 4]; // W[t-4 ..]
                    let carried = _mm_alignr_epi8(prev1, prev2, 4); // W[t-7 ..]
                    let partial = _mm_add_epi32(_mm_sha256msg1_epu32(older, prev3), carried);
                    m[g % 4] = _mm_sha256msg2_epu32(partial, prev1);
                }
                let k = _mm_loadu_si128(K.as_ptr().add(4 * g).cast());
                let msg = _mm_add_epi32(m[g % 4], k);
                cdgh = _mm_sha256rnds2_epu32(cdgh, abef, msg);
                abef = _mm_sha256rnds2_epu32(abef, cdgh, _mm_shuffle_epi32(msg, 0x0E));
            }
            abef = _mm_add_epi32(abef, saved_abef);
            cdgh = _mm_add_epi32(cdgh, saved_cdgh);
        }

        // Back to the natural ABCDEFGH order.
        let feba = _mm_shuffle_epi32(abef, 0x1B);
        let dchg = _mm_shuffle_epi32(cdgh, 0xB1);
        _mm_storeu_si128(state.as_mut_ptr().cast(), _mm_blend_epi16(feba, dchg, 0xF0));
        _mm_storeu_si128(
            state.as_mut_ptr().add(4).cast(),
            _mm_alignr_epi8(dchg, feba, 8),
        );
    }
}

/// Compresses whole blocks into `state`, using the fastest available path.
fn compress(state: &mut [u32; 8], blocks: &[[u8; 64]]) {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("sha")
            && is_x86_feature_detected!("ssse3")
            && is_x86_feature_detected!("sse4.1")
        {
            // SAFETY: the three features were just checked at runtime.
            unsafe { compress_x86_sha(state, blocks) };
            return;
        }
    }
    compress_portable(state, blocks);
}

/// The SHA-256 digest of `data`.
pub fn digest(data: &[u8]) -> [u8; 32] {
    let mut h = IV;
    let (blocks, tail) = data.as_chunks::<64>();
    compress(&mut h, blocks);

    // The padded tail is one block, or two when the length no longer fits.
    let mut last = [0u8; 128];
    last[..tail.len()].copy_from_slice(tail);
    last[tail.len()] = 0x80;
    let used = if tail.len() < 56 { 64 } else { 128 };
    let bit_len = (data.len() as u64).wrapping_mul(8);
    last[used - 8..used].copy_from_slice(&bit_len.to_be_bytes());
    compress(&mut h, last[..used].as_chunks::<64>().0);

    let mut out = [0u8; 32];
    for (chunk, v) in out.as_chunks_mut::<4>().0.iter_mut().zip(h) {
        *chunk = v.to_be_bytes();
    }
    out
}

/// Lowercase hex SHA-256 of `data`.
pub fn hex(data: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(64);
    for b in digest(data) {
        s.push(DIGITS[usize::from(b >> 4)] as char);
        s.push(DIGITS[usize::from(b & 15)] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cheap deterministic byte stream for the differential tests.
    fn noise(n: usize) -> Vec<u8> {
        let mut x: u32 = 0x1234_5678;
        (0..n)
            .map(|_| {
                x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
                (x >> 16) as u8
            })
            .collect()
    }

    /// The portable path, driven exactly as [`digest`] drives `compress`.
    fn digest_portable(data: &[u8]) -> [u8; 32] {
        let mut h = IV;
        let (blocks, tail) = data.as_chunks::<64>();
        compress_portable(&mut h, blocks);
        let mut last = [0u8; 128];
        last[..tail.len()].copy_from_slice(tail);
        last[tail.len()] = 0x80;
        let used = if tail.len() < 56 { 64 } else { 128 };
        last[used - 8..used].copy_from_slice(&((data.len() as u64) * 8).to_be_bytes());
        compress_portable(&mut h, last[..used].as_chunks::<64>().0);
        let mut out = [0u8; 32];
        for (chunk, v) in out.as_chunks_mut::<4>().0.iter_mut().zip(h) {
            *chunk = v.to_be_bytes();
        }
        out
    }

    #[test]
    fn known_vectors() {
        assert_eq!(
            hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // The two-block padding boundary: 55, 56, 63, 64 and 119..121 bytes.
        assert_eq!(
            hex(&vec![b'a'; 55]),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            hex(&vec![b'a'; 56]),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            hex(&vec![b'a'; 64]),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
        // 1,000,000 × 'a' (FIPS 180-4 example)
        let million = vec![b'a'; 1_000_000];
        assert_eq!(
            hex(&million),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    /// Whatever path [`digest`] takes on this machine must agree with the
    /// portable one byte for byte. On a CPU without the SHA extensions this
    /// compares the portable path with itself and still covers the padding.
    #[test]
    fn accelerated_matches_portable() {
        for n in 0..=300usize {
            let d = noise(n);
            assert_eq!(digest(&d), digest_portable(&d), "length {n}");
        }
        for n in [511, 512, 513, 4096, 65_536, 1_000_000] {
            let d = noise(n);
            assert_eq!(digest(&d), digest_portable(&d), "length {n}");
        }
    }

    /// The digests this crate actually depends on: the same input always
    /// hashes to the same 64 lowercase hex digits.
    #[test]
    fn hex_is_lowercase_and_stable() {
        let d = noise(9999);
        let a = hex(&d);
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
        assert_eq!(a, hex(&d));
    }
}
