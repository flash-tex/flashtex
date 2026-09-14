//! Colours of colortbl fills and rules, resolved to sRGB for painting.
//!
//! A minimal stand-in for the colour model of PR #150/#158 (which replays
//! xcolor's arithmetic exactly and paints in the colour's own model): the
//! xcolor.sty v3.02 base colours (`\definecolorset` in their own models),
//! `\definecolor{name}{model}{spec}` read from the source, the models `rgb`,
//! `RGB`, `HTML`, `gray` and `cmyk`, and the expressions `name`,
//! `name!p`, `a!p!b` and chains, mixed in the model of the first colour as
//! xcolor does. Anything else resolves to `None` (reported by the caller).
//! Replace with the #150 model once it is merged.

/// A colour in its own model.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Color {
    Rgb([f64; 3]),
    Cmyk([f64; 4]),
    Gray(f64),
}

impl Color {
    fn rgb(self) -> [f64; 3] {
        match self {
            Color::Rgb(c) => c,
            Color::Gray(g) => [g, g, g],
            // xcolor `\XC@cnv@cmyk@rgb`: 1 - min(1, c + k).
            Color::Cmyk([c, m, y, k]) => [1.0 - (c + k).min(1.0), 1.0 - (m + k).min(1.0), 1.0 - (y + k).min(1.0)],
        }
    }

    /// `self` converted to the model of `like`.
    fn to_model_of(self, like: Color) -> Color {
        match like {
            Color::Rgb(_) => Color::Rgb(self.rgb()),
            Color::Gray(_) => match self {
                Color::Gray(g) => Color::Gray(g),
                // xcolor `\XC@cnv@rgb@gray`: .3r + .59g + .11b.
                other => {
                    let [r, g, b] = other.rgb();
                    Color::Gray(0.3 * r + 0.59 * g + 0.11 * b)
                }
            },
            Color::Cmyk(_) => match self {
                Color::Cmyk(c) => Color::Cmyk(c),
                Color::Gray(g) => Color::Cmyk([0.0, 0.0, 0.0, 1.0 - g]),
                Color::Rgb([r, g, b]) => {
                    let (c, m, y) = (1.0 - r, 1.0 - g, 1.0 - b);
                    let k = c.min(m).min(y);
                    Color::Cmyk([c - k, m - k, y - k, k])
                }
            },
        }
    }

    fn white_like(self) -> Color {
        match self {
            Color::Rgb(_) => Color::Rgb([1.0; 3]),
            Color::Gray(_) => Color::Gray(1.0),
            Color::Cmyk(_) => Color::Cmyk([0.0; 4]),
        }
    }

    /// `p` percent of `self`, the rest `other` (in `self`'s model).
    fn mix(self, p: f64, other: Color) -> Color {
        let t = (p / 100.0).clamp(0.0, 1.0);
        let other = other.to_model_of(self);
        let f = |a: f64, b: f64| t * a + (1.0 - t) * b;
        match (self, other) {
            (Color::Rgb(a), Color::Rgb(b)) => Color::Rgb([f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]),
            (Color::Gray(a), Color::Gray(b)) => Color::Gray(f(a, b)),
            (Color::Cmyk(a), Color::Cmyk(b)) => Color::Cmyk([f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])]),
            _ => self,
        }
    }
}

/// xcolor.sty's base colours.
fn base(name: &str) -> Option<Color> {
    Some(match name {
        "red" => Color::Rgb([1.0, 0.0, 0.0]),
        "green" => Color::Rgb([0.0, 1.0, 0.0]),
        "blue" => Color::Rgb([0.0, 0.0, 1.0]),
        "brown" => Color::Rgb([0.75, 0.5, 0.25]),
        "lime" => Color::Rgb([0.75, 1.0, 0.0]),
        "orange" => Color::Rgb([1.0, 0.5, 0.0]),
        "pink" => Color::Rgb([1.0, 0.75, 0.75]),
        "purple" => Color::Rgb([0.75, 0.0, 0.25]),
        "teal" => Color::Rgb([0.0, 0.5, 0.5]),
        "violet" => Color::Rgb([0.5, 0.0, 0.5]),
        "cyan" => Color::Cmyk([1.0, 0.0, 0.0, 0.0]),
        "magenta" => Color::Cmyk([0.0, 1.0, 0.0, 0.0]),
        "yellow" => Color::Cmyk([0.0, 0.0, 1.0, 0.0]),
        "olive" => Color::Cmyk([0.0, 0.0, 1.0, 0.5]),
        "black" => Color::Gray(0.0),
        "darkgray" => Color::Gray(0.25),
        "gray" => Color::Gray(0.5),
        "lightgray" => Color::Gray(0.75),
        "white" => Color::Gray(1.0),
        _ => return None,
    })
}

fn numbers(spec: &str) -> Vec<f64> {
    spec.split([',', ' ']).filter(|s| !s.trim().is_empty()).filter_map(|s| s.trim().parse().ok()).collect()
}

fn model_color(model: &str, spec: &str) -> Option<Color> {
    let v = numbers(spec);
    match model.trim() {
        "rgb" if v.len() == 3 => Some(Color::Rgb([v[0], v[1], v[2]])),
        "RGB" if v.len() == 3 => Some(Color::Rgb([v[0] / 255.0, v[1] / 255.0, v[2] / 255.0])),
        "gray" if v.len() == 1 => Some(Color::Gray(v[0])),
        "Gray" if v.len() == 1 => Some(Color::Gray(v[0] / 15.0)),
        "cmyk" if v.len() == 4 => Some(Color::Cmyk([v[0], v[1], v[2], v[3]])),
        "HTML" => {
            let s = spec.trim();
            if s.len() != 6 {
                return None;
            }
            let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok().map(|b| f64::from(b) / 255.0);
            Some(Color::Rgb([byte(0)?, byte(2)?, byte(4)?]))
        }
        _ => None,
    }
}

/// `\definecolor{name}{model}{spec}` definitions in `source`, in order.
pub fn definitions(source: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("\\definecolor{") {
        rest = &rest[at + "\\definecolor{".len()..];
        let mut args = vec![];
        let mut s = rest;
        for i in 0..3 {
            if i > 0 {
                s = s.trim_start();
                let Some(r) = s.strip_prefix('{') else { break };
                s = r;
            }
            let Some(end) = s.find('}') else { break };
            args.push(s[..end].trim().to_string());
            s = &s[end + 1..];
        }
        if let [name, model, spec] = args.as_slice() {
            out.push((name.clone(), model.clone(), spec.clone()));
        }
    }
    out
}

fn named(name: &str, defined: &[(String, String, String)]) -> Option<Color> {
    if let Some((_, model, spec)) = defined.iter().rev().find(|(n, ..)| n == name) {
        return model_color(model, spec);
    }
    base(name)
}

/// sRGB of `[model]{spec}`.
pub fn resolve(model: Option<&str>, spec: &str, defined: &[(String, String, String)]) -> Option<[f64; 3]> {
    if let Some(model) = model {
        return model_color(model, spec).map(Color::rgb);
    }
    let mut parts = spec.split('!').map(str::trim);
    let mut color = named(parts.next()?, defined)?;
    loop {
        let Some(p) = parts.next() else { break };
        let p: f64 = p.parse().ok()?;
        let other = match parts.next() {
            Some(name) => named(name, defined)?,
            None => color.white_like(),
        };
        color = color.mix(p, other);
    }
    Some(color.rgb())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xcolor_expressions() {
        let close = |a: [f64; 3], b: [f64; 3]| assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9), "{a:?} != {b:?}");
        close(resolve(None, "gray!20", &[]).unwrap(), [0.9; 3]);
        close(resolve(None, "red!50!blue", &[]).unwrap(), [0.5, 0.0, 0.5]);
        close(resolve(Some("HTML"), "FF8000", &[]).unwrap(), [1.0, 128.0 / 255.0, 0.0]);
        let defined = definitions("\\definecolor{lg}{gray}{0.9} x");
        close(resolve(None, "lg", &defined).unwrap(), [0.9; 3]);
        assert!(resolve(None, "nosuch", &[]).is_none());
    }
}
