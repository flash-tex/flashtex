//! PDF content-stream fragment generation (proposal; not part of runtime-v1).
//!
//! The output is a string of content-stream operators plus the resource
//! names it references. It never builds PDF objects: `crates/pdf` (or any
//! other writer) owns the page, its `/Resources` dictionary, and the image
//! XObjects. The writer must:
//!
//! - add one `/ExtGState` entry per [`ExtGState`] (see
//!   [`ExtGState::dictionary`]) under the returned name;
//! - add one image XObject per [`ImageResource`] under the returned name;
//! - append `content` to the page stream, ideally wrapped in `q … Q`, since
//!   top-level colour and line-width changes are not undone.
//!
//! Coordinate handling: the display list is top-left/y-down; PDF is
//! bottom-left/y-up. The flip `F = [1 0 0 -1 0 H]` (with `H` the page height)
//! is applied to every leaf coordinate, and each group's transform `T` is
//! emitted as `F·T·F⁻¹` so nested groups compose correctly without any
//! per-level height bookkeeping. Rules therefore come out exactly as
//! `crates/pdf` writes its fraction bars: `x (H - y - h) w h re f`.
//!
//! Numbers use `num` (three decimals, trailing zeros trimmed, `-0` → `0`),
//! the same formatting `crates/pdf` uses; matrix entries use six decimals.
//!
//! Group opacity becomes a per-leaf effective alpha via `gs`; it is not a PDF
//! transparency group, so overlapping children of a translucent group show
//! through each other. Dashes, caps, and joins are emitted only when they
//! differ from PDF's initial graphics state (butt, miter, limit 10, solid).

use crate::clip::Clip;
use crate::color::Color;
use crate::display_list::{DisplayList, ValidationError};
use crate::geom::{Point, Transform};
use crate::item::{Group, Image, Item, PathFill, PathStroke, Rule};
use crate::path::{Dash, FillRule, LineCap, LineJoin, Path, PathCommand, StrokeStyle};
use std::fmt::Write as _;

/// An `/ExtGState` the fragment references by name.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtGState {
    /// Resource name without the leading slash, e.g. `GS0`.
    pub name: String,
    /// Straight alpha used for both `ca` (fill) and `CA` (stroke).
    pub alpha: f64,
}

impl ExtGState {
    /// The dictionary the writer should register under `name`.
    pub fn dictionary(&self) -> String {
        format!(
            "<< /Type /ExtGState /ca {} /CA {} >>",
            num(self.alpha),
            num(self.alpha)
        )
    }
}

/// An image XObject the fragment references by name.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageResource {
    /// Resource name without the leading slash, e.g. `Im0`.
    pub name: String,
    pub content_hash: String,
}

/// Generated content and the resources it requires.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PdfFragment {
    pub content: String,
    pub ext_g_states: Vec<ExtGState>,
    pub images: Vec<ImageResource>,
}

/// Generates the content-stream fragment for a whole display list.
pub fn content_stream(list: &DisplayList) -> Result<PdfFragment, ValidationError> {
    if let Some(e) = list.validate().into_iter().next() {
        return Err(e);
    }
    let flip = Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, list.page_size.height);
    let mut w = Writer {
        out: String::new(),
        flip,
        states: vec![GState::default()],
        ext: Vec::new(),
        images: Vec::new(),
    };
    w.items(&list.items, 1.0);
    Ok(PdfFragment {
        content: w.out,
        ext_g_states: w.ext,
        images: w.images,
    })
}

/// Formats a coordinate the way `crates/pdf` does: at most three decimals,
/// trailing zeros trimmed, never `-0`.
pub fn num(v: f64) -> String {
    trim(format!("{v:.3}"))
}

/// Six-decimal formatting for matrix coefficients.
pub fn num_matrix(v: f64) -> String {
    trim(format!("{v:.6}"))
}

fn trim(s: String) -> String {
    let t = s.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" {
        "0".to_string()
    } else {
        t.to_string()
    }
}

#[derive(Clone, Debug)]
struct GState {
    fill: Option<Color>,
    stroke: Option<Color>,
    width: Option<f64>,
    cap: LineCap,
    join: LineJoin,
    miter: f64,
    dash: Option<Dash>,
    alpha: f64,
}

impl Default for GState {
    fn default() -> Self {
        GState {
            fill: None,
            stroke: None,
            width: None,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter: 10.0,
            dash: None,
            alpha: 1.0,
        }
    }
}

struct Writer {
    out: String,
    flip: Transform,
    states: Vec<GState>,
    ext: Vec<ExtGState>,
    images: Vec<ImageResource>,
}

impl Writer {
    fn state(&mut self) -> &mut GState {
        self.states.last_mut().expect("state stack is never empty")
    }

    fn items(&mut self, items: &[Item], alpha: f64) {
        for item in items {
            match item {
                Item::Rule(r) => self.rule(r, alpha),
                Item::PathFill(p) => self.fill(p, alpha),
                Item::PathStroke(p) => self.stroke(p, alpha),
                Item::Image(i) => self.image(i, alpha),
                Item::Group(g) => self.group(g, alpha),
            }
        }
    }

    fn push(&mut self) {
        self.out.push_str("q\n");
        let top = self.state().clone();
        self.states.push(top);
    }

    fn pop(&mut self) {
        self.out.push_str("Q\n");
        self.states.pop();
    }

    fn group(&mut self, g: &Group, alpha: f64) {
        self.push();
        if !g.transform.is_identity() {
            let m = self.flip.then(&g.transform).then(&self.flip);
            self.cm(&m);
        }
        if let Some(clip) = &g.clip {
            match clip {
                Clip::Rect(r) => {
                    let r = r.normalized();
                    let _ = writeln!(
                        self.out,
                        "{} {} {} {} re W n",
                        num(r.x),
                        num(self.flip.f - r.y - r.height),
                        num(r.width),
                        num(r.height)
                    );
                }
                Clip::Path { path, rule } => {
                    self.path(path);
                    self.out.push_str(match rule {
                        FillRule::NonZero => "W n\n",
                        FillRule::EvenOdd => "W* n\n",
                    });
                }
            }
        }
        self.items(&g.items, alpha * g.opacity);
        self.pop();
    }

    fn cm(&mut self, m: &Transform) {
        let c = m.coefficients().map(num_matrix);
        let _ = writeln!(
            self.out,
            "{} {} {} {} {} {} cm",
            c[0], c[1], c[2], c[3], c[4], c[5]
        );
    }

    fn set_alpha(&mut self, alpha: f64) {
        let alpha = alpha.clamp(0.0, 1.0);
        if (self.state().alpha - alpha).abs() < 1e-9 {
            return;
        }
        let key = num(alpha);
        let name = match self.ext.iter().find(|e| num(e.alpha) == key) {
            Some(e) => e.name.clone(),
            None => {
                let name = format!("GS{}", self.ext.len());
                self.ext.push(ExtGState {
                    name: name.clone(),
                    alpha,
                });
                name
            }
        };
        let _ = writeln!(self.out, "/{name} gs");
        self.state().alpha = alpha;
    }

    fn set_fill(&mut self, color: Color) {
        let color = color.clamped();
        if self.state().fill == Some(color) {
            return;
        }
        self.out.push_str(&color_op(color, false));
        self.state().fill = Some(color);
    }

    fn set_stroke(&mut self, color: Color) {
        let color = color.clamped();
        if self.state().stroke == Some(color) {
            return;
        }
        self.out.push_str(&color_op(color, true));
        self.state().stroke = Some(color);
    }

    fn set_stroke_style(&mut self, s: &StrokeStyle) {
        if self.state().width != Some(s.width) {
            let _ = writeln!(self.out, "{} w", num(s.width));
            self.state().width = Some(s.width);
        }
        if self.state().cap != s.cap {
            let _ = writeln!(
                self.out,
                "{} J",
                match s.cap {
                    LineCap::Butt => 0,
                    LineCap::Round => 1,
                    LineCap::Square => 2,
                }
            );
            self.state().cap = s.cap;
        }
        if self.state().join != s.join {
            let _ = writeln!(
                self.out,
                "{} j",
                match s.join {
                    LineJoin::Miter => 0,
                    LineJoin::Round => 1,
                    LineJoin::Bevel => 2,
                }
            );
            self.state().join = s.join;
        }
        if self.state().miter != s.miter_limit {
            let _ = writeln!(self.out, "{} M", num(s.miter_limit));
            self.state().miter = s.miter_limit;
        }
        if self.state().dash != s.dash {
            match &s.dash {
                Some(d) => {
                    let parts: Vec<String> = d.array.iter().map(|v| num(*v)).collect();
                    let _ = writeln!(self.out, "[{}] {} d", parts.join(" "), num(d.phase));
                }
                None => self.out.push_str("[] 0 d\n"),
            }
            self.state().dash = s.dash.clone();
        }
    }

    fn rule(&mut self, r: &Rule, alpha: f64) {
        let paint = r.paint.clamped();
        self.set_alpha(paint.alpha * alpha);
        self.set_fill(paint.color);
        let rect = r.rect.normalized();
        let _ = writeln!(
            self.out,
            "{} {} {} {} re f",
            num(rect.x),
            num(self.flip.f - rect.y - rect.height),
            num(rect.width),
            num(rect.height)
        );
    }

    fn fill(&mut self, p: &PathFill, alpha: f64) {
        if p.path.is_empty() {
            return;
        }
        let paint = p.paint.clamped();
        self.set_alpha(paint.alpha * alpha);
        self.set_fill(paint.color);
        self.path(&p.path);
        self.out.push_str(match p.rule {
            FillRule::NonZero => "f\n",
            FillRule::EvenOdd => "f*\n",
        });
    }

    fn stroke(&mut self, p: &PathStroke, alpha: f64) {
        if p.path.is_empty() {
            return;
        }
        let paint = p.paint.clamped();
        self.set_alpha(paint.alpha * alpha);
        self.set_stroke(paint.color);
        self.set_stroke_style(&p.style);
        self.path(&p.path);
        self.out.push_str("S\n");
    }

    fn image(&mut self, i: &Image, alpha: f64) {
        let name = match self
            .images
            .iter()
            .find(|r| r.content_hash == i.content_hash)
        {
            Some(r) => r.name.clone(),
            None => {
                let name = format!("Im{}", self.images.len());
                self.images.push(ImageResource {
                    name: name.clone(),
                    content_hash: i.content_hash.clone(),
                });
                name
            }
        };
        self.push();
        self.set_alpha(i.alpha.clamp(0.0, 1.0) * alpha);
        // Unit square (row 0 at v = 1) -> image box (row 0 at y = 0) -> local -> flipped local.
        let unit_to_box = Transform::new(i.width_pt, 0.0, 0.0, -i.height_pt, 0.0, i.height_pt);
        let m = unit_to_box.then(&i.transform).then(&self.flip);
        self.cm(&m);
        let _ = writeln!(self.out, "/{name} Do");
        self.pop();
    }

    fn path(&mut self, path: &Path) {
        let f = self.flip;
        let pt = |p: Point| {
            let q = f.apply(p);
            format!("{} {}", num(q.x), num(q.y))
        };
        let mut current = Point::ZERO;
        let mut start = Point::ZERO;
        for c in path.commands() {
            match *c {
                PathCommand::MoveTo(p) => {
                    let _ = writeln!(self.out, "{} m", pt(p));
                    current = p;
                    start = p;
                }
                PathCommand::LineTo(p) => {
                    let _ = writeln!(self.out, "{} l", pt(p));
                    current = p;
                }
                PathCommand::QuadTo(q, p) => {
                    // Degree elevation: exact cubic equivalent.
                    let c1 = current.lerp(q, 2.0 / 3.0);
                    let c2 = p.lerp(q, 2.0 / 3.0);
                    let _ = writeln!(self.out, "{} {} {} c", pt(c1), pt(c2), pt(p));
                    current = p;
                }
                PathCommand::CubicTo(a, b, p) => {
                    let _ = writeln!(self.out, "{} {} {} c", pt(a), pt(b), pt(p));
                    current = p;
                }
                PathCommand::Close => {
                    self.out.push_str("h\n");
                    current = start;
                }
            }
        }
    }
}

fn color_op(color: Color, stroking: bool) -> String {
    match (color, stroking) {
        (Color::Gray(g), false) => format!("{} g\n", num(g)),
        (Color::Gray(g), true) => format!("{} G\n", num(g)),
        (Color::Rgb(r, g, b), false) => format!("{} {} {} rg\n", num(r), num(g), num(b)),
        (Color::Rgb(r, g, b), true) => format!("{} {} {} RG\n", num(r), num(g), num(b)),
        (Color::Cmyk(c, m, y, k), false) => {
            format!("{} {} {} {} k\n", num(c), num(m), num(y), num(k))
        }
        (Color::Cmyk(c, m, y, k), true) => {
            format!("{} {} {} {} K\n", num(c), num(m), num(y), num(k))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_match_pdf_crate_formatting() {
        assert_eq!(num(612.0), "612");
        assert_eq!(num(708.5), "708.5");
        assert_eq!(num(0.1 + 0.2), "0.3");
        assert_eq!(num(-0.0001), "0");
        assert_eq!(num(1e-7), "0");
        assert_eq!(num_matrix(std::f64::consts::FRAC_1_SQRT_2), "0.707107");
        assert_eq!(num_matrix(-0.0), "0");
    }
}
