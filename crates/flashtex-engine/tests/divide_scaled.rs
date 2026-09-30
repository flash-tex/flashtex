//! `divide_scaled` (pdftex.web section 689) as `changes/throughput.ch`
//! computes it, one 64-bit division, against pdftex.web's own digit loop,
//! transcribed below with the wrapping 32-bit arithmetic the translation
//! gives it. Result and `scaled_out` must be equal for every argument that
//! does not stop at `pdf_error` (m = 0, |m| >= max_integer div 10): the edge
//! values, every small case, and random ones over the whole range.

use flashtex_engine::Globals;

/// pdftex.web's `divide_scaled`, as `tools/web2rust` translated it before
/// `changes/throughput.ch` (i32, wrapping `*`, `+`, `-`; truncating `/`, `%`).
fn reference(ten_pow: &[i32; 10], mut s: i32, mut m: i32, dd: i32) -> (i32, i32) {
    let mut sign = 1i32;
    if s < 0 {
        sign = sign.wrapping_neg();
        s = s.wrapping_neg();
    }
    if m < 0 {
        sign = sign.wrapping_neg();
        m = m.wrapping_neg();
    }
    let mut q = s / m;
    let mut r = s % m;
    let mut i = 1;
    while i <= dd {
        q = 10i32.wrapping_mul(q).wrapping_add(10i32.wrapping_mul(r) / m);
        r = 10i32.wrapping_mul(r) % m;
        i += 1;
    }
    if 2i32.wrapping_mul(r) >= m {
        q = q.wrapping_add(1);
        r = r.wrapping_sub(m);
    }
    let scaled_out = sign.wrapping_mul(s.wrapping_sub(r / ten_pow[dd as usize]));
    (sign.wrapping_mul(q), scaled_out)
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}

#[test]
fn divide_scaled_matches_the_digit_loop() {
    let mut g = Globals::new();
    let mut tp = [0i32; 10];
    tp[0] = 1;
    for i in 1..10 {
        tp[i] = 10 * tp[i - 1];
    }
    for (i, &v) in tp.iter().enumerate() {
        g.ten_pow[i] = v;
    }
    // Largest |m| that does not stop at pdf_error: m < max_integer div 10.
    let m_max = i32::MAX / 10 - 1;
    let mut n = 0u64;
    let mut check = |g: &mut Globals, s: i32, m: i32, dd: i32| {
        let want = reference(&tp, s, m, dd);
        let got = g.divide_scaled(s, m, dd);
        assert_eq!((got, g.scaled_out), want, "divide_scaled({s}, {m}, {dd})");
        n += 1;
    };
    let edges_s = [
        i32::MIN,
        i32::MIN + 1,
        -65536 * 16384,
        -1,
        0,
        1,
        65535,
        65536,
        i32::MAX / 10,
        i32::MAX - 1,
        i32::MAX,
    ];
    let edges_m = [1, 2, 3, 7, 10, 65536, 65782, 6578176, m_max - 1, m_max];
    for dd in 0..10 {
        for &s in &edges_s {
            for &m in &edges_m {
                check(&mut g, s, m, dd);
                check(&mut g, s, -m, dd);
            }
        }
        for s in -300..=300 {
            for m in 1..=40 {
                check(&mut g, s, m, dd);
                check(&mut g, s, -m, dd);
            }
        }
    }
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..3_000_000 {
        let r = rng.next();
        let s = r as u32 as i32;
        // m over the whole allowed range, and small m, where quotients
        // are large and the 32-bit loop wraps.
        let mm = if r >> 63 == 0 {
            (((r >> 32) as u32) % (m_max as u32) + 1) as i32
        } else {
            (((r >> 32) as u32) % 5000 + 1) as i32
        };
        let m = if (r >> 62) & 1 == 0 { mm } else { -mm };
        let dd = (rng.next() % 10) as i32;
        check(&mut g, s, m, dd);
    }
    assert!(n > 3_000_000);
}
