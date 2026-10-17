//! Exact decimal arithmetic for PDF content-stream numbers.
//!
//! Every number pdfTeX writes into a content stream is a decimal with at
//! most a handful of fraction digits (`pdf_print_bp`, `pdf_print_real`, the
//! `/Widths` in tenths). Positions are sums and products of such numbers, so
//! they are held here as fixed-point integers in units of 10^-12 bp, which
//! represent every value pdfTeX produces exactly; a product rounds only if
//! it needs more than 12 fraction digits (a rotation composed with a
//! rotation), far below the 0.5 sp to which positions are finally rounded.

use std::ops::{Add, Neg, Sub};

/// 10^12: one bp.
pub const ONE: i128 = 1_000_000_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fx(pub i128);

/// `n / d` rounded to nearest, halves away from zero (d > 0).
#[inline]
pub fn div_round(n: i128, d: i128) -> i128 {
    // (in 64 bits when both fit: a 128-bit division is a library call)
    if let (Ok(n), Ok(d)) = (i64::try_from(n), i64::try_from(d)) {
        if d > 0 && d < 1 << 62 {
            let (q, r) = (n / d, n % d);
            return (if 2 * r.abs() >= d { q + n.signum() } else { q }) as i128;
        }
    }
    let q = n / d;
    let r = n % d;
    if 2 * r.abs() >= d {
        q + n.signum()
    } else {
        q
    }
}

impl Fx {
    pub const ZERO: Fx = Fx(0);
    pub const ONE: Fx = Fx(ONE);

    pub fn from_int(i: i64) -> Fx {
        Fx(i as i128 * ONE)
    }

    /// A PDF number (`[+-]digits[.digits]`, also `.5`, `5.`); `None` if it
    /// is not one. More than 12 fraction digits are rounded.
    pub fn parse(b: &[u8]) -> Option<Fx> {
        let (neg, mut s) = match b.first()? {
            b'-' => (true, &b[1..]),
            b'+' => (false, &b[1..]),
            _ => (false, b),
        };
        // PDF readers accept a second sign ("--5"); pdfTeX never writes one.
        if let Some(b'-') = s.first() {
            s = &s[1..];
        }
        // (in 64 bits: the integer part stops at 10^15 and the fraction
        // holds 12 digits, so neither overflows)
        let mut int: u64 = 0;
        let mut frac: u64 = 0;
        let mut fdigits = 0u32;
        let mut seen_dot = false;
        let mut any = false;
        let mut round_up = false;
        for &c in s {
            match c {
                b'0'..=b'9' => {
                    any = true;
                    let d = (c - b'0') as u64;
                    if seen_dot {
                        if fdigits < 12 {
                            frac = frac * 10 + d;
                            fdigits += 1;
                        } else if fdigits == 12 {
                            round_up = d >= 5;
                            fdigits += 1;
                        }
                    } else {
                        int = int * 10 + d;
                        if int > 1_000_000_000_000_000 {
                            return None;
                        }
                    }
                }
                b'.' if !seen_dot => seen_dot = true,
                _ => return None,
            }
        }
        if !any {
            return None;
        }
        let fd = fdigits.min(12);
        let mut v = int as i128 * ONE + (frac * 10u64.pow(12 - fd)) as i128;
        if round_up {
            v += 1;
        }
        Some(Fx(if neg { -v } else { v }))
    }

    /// The product, rounded to 10^-12.
    pub fn times(self, o: Fx) -> Fx {
        match self.0.checked_mul(o.0) {
            Some(p) => match o.int64() {
                // An integer factor (a `TJ` adjustment, a scale): the
                // product is exact, `self × k` (the same value, without
                // dividing).
                Some(k) => Fx(self.0 * k as i128),
                None => Fx(div_round(p, ONE)),
            },
            // Beyond ±1.7e14 bp² (a corrupt stream): saturate.
            None => Fx(if (self.0 < 0) != (o.0 < 0) {
                i128::MIN / 2
            } else {
                i128::MAX / 2
            }),
        }
    }

    /// `self * n / d` with integers, rounded.
    pub fn mul_div(self, n: i64, d: i64) -> Fx {
        let d = d as i128;
        let (n, d) = if d < 0 {
            (-(n as i128), -d)
        } else {
            (n as i128, d)
        };
        match self.0.checked_mul(n) {
            Some(p) => Fx(div_round(p, d)),
            None => Fx(div_round(self.0, d).saturating_mul(n)),
        }
    }

    /// Division (for the inverse of a matrix only), rounded.
    pub fn over(self, o: Fx) -> Option<Fx> {
        if o.0 == 0 {
            return None;
        }
        let n = self.0.checked_mul(ONE)?;
        let (n, d) = if o.0 < 0 { (-n, -o.0) } else { (n, o.0) };
        Some(Fx(div_round(n, d)))
    }

    pub fn to_f64(self) -> f64 {
        // Below 2^53 the value and 10^12 are both exact doubles, so IEEE
        // division gives the double nearest to the decimal (the same value
        // as the parse below, without formatting: one per glyph origin; an
        // integer exactly). In 64 bits: no 128-bit division or conversion.
        if self.0.unsigned_abs() <= 1u128 << 53 {
            return self.0 as i64 as f64 / ONE as f64;
        }
        // Exact enough: the integer and fraction parts separately, so that a
        // decimal like 9.9626 becomes the double nearest to it.
        let int = self.0 / ONE;
        let frac = self.0 % ONE;
        if frac == 0 {
            return int as f64;
        }
        // Format and parse: the nearest double to the decimal, as a PDF
        // reader's strtod gives.
        let neg = self.0 < 0;
        let s = format!(
            "{}{}.{:012}",
            if neg { "-" } else { "" },
            int.abs(),
            frac.abs()
        );
        s.parse().unwrap_or(0.0)
    }

    /// The number as the reference PDF viewer (Core Graphics) reads it from
    /// a content stream: its digits as an integer `m`, times the double
    /// nearest to 10^-k for k fraction digits (`2.475` is `2475 × 0.001`),
    /// which is not always the double nearest to the decimal. Measured on
    /// the parity fixtures (DESIGN.md §6.2, J1): this, not the nearest
    /// double, puts every glyph on the same side of a pixel edge as Core
    /// Graphics' rendering of the PDF. Trailing zeros do not count as
    /// digits; beyond 12 fraction digits the value is already rounded.
    pub fn viewer_f64(self) -> f64 {
        const NEG_POW10: [f64; 13] = [
            1.0, 0.1, 0.01, 0.001, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10, 1e-11, 1e-12,
        ];
        let a = self.0.unsigned_abs();
        if a == 0 {
            return 0.0;
        }
        // (in 64 bits when it fits, as every number of a page's stream
        // does: a 128-bit remainder by 10 is a library call per digit)
        let (m, k) = match u64::try_from(a) {
            Ok(mut m) => {
                // min(12, trailing zeros) by 8, 4, 2, 1 (12 = 8 + 4)
                let mut k = 12;
                for (p, n) in [(100_000_000, 8), (10_000, 4), (100, 2), (10, 1)] {
                    if k >= n && m.is_multiple_of(p) {
                        m /= p;
                        k -= n;
                    }
                }
                (m as u128, k)
            }
            Err(_) => {
                let mut m = a;
                let mut k = 12;
                while k > 0 && m.is_multiple_of(10) {
                    m /= 10;
                    k -= 1;
                }
                (m, k)
            }
        };
        if m > 1u128 << 53 {
            return self.to_f64();
        }
        let v = m as u64 as f64 * NEG_POW10[k];
        if self.0 < 0 {
            -v
        } else {
            v
        }
    }

    /// bp to sp, rounded to nearest (halves away from zero): x · 65781.76.
    pub fn to_sp(self) -> i64 {
        // x · 6578176 / 10^14 = x · 803 / (2 · 5^14), the same rational in
        // lowest terms (6578176 = 2^13 · 803), so the same rounding. Below
        // 2^53 (9,007 bp) the product fits 64 bits.
        if self.0.unsigned_abs() <= 1u128 << 53 {
            const D: i64 = 12_207_031_250;
            let n = self.0 as i64 * 803;
            let (q, r) = (n / D, n % D);
            return if 2 * r.abs() >= D { q + n.signum() } else { q };
        }
        let n = self.0.saturating_mul(6_578_176);
        let v = div_round(n, 100 * ONE);
        v.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// The value as an integer, when it is one (and fits 64 bits).
    #[inline]
    fn int64(self) -> Option<i64> {
        let v = i64::try_from(self.0).ok()?;
        (v % ONE as i64 == 0).then_some(v / ONE as i64)
    }
}

impl Add for Fx {
    type Output = Fx;
    fn add(self, o: Fx) -> Fx {
        Fx(self.0.saturating_add(o.0))
    }
}

impl Sub for Fx {
    type Output = Fx;
    fn sub(self, o: Fx) -> Fx {
        Fx(self.0.saturating_sub(o.0))
    }
}

impl Neg for Fx {
    type Output = Fx;
    fn neg(self) -> Fx {
        Fx(-self.0)
    }
}

/// A PDF matrix [a b c d e f].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Mat(pub [Fx; 6]);

impl Mat {
    pub const IDENTITY: Mat = Mat([Fx::ONE, Fx::ZERO, Fx::ZERO, Fx::ONE, Fx::ZERO, Fx::ZERO]);

    pub fn translate(e: Fx, f: Fx) -> Mat {
        Mat([Fx::ONE, Fx::ZERO, Fx::ZERO, Fx::ONE, e, f])
    }

    /// `self × o` in the PDF's row-vector convention: apply `self`, then `o`.
    #[inline]
    pub fn then(&self, o: &Mat) -> Mat {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = o.0;
        // Fast path: `o` a pure translation, or `self` one.
        if a2 == Fx::ONE && b2.is_zero() && c2.is_zero() && d2 == Fx::ONE {
            return Mat([a, b, c, d, e + e2, f + f2]);
        }
        Mat([
            a.times(a2) + b.times(c2),
            a.times(b2) + b.times(d2),
            c.times(a2) + d.times(c2),
            c.times(b2) + d.times(d2),
            e.times(a2) + f.times(c2) + e2,
            e.times(b2) + f.times(d2) + f2,
        ])
    }

    /// The image of point (x, y).
    #[inline]
    pub fn apply(&self, x: Fx, y: Fx) -> (Fx, Fx) {
        let [a, b, c, d, e, f] = self.0;
        if a == Fx::ONE && b.is_zero() && c.is_zero() && d == Fx::ONE {
            return (x + e, y + f);
        }
        (x.times(a) + y.times(c) + e, x.times(b) + y.times(d) + f)
    }

    /// Whether the linear part is the identity.
    #[inline]
    pub fn is_translation(&self) -> bool {
        let [a, b, c, d, _, _] = self.0;
        a == Fx::ONE && b.is_zero() && c.is_zero() && d == Fx::ONE
    }

    pub fn to_f64(&self) -> [f64; 6] {
        self.0.map(Fx::to_f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64_conversions() {
        let p = |s: &str| Fx::parse(s.as_bytes()).unwrap();
        // to_f64: the nearest double (fast path and parse path agree).
        for s in [
            "9.9626",
            "-237.283",
            "0.000000000001",
            "123456.789012345678",
            "9100000.5",
        ] {
            assert_eq!(p(s).to_f64(), s.parse::<f64>().unwrap());
        }
        // viewer_f64: digits × 10^-k.
        assert_eq!(p("237.283").viewer_f64(), 237283.0 * 0.001);
        assert_eq!(p("-10.516").viewer_f64(), -(10516.0 * 0.001));
        assert_eq!(p("72").viewer_f64(), 72.0);
        assert_eq!(p("1.500").viewer_f64(), 15.0 * 0.1);
        assert_ne!(p("237.283").viewer_f64(), 237.283);
    }

    fn fx(s: &str) -> Fx {
        Fx::parse(s.as_bytes()).unwrap()
    }

    #[test]
    fn parse_and_round() {
        assert_eq!(fx("9.9626").0, 9_962_600_000_000);
        assert_eq!(fx("-.5").0, -500_000_000_000);
        assert_eq!(fx("5.").0, 5 * ONE);
        assert_eq!(fx("0.0000000000005").0, 1);
        assert!(Fx::parse(b"1.2.3").is_none());
        assert!(Fx::parse(b"-").is_none());
        assert_eq!(fx("1").to_sp(), 65782); // 65781.76
        assert_eq!(fx("-1").to_sp(), -65782);
        assert_eq!(fx("72.27").to_sp(), 4_754_048); // 4754047.7952
        assert_eq!(fx("9.9626").to_f64(), 9.9626);
        assert_eq!(div_round(5, 2), 3);
        assert_eq!(div_round(-5, 2), -3);
        assert_eq!(div_round(4, 3), 1);
    }

    // The 128-bit versions the 64-bit fast paths replace (as they were
    // before CLASSIC-HOTSPOTS): every result must be the same.
    fn div_round_ref(n: i128, d: i128) -> i128 {
        let q = n / d;
        let r = n % d;
        if 2 * r.abs() >= d {
            q + n.signum()
        } else {
            q
        }
    }
    fn times_ref(a: Fx, o: Fx) -> Fx {
        match a.0.checked_mul(o.0) {
            Some(p) => Fx(div_round_ref(p, ONE)),
            None => Fx(if (a.0 < 0) != (o.0 < 0) {
                i128::MIN / 2
            } else {
                i128::MAX / 2
            }),
        }
    }
    fn mul_div_ref(a: Fx, n: i64, d: i64) -> Fx {
        let d = d as i128;
        let (n, d) = if d < 0 {
            (-(n as i128), -d)
        } else {
            (n as i128, d)
        };
        match a.0.checked_mul(n) {
            Some(p) => Fx(div_round_ref(p, d)),
            None => Fx(div_round_ref(a.0, d).saturating_mul(n)),
        }
    }
    fn to_f64_ref(x: Fx) -> f64 {
        let int = x.0 / ONE;
        let frac = x.0 % ONE;
        if frac == 0 {
            return int as f64;
        }
        if x.0.unsigned_abs() <= 1u128 << 53 {
            return x.0 as f64 / ONE as f64;
        }
        let neg = x.0 < 0;
        let s = format!(
            "{}{}.{:012}",
            if neg { "-" } else { "" },
            int.abs(),
            frac.abs()
        );
        s.parse().unwrap_or(0.0)
    }
    fn viewer_f64_ref(x: Fx) -> f64 {
        const NEG_POW10: [f64; 13] = [
            1.0, 0.1, 0.01, 0.001, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10, 1e-11, 1e-12,
        ];
        let mut m = x.0.unsigned_abs();
        let mut k = 12;
        while k > 0 && m.is_multiple_of(10) {
            m /= 10;
            k -= 1;
        }
        if m > 1u128 << 53 {
            return to_f64_ref(x);
        }
        let v = m as f64 * NEG_POW10[k];
        if x.0 < 0 {
            -v
        } else {
            v
        }
    }
    fn to_sp_ref(x: Fx) -> i64 {
        let n = x.0.saturating_mul(6_578_176);
        let v = div_round_ref(n, 100 * ONE);
        v.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }
    fn parse_ref(b: &[u8]) -> Option<Fx> {
        let (neg, mut s) = match b.first()? {
            b'-' => (true, &b[1..]),
            b'+' => (false, &b[1..]),
            _ => (false, b),
        };
        if let Some(b'-') = s.first() {
            s = &s[1..];
        }
        let mut int: i128 = 0;
        let mut frac: i128 = 0;
        let mut fdigits = 0u32;
        let mut seen_dot = false;
        let mut any = false;
        let mut round_up = false;
        for &c in s {
            match c {
                b'0'..=b'9' => {
                    any = true;
                    let d = (c - b'0') as i128;
                    if seen_dot {
                        if fdigits < 12 {
                            frac = frac * 10 + d;
                            fdigits += 1;
                        } else if fdigits == 12 {
                            round_up = d >= 5;
                            fdigits += 1;
                        }
                    } else {
                        int = int.checked_mul(10)?.checked_add(d)?;
                        if int > 1_000_000_000_000_000 {
                            return None;
                        }
                    }
                }
                b'.' if !seen_dot => seen_dot = true,
                _ => return None,
            }
        }
        if !any {
            return None;
        }
        let fd = fdigits.min(12);
        let mut v = int * ONE + frac * 10i128.pow(12 - fd);
        if round_up {
            v += 1;
        }
        Some(Fx(if neg { -v } else { v }))
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        /// A value like the writer's numbers: `k` digits, `f` of them
        /// fraction digits, either sign; or raw bits at every scale.
        fn fx(&mut self) -> Fx {
            let r = self.next();
            let v: i128 = match r % 4 {
                0 => {
                    let digits = 1 + (self.next() % 18) as u32;
                    let f = (self.next() % 13) as u32;
                    let m = (self.next() % 10u64.pow(digits.min(18))) as i128;
                    m * 10i128.pow(12 - f.min(12))
                }
                1 => (self.next() as i64 >> (self.next() % 64)) as i128,
                2 => ((self.next() as i128) << 64 | self.next() as i128) >> (self.next() % 127),
                _ => (self.next() % 2_000_000) as i128 * 10i128.pow((self.next() % 13) as u32),
            };
            Fx(if self.next() & 1 == 0 { v } else { -v })
        }
    }

    fn edge_values() -> Vec<Fx> {
        let mut v = vec![0i128, 1, 5, 9, 10, 11, 99, 100, 101];
        for e in 0..39u32 {
            let p = 10i128.pow(e);
            v.extend([p - 1, p, p + 1, p / 2, p / 2 + 1, 5 * p]);
        }
        for e in [52u32, 53, 54, 62, 63, 64, 100, 125, 126] {
            let p = 1i128 << e;
            v.extend([p - 1, p, p + 1]);
        }
        // to_sp ties: x · 803 an odd multiple of 6103515625
        for k in [1i128, 3, 5, 1001, 123_457, 1_475_739_525] {
            v.push(k * 6_103_515_625 * 2 / 2);
            v.push(k * 15_201_157_587);
        }
        v.extend([i128::MAX, i128::MIN + 1, i128::MAX / 2, i128::MIN / 2]);
        let mut out: Vec<Fx> = v.iter().map(|&x| Fx(x)).collect();
        out.extend(v.iter().map(|&x| Fx(-x)));
        out
    }

    #[test]
    fn fast_paths_equal_the_128_bit_arithmetic() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let n: usize = std::env::var("FX_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300_000);
        let mut vals = edge_values();
        vals.extend((0..n).map(|_| rng.fx()));
        for (i, &x) in vals.iter().enumerate() {
            assert_eq!(x.to_sp(), to_sp_ref(x), "to_sp {x:?}");
            assert_eq!(
                x.to_f64().to_bits(),
                to_f64_ref(x).to_bits(),
                "to_f64 {x:?}"
            );
            assert_eq!(
                x.viewer_f64().to_bits(),
                viewer_f64_ref(x).to_bits(),
                "viewer_f64 {x:?}"
            );
            let y = vals[(i * 7919 + 13) % vals.len()];
            let z = Fx(y.0 / ONE * ONE); // an integer factor
            for o in [y, z, Fx::ONE, Fx::ZERO, -Fx::ONE, Fx::from_int(1000)] {
                assert_eq!(x.times(o), times_ref(x, o), "times {x:?} {o:?}");
            }
            for (nn, d) in [
                (1, 100),
                (-1, 1000),
                (803, 7),
                (1, 1),
                (-3, -10),
                (7_000, 10_000),
            ] {
                assert_eq!(x.mul_div(nn, d), mul_div_ref(x, nn, d), "mul_div {x:?}");
            }
            let d = (y.0.unsigned_abs() % (1u128 << 64)) as i128 + 1;
            assert_eq!(
                div_round(x.0 / 4, d),
                div_round_ref(x.0 / 4, d),
                "div_round {x:?} {d}"
            );
        }
    }

    #[test]
    fn parse_equals_the_128_bit_parse() {
        let mut rng = Rng(0xD1B5_4A32_D192_ED03);
        let mut cases: Vec<String> = vec![
            "0",
            "-0",
            "+0",
            ".5",
            "5.",
            "-.5",
            "--5",
            "1.2.3",
            "-",
            "",
            ".",
            "1e5",
            "0.0000000000005",
            "0.0000000000004999",
            "1000000000000000",
            "1000000000000001",
            "999999999999999.999999999999",
            "-1000000000000000.0000000000005",
            "00000000000000000000000001.5",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        for _ in 0..200_000 {
            let int = rng.next() % 10u64.pow((rng.next() % 17) as u32);
            let fd = (rng.next() % 16) as usize;
            let frac: String = (0..fd)
                .map(|_| (b'0' + (rng.next() % 10) as u8) as char)
                .collect();
            let sign = ["", "-", "+", "--"][(rng.next() % 4) as usize];
            cases.push(if fd == 0 {
                format!("{sign}{int}")
            } else {
                format!("{sign}{int}.{frac}")
            });
        }
        for c in &cases {
            assert_eq!(Fx::parse(c.as_bytes()), parse_ref(c.as_bytes()), "{c:?}");
        }
    }

    #[test]
    fn matrices() {
        let t = Mat::translate(fx("10"), fx("20"));
        let s = Mat([fx("2"), Fx::ZERO, Fx::ZERO, fx("2"), Fx::ZERO, Fx::ZERO]);
        let m = t.then(&s);
        assert_eq!(m.apply(fx("1"), fx("1")), (fx("22"), fx("42")));
        let r = s.then(&t);
        assert_eq!(r.apply(fx("1"), fx("1")), (fx("12"), fx("22")));
    }
}
