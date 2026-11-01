//! pdfTeX / e-TeX integer arithmetic, transcribed from the primary sources so
//! the results agree to the scaled point.
//!
//! Sources (TeX Live trunk, `texk/web2c/pdftexdir/`, pdfTeX 1.40.27):
//! * `pdftex.web` `round_xn_over_d` (§ "function round_xn_over_d"), `divide_scaled`,
//!   `fix_int`, TeX's `badness` (§108 of tex.web, unchanged in pdfTeX).
//! * `utils.c` `extxnoverd` (double precision, round half away from zero).
//! * e-TeX `\numexpr` scaling (`a*b/c`, rounded half away from zero) as used by
//!   microtype's `\MT@scale` (microtype.sty v3.2d line 412).

/// A TeX dimension in scaled points (2^16 sp = 1 pt).
pub type Scaled = i32;

/// TeX's `inf_bad`.
pub const INF_BAD: i32 = 10_000;
/// `unity`: 1pt in sp.
pub const UNITY: i64 = 65_536;

/// `round_xn_over_d(x, n, d)`: `x*n/d` rounded half up in magnitude, sign of `x`.
/// `n`, `d` must be non-negative (as in every pdfTeX call site).
pub fn round_xn_over_d(x: Scaled, n: i32, d: i32) -> Scaled {
    debug_assert!(n >= 0 && d > 0);
    let positive = x >= 0;
    let ax = (x as i64).abs();
    let n = n as i64;
    let d = d as i64;
    // The web code splits into 15-bit halves to stay in 31 bits; with i64 the
    // quotient and remainder are identical.
    let prod = ax * n;
    let mut u = prod / d;
    let v = prod % d;
    if 2 * v >= d {
        u += 1;
    }
    let u = u as Scaled;
    if positive { u } else { -u }
}

/// pdfTeX `divide_scaled(s, m, dd)`: `(s/m)·10^dd` rounded half away from zero.
pub fn divide_scaled(s: Scaled, m: Scaled, dd: u32) -> Scaled {
    assert!(m != 0, "divide_scaled: divided by zero");
    let mut sign = 1i64;
    let mut s = s as i64;
    let mut m = m as i64;
    if s < 0 {
        sign = -sign;
        s = -s;
    }
    if m < 0 {
        sign = -sign;
        m = -m;
    }
    let mut q = s / m;
    let mut r = s % m;
    for _ in 0..dd {
        q = 10 * q + (10 * r) / m;
        r = (10 * r) % m;
    }
    if 2 * r >= m {
        q += 1;
    }
    (sign * q) as Scaled
}

/// pdfTeX `extxnoverd` (utils.c): `x*n/d` in `double`, +0.5 if the result is
/// above `DBL_EPSILON` else −0.5, then truncated toward zero.
pub fn ext_xn_over_d(x: i32, n: i32, d: i32) -> i32 {
    let mut r = (x as f64) * (n as f64) / (d as f64);
    if r > f64::EPSILON {
        r += 0.5;
    } else {
        r -= 0.5;
    }
    r as i32
}

/// pdfTeX `fix_int(val, min, max)`.
pub fn fix_int(val: i32, min: i32, max: i32) -> i32 {
    val.clamp(min, max)
}

/// e-TeX `\numexpr a*b/c` (and `a*b` when `c == 0`), as microtype's `\MT@scale`
/// uses it: the product is exact, the quotient rounded half away from zero.
pub fn numexpr_scale(a: i32, b: i32, c: i32) -> i32 {
    if c == 0 {
        return a.wrapping_mul(b);
    }
    let num = a as i64 * b as i64;
    let den = c as i64;
    let neg = (num < 0) != (den < 0);
    let (n, d) = (num.abs(), den.abs());
    let mut q = n / d;
    if 2 * (n % d) >= d {
        q += 1;
    }
    (if neg { -q } else { q }) as i32
}

/// TeX's `store_scaled` (tex.web §571–572): a TFM `fix_word` (2^-20 of the
/// design size) scaled to a font loaded at `z` sp, exactly as `read_font_info`
/// computes every width, kern and `\fontdimen` of that font.
pub fn tfm_scaled(fix_word: i32, z: Scaled) -> Scaled {
    let mut z = z as i64;
    let mut alpha: i64 = 16;
    while z >= 0o40000000 {
        z /= 2;
        alpha += alpha;
    }
    let beta = 256 / alpha;
    let alpha = alpha * z;
    let [a, b, c, d] = fix_word.to_be_bytes().map(i64::from);
    let sw = (((d * z) / 256 + c * z) / 256 + b * z) / beta;
    (match a {
        0 => sw,
        _ => sw - alpha,
    }) as Scaled
}

/// TeX's `badness(t, s)` (tex.web §108).
pub fn badness(t: Scaled, s: Scaled) -> i32 {
    if t == 0 {
        return 0;
    }
    if s <= 0 {
        return INF_BAD;
    }
    let r: i64 = if t <= 7_230_584 {
        (t as i64 * 297) / s as i64
    } else if s >= 1_663_497 {
        t as i64 / (s as i64 / 297)
    } else {
        t as i64
    };
    if r > 1290 {
        INF_BAD
    } else {
        ((r * r * r + 0x20000) / 0x40000) as i32
    }
}

/// Print a dimension the way TeX's `print_scaled` does (shortest decimal that
/// reads back to the same sp). Used to compare with `\showbox` output.
pub fn print_scaled(s: Scaled) -> String {
    let mut out = String::new();
    let mut s = s as i64;
    if s < 0 {
        out.push('-');
        s = -s;
    }
    out.push_str(&(s / UNITY).to_string());
    out.push('.');
    let mut s = 10 * (s % UNITY) + 5;
    let mut delta = 10i64;
    loop {
        if delta > UNITY {
            s += 0o100000 - 50000; // round the last digit
        }
        out.push(char::from(b'0' + (s / UNITY) as u8));
        s = 10 * (s % UNITY);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
    out
}

/// Inverse of `print_scaled` for the decimals TeX writes (`round_decimals`
/// semantics of `scan_dimen`: 17 digits max, rounded).
pub fn parse_scaled(text: &str) -> Option<Scaled> {
    let (neg, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((a, b)) => (a, b),
        None => (body, ""),
    };
    let int: i64 = if int_part.is_empty() { 0 } else { int_part.parse().ok()? };
    let digits: Vec<i64> = frac_part
        .chars()
        .take(17)
        .map(|c| c.to_digit(10).map(|d| d as i64))
        .collect::<Option<Vec<_>>>()?;
    // tex.web §102 round_decimals
    let mut a: i64 = 0;
    for &d in digits.iter().rev() {
        a = (a + d * 0o400000) / 10;
    }
    let frac = (a + 1) / 2;
    let v = int * UNITY + frac;
    Some(if neg { -(v as i32) } else { v as i32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_helpers() {
        assert_eq!(round_xn_over_d(10, 1, 4), 3); // 2.5 -> 3
        assert_eq!(round_xn_over_d(-10, 1, 4), -3);
        assert_eq!(divide_scaled(1, 3, 3), 333);
        assert_eq!(divide_scaled(2, 3, 3), 667);
        assert_eq!(divide_scaled(-2, 3, 3), -667);
        assert_eq!(ext_xn_over_d(-15, 1, 2), -8);
        assert_eq!(ext_xn_over_d(15, 1, 2), 8);
        assert_eq!(numexpr_scale(7, 1, 2), 4);
        assert_eq!(numexpr_scale(-7, 1, 2), -4);
    }

    #[test]
    fn tfm_scaling_is_store_scaled() {
        // 1.0 design size at 10pt, halves, negative words, and a size above
        // 2^23 sp where TeX halves z first.
        assert_eq!(tfm_scaled(1 << 20, 655_360), 655_360);
        assert_eq!(tfm_scaled(1 << 19, 655_360), 327_680);
        assert_eq!(tfm_scaled(-(1 << 20), 655_360), -655_360);
        assert_eq!(tfm_scaled(1 << 20, 717_619), 717_619);
        assert_eq!(tfm_scaled(1 << 20, 40 * 65_536), 40 * 65_536);
        // TeX truncates: 0.5 fix of a 1sp-odd size.
        assert_eq!(tfm_scaled(1 << 19, 717_619), 358_809);
    }

    #[test]
    fn badness_matches_tex() {
        assert_eq!(badness(0, 0), 0);
        assert_eq!(badness(1, 0), INF_BAD);
        assert_eq!(badness(65536, 65536), 100);
        // r = 148; (148^3 + 2^17) / 2^18 = 12
        assert_eq!(badness(32768, 65536), 12);
    }

    #[test]
    fn scaled_roundtrip() {
        for s in [0, 1, 65536, 237927, -59419, 1_000_000, 123_456_789] {
            assert_eq!(parse_scaled(&print_scaled(s)), Some(s), "{s}");
        }
        assert_eq!(print_scaled(65536 * 3 / 2), "1.5");
        assert_eq!(parse_scaled("10.88788"), Some(713_548));
    }
}
