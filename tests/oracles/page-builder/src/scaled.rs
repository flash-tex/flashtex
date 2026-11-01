//! Scaled-point arithmetic exactly as TeX does it (tex.web part 7).

/// A dimension in scaled points (1pt = 65536sp).
pub type Scaled = i32;

pub const UNITY: Scaled = 65536;
/// `\maxdimen` (§421).
pub const MAX_DIMEN: Scaled = 0x3FFF_FFFF;
/// `prev_depth` value that suppresses interline glue (-1000pt, §212).
pub const IGNORE_DEPTH: Scaled = -65_536_000;

pub const INF_BAD: i32 = 10_000;
pub const INF_PENALTY: i32 = 10_000;
pub const EJECT_PENALTY: i32 = -10_000;
/// §833.
pub const AWFUL_BAD: i32 = 0x3FFF_FFFF;
/// §974.
pub const DEPLORABLE: i32 = 100_000;

/// Converts points to scaled points (rounding to nearest).
pub fn pt(points: f64) -> Scaled {
    (points * 65536.0).round() as Scaled
}

/// Converts scaled points to points.
pub fn to_pt(s: Scaled) -> f64 {
    f64::from(s) / 65536.0
}

/// `badness(t, s)` (§108): approximately `100(t/s)^3`, capped at `INF_BAD`.
pub fn badness(t: Scaled, s: Scaled) -> i32 {
    if t == 0 {
        return 0;
    }
    if s <= 0 {
        return INF_BAD;
    }
    let (t, s) = (i64::from(t), i64::from(s));
    let r = if t <= 7_230_584 {
        (t * 297) / s
    } else if s >= 1_663_497 {
        t / (s / 297)
    } else {
        t
    };
    if r > 1290 {
        INF_BAD
    } else {
        ((r * r * r + 0x20000) / 0x40000) as i32
    }
}

/// `x_over_n` (§106): division truncating toward zero.
pub fn x_over_n(x: Scaled, n: i32) -> Scaled {
    if n == 0 {
        return 0; // TeX reports arith_error and returns 0.
    }
    ((i64::from(x)) / i64::from(n)) as Scaled
}

/// `print_scaled` (§103): the shortest decimal that reads back to `s`.
pub fn print_scaled(s: Scaled) -> String {
    let mut out = String::new();
    let mut s = i64::from(s);
    if s < 0 {
        out.push('-');
        s = -s;
    }
    let unity = i64::from(UNITY);
    out.push_str(&(s / unity).to_string());
    out.push('.');
    s = 10 * (s % unity) + 5;
    let mut delta: i64 = 10;
    loop {
        if delta > unity {
            s += 0o100000 - 50000; // round the last digit
        }
        out.push(char::from(b'0' + (s / unity) as u8));
        s = 10 * (s % unity);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
    out
}

/// Parses a decimal point value the way TeX's scanner does (§448, §452
/// `round_decimals`), so `parse_pt(print_scaled(s)) == s` for every `s`.
pub fn parse_pt(text: &str) -> Option<Scaled> {
    let text = text.trim().trim_end_matches("pt");
    let (neg, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    let int: i64 = if int_part.is_empty() { 0 } else { int_part.parse().ok()? };
    let digits: Vec<u8> = frac_part.bytes().take(17).map(|b| b.wrapping_sub(b'0')).collect();
    if digits.iter().any(|d| *d > 9) {
        return None;
    }
    let mut a: i64 = 0;
    for d in digits.iter().rev() {
        a = (a + i64::from(*d) * 0o400000) / 10;
    }
    let frac = (a + 1) / 2;
    let v = int * i64::from(UNITY) + frac;
    Some(if neg { -v } else { v } as Scaled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badness_matches_tex() {
        assert_eq!(badness(0, UNITY), 0);
        assert_eq!(badness(UNITY, 0), INF_BAD);
        assert_eq!(badness(UNITY, UNITY), 100);
        assert_eq!(badness(pt(0.5), UNITY), 12);
        assert_eq!(badness(pt(14.5), pt(9.0)), 417);
    }

    #[test]
    fn print_scaled_round_trips() {
        for s in [0, 1, -1, 65536, 200_000, 3_05556, -12_345_678, MAX_DIMEN, pt(3.05556), pt(0.0001)] {
            assert_eq!(parse_pt(&print_scaled(s)), Some(s), "{s}");
        }
        assert_eq!(print_scaled(pt(10.0)), "10.0");
        assert_eq!(print_scaled(MAX_DIMEN), "16383.99998");
        assert_eq!(print_scaled(-pt(0.5)), "-0.5");
    }
}
