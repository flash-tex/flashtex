//! xcolor names and mixing expressions (`red`, `red!40`, `blue!30!black`).
//!
//! The base colours carry the per-model values xcolor itself declares, so a
//! mix happens in the model of the first colour exactly as xcolor does it,
//! and the output model (gray / RGB / CMYK) matches what pdfTeX would write.

use std::collections::HashMap;

use crate::color::Color;

/// `(name, rgb, cmyk, gray, native model)`; native: 0 rgb, 1 cmyk, 2 gray.
type Base = (&'static str, [f64; 3], [f64; 4], f64, u8);

const BASE: &[Base] = &[
    ("red", [1.0, 0.0, 0.0], [0.0, 1.0, 1.0, 0.0], 0.3, 0),
    ("green", [0.0, 1.0, 0.0], [1.0, 0.0, 1.0, 0.0], 0.59, 0),
    ("blue", [0.0, 0.0, 1.0], [1.0, 1.0, 0.0, 0.0], 0.11, 0),
    ("brown", [0.75, 0.5, 0.25], [0.0, 0.25, 0.5, 0.25], 0.5475, 0),
    ("lime", [0.75, 1.0, 0.0], [0.25, 0.0, 1.0, 0.0], 0.815, 0),
    ("orange", [1.0, 0.5, 0.0], [0.0, 0.5, 1.0, 0.0], 0.595, 0),
    ("pink", [1.0, 0.75, 0.75], [0.0, 0.25, 0.25, 0.0], 0.825, 0),
    ("purple", [0.75, 0.0, 0.25], [0.0, 0.75, 0.5, 0.25], 0.2525, 0),
    ("teal", [0.0, 0.5, 0.5], [0.5, 0.0, 0.0, 0.5], 0.35, 0),
    ("violet", [0.5, 0.0, 0.5], [0.0, 0.5, 0.0, 0.5], 0.205, 0),
    ("cyan", [0.0, 1.0, 1.0], [1.0, 0.0, 0.0, 0.0], 0.7, 1),
    ("magenta", [1.0, 0.0, 1.0], [0.0, 1.0, 0.0, 0.0], 0.41, 1),
    ("yellow", [1.0, 1.0, 0.0], [0.0, 0.0, 1.0, 0.0], 0.89, 1),
    ("olive", [0.5, 0.5, 0.0], [0.0, 0.0, 1.0, 0.5], 0.39, 1),
    ("black", [0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 0.0, 2),
    ("darkgray", [0.25, 0.25, 0.25], [0.0, 0.0, 0.0, 0.75], 0.25, 2),
    ("gray", [0.5, 0.5, 0.5], [0.0, 0.0, 0.0, 0.5], 0.5, 2),
    ("lightgray", [0.75, 0.75, 0.75], [0.0, 0.0, 0.0, 0.25], 0.75, 2),
    ("white", [1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 0.0], 1.0, 2),
];

/// User-defined colours (`\definecolor`, `\colorlet`).
#[derive(Clone, Debug, Default)]
pub struct Palette {
    defined: HashMap<String, Color>,
}

fn base(name: &str) -> Option<&'static Base> {
    BASE.iter().find(|b| b.0 == name)
}

/// Converts `c` into the model of `like`, using xcolor's table values for
/// base colours when available (`name`).
fn to_model(c: Color, like: Color, name: Option<&str>) -> Color {
    if let Some(b) = name.and_then(base) {
        return match like {
            Color::Gray(_) => Color::Gray(b.3),
            Color::Rgb(..) => Color::Rgb(b.1[0], b.1[1], b.1[2]),
            Color::Cmyk(..) => Color::Cmyk(b.2[0], b.2[1], b.2[2], b.2[3]),
        };
    }
    match (c, like) {
        (Color::Gray(g), Color::Rgb(..)) => Color::Rgb(g, g, g),
        (Color::Gray(g), Color::Cmyk(..)) => Color::Cmyk(0.0, 0.0, 0.0, 1.0 - g),
        (Color::Rgb(r, g, b), Color::Gray(_)) => Color::Gray(0.3 * r + 0.59 * g + 0.11 * b),
        (Color::Rgb(r, g, b), Color::Cmyk(..)) => {
            let (c, m, y) = (1.0 - r, 1.0 - g, 1.0 - b);
            let k = c.min(m).min(y);
            Color::Cmyk(c - k, m - k, y - k, k)
        }
        (Color::Cmyk(..), Color::Rgb(..)) => {
            let (r, g, b) = c.to_rgb();
            Color::Rgb(r, g, b)
        }
        (Color::Cmyk(..), Color::Gray(_)) => {
            let (r, g, b) = c.to_rgb();
            Color::Gray(0.3 * r + 0.59 * g + 0.11 * b)
        }
        _ => c,
    }
}

fn mix(a: Color, b: Color, t: f64) -> Color {
    let l = |x: f64, y: f64| t * x + (1.0 - t) * y;
    match (a, b) {
        (Color::Gray(x), Color::Gray(y)) => Color::Gray(l(x, y)),
        (Color::Rgb(r, g, bb), Color::Rgb(r2, g2, b2)) => Color::Rgb(l(r, r2), l(g, g2), l(bb, b2)),
        (Color::Cmyk(c, m, y, k), Color::Cmyk(c2, m2, y2, k2)) => {
            Color::Cmyk(l(c, c2), l(m, m2), l(y, y2), l(k, k2))
        }
        _ => a,
    }
}

impl Palette {
    pub fn define(&mut self, name: &str, color: Color) {
        self.defined.insert(name.trim().to_string(), color);
    }

    fn named(&self, name: &str) -> Option<Color> {
        let name = name.trim();
        if let Some(c) = self.defined.get(name) {
            return Some(*c);
        }
        base(name).map(|b| match b.4 {
            0 => Color::Rgb(b.1[0], b.1[1], b.1[2]),
            1 => Color::Cmyk(b.2[0], b.2[1], b.2[2], b.2[3]),
            _ => Color::Gray(b.3),
        })
    }

    /// Parses an xcolor expression; `None` when it is not a colour.
    pub fn parse(&self, expr: &str) -> Option<Color> {
        let expr = expr.trim();
        if expr.is_empty() {
            return None;
        }
        let parts: Vec<&str> = expr.split('!').map(str::trim).collect();
        let first_name = parts[0];
        let mut c = self.named(first_name)?;
        let mut i = 1;
        while i < parts.len() {
            let pct: f64 = parts[i].parse().ok()?;
            let t = (pct / 100.0).clamp(0.0, 1.0);
            let (other, other_name) = match parts.get(i + 1) {
                Some(n) if !n.is_empty() => (self.named(n)?, Some(*n)),
                _ => (Color::WHITE, Some("white")),
            };
            let other_name = other_name.filter(|n| !self.defined.contains_key(*n));
            let other = to_model(other, c, other_name);
            c = mix(c, other, t);
            i += 2;
        }
        Some(c)
    }

    /// `\definecolor{name}{model}{values}`.
    pub fn define_from_spec(&mut self, name: &str, model: &str, values: &str) -> Result<(), String> {
        let nums: Vec<f64> = values
            .split(',')
            .map(|v| v.trim().parse::<f64>())
            .collect::<Result<_, _>>()
            .map_err(|_| format!("bad colour values `{values}`"))?;
        let c = match (model.trim(), nums.as_slice()) {
            ("rgb", [r, g, b]) => Color::Rgb(*r, *g, *b),
            ("RGB", [r, g, b]) => Color::Rgb(r / 255.0, g / 255.0, b / 255.0),
            ("gray", [g]) => Color::Gray(*g),
            ("cmyk", [c, m, y, k]) => Color::Cmyk(*c, *m, *y, *k),
            _ => return Err(format!("unsupported colour model `{model}` with values `{values}`")),
        };
        self.define(name, c);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixes_in_the_first_model() {
        let p = Palette::default();
        assert_eq!(p.parse("red"), Some(Color::Rgb(1.0, 0.0, 0.0)));
        assert_eq!(p.parse("red!50"), Some(Color::Rgb(1.0, 0.5, 0.5)));
        assert_eq!(p.parse("blue!20!black"), Some(Color::Rgb(0.0, 0.0, 0.2)));
        assert_eq!(p.parse("cyan"), Some(Color::Cmyk(1.0, 0.0, 0.0, 0.0)));
        assert_eq!(p.parse("black!30"), Some(Color::Gray(0.7)));
        assert_eq!(p.parse("thick"), None);
    }
}
