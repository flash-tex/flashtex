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

/// Parse a decimal constant per TeXbook ch. 24 (`<digit>* (. <digit>*)?`)
/// returning its value scaled by `sp_per_unit`, rounded per TeX's
/// round-half-up-away-from-zero on the internal fixed-point remainder.
pub fn scale_decimal(int_part: i64, frac_digits: &str, sp_per_unit: f64) -> i64 {
    let frac: f64 = if frac_digits.is_empty() {
        0.0
    } else {
        format!("0.{frac_digits}").parse().unwrap_or(0.0)
    };
    let magnitude = int_part.unsigned_abs() as f64 + frac;
    let sp = magnitude * sp_per_unit;
    // Real TeX's unit conversion (`xn_over_d` applied to the num/den pair
    // for each unit, tex.web ch. 24) is exact-rational integer division,
    // which truncates rather than rounds -- e.g. 1in = 72.27pt converts
    // to 4736286sp, not the rounded 4736287. Verified against real TeX
    // via the oracle corpus (`dimen_in`).
    let truncated = sp.trunc() as i64;
    if int_part < 0 {
        -truncated
    } else {
        truncated
    }
}

/// tex.web §102 `round_decimals`: the digits after a decimal point as a
/// fraction of 2^16, rounded.
pub(crate) fn round_decimals(digits: &str) -> i64 {
    let mut a: i64 = 0;
    for d in digits.bytes().take(17).rev() {
        a = (a + (d - b'0') as i64 * 131072) / 10;
    }
    (a + 1) / 2
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
