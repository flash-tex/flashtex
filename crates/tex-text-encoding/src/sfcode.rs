//! Space factor (TeX §1034, §1041–1044) with LaTeX's `\sfcode` table.
//!
//! Sources: IniTeX sets `\sfcode` of `A`–`Z` to 999 (tex.web §232);
//! `latex.ltx` sets `\sfcode` 999 for the 8-bit uppercase ranges
//! `"80`–`"9C` and `"C0`–`"DF` (latex.ltx:22095–22106, the T1 layout),
//! `\sfcode` 0 for `)`, `'`, `]` (latex.ltx:648), and `\nonfrenchspacing`
//! (latex.ltx:554) `.?!` 3000, `:` 2000, `;` 1500, `,` 1250 — the document
//! default. `\frenchspacing` (latex.ltx:552) resets those six to 1000.
//! `\@` is `\spacefactor\@m{}` (latex.ltx:9422).

use crate::tfm::{Scaled, ScaledFont};

/// A glue specification in scaled points (finite orders only here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlueSpec {
    pub width: Scaled,
    pub stretch: Scaled,
    pub shrink: Scaled,
}

/// The 256-entry `\sfcode` table in force for 8-bit text characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SfCodes(pub [u16; 256]);

impl Default for SfCodes {
    fn default() -> Self {
        let mut t = [1000u16; 256];
        for c in b'A'..=b'Z' {
            t[c as usize] = 999;
        }
        t[0x80..=0x9C].fill(999);
        t[0xC0..=0xDF].fill(999);
        t[b')' as usize] = 0;
        t[b'\'' as usize] = 0;
        t[b']' as usize] = 0;
        let mut s = SfCodes(t);
        s.nonfrenchspacing();
        s
    }
}

impl SfCodes {
    pub fn frenchspacing(&mut self) {
        for c in b".?!:;," {
            self.0[*c as usize] = 1000;
        }
    }

    pub fn nonfrenchspacing(&mut self) {
        for (c, v) in [
            (b'.', 3000),
            (b'?', 3000),
            (b'!', 3000),
            (b':', 2000),
            (b';', 1500),
            (b',', 1250),
        ] {
            self.0[c as usize] = v;
        }
    }

    pub fn get(&self, c: u8) -> u16 {
        self.0[c as usize]
    }
}

/// `adjust_space_factor` (§1034) for one character with `\sfcode` `main_s`.
pub fn adjust_space_factor(space_factor: i32, main_s: u16) -> i32 {
    let main_s = main_s as i32;
    if main_s == 1000 {
        1000
    } else if main_s < 1000 {
        if main_s > 0 {
            main_s
        } else {
            space_factor
        }
    } else if space_factor < 1000 {
        1000
    } else {
        main_s
    }
}

/// tex.web §107 `xn_over_d` (truncating, sign-symmetric).
pub fn xn_over_d(x: Scaled, n: i32, d: i32) -> Scaled {
    let v = (x as i64).abs() * n as i64 / d as i64;
    (if x < 0 { -v } else { v }) as Scaled
}

/// The interword glue TeX appends for a space token (§1041–1044) with
/// `\spaceskip` and `\xspaceskip` both zero (the LaTeX default).
pub fn interword_glue(font: &ScaledFont, space_factor: i32) -> GlueSpec {
    let mut g = GlueSpec {
        width: font.space(),
        stretch: font.space_stretch(),
        shrink: font.space_shrink(),
    };
    if space_factor == 1000 {
        return g;
    }
    if space_factor >= 2000 {
        g.width += font.extra_space();
    }
    g.stretch = xn_over_d(g.stretch, space_factor, 1000);
    g.shrink = xn_over_d(g.shrink, 1000, space_factor);
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase_then_period_keeps_normal_space() {
        let sf = SfCodes::default();
        let mut f = 1000;
        for c in b"A." {
            f = adjust_space_factor(f, sf.get(*c));
        }
        assert_eq!(f, 1000);
        let mut f = 1000;
        for c in b"a.)" {
            f = adjust_space_factor(f, sf.get(*c));
        }
        assert_eq!(f, 3000);
    }

    #[test]
    fn xn_over_d_truncates_toward_zero() {
        assert_eq!(xn_over_d(109226, 3000, 1000), 327678);
        assert_eq!(xn_over_d(72818, 1000, 3000), 24272);
        assert_eq!(xn_over_d(-7, 1, 2), -3);
    }
}
