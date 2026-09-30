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
pub fn div_round(n: i128, d: i128) -> i128 {
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

    /// The product, rounded to 10^-12.
    pub fn times(self, o: Fx) -> Fx {
        match self.0.checked_mul(o.0) {
            Some(p) => Fx(div_round(p, ONE)),
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

    /// bp to sp, rounded to nearest (halves away from zero): x · 65781.76.
    pub fn to_sp(self) -> i64 {
        let n = self.0.saturating_mul(6_578_176);
        let v = div_round(n, 100 * ONE);
        v.clamp(i64::MIN as i128, i64::MAX as i128) as i64
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
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
    pub fn apply(&self, x: Fx, y: Fx) -> (Fx, Fx) {
        let [a, b, c, d, e, f] = self.0;
        if a == Fx::ONE && b.is_zero() && c.is_zero() && d == Fx::ONE {
            return (x + e, y + f);
        }
        (x.times(a) + y.times(c) + e, x.times(b) + y.times(d) + f)
    }

    /// Whether the linear part is the identity.
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
