//! Minimal static representation of TeX font metric (TFM) data.
//!
//! Only what the layout rules consume is kept: per-character box dimensions,
//! italic correction, the kern against the font's skew character (used for
//! accent placement), the `next_larger` chain used to pick delimiter and big
//! operator sizes, the extensible recipe (recorded but not built yet), and the
//! ligature/kern program that TeX's `make_ord` consults between adjacent math
//! characters (tex.web §752).
//!
//! Dimensions are stored as raw TFM fixwords and scaled with [`scale`], a
//! transcription of TeX's integer algorithm (tex.web §571–572), so the
//! resulting dimensions agree with TeX's own boxes to the scaled point.

/// One scaled point: TeX's internal unit, 1/65536 pt.
pub const SP_PER_PT: f64 = 65536.0;

/// One character of a TFM font; dimensions are fixwords (value / 2^20 em).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TfmChar {
    pub code: u8,
    pub width: i32,
    pub height: i32,
    pub depth: i32,
    pub italic: i32,
    /// Kern between this character and the font's skew character, or 0.
    pub skew_kern: i32,
    /// Next larger character in the same font, or `u8::MAX`.
    pub next_larger: u8,
    /// Extensible recipe `[top, mid, bot, rep]`, each `u8::MAX` when absent.
    pub extensible: [u8; 4],
}

/// Compact constructor used by the generated table.
#[allow(clippy::too_many_arguments)]
pub const fn c(
    code: u8,
    width: i32,
    height: i32,
    depth: i32,
    italic: i32,
    skew_kern: i32,
    next_larger: u8,
    extensible: [u8; 4],
) -> TfmChar {
    TfmChar {
        code,
        width,
        height,
        depth,
        italic,
        skew_kern,
        next_larger,
        extensible,
    }
}

/// A TFM font: fontdimen parameters and its character table.
#[derive(Debug)]
pub struct TfmFont {
    pub name: &'static str,
    pub design_size: f64,
    pub skew_char: u8,
    /// `params[i]` is fontdimen `i + 1` as a fixword (fontdimen 1, the slant,
    /// is a pure number that TeX never scales).
    pub params: &'static [i32],
    pub chars: &'static [TfmChar],
    /// `(code << 16) | index`: the first instruction in `lig_kern` of every
    /// character that has a lig/kern program, sorted by code. A program whose
    /// first word is a far restart (tex.web §545) is already resolved.
    pub lig_kern_starts: &'static [u32],
    /// The TFM lig/kern program, one instruction per word: skip byte, next
    /// character, op byte, remainder (tex.web §545).
    pub lig_kern: &'static [u32],
    /// The TFM kern table as fixwords, indexed by kern instructions.
    pub kerns: &'static [i32],
}

/// The instruction a font's lig/kern program gives for a character pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LigKern {
    /// Insert a kern of this many fixwords between the two characters.
    Kern(i32),
    /// A ligature: `op` is the TFM op byte (0 is `=:`, tex.web §545) and
    /// `rem` the ligature character.
    Ligature { op: u8, rem: u8 },
}

/// Scales a fixword to points at font size `at_pt`, exactly as TeX does
/// (`store_scaled`, tex.web §571–572): `z` is the size in scaled points and
/// the fixword bytes are combined with truncating integer arithmetic.
pub fn scale(fixword: i32, at_pt: f64) -> f64 {
    let mut z = (at_pt * SP_PER_PT).round() as i64;
    let mut alpha: i64 = 16;
    while z >= 0o40000000 {
        z /= 2;
        alpha *= 2;
    }
    let beta = 256 / alpha;
    let alpha = alpha * z;
    let u = fixword as u32;
    let a = (u >> 24) as i64;
    let b = ((u >> 16) & 255) as i64;
    let c = ((u >> 8) & 255) as i64;
    let d = (u & 255) as i64;
    let sw = (((d * z) / 256 + c * z) / 256 + b * z) / beta;
    let sp = if a == 0 {
        sw
    } else if a == 255 {
        sw - alpha
    } else {
        // Out-of-range fixword; TeX would reject the font. Fall back to the
        // real-valued scaling so a bad table cannot panic the layout.
        return fixword as f64 / (1 << 20) as f64 * at_pt;
    };
    sp as f64 / SP_PER_PT
}

impl TfmFont {
    pub fn char(&self, code: u8) -> Option<&TfmChar> {
        self.chars.iter().find(|c| c.code == code)
    }

    /// fontdimen `n` (1-based) scaled to `at_pt`, 0 when absent.
    pub fn fontdimen(&self, n: usize, at_pt: f64) -> f64 {
        match self.params.get(n.wrapping_sub(1)) {
            Some(&p) => scale(p, at_pt),
            None => 0.0,
        }
    }

    pub fn next_larger(&self, ch: &TfmChar) -> Option<&TfmChar> {
        if ch.next_larger == u8::MAX {
            None
        } else {
            self.char(ch.next_larger)
        }
    }

    pub fn is_extensible(&self, ch: &TfmChar) -> bool {
        ch.extensible[3] != u8::MAX
    }

    /// The first instruction of `left`'s lig/kern program whose next
    /// character is `right`, as TeX's program walk finds it (tex.web §752,
    /// §909): a pair has at most one effective instruction, and a ligature
    /// earlier in the program shadows any later kern for the same pair.
    pub fn lig_kern(&self, left: u8, right: u8) -> Option<LigKern> {
        let i = self
            .lig_kern_starts
            .binary_search_by_key(&left, |w| (w >> 16) as u8)
            .ok()?;
        let mut a = (self.lig_kern_starts[i] & 0xFFFF) as usize;
        loop {
            let w = *self.lig_kern.get(a)?;
            let (skip, next, op, rem) = ((w >> 24) as u8, (w >> 16) as u8, (w >> 8) as u8, w as u8);
            if next == right && skip <= 128 {
                return Some(if op >= 128 {
                    LigKern::Kern(*self.kerns.get(256 * (op as usize - 128) + rem as usize)?)
                } else {
                    LigKern::Ligature { op, rem }
                });
            }
            if skip >= 128 {
                return None;
            }
            a += skip as usize + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_matches_tex_truncation() {
        // cmex10 fontdimen 8 (0.039999 em) at 10pt is 0.39998pt in TeX's
        // \showbox output, not 0.39999: the low bits truncate.
        let theta = scale(41942, 10.0);
        assert!((theta - 26213.0 / SP_PER_PT).abs() < 1e-12);
        assert_eq!(format!("{:.5}", theta), "0.39998");
        // cmsy10 quad (1.000003 em) at 10pt is 655361sp.
        assert_eq!(scale(1048579, 10.0) * SP_PER_PT, 655361.0);
        // Negative fixwords (the depth of cmr10 '=') keep their sign.
        assert!(scale(-139594, 10.0) < 0.0);
    }
}
