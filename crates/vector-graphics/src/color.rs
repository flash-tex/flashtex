//! Colour model.
//!
//! Semantics, stated so consumers do not have to guess:
//!
//! - Components are `f64` in `0.0..=1.0`. Out-of-range or non-finite values
//!   are clamped by [`Color::clamped`]; serializers always clamp.
//! - Alpha is **straight** (non-premultiplied) coverage in `0.0..=1.0` and
//!   lives on [`Paint`], separate from the colour, mirroring PDF's `ca`/`CA`.
//! - RGB is assumed to be sRGB and gray is assumed to be sRGB-gray; CMYK is
//!   device CMYK. **No colour management is implemented**: [`Color::to_rgb`]
//!   is a naive preview conversion (`1 - min(1, c + k)`) and is not
//!   colorimetrically accurate.

/// A device colour without alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Color {
    Gray(f64),
    Rgb(f64, f64, f64),
    Cmyk(f64, f64, f64, f64),
}

impl Color {
    pub const BLACK: Color = Color::Gray(0.0);
    pub const WHITE: Color = Color::Gray(1.0);

    /// Clamps every component to `0.0..=1.0` (NaN becomes 0).
    pub fn clamped(self) -> Color {
        fn c(v: f64) -> f64 {
            if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
        }
        match self {
            Color::Gray(g) => Color::Gray(c(g)),
            Color::Rgb(r, g, b) => Color::Rgb(c(r), c(g), c(b)),
            Color::Cmyk(cy, m, y, k) => Color::Cmyk(c(cy), c(m), c(y), c(k)),
        }
    }

    /// Naive sRGB preview conversion; see the module documentation.
    pub fn to_rgb(self) -> (f64, f64, f64) {
        match self.clamped() {
            Color::Gray(g) => (g, g, g),
            Color::Rgb(r, g, b) => (r, g, b),
            Color::Cmyk(c, m, y, k) => (
                1.0 - (c + k).min(1.0),
                1.0 - (m + k).min(1.0),
                1.0 - (y + k).min(1.0),
            ),
        }
    }
}

/// A colour with straight alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub color: Color,
    /// Straight alpha, `0.0` transparent to `1.0` opaque.
    pub alpha: f64,
}

impl Paint {
    pub const BLACK: Paint = Paint {
        color: Color::BLACK,
        alpha: 1.0,
    };

    pub const fn opaque(color: Color) -> Paint {
        Paint { color, alpha: 1.0 }
    }

    pub const fn new(color: Color, alpha: f64) -> Paint {
        Paint { color, alpha }
    }

    pub fn clamped(self) -> Paint {
        let alpha = if self.alpha.is_nan() {
            1.0
        } else {
            self.alpha.clamp(0.0, 1.0)
        };
        Paint {
            color: self.color.clamped(),
            alpha,
        }
    }
}

impl Default for Paint {
    fn default() -> Self {
        Paint::BLACK
    }
}
