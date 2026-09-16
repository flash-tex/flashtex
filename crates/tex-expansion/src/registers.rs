//! `\count`/`\dimen`/`\skip`/`\toks` registers, `\advance`/`\multiply`/
//! `\divide`, and dimension/glue parsing with units (TeXbook ch. 24-27).
//!
//! Register *values* are always assigned globally-or-locally through the
//! same save-stack mechanism as control sequences (TeX treats `\count17=5`
//! as a local assignment restorable at group end); we model that with a
//! small parallel table here rather than overloading `SymbolTable`, since
//! registers are indexed by number (0..=255 in original TeX; we allow
//! wider indices since FlashTeX need not reproduce that memory limit).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Glue {
    /// All values in scaled points (sp), 1pt = 65536sp, matching TeX's
    /// internal fixed-point representation exactly so oracle comparisons
    /// against real TeX's `\showthe` output match bit-for-bit.
    pub value: i64,
    pub stretch: i64,
    pub shrink: i64,
    pub stretch_fil: u8, // 0 = pt, 1 = fil, 2 = fill, 3 = filll
    pub shrink_fil: u8,
}

impl Glue {
    pub fn fixed(value: i64) -> Self {
        Glue { value, stretch: 0, shrink: 0, stretch_fil: 0, shrink_fil: 0 }
    }
}

/// Callback the host (compiler) provides so `em`/`ex` units resolve using
/// the current font's metrics, per TeXbook ch. 24 ("quad" and "x-height").
/// Returned in scaled points.
pub trait FontMetrics {
    fn quad_sp(&self) -> i64;
    fn x_height_sp(&self) -> i64;
    /// The quad of the font `font` selects: the engine's font selector, as
    /// changed by host font switches ([`FontSwitch`]). Hosts that declare no
    /// switches only ever see `0`.
    fn quad_sp_in(&self, font: u32) -> i64 {
        let _ = font;
        self.quad_sp()
    }
    fn x_height_sp_in(&self, font: u32) -> i64 {
        let _ = font;
        self.x_height_sp()
    }
}

/// What a host font command (`\small`, `\bfseries`, `\textbf`) does to the
/// engine's font selector, an opaque `u32` the host's [`FontMetrics`] decodes:
/// `((font & !clear) | set) ^ toggle`. The selector is saved and restored by
/// TeX grouping like any local assignment. An `argument` switch
/// (`\textbf{..}`) applies inside the brace group that follows it instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontSwitch {
    pub clear: u32,
    pub set: u32,
    pub toggle: u32,
    pub argument: bool,
}

impl FontSwitch {
    pub fn apply(self, font: u32) -> u32 {
        ((font & !self.clear) | self.set) ^ self.toggle
    }
}

/// A metrics provider usable when no real font is wired up yet (tests,
/// early adoption): uses the common 10pt CM defaults (quad = 10pt,
/// x-height ~ 4.3055pt), clearly documented as a placeholder.
pub struct DefaultFontMetrics;
impl FontMetrics for DefaultFontMetrics {
    fn quad_sp(&self) -> i64 {
        10 * 65536
    }
    fn x_height_sp(&self) -> i64 {
        (4.30554 * 65536.0) as i64
    }
}

/// Parse a TeX unit-of-measure suffix at the start of `s`, returning the sp
/// per unit for absolute units. Font-relative units (`em`, `ex`) are
/// resolved by the caller via `FontMetrics` since they need font context.
pub fn absolute_unit_sp_per_unit(unit: &str) -> Option<f64> {
    // 1pt = 65536sp exactly; conversions per TeXbook ch. 10 table.
    match unit {
        "pt" => Some(65536.0),
        "sp" => Some(1.0),
        "in" => Some(65536.0 * 72.27),
        "pc" => Some(65536.0 * 12.0),
        "bp" => Some(65536.0 * 72.27 / 72.0),
        "cm" => Some(65536.0 * 72.27 / 2.54),
        "mm" => Some(65536.0 * 72.27 / 25.4),
        "dd" => Some(65536.0 * 1238.0 / 1157.0),
        "cc" => Some(65536.0 * 1238.0 / 1157.0 * 12.0),
        _ => None,
    }
}

/// tex.web §102 `round_decimals`: the digits after a decimal point as a
/// fraction of one unit in 65536ths, rounded half up with integer
/// arithmetic only. At most the first 17 digits are used; further digits
/// are consumed but ignored, exactly as TeX's `scan_dimen` does
/// (`if k<17`). The caller provides ASCII decimal digits, as TeX's
/// scanner does.
pub fn round_decimals(frac_digits: &str) -> i64 {
    let mut a: i64 = 0;
    for d in frac_digits.bytes().take(17).rev() {
        a = (a + (d - b'0') as i64 * 131072) / 10;
    }
    (a + 1) / 2
}

/// Parse a decimal constant per TeXbook ch. 24 (`<digit>* (. <digit>*)?`)
/// returning its value scaled by `sp_per_unit`, with TeX's exact
/// fixed-point integer algorithm: `round_decimals` (§102) converts the
/// fractional digits to 65536ths of a unit, and the §458 scaling step
/// combines the integer and fractional parts (`n * 65536 + f`) before
/// converting to scaled points. The conversion truncates toward zero,
/// matching TeX's `xn_over_d` division; for `pt` the result is bit-exact
/// with no floating-point rounding in the value itself.
pub fn scale_decimal(int_part: i64, frac_digits: &str, sp_per_unit: f64) -> i64 {
    let frac = round_decimals(frac_digits);
    // The §458 scaling step: total magnitude in 65536ths of the unit.
    // `i128` so no realistic integer part can overflow before the float
    // conversion below (TeX itself errors out long before that).
    let magnitude = int_part.unsigned_abs() as i128 * 65536 + frac as i128;
    // Dividing first keeps the multiplier exact for `pt` (1.0) and `sp`
    // (2^-16), so those units never touch floating-point rounding in the
    // value itself. The final truncation matches TeX's `xn_over_d`
    // integer division -- e.g. 1in = 72.27pt converts to 4736286sp, not
    // the rounded 4736287. Verified against real TeX via the oracle
    // corpus (`dimen_in`).
    let sp = magnitude as f64 * (sp_per_unit / 65536.0);
    let truncated = sp.trunc() as i64;
    if int_part < 0 {
        -truncated
    } else {
        truncated
    }
}

/// tex.web §107 `xn_over_d`: x*n/d truncated toward zero.
pub(crate) fn xn_over_d(x: i64, n: i64, d: i64) -> i64 {
    let r = (x.abs() as i128 * n as i128) / d as i128;
    if x < 0 {
        -(r as i64)
    } else {
        r as i64
    }
}

/// tex.web §455: `<decimal><internal dimen>` and the font units `em`/`ex`
/// (whose `v` is the font's quad or x-height in scaled points) scale as
/// `n*v + xn_over_d(v, f, 2^16)`, with `f` the fraction rounded to 2^16.
/// `int_part` and `v` carry their own signs; the caller attaches the
/// number's sign.
pub fn scale_internal_dimen(int_part: i64, frac_digits: &str, v: i64) -> i64 {
    int_part * v + xn_over_d(v, round_decimals(frac_digits), 65536)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_decimals_matches_tex_web_102() {
        assert_eq!(round_decimals(""), 0);
        assert_eq!(round_decimals("5"), 32768);
        assert_eq!(round_decimals("3"), 19661);
        assert_eq!(round_decimals("00001"), 1);
        assert_eq!(round_decimals("99998"), 65535);
        assert_eq!(round_decimals("99999"), 65535);
        // Seventeen 9s round up to a full unit.
        assert_eq!(round_decimals("99999999999999999"), 65536);
    }

    #[test]
    fn round_decimals_ignores_digits_past_17() {
        // TeX's `scan_dimen` keeps only the first 17 fraction digits.
        assert_eq!(
            round_decimals("23456789012345678"),
            round_decimals("2345678901234567")
        );
        assert_eq!(round_decimals("2345678901234567"), 15373);
    }

    #[test]
    fn scale_decimal_matches_pdftex_showthe() {
        const PT: f64 = 65536.0;
        // `\dimen0=16383.99998pt`: real TeX reads 1073741823sp.
        assert_eq!(scale_decimal(16383, "99998", PT), 1073741823);
        // `\dimen0=0.00001pt`: 0.65536sp rounds half up to 1sp.
        assert_eq!(scale_decimal(0, "00001", PT), 1);
        // `\dimen0=1.5pt` is 98304sp.
        assert_eq!(scale_decimal(1, "5", PT), 98304);
        // `\dimen0=-0.5sp`: half an sp truncates to 0 (sign by caller).
        assert_eq!(scale_decimal(0, "5", 1.0), 0);
        // `\maxdimen` is 16383.99999pt = 2^30 - 1 sp.
        assert_eq!(scale_decimal(16383, "99999", PT), 1073741823);
        // An 18-digit fraction equals its 17-digit prefix.
        assert_eq!(
            scale_decimal(1, "23456789012345678", PT),
            scale_decimal(1, "2345678901234567", PT)
        );
        assert_eq!(scale_decimal(1, "23456789012345678", PT), 80909);
    }
}
