//! Paths made of lines and Bézier curves, with exact bounds, flattening with a
//! guaranteed tolerance, containment tests, and stroke parameters.

use crate::geom::{Point, Rect, Transform};

/// One path command. Coordinates are absolute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    /// Quadratic Bézier: control point, end point.
    QuadTo(Point, Point),
    /// Cubic Bézier: first control, second control, end point.
    CubicTo(Point, Point, Point),
    /// Closes the current subpath with a straight line to its start.
    Close,
}

/// How the inside of a path is decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// Shape of stroke ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// Shape of stroke corners.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// A dash pattern: alternating on/off lengths in user units plus a phase.
#[derive(Clone, Debug, PartialEq)]
pub struct Dash {
    pub array: Vec<f64>,
    pub phase: f64,
}

/// Stroke parameters. Widths are in the path's own coordinate space.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    pub dash: Option<Dash>,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        StrokeStyle {
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash: None,
        }
    }
}

impl StrokeStyle {
    pub fn with_width(width: f64) -> StrokeStyle {
        StrokeStyle {
            width,
            ..StrokeStyle::default()
        }
    }

    /// Conservative distance a stroke can extend beyond its centre line: half
    /// the width, scaled by the miter limit for miter joins and by √2 for
    /// square caps (a square cap corner on a diagonal segment).
    pub fn outset(&self) -> f64 {
        let half = self.width.abs() / 2.0;
        let join = if self.join == LineJoin::Miter {
            self.miter_limit.max(1.0)
        } else {
            1.0
        };
        let cap = if self.cap == LineCap::Square {
            std::f64::consts::SQRT_2
        } else {
            1.0
        };
        half * join.max(cap)
    }
}

/// A flattened subpath.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
}

/// A sequence of subpaths.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Path {
    commands: Vec<PathCommand>,
}

impl Path {
    pub fn new() -> Path {
        Path::default()
    }

    pub fn from_commands(commands: Vec<PathCommand>) -> Path {
        Path { commands }
    }

    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn move_to(&mut self, p: Point) -> &mut Path {
        self.commands.push(PathCommand::MoveTo(p));
        self
    }

    pub fn line_to(&mut self, p: Point) -> &mut Path {
        self.commands.push(PathCommand::LineTo(p));
        self
    }

    pub fn quad_to(&mut self, c: Point, p: Point) -> &mut Path {
        self.commands.push(PathCommand::QuadTo(c, p));
        self
    }

    pub fn cubic_to(&mut self, c1: Point, c2: Point, p: Point) -> &mut Path {
        self.commands.push(PathCommand::CubicTo(c1, c2, p));
        self
    }

    pub fn close(&mut self) -> &mut Path {
        self.commands.push(PathCommand::Close);
        self
    }

    /// A closed rectangle subpath (clockwise on screen: top-left, top-right,
    /// bottom-right, bottom-left).
    pub fn rect(r: Rect) -> Path {
        let c = r.corners();
        let mut p = Path::new();
        p.move_to(c[0])
            .line_to(c[1])
            .line_to(c[2])
            .line_to(c[3])
            .close();
        p
    }

    /// Appends all of `other`'s commands.
    pub fn append(&mut self, other: &Path) -> &mut Path {
        self.commands.extend_from_slice(&other.commands);
        self
    }

    pub fn is_finite(&self) -> bool {
        self.commands.iter().all(|c| match c {
            PathCommand::MoveTo(p) | PathCommand::LineTo(p) => p.is_finite(),
            PathCommand::QuadTo(a, p) => a.is_finite() && p.is_finite(),
            PathCommand::CubicTo(a, b, p) => a.is_finite() && b.is_finite() && p.is_finite(),
            PathCommand::Close => true,
        })
    }

    /// Applies `t` to every point.
    pub fn transformed(&self, t: &Transform) -> Path {
        let commands = self
            .commands
            .iter()
            .map(|c| match *c {
                PathCommand::MoveTo(p) => PathCommand::MoveTo(t.apply(p)),
                PathCommand::LineTo(p) => PathCommand::LineTo(t.apply(p)),
                PathCommand::QuadTo(a, p) => PathCommand::QuadTo(t.apply(a), t.apply(p)),
                PathCommand::CubicTo(a, b, p) => {
                    PathCommand::CubicTo(t.apply(a), t.apply(b), t.apply(p))
                }
                PathCommand::Close => PathCommand::Close,
            })
            .collect();
        Path { commands }
    }

    /// Exact axis-aligned bounds of the curve itself (control points that lie
    /// outside the curve do not enlarge it). `None` for a path with no points.
    pub fn bounds(&self) -> Option<Rect> {
        let mut acc: Option<Rect> = None;
        let mut include = |p: Point| {
            let r = Rect::from_points(p, p);
            acc = Some(match acc {
                Some(a) => a.union(&r),
                None => r,
            });
        };
        let mut current = Point::ZERO;
        let mut start = Point::ZERO;
        for c in &self.commands {
            match *c {
                PathCommand::MoveTo(p) => {
                    include(p);
                    current = p;
                    start = p;
                }
                PathCommand::LineTo(p) => {
                    include(p);
                    current = p;
                }
                PathCommand::QuadTo(a, p) => {
                    include(p);
                    for t in quad_extrema(current.x, a.x, p.x)
                        .into_iter()
                        .chain(quad_extrema(current.y, a.y, p.y))
                    {
                        include(quad_point(current, a, p, t));
                    }
                    current = p;
                }
                PathCommand::CubicTo(a, b, p) => {
                    include(p);
                    for t in cubic_extrema(current.x, a.x, b.x, p.x)
                        .into_iter()
                        .chain(cubic_extrema(current.y, a.y, b.y, p.y))
                    {
                        include(cubic_point(current, a, b, p, t));
                    }
                    current = p;
                }
                PathCommand::Close => current = start,
            }
        }
        acc
    }

    /// Flattens curves into line segments so that no point on a curve is
    /// further than `tolerance` from the polyline. Uses uniform subdivision
    /// with the analytic second-difference bound, so the count is
    /// deterministic for a given input.
    pub fn flatten(&self, tolerance: f64) -> Vec<Polyline> {
        let tolerance = if tolerance > 0.0 && tolerance.is_finite() {
            tolerance
        } else {
            0.25
        };
        let mut out: Vec<Polyline> = Vec::new();
        let mut current = Point::ZERO;
        let mut start = Point::ZERO;
        let mut open: Option<Polyline> = None;
        let flush = |open: &mut Option<Polyline>, out: &mut Vec<Polyline>| {
            if let Some(pl) = open.take()
                && !pl.points.is_empty()
            {
                out.push(pl);
            }
        };
        for c in &self.commands {
            match *c {
                PathCommand::MoveTo(p) => {
                    flush(&mut open, &mut out);
                    open = Some(Polyline {
                        points: vec![p],
                        closed: false,
                    });
                    current = p;
                    start = p;
                }
                PathCommand::LineTo(p) => {
                    let pl = open.get_or_insert_with(|| Polyline {
                        points: vec![current],
                        closed: false,
                    });
                    pl.points.push(p);
                    current = p;
                }
                PathCommand::QuadTo(a, p) => {
                    let pl = open.get_or_insert_with(|| Polyline {
                        points: vec![current],
                        closed: false,
                    });
                    let n = quad_segment_count(current, a, p, tolerance);
                    for i in 1..=n {
                        pl.points
                            .push(quad_point(current, a, p, i as f64 / n as f64));
                    }
                    current = p;
                }
                PathCommand::CubicTo(a, b, p) => {
                    let pl = open.get_or_insert_with(|| Polyline {
                        points: vec![current],
                        closed: false,
                    });
                    let n = cubic_segment_count(current, a, b, p, tolerance);
                    for i in 1..=n {
                        pl.points
                            .push(cubic_point(current, a, b, p, i as f64 / n as f64));
                    }
                    current = p;
                }
                PathCommand::Close => {
                    if let Some(pl) = open.as_mut() {
                        pl.closed = true;
                    }
                    flush(&mut open, &mut out);
                    current = start;
                    // A drawing command after Close starts at the subpath start.
                }
            }
        }
        flush(&mut open, &mut out);
        out
    }

    /// Whether `p` is inside the filled path under `rule`, evaluated on the
    /// flattened outline (every subpath is treated as closed, as PDF does).
    pub fn contains(&self, p: Point, rule: FillRule, tolerance: f64) -> bool {
        polylines_contain(&self.flatten(tolerance), p, rule)
    }

    /// Shortest distance from `p` to the flattened outline (open or closed).
    pub fn distance_to_outline(&self, p: Point, tolerance: f64) -> f64 {
        let mut best = f64::INFINITY;
        for pl in self.flatten(tolerance) {
            let n = pl.points.len();
            if n == 1 {
                best = best.min(p.distance(pl.points[0]));
            }
            for i in 1..n {
                best = best.min(p.distance_to_segment(pl.points[i - 1], pl.points[i]));
            }
            if pl.closed && n > 1 {
                best = best.min(p.distance_to_segment(pl.points[n - 1], pl.points[0]));
            }
        }
        best
    }
}

/// Winding-based containment over flattened subpaths. Subpaths are implicitly
/// closed for filling.
pub fn polylines_contain(polylines: &[Polyline], p: Point, rule: FillRule) -> bool {
    let mut winding: i32 = 0;
    for pl in polylines {
        let n = pl.points.len();
        if n < 2 {
            continue;
        }
        for i in 0..n {
            let a = pl.points[i];
            let b = pl.points[(i + 1) % n];
            // Upward or downward crossing of the horizontal ray to +x.
            if a.y <= p.y {
                if b.y > p.y && cross(a, b, p) > 0.0 {
                    winding += 1;
                }
            } else if b.y <= p.y && cross(a, b, p) < 0.0 {
                winding -= 1;
            }
        }
    }
    match rule {
        FillRule::NonZero => winding != 0,
        FillRule::EvenOdd => winding % 2 != 0,
    }
}

fn cross(a: Point, b: Point, p: Point) -> f64 {
    (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y)
}

pub(crate) fn quad_point(p0: Point, p1: Point, p2: Point, t: f64) -> Point {
    let mt = 1.0 - t;
    Point::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

pub(crate) fn cubic_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let mt = 1.0 - t;
    let a = mt * mt * mt;
    let b = 3.0 * mt * mt * t;
    let c = 3.0 * mt * t * t;
    let d = t * t * t;
    Point::new(
        a * p0.x + b * p1.x + c * p2.x + d * p3.x,
        a * p0.y + b * p1.y + c * p2.y + d * p3.y,
    )
}

/// Segment count so that the uniform polyline stays within `tol` of a cubic.
/// With `L = max |second difference|`, `|B''| ≤ 6L` and the chord error of
/// `n` segments is at most `|B''| / (8 n²)`, so `n = ceil(sqrt(0.75 L / tol))`.
fn cubic_segment_count(p0: Point, p1: Point, p2: Point, p3: Point, tol: f64) -> usize {
    let d1 = Point::new(p0.x - 2.0 * p1.x + p2.x, p0.y - 2.0 * p1.y + p2.y);
    let d2 = Point::new(p1.x - 2.0 * p2.x + p3.x, p1.y - 2.0 * p2.y + p3.y);
    let l = d1.distance(Point::ZERO).max(d2.distance(Point::ZERO));
    segment_count(0.75 * l / tol)
}

/// For a quadratic `|B''| = 2L`, so `n = ceil(sqrt(L / (4 tol)))`.
fn quad_segment_count(p0: Point, p1: Point, p2: Point, tol: f64) -> usize {
    let d = Point::new(p0.x - 2.0 * p1.x + p2.x, p0.y - 2.0 * p1.y + p2.y);
    let l = d.distance(Point::ZERO);
    segment_count(l / (4.0 * tol))
}

fn segment_count(n_squared: f64) -> usize {
    if !n_squared.is_finite() {
        return 1;
    }
    (n_squared.sqrt().ceil() as usize).clamp(1, 1 << 16)
}

/// Parameter values in (0, 1) where a quadratic's derivative vanishes.
fn quad_extrema(p0: f64, p1: f64, p2: f64) -> Vec<f64> {
    let denom = p0 - 2.0 * p1 + p2;
    if denom == 0.0 {
        return Vec::new();
    }
    let t = (p0 - p1) / denom;
    if t > 0.0 && t < 1.0 {
        vec![t]
    } else {
        Vec::new()
    }
}

/// Parameter values in (0, 1) where a cubic's derivative vanishes.
fn cubic_extrema(p0: f64, p1: f64, p2: f64, p3: f64) -> Vec<f64> {
    let a = 3.0 * (-p0 + 3.0 * p1 - 3.0 * p2 + p3);
    let b = 6.0 * (p0 - 2.0 * p1 + p2);
    let c = 3.0 * (p1 - p0);
    let mut out = Vec::new();
    let mut push = |t: f64| {
        if t > 0.0 && t < 1.0 {
            out.push(t);
        }
    };
    if a.abs() < 1e-12 {
        if b.abs() > 1e-12 {
            push(-c / b);
        }
        return out;
    }
    {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let s = disc.sqrt();
            push((-b + s) / (2.0 * a));
            push((-b - s) / (2.0 * a));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_exclude_control_points() {
        let mut p = Path::new();
        p.move_to(Point::new(0.0, 0.0)).cubic_to(
            Point::new(0.0, 100.0),
            Point::new(100.0, 100.0),
            Point::new(100.0, 0.0),
        );
        let b = p.bounds().unwrap();
        assert_eq!(b.x, 0.0);
        assert_eq!(b.width, 100.0);
        // Max y is at t = 0.5: 3/4 * 100.
        assert!((b.max_y() - 75.0).abs() < 1e-9, "{b:?}");
    }

    #[test]
    fn even_odd_versus_nonzero() {
        let mut p = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        p.append(&Path::rect(Rect::new(2.0, 2.0, 6.0, 6.0)));
        let inner = Point::new(5.0, 5.0);
        assert!(p.contains(inner, FillRule::NonZero, 0.1));
        assert!(!p.contains(inner, FillRule::EvenOdd, 0.1));
        assert!(p.contains(Point::new(1.0, 1.0), FillRule::EvenOdd, 0.1));
        assert!(!p.contains(Point::new(11.0, 1.0), FillRule::NonZero, 0.1));
    }

    #[test]
    fn flatten_keeps_closed_flag_and_starts() {
        let mut p = Path::rect(Rect::new(0.0, 0.0, 1.0, 1.0));
        p.line_to(Point::new(5.0, 5.0));
        let pl = p.flatten(0.1);
        assert_eq!(pl.len(), 2);
        assert!(pl[0].closed);
        assert_eq!(pl[1].points[0], Point::ZERO);
        assert!(!pl[1].closed);
    }
}
