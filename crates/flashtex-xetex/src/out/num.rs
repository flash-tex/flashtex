//! Numbers as the PDF writer writes them: at most `digits` fraction
//! digits, trailing zeros and a trailing point dropped, `-0` written `0`.
//! The text is `std`'s `format!("{v:.digits$}")` so trimmed, byte for byte;
//! [`push_num`] makes it without `format!` (FMT-NUM: the preview PDF is
//! written at every compile, and `format!` was most of its cost).

/// `v` with at most `digits` fraction digits, trailing zeros and a
/// trailing point dropped; `-0` is `0`. The text is `std`'s `{v:.digits$}`
/// so trimmed ([`push_num`]).
pub fn fmt_num(v: f64, digits: usize) -> String {
    let mut out = Vec::with_capacity(24);
    push_num(&mut out, v, digits);
    // push_num writes ASCII only
    String::from_utf8(out).unwrap_or_default()
}

/// The powers of ten the fast path scales by: `10^digits` for
/// `digits <= MAX_FAST_DIGITS`.
const POW10: [u64; 19] = {
    let mut t = [1u64; 19];
    let mut i = 1;
    while i < 19 {
        t[i] = t[i - 1] * 10;
        i += 1;
    }
    t
};
const MAX_FAST_DIGITS: usize = 18;

/// `|v| * 10^digits` rounded to an integer as `std`'s `{v:.digits$}`
/// rounds: from the exact binary value, ties to even. With the sign of
/// `v`. None: `v` is not finite, `digits` is above 18, or the result
/// does not fit 128 bits (`std` then writes it).
///
/// `v = m * 2^e` exactly (`m < 2^53`), so `|v| * 10^digits = p * 2^e`
/// with `p = m * 10^digits < 2^113`: an integer shifted, whose dropped
/// bits decide the rounding with no error.
#[inline]
fn scaled(v: f64, digits: usize) -> Option<(bool, u128)> {
    if digits > MAX_FAST_DIGITS {
        return None;
    }
    let bits = v.to_bits();
    let neg = bits >> 63 != 0;
    let be = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    let (m, e) = match be {
        0x7ff => return None,
        0 => (frac, -1074),
        _ => (frac | 1u64 << 52, be - 1075),
    };
    let p = m as u128 * POW10[digits] as u128;
    if e >= 0 {
        // an integer: exact when it fits
        let e = e as u32;
        if p != 0 && p.leading_zeros() <= e {
            return None;
        }
        return Some((neg, p << e));
    }
    let s = (-e) as u32;
    if s >= 114 {
        // p < 2^113 < 2^(s - 1), half a unit: rounds to 0, never a tie
        return Some((neg, 0));
    }
    let q = p >> s;
    let r = p & ((1u128 << s) - 1);
    let half = 1u128 << (s - 1);
    let up = r > half || (r == half && q & 1 == 1);
    Some((neg, q + up as u128))
}

/// Appends `v` with at most `digits` fraction digits, trailing zeros and
/// a trailing point dropped, and `-0` written `0`: byte for byte what
/// `format!("{v:.digits$}")` so trimmed gives (the exhaustive and
/// randomised tests below compare the two), without `format!`.
pub fn push_num(out: &mut Vec<u8>, v: f64, digits: usize) {
    // the writer's digit counts, with constant divisors
    match digits {
        3 => push_num_at(out, v, 3),
        5 => push_num_at(out, v, 5),
        6 => push_num_at(out, v, 6),
        8 => push_num_at(out, v, 8),
        _ => push_num_at(out, v, digits),
    }
}

#[inline(always)]
fn push_num_at(out: &mut Vec<u8>, v: f64, digits: usize) {
    let (neg, q) = match scaled(v, digits) {
        Some((neg, q)) if q <= u64::MAX as u128 => (neg, q as u64),
        // not finite, or 2^64 units and more: `std`'s text
        _ => {
            out.extend_from_slice(fmt_num_std(v, digits).as_bytes());
            return;
        }
    };
    if q == 0 {
        // `0.000` and `-0.000` both trim to `0`
        out.push(b'0');
        return;
    }
    let p = POW10[digits];
    let (int, fr) = (q / p, q % p);
    // the point at DOT, the fraction after it (`digits` digits, then its
    // trailing zeros dropped), the integer and the sign before it
    const DOT: usize = 24;
    let mut buf = [0u8; DOT + 1 + MAX_FAST_DIGITS + 1];
    let mut end = DOT;
    if fr != 0 {
        buf[DOT] = b'.';
        end = DOT + 1 + digits;
        put_fixed(&mut buf[DOT + 1..end], fr);
        // fr != 0: a digit before the point stays
        while buf[end - 1] == b'0' {
            end -= 1;
        }
    }
    let mut i = DOT - put_uint(&mut buf[..DOT], int);
    if neg {
        i -= 1;
        buf[i] = b'-';
    }
    out.extend_from_slice(&buf[i..end]);
}

/// "00" to "99".
const PAIRS: &[u8; 200] = b"0001020304050607080910111213141516171819\
2021222324252627282930313233343536373839\
4041424344454647484950515253545556575859\
6061626364656667686970717273747576777879\
8081828384858687888990919293949596979899";

/// `v` in all of `dst`, zero-padded on the left (`v < 10^dst.len()`).
#[inline(always)]
fn put_fixed(dst: &mut [u8], mut v: u64) {
    let mut j = dst.len();
    while j >= 2 {
        let r = (v % 100) as usize * 2;
        v /= 100;
        dst[j - 2..j].copy_from_slice(&PAIRS[r..r + 2]);
        j -= 2;
    }
    if j == 1 {
        dst[0] = b'0' + (v % 10) as u8;
    }
}

/// `v` in decimal, right-aligned at the end of `dst` (at least 20
/// bytes); the number of digits.
#[inline(always)]
fn put_uint(dst: &mut [u8], mut v: u64) -> usize {
    let mut j = dst.len();
    while v >= 100 {
        let r = (v % 100) as usize * 2;
        v /= 100;
        dst[j - 2..j].copy_from_slice(&PAIRS[r..r + 2]);
        j -= 2;
    }
    if v >= 10 {
        let r = v as usize * 2;
        dst[j - 2..j].copy_from_slice(&PAIRS[r..r + 2]);
        j -= 2;
    } else {
        j -= 1;
        dst[j] = b'0' + v as u8;
    }
    dst.len() - j
}

/// An unsigned integer in decimal.
#[inline]
pub fn push_uint(out: &mut Vec<u8>, v: u64) {
    let mut buf = [0u8; 20];
    let n = put_uint(&mut buf, v);
    out.extend_from_slice(&buf[buf.len() - n..]);
}

/// `fmt_num(v, digits).parse::<f64>().unwrap_or(v)`: `v` rounded to
/// `digits` decimals as written, read back, without the text. The text
/// is `±q / 10^digits` exactly and `parse` rounds it correctly; for
/// `q < 2^53` (and `digits <= 18`) both operands are exact doubles and
/// IEEE division rounds the quotient correctly too.
pub fn round_num(v: f64, digits: usize) -> f64 {
    if let Some((neg, q)) = scaled(v, digits) {
        if q == 0 {
            return 0.0;
        }
        if q < 1u128 << 53 {
            let r = q as f64 / POW10[digits] as f64;
            return if neg { -r } else { r };
        }
    }
    fmt_num(v, digits).parse::<f64>().unwrap_or(v)
}

/// The reference: `std`'s fixed-point text, trimmed. [`push_num`] gives
/// the same bytes; it falls back to this outside its fast path.
pub(crate) fn fmt_num_std(v: f64, digits: usize) -> String {
    let mut s = format!("{v:.digits$}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// splitmix64
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
    }

    /// `push_num` and `round_num` against the reference for `v` at
    /// `digits`; the failure, if any.
    fn check(v: f64, digits: usize, out: &mut Vec<u8>) -> Result<(), String> {
        out.clear();
        push_num(out, v, digits);
        let want = fmt_num_std(v, digits);
        if out.as_slice() != want.as_bytes() {
            return Err(format!(
                "push_num({v:e} = {:#x}, {digits}) = {:?}, std {want:?}",
                v.to_bits(),
                String::from_utf8_lossy(out)
            ));
        }
        let r = round_num(v, digits);
        let w = want.parse::<f64>().unwrap_or(v);
        if r.to_bits() != w.to_bits() && !(r.is_nan() && w.is_nan()) {
            return Err(format!("round_num({v:e}, {digits}) = {r:e}, parse {w:e}"));
        }
        Ok(())
    }

    /// The values a case draws from: all bit patterns, the writer's
    /// (sp / 65781.76 and page heights less them, scales), exact ties at
    /// `digits` and their neighbours, and values next to powers of ten
    /// and to the carries `0.9999995`.
    fn sample(r: &mut Rng, digits: usize) -> f64 {
        let x = r.next();
        let k = 65781.76;
        let v = match x % 10 {
            0 => f64::from_bits(r.next()),
            1 => (r.next() as i32) as f64 / k,
            2 => (r.next() % 20_000) as f64 / 10.0 - (r.next() as i32) as f64 / k,
            3 | 4 => {
                // an exact tie: odd j / 2^(digits + 1) (with 5^digits | j
                // implied: j / 2^(d+1) = (2q + 1) / (2 * 10^d) needs it)
                let p5 = 5u64.pow(digits.min(22) as u32);
                let lim = (1u64 << 53) / p5.max(1);
                let j = (r.next() % lim.max(1)) | 1;
                (j * p5) as f64 / 2f64.powi(digits as i32 + 1)
            }
            5 => {
                // a random mantissa at a moderate exponent
                let m = (r.next() >> 11) as f64 / (1u64 << 53) as f64;
                m * 2f64.powi((r.next() % 80) as i32 - 40)
            }
            6 => {
                // next to 10^e and to the carry (10^e - 0.5) / 10^d
                let e = (r.next() % 24) as i32 - 8;
                if r.next() & 1 == 0 {
                    10f64.powi(e)
                } else {
                    (10f64.powi(e) - 0.5) / 10f64.powi(digits as i32)
                }
            }
            7 => (r.next() % 2_000_001) as f64 / 1e6 - 1.0,
            8 => (r.next() as i64) as f64 / 2f64.powi((r.next() % 120) as i32),
            _ => (r.next() as i32) as f64 / 1000.0,
        };
        // a few ulps either side, or the value; either sign
        let v = match r.next() % 4 {
            0 => f64::from_bits(v.to_bits().wrapping_add(r.next() % 3)),
            1 => f64::from_bits(v.to_bits().wrapping_sub(r.next() % 3)),
            _ => v,
        };
        if r.next() & 1 == 0 {
            -v
        } else {
            v
        }
    }

    fn edge_cases() -> Vec<f64> {
        let mut v = vec![
            0.0,
            -0.0,
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::from_bits(0x000f_ffff_ffff_ffff),
            f64::EPSILON,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            0.5,
            1.5,
            2.5,
            0.25,
            0.125,
            0.0005,
            0.00005,
            0.000005,
            0.0000005,
            0.9999995,
            0.999995,
            0.99995,
            9.9999995,
            99999.999995,
            1e15,
            1e16,
            1e17,
            1e22,
            1e23,
            2f64.powi(53),
            2f64.powi(64),
            2f64.powi(74),
            2f64.powi(75),
            2f64.powi(100),
            2f64.powi(127),
            2f64.powi(-60),
            2f64.powi(-61),
            2f64.powi(-113),
            2f64.powi(-114),
            2f64.powi(-1000),
            i32::MAX as f64 / 65781.76,
            i32::MIN as f64 / 65781.76,
            u64::MAX as f64,
            -(u64::MAX as f64),
        ];
        for e in -30..40 {
            let p = 10f64.powi(e);
            v.extend([
                p,
                f64::from_bits(p.to_bits() + 1),
                f64::from_bits(p.to_bits() - 1),
            ]);
        }
        let mut all = vec![];
        for x in v {
            for y in [x, -x] {
                all.extend([
                    y,
                    f64::from_bits(y.to_bits().wrapping_add(1)),
                    f64::from_bits(y.to_bits().wrapping_sub(1)),
                ]);
            }
        }
        all
    }

    #[test]
    fn push_num_edges() {
        let mut out = Vec::new();
        for v in edge_cases() {
            for d in 0..=22 {
                check(v, d, &mut out).unwrap();
            }
        }
        // every exact tie at small numerators, all digit counts
        for d in 0..=12usize {
            let den = 2f64.powi(d as i32 + 1);
            let p5 = 5u64.pow(d as u32);
            for j in (1..20_000u64).step_by(2) {
                let t = (j * p5) as f64 / den;
                for v in [t, -t] {
                    check(v, d, &mut out).unwrap();
                }
            }
        }
    }

    /// `n` random values per digit count 0..=20 over `threads` threads
    /// (FMT_NUM_CASES, default 300 000 in all: the full run is
    /// `FMT_NUM_CASES=1000000000 cargo test --release fmt_num_random`).
    #[test]
    fn fmt_num_random() {
        let total: u64 = std::env::var("FMT_NUM_CASES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300_000);
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .min(8) as u64;
        let per = total.div_ceil(threads);
        let hs: Vec<_> = (0..threads)
            .map(|t| {
                std::thread::spawn(move || {
                    let mut r = Rng(0x5eed_0000 + t);
                    let mut out = Vec::new();
                    for i in 0..per {
                        // the digits the writer uses, more often
                        let d = match i % 8 {
                            0..=1 => 5,
                            2..=3 => 6,
                            4 => 3,
                            5 => 8,
                            _ => (r.next() % 21) as usize,
                        };
                        let v = sample(&mut r, d);
                        check(v, d, &mut out)?;
                    }
                    Ok::<u64, String>(per)
                })
            })
            .collect();
        let mut n = 0;
        for h in hs {
            n += h.join().unwrap().unwrap();
        }
        assert!(n >= total);
    }

    /// Every scaled point an i32 holds, as the writer's coordinates
    /// (`sp / 65781.76`, five decimals) and sizes (six). About 4.3e9
    /// values per digit count: `cargo test --release -- --ignored
    /// fmt_num_all_sp`.
    #[test]
    #[ignore]
    fn fmt_num_all_sp() {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1) as i64;
        let lo = i32::MIN as i64;
        let span = (1i64 << 32) / threads + 1;
        let hs: Vec<_> = (0..threads)
            .map(|t| {
                std::thread::spawn(move || {
                    let mut out = Vec::new();
                    let a = lo + t * span;
                    let b = (a + span).min(i32::MAX as i64 + 1);
                    for sp in a..b {
                        let v = sp as f64 / 65781.76;
                        check(v, 5, &mut out)?;
                        check(v, 6, &mut out)?;
                    }
                    Ok::<i64, String>(b - a)
                })
            })
            .collect();
        let mut n = 0;
        for h in hs {
            n += h.join().unwrap().unwrap();
        }
        assert_eq!(n, 1i64 << 32);
    }
}
