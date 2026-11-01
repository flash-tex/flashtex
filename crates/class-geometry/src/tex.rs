//! TeX's integer arithmetic on scaled points, re-implemented from the
//! algorithms described in *TeX: The Program* (§§100–108 `half`,
//! `round_decimals`, `print_scaled`, `nx_plus_y`, `x_over_n`, `xn_over_d`;
//! §§448–461 `scan_dimen`). Every length in this crate is an `Sp` so that
//! results agree with pdflatex to the scaled point instead of drifting
//! through floating point.

use std::fmt;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// 2^16: one TeX point in scaled points.
pub const UNITY: i64 = 65_536;
const TWO: i64 = 131_072;

/// A length in TeX scaled points (1pt = 65536sp, 72.27pt = 1in).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sp(pub i64);

impl Sp {
    pub const ZERO: Sp = Sp(0);

    /// `n\p@` — an integer number of points.
    pub const fn pt(n: i64) -> Sp {
        Sp(n * UNITY)
    }

    /// Parse a TeX dimension with a physical unit (`345pt`, `8.5in`,
    /// `-.4in`, `297mm`, `2.5cm`, `3bp`, `12dd`, `1cc`, `1pc`, `100sp`)
    /// exactly as `scan_dimen` would. Font-relative units are rejected; use
    /// [`Sp::scaled`] with the font's em/ex.
    pub fn parse(s: &str) -> Option<Sp> {
        let s = s.trim();
        let (neg, body) = match s.strip_prefix('-') {
            Some(rest) => (true, rest.trim_start()),
            None => (false, s.strip_prefix('+').unwrap_or(s)),
        };
        let unit_at = body.find(|c: char| c.is_ascii_alphabetic())?;
        let (num, unit) = body.split_at(unit_at);
        let unit = unit.trim().to_ascii_lowercase();
        let unit = unit.strip_prefix("true").unwrap_or(&unit).trim();
        let (int, frac) = parse_decimal(num.trim())?;
        let v = scan_unit(int, frac, unit)?;
        Some(if neg { -v } else { v })
    }

    /// `<factor><internal dimen>`, e.g. `.4\@tempdima` or `1.5em`: TeX
    /// §455 — `nx_plus_y(int, v, xn_over_d(v, frac, 2^16))`.
    pub fn scaled(self, factor: &str) -> Option<Sp> {
        let f = factor.trim();
        let (neg, f) = match f.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, f),
        };
        let (int, frac) = parse_decimal(f)?;
        let v = nx_plus_y(int, self, xn_over_d(self, frac, UNITY).0 .0);
        Some(if neg { -v } else { v })
    }

    /// `\multiply` by an integer.
    pub fn times(self, n: i64) -> Sp {
        Sp(self.0 * n)
    }

    /// `\divide` by an integer: TeX truncates toward zero (`x_over_n`).
    pub fn over(self, n: i64) -> Sp {
        x_over_n(self, n).0
    }

    /// `\@settopoint`: `\divide#1\p@\multiply#1\p@` (latex.ltx line 10261).
    pub fn settopoint(self) -> Sp {
        Sp((self.0 / UNITY) * UNITY)
    }

    /// Value in (TeX) points as a float, for display only.
    pub fn to_pt(self) -> f64 {
        self.0 as f64 / UNITY as f64
    }

    /// Value in PDF big points (72/in), for adapters only.
    pub fn to_bp(self) -> f64 {
        self.to_pt() * 72.0 / 72.27
    }
}

impl Add for Sp {
    type Output = Sp;
    fn add(self, o: Sp) -> Sp {
        Sp(self.0 + o.0)
    }
}
impl Sub for Sp {
    type Output = Sp;
    fn sub(self, o: Sp) -> Sp {
        Sp(self.0 - o.0)
    }
}
impl AddAssign for Sp {
    fn add_assign(&mut self, o: Sp) {
        self.0 += o.0;
    }
}
impl SubAssign for Sp {
    fn sub_assign(&mut self, o: Sp) {
        self.0 -= o.0;
    }
}
impl Neg for Sp {
    type Output = Sp;
    fn neg(self) -> Sp {
        Sp(-self.0)
    }
}

/// `\the<dimen>` output: TeX's `print_scaled` (§103) followed by `pt`.
impl fmt::Display for Sp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = self.0;
        let mut out = String::new();
        if s < 0 {
            out.push('-');
            s = -s;
        }
        out.push_str(&(s / UNITY).to_string());
        out.push('.');
        s = 10 * (s % UNITY) + 5;
        let mut delta = 10;
        loop {
            if delta > UNITY {
                s += 0x8000 - 50_000; // round the last digit
            }
            out.push(char::from(b'0' + (s / UNITY) as u8));
            s = 10 * (s % UNITY);
            delta *= 10;
            if s <= delta {
                break;
            }
        }
        write!(f, "{out}pt")
    }
}

/// Split `12.34` into (12, fraction scaled to 2^16 with `round_decimals`).
fn parse_decimal(s: &str) -> Option<(i64, i64)> {
    let s = s.replace(',', ".");
    let (i, d) = match s.find('.') {
        Some(p) => (&s[..p], &s[p + 1..]),
        None => (s.as_str(), ""),
    };
    if i.is_empty() && d.is_empty() {
        return None;
    }
    if !i.chars().all(|c| c.is_ascii_digit()) || !d.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let int: i64 = if i.is_empty() { 0 } else { i.parse().ok()? };
    let digits: Vec<i64> = d.bytes().take(17).map(|b| (b - b'0') as i64).collect();
    Some((int, round_decimals(&digits)))
}

/// §102 `round_decimals`: 0.d1d2..dk -> nearest multiple of 2^-16.
pub fn round_decimals(digits: &[i64]) -> i64 {
    let mut a = 0;
    for &d in digits.iter().rev() {
        a = (a + d * TWO) / 10;
    }
    (a + 1) / 2
}

/// §105 `nx_plus_y` (without overflow detection).
pub fn nx_plus_y(n: i64, x: Sp, y: i64) -> Sp {
    Sp(n * x.0 + y)
}

/// §106 `x_over_n`: quotient truncated toward zero, and the remainder.
pub fn x_over_n(x: Sp, n: i64) -> (Sp, i64) {
    if n == 0 {
        return (Sp(0), x.0);
    }
    (Sp(x.0 / n), x.0 % n)
}

/// §107 `xn_over_d`: x*n/d for nonnegative n,d, truncated, sign of x.
pub fn xn_over_d(x: Sp, n: i64, d: i64) -> (Sp, i64) {
    let pos = x.0 >= 0;
    let xa = x.0.abs();
    let t = xa * n;
    let q = t / d;
    let r = t % d;
    if pos {
        (Sp(q), r)
    } else {
        (Sp(-q), -r)
    }
}

/// §458: physical units. `int`/`frac` are the integer part and the 2^16
/// fraction of the scanned decimal.
fn scan_unit(int: i64, frac: i64, unit: &str) -> Option<Sp> {
    let (num, denom) = match unit {
        "pt" => return Some(Sp(int * UNITY + frac)),
        "sp" => return Some(Sp(int)),
        "in" => (7227, 100),
        "pc" => (12, 1),
        "cm" => (7227, 254),
        "mm" => (7227, 2540),
        "bp" => (7227, 7200),
        "dd" => (1238, 1157),
        "cc" => (14856, 1157),
        _ => return None,
    };
    let (q, rem) = xn_over_d(Sp(int), num, denom);
    let f = (num * frac + UNITY * rem) / denom;
    let int2 = q.0 + f / UNITY;
    let frac2 = f % UNITY;
    Some(Sp(int2 * UNITY + frac2))
}

/// `\strip@pt`-style decimal of a length, as geometry's `\Gm@setsize` uses:
/// the digits `print_scaled` would print, without the unit.
pub fn strip_pt(v: Sp) -> String {
    let s = v.to_string();
    s.trim_end_matches("pt").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_match_tex() {
        assert_eq!(Sp::parse("1in").unwrap(), Sp(4_736_286)); // 72.26999pt
        assert_eq!(Sp::parse("72.27pt").unwrap(), Sp(4_736_287));
        assert_eq!(Sp::parse("13.6pt").unwrap(), Sp(891_290));
        assert_eq!(Sp::parse("8.5in").unwrap().to_string(), "614.295pt");
        assert_eq!(Sp::parse("11in").unwrap().to_string(), "794.96999pt");
        assert_eq!(Sp::pt(345).to_string(), "345.0pt");
        assert_eq!(Sp::parse("-.4in").unwrap(), -Sp::parse(".4in").unwrap());
    }

    #[test]
    fn print_scaled_round_trips() {
        for v in [
            0,
            1,
            7,
            65535,
            65536,
            891_290,
            4_736_286,
            123_456_789,
            -3_000_001,
        ] {
            let s = Sp(v).to_string();
            assert_eq!(Sp::parse(&s).unwrap(), Sp(v), "{s}");
        }
    }
}
