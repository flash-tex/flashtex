//! TeX's `\accent` primitive (tex.web §1123–1125, TeXbook ch. 21 p. 286).
//!
//! OT1 builds every `\DeclareTextAccent` this way: LaTeX's `\add@accent`
//! (latex.ltx) expands to `\accent<slot> <base>`. T1 uses the same primitive
//! only when no `\DeclareTextComposite` provides a precomposed slot.

use crate::tfm::{Scaled, ScaledFont};

/// Result of `make_accent` for an accent followed by a base character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPlacement {
    /// `\kern` (subtype acc_kern) before the accent: `delta`.
    pub kern_before: Scaled,
    /// `shift_amount` of the box holding the accent (`x - h`; negative raises it).
    /// `None` when the base height equals the accent font's x-height and the
    /// accent character is appended unboxed.
    pub shift: Option<Scaled>,
    /// `\kern` after the accent: `-a - delta`.
    pub kern_after: Scaled,
    /// Accent character width `a`.
    pub accent_width: Scaled,
}

impl AccentPlacement {
    /// Horizontal offset of the accent glyph origin relative to the base glyph origin.
    pub fn accent_x_relative_to_base(&self) -> Scaled {
        // accent starts at kern_before; base starts at kern_before + a + kern_after = 0.
        self.kern_before
    }

    /// Vertical raise of the accent baseline (positive = up).
    pub fn accent_raise(&self) -> Scaled {
        -self.shift.unwrap_or(0)
    }
}

/// Pascal/web2c `round` for reals: half away from zero.
pub fn tex_round(r: f64) -> Scaled {
    if r >= 0.0 {
        (r + 0.5).floor() as Scaled
    } else {
        -((-r + 0.5).floor() as Scaled)
    }
}

/// §1123/§1125. `accent_font` is the font current at `\accent` (its slant `s`
/// and x-height `x` are used); `base` is the font current when the base
/// character is read (after any intervening assignments such as a font switch)
/// and the base character.
pub fn make_accent(
    accent_font: &ScaledFont,
    accent: u8,
    base: (&ScaledFont, u8),
) -> AccentPlacement {
    let s = accent_font.slant() as f64 / 65536.0;
    let a = accent_font.width(accent);
    let x = accent_font.x_height();
    let (bf, bc) = base;
    let t = bf.slant() as f64 / 65536.0;
    let w = bf.width(bc);
    let h = bf.height(bc);
    let shift = if h != x { Some(x - h) } else { None };
    let delta = tex_round((w - a) as f64 / 2.0 + h as f64 * t - x as f64 * s);
    AccentPlacement {
        kern_before: delta,
        shift,
        kern_after: -a - delta,
        accent_width: a,
    }
}

#[cfg(test)]
mod tests {
    use super::tex_round;

    #[test]
    fn rounds_half_away_from_zero() {
        assert_eq!(tex_round(2.5), 3);
        assert_eq!(tex_round(-2.5), -3);
        assert_eq!(tex_round(-2.4), -2);
    }
}
