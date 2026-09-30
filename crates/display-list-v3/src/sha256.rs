//! SHA-256 (FIPS 180-4), for page content hashes and resource keys. Small and
//! dependency-free so that the engine can share this crate without pulling
//! crates.io code into its builds.

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

/// An incremental SHA-256.
#[derive(Clone)]
pub struct Sha256 {
    h: [u32; 8],
    buf: [u8; 64],
    n: usize,
    len: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Sha256 {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: [0; 64],
            n: 0,
            len: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.len = self.len.wrapping_add(data.len() as u64);
        if self.n > 0 {
            let take = (64 - self.n).min(data.len());
            self.buf[self.n..self.n + take].copy_from_slice(&data[..take]);
            self.n += take;
            data = &data[take..];
            if self.n < 64 {
                return;
            }
            let block = self.buf;
            self.blocks(&block);
            self.n = 0;
        }
        let whole = data.len() / 64 * 64;
        self.blocks(&data[..whole]);
        data = &data[whole..];
        self.buf[..data.len()].copy_from_slice(data);
        self.n = data.len();
    }

    pub fn finish(mut self) -> [u8; 32] {
        let bits = self.len.wrapping_mul(8);
        self.update(&[0x80]);
        while self.n != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        let mut out = [0u8; 32];
        for (i, w) in self.h.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&w.to_be_bytes());
        }
        out
    }

    /// Compress whole 64-byte blocks: with the ARMv8 SHA-256 instructions
    /// where the CPU has them (every Apple silicon Mac), else in software.
    fn blocks(&mut self, data: &[u8]) {
        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("sha2") {
                // SAFETY: the CPU has the SHA-256 instructions.
                unsafe { compress_arm(&mut self.h, data) };
                return;
            }
        }
        for b in data.as_chunks::<64>().0 {
            self.block(b);
        }
    }

    fn block(&mut self, b: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b_, mut c, mut d, mut e, mut f, mut g, mut h] = self.h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b_) ^ (a & c) ^ (b_ & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b_;
            b_ = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in self.h.iter_mut().zip([a, b_, c, d, e, f, g, h]) {
            *x = x.wrapping_add(y);
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "sha2")]
unsafe fn compress_arm(h: &mut [u32; 8], data: &[u8]) {
    use std::arch::aarch64::*;
    let mut abcd = vld1q_u32(h.as_ptr());
    let mut efgh = vld1q_u32(h.as_ptr().add(4));
    for block in data.as_chunks::<64>().0 {
        let (abcd0, efgh0) = (abcd, efgh);
        let p = block.as_ptr();
        let mut m = [
            vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(p))),
            vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(p.add(16)))),
            vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(p.add(32)))),
            vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(p.add(48)))),
        ];
        for i in 0..16 {
            let t = vaddq_u32(m[i % 4], vld1q_u32(K.as_ptr().add(4 * i)));
            let prev = abcd;
            abcd = vsha256hq_u32(prev, efgh, t);
            efgh = vsha256h2q_u32(efgh, prev, t);
            if i < 12 {
                m[i % 4] = vsha256su1q_u32(
                    vsha256su0q_u32(m[i % 4], m[(i + 1) % 4]),
                    m[(i + 2) % 4],
                    m[(i + 3) % 4],
                );
            }
        }
        abcd = vaddq_u32(abcd, abcd0);
        efgh = vaddq_u32(efgh, efgh0);
    }
    vst1q_u32(h.as_mut_ptr(), abcd);
    vst1q_u32(h.as_mut_ptr().add(4), efgh);
}

/// SHA-256 of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut s = Sha256::new();
    s.update(data);
    s.finish()
}

/// Lower-case hex.
pub fn hex(bytes: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(D[(b >> 4) as usize] as char);
        s.push(D[(b & 15) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_answers() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let long = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(
            hex(&sha256(long)),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        let mut s = Sha256::new();
        for chunk in long.chunks(7) {
            s.update(chunk);
        }
        assert_eq!(s.finish(), sha256(long));
        // The hardware and software paths agree on every length.
        let data: Vec<u8> = (0..5000u32)
            .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
            .collect();
        for n in [0, 1, 55, 56, 63, 64, 65, 127, 128, 1000, 5000] {
            let mut soft = Sha256::new();
            for b in data[..n / 64 * 64].as_chunks::<64>().0 {
                soft.block(b);
            }
            soft.len = (n / 64 * 64) as u64;
            soft.update(&data[n / 64 * 64..n]);
            assert_eq!(soft.finish(), sha256(&data[..n]), "length {n}");
        }
        let million = vec![b'a'; 1_000_000];
        assert_eq!(
            hex(&sha256(&million)),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }
}
