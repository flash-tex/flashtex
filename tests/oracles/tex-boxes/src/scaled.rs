//! Scaled-point arithmetic exactly as in tex.web part 7 (§§99–109) and the
//! dimension-unit conversions of §§453–458.
//!
//! All TeX dimensions are integers in scaled points (sp, 2^-16 pt). Every
//! routine here reproduces the integer-division behaviour of tex.web so that
//! results match pdfTeX to the sp.

/// A TeX dimension in scaled points.
pub type Scaled = i32;

/// `unity` (§101): one point.
pub const UNITY: Scaled = 65536;
/// `max_dimen` (§421): 2^30 - 1 sp, i.e. 16383.99999pt.
pub const MAX_DIMEN: Scaled = 0o7777777777;
/// `null_flag` (§138): marks a running rule dimension.
pub const NULL_FLAG: Scaled = -0o10000000000;
/// `default_rule` (§463): 0.4pt.
pub const DEFAULT_RULE: Scaled = 26214;
/// `ignore_depth` (§212): -1000pt, the `\prevdepth` sentinel.
pub const IGNORE_DEPTH: Scaled = -65536000;

/// `half` (§100).
pub fn half(x: i32) -> i32 {
    if x & 1 != 0 { (x + 1).div_euclid(2) } else { x.div_euclid(2) }
}

/// Pascal `div` truncates toward zero, as does Rust `/`.
#[inline]
fn pdiv(a: i64, b: i64) -> i64 {
    a / b
}

/// `round_decimals` (§102): converts the decimal fraction `.d0 d1 ... d(k-1)`
/// (at most 17 digits are significant) into a correctly rounded scaled value.
pub fn round_decimals(digits: &[u8]) -> Scaled {
    let k = digits.len().min(17);
    let mut a: i64 = 0;
    for &d in digits[..k].iter().rev() {
        a = (a + i64::from(d) * 2 * 65536) / 10;
    }
    ((a + 1) / 2) as Scaled
}

/// `print_scaled` (§103): the shortest decimal that reads back to `s`.
pub fn print_scaled(s: Scaled) -> String {
    let mut out = String::new();
    let mut s = i64::from(s);
    if s < 0 {
        out.push('-');
        s = -s;
    }
    out.push_str(&(s / 65536).to_string());
    out.push('.');
    s = 10 * (s % 65536) + 5;
    let mut delta: i64 = 10;
    loop {
        if delta > 65536 {
            s = s + 0o100000 - 50000; // round the last digit
        }
        out.push(char::from(b'0' + (s / 65536) as u8));
        s = 10 * (s % 65536);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
    out
}

/// Error flag raised by the arithmetic routines (`arith_error`, §104).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArithError;

/// `mult_and_add` (§105) with `max_answer = 2^30-1` (`nx_plus_y`).
pub fn nx_plus_y(n: i32, x: Scaled, y: Scaled) -> Result<Scaled, ArithError> {
    mult_and_add(n, x, y, 0o7777777777)
}

/// `mult_and_add` (§105).
pub fn mult_and_add(n: i32, x: Scaled, y: Scaled, max_answer: Scaled) -> Result<Scaled, ArithError> {
    let (mut n, mut x) = (i64::from(n), i64::from(x));
    let (y, max_answer) = (i64::from(y), i64::from(max_answer));
    if n < 0 {
        x = -x;
        n = -n;
    }
    if n == 0 {
        Ok(y as Scaled)
    } else if x <= pdiv(max_answer - y, n) && -x <= pdiv(max_answer + y, n) {
        Ok((n * x + y) as Scaled)
    } else {
        Err(ArithError)
    }
}

/// `x_over_n` (§106): returns `(quotient, remainder)`.
pub fn x_over_n(x: Scaled, n: i32) -> Result<(Scaled, Scaled), ArithError> {
    if n == 0 {
        return Err(ArithError);
    }
    let (mut x, mut n, mut negative) = (i64::from(x), i64::from(n), false);
    if n < 0 {
        x = -x;
        n = -n;
        negative = true;
    }
    let (q, mut r) = if x >= 0 { (x / n, x % n) } else { (-((-x) / n), -((-x) % n)) };
    if negative {
        r = -r;
    }
    Ok((q as Scaled, r as Scaled))
}

/// `xn_over_d` (§107): `x*n/d` in 1.5-precision arithmetic; returns
/// `(result, remainder)`.
pub fn xn_over_d(x: Scaled, n: i32, d: i32) -> Result<(Scaled, Scaled), ArithError> {
    let positive = x >= 0;
    let x = i64::from(x).abs();
    let (n, d) = (i64::from(n), i64::from(d));
    let t = (x % 0o100000) * n;
    let mut u = (x / 0o100000) * n + (t / 0o100000);
    let v = (u % d) * 0o100000 + (t % 0o100000);
    if u / d >= 0o100000 {
        return Err(ArithError);
    }
    u = 0o100000 * (u / d) + (v / d);
    if positive { Ok((u as Scaled, (v % d) as Scaled)) } else { Ok((-u as Scaled, -(v % d) as Scaled)) }
}

/// Physical units accepted by `scan_dimen` (§§453, 458).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Pt,
    In,
    Pc,
    Cm,
    Mm,
    Bp,
    Dd,
    Cc,
    Sp,
}

impl Unit {
    pub fn parse(s: &str) -> Option<Unit> {
        Some(match s {
            "pt" => Unit::Pt,
            "in" => Unit::In,
            "pc" => Unit::Pc,
            "cm" => Unit::Cm,
            "mm" => Unit::Mm,
            "bp" => Unit::Bp,
            "dd" => Unit::Dd,
            "cc" => Unit::Cc,
            "sp" => Unit::Sp,
            _ => return None,
        })
    }
}

/// Converts `integer . fraction_digits <unit>` to sp exactly as
/// `scan_dimen` does (§§448, 452–453, 458) with `\mag=1000` and no `true`.
pub fn dimen_from_parts(negative: bool, integer: i32, frac_digits: &[u8], unit: Unit) -> Result<Scaled, ArithError> {
    let mut cur_val = i64::from(integer);
    let mut f = i64::from(round_decimals(frac_digits));
    let (num, denom) = match unit {
        Unit::Pt => (1, 1),
        Unit::In => (7227, 100),
        Unit::Pc => (12, 1),
        Unit::Cm => (7227, 254),
        Unit::Mm => (7227, 2540),
        Unit::Bp => (7227, 7200),
        Unit::Dd => (1238, 1157),
        Unit::Cc => (14856, 1157),
        Unit::Sp => {
            return finish_sign(negative, cur_val);
        }
    };
    if unit != Unit::Pt {
        let (q, rem) = xn_over_d(cur_val as Scaled, num, denom)?;
        cur_val = i64::from(q);
        f = (i64::from(num) * f + 0o200000 * i64::from(rem)) / i64::from(denom);
        cur_val += f / 0o200000;
        f %= 0o200000;
    }
    // attach_fraction
    if cur_val >= 0o40000 {
        return Err(ArithError);
    }
    cur_val = cur_val * 65536 + f;
    finish_sign(negative, cur_val)
}

fn finish_sign(negative: bool, v: i64) -> Result<Scaled, ArithError> {
    if v.abs() >= 0o10000000000 {
        return Err(ArithError);
    }
    Ok(if negative { -v as Scaled } else { v as Scaled })
}

/// `<factor><internal dimen>` (§455 `found:`): `integer.frac * v`.
pub fn scale_internal(negative: bool, integer: i32, frac_digits: &[u8], v: Scaled) -> Result<Scaled, ArithError> {
    let f = round_decimals(frac_digits);
    let (part, _) = xn_over_d(v, f, 0o200000)?;
    let r = nx_plus_y(integer, v, part)?;
    finish_sign(negative, i64::from(r))
}

/// Parses a TeX dimension literal such as `-1.5pt`, `3cm`, `.25in`, `12sp`
/// (optional spaces between number and unit; `,` accepted as decimal point).
pub fn parse_dimen(s: &str) -> Option<Scaled> {
    let s = s.trim();
    let (negative, rest) = strip_signs(s);
    let (int, frac, rest) = split_number(rest)?;
    let unit = Unit::parse(rest.trim())?;
    dimen_from_parts(negative, int, &frac, unit).ok()
}

/// Strips TeX's optional sign sequence (§441), returning the net sign.
pub fn strip_signs(s: &str) -> (bool, &str) {
    let mut negative = false;
    let mut rest = s.trim_start();
    loop {
        if let Some(r) = rest.strip_prefix('-') {
            negative = !negative;
            rest = r.trim_start();
        } else if let Some(r) = rest.strip_prefix('+') {
            rest = r.trim_start();
        } else {
            return (negative, rest);
        }
    }
}

/// Splits a decimal constant into integer part and fraction digits.
pub fn split_number(s: &str) -> Option<(i32, Vec<u8>, &str)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut int: i64 = 0;
    let mut any = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        int = int * 10 + i64::from(bytes[i] - b'0');
        if int > i64::from(i32::MAX) {
            return None;
        }
        i += 1;
        any = true;
    }
    let mut frac = Vec::new();
    if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b',') {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            frac.push(bytes[i] - b'0');
            i += 1;
            any = true;
        }
    }
    if !any {
        return None;
    }
    Some((int as i32, frac, &s[i..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_scaled_matches_tex() {
        assert_eq!(print_scaled(0), "0.0");
        assert_eq!(print_scaled(65536), "1.0");
        assert_eq!(print_scaled(26214), "0.4");
        assert_eq!(print_scaled(-32768), "-0.5");
        assert_eq!(print_scaled(1), "0.00002");
        assert_eq!(print_scaled(MAX_DIMEN), "16383.99998");
    }

    #[test]
    fn units_match_tex() {
        assert_eq!(parse_dimen("1pt"), Some(65536));
        assert_eq!(parse_dimen("0.4pt"), Some(26214));
        assert_eq!(parse_dimen("1in"), Some(4736286));
        assert_eq!(parse_dimen("1cm"), Some(1864679));
        assert_eq!(parse_dimen("1bp"), Some(65781));
        assert_eq!(parse_dimen("- 3sp"), Some(-3));
    }

    #[test]
    fn factor_times_internal() {
        // .7\baselineskip with \baselineskip=12pt (LaTeX \strutbox height)
        assert_eq!(scale_internal(false, 0, &[7], 12 * UNITY), Ok(550500));
        assert_eq!(scale_internal(false, 0, &[3], 12 * UNITY), Ok(235932)); // .3 -> 19661 (round_decimals), confirmed by pdfTeX
    }
}
