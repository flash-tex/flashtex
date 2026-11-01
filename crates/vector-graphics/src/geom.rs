//! Points, sizes, rectangles, and affine transforms.
//!
//! All coordinates are PostScript points with the origin at the top-left of the
//! page and `y` growing downwards, matching runtime-v1 text items. The PDF
//! serializer is the only place that flips to PDF's bottom-left convention.

use std::fmt;

/// A point in points (pt). `y` grows downwards.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn distance(self, other: Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn lerp(self, other: Point, t: f64) -> Point {
        Point::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }

    /// Distance from this point to the segment `a`–`b`.
    pub fn distance_to_segment(self, a: Point, b: Point) -> f64 {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len2 = dx * dx + dy * dy;
        if len2 == 0.0 {
            return self.distance(a);
        }
        let t = (((self.x - a.x) * dx + (self.y - a.y) * dy) / len2).clamp(0.0, 1.0);
        self.distance(Point::new(a.x + t * dx, a.y + t * dy))
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// A width/height pair in points.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub const fn new(width: f64, height: f64) -> Size {
        Size { width, height }
    }
}

/// An axis-aligned rectangle with a top-left origin. Normalised rectangles have
/// non-negative width and height; use [`Rect::normalized`] for user input.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// The smallest rectangle containing both points.
    pub fn from_points(a: Point, b: Point) -> Rect {
        let x0 = a.x.min(b.x);
        let y0 = a.y.min(b.y);
        Rect::new(x0, y0, a.x.max(b.x) - x0, a.y.max(b.y) - y0)
    }

    /// Flips negative extents so the origin is the top-left corner.
    pub fn normalized(self) -> Rect {
        Rect::from_points(
            Point::new(self.x, self.y),
            Point::new(self.x + self.width, self.y + self.height),
        )
    }

    pub fn min_x(&self) -> f64 {
        self.x
    }
    pub fn min_y(&self) -> f64 {
        self.y
    }
    pub fn max_x(&self) -> f64 {
        self.x + self.width
    }
    pub fn max_y(&self) -> f64 {
        self.y + self.height
    }
    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// True when the rectangle encloses no area.
    pub fn is_empty(&self) -> bool {
        !(self.width > 0.0 && self.height > 0.0)
    }

    pub fn is_finite(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
    }

    /// Closed containment: edges count as inside.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min_x() && p.x <= self.max_x() && p.y >= self.min_y() && p.y <= self.max_y()
    }

    /// Intersection, or `None` when the rectangles share no area. Touching
    /// edges produce `None`.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x0 = self.min_x().max(other.min_x());
        let y0 = self.min_y().max(other.min_y());
        let x1 = self.max_x().min(other.max_x());
        let y1 = self.max_y().min(other.max_y());
        if x1 > x0 && y1 > y0 {
            Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
        } else {
            None
        }
    }

    /// Smallest rectangle containing both.
    pub fn union(&self, other: &Rect) -> Rect {
        let x0 = self.min_x().min(other.min_x());
        let y0 = self.min_y().min(other.min_y());
        let x1 = self.max_x().max(other.max_x());
        let y1 = self.max_y().max(other.max_y());
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// Grows every edge outward by `amount` (negative shrinks).
    pub fn expand(&self, amount: f64) -> Rect {
        Rect::new(
            self.x - amount,
            self.y - amount,
            self.width + 2.0 * amount,
            self.height + 2.0 * amount,
        )
    }

    /// Corners in top-left, top-right, bottom-right, bottom-left order.
    pub fn corners(&self) -> [Point; 4] {
        [
            Point::new(self.min_x(), self.min_y()),
            Point::new(self.max_x(), self.min_y()),
            Point::new(self.max_x(), self.max_y()),
            Point::new(self.min_x(), self.max_y()),
        ]
    }

    /// Axis-aligned bounding box of the transformed corners.
    pub fn transformed_bounds(&self, t: &Transform) -> Rect {
        let c = self.corners().map(|p| t.apply(p));
        let mut r = Rect::from_points(c[0], c[1]);
        for p in &c[2..] {
            r = r.union(&Rect::from_points(*p, *p));
        }
        r
    }
}

/// An affine transform `[a b c d e f]` in the PDF convention:
/// `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
///
/// Composition follows *apply order*: `s.then(&t)` maps a point through `s`
/// first and then through `t`. This is the same order in which the six numbers
/// would be emitted as successive `cm` operators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Transform::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Transform = Transform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Transform {
        Transform { a, b, c, d, e, f }
    }

    pub const fn translate(tx: f64, ty: f64) -> Transform {
        Transform::new(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    pub const fn scale(sx: f64, sy: f64) -> Transform {
        Transform::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// Rotation by `radians` about the origin. In this crate's y-down space a
    /// positive angle turns the positive x axis towards positive y, which
    /// appears clockwise on screen.
    pub fn rotate(radians: f64) -> Transform {
        let (s, c) = radians.sin_cos();
        Transform::new(c, s, -s, c, 0.0, 0.0)
    }

    /// Skew: `x' = x + tan(ax)·y`, `y' = tan(ay)·x + y`.
    pub fn skew(ax: f64, ay: f64) -> Transform {
        Transform::new(1.0, ay.tan(), ax.tan(), 1.0, 0.0, 0.0)
    }

    /// Rotation about an arbitrary point.
    pub fn rotate_about(radians: f64, center: Point) -> Transform {
        Transform::translate(-center.x, -center.y)
            .then(&Transform::rotate(radians))
            .then(&Transform::translate(center.x, center.y))
    }

    /// `self` first, then `other`.
    pub fn then(&self, other: &Transform) -> Transform {
        let s = self;
        let o = other;
        Transform::new(
            s.a * o.a + s.b * o.c,
            s.a * o.b + s.b * o.d,
            s.c * o.a + s.d * o.c,
            s.c * o.b + s.d * o.d,
            s.e * o.a + s.f * o.c + o.e,
            s.e * o.b + s.f * o.d + o.f,
        )
    }

    /// `other` first, then `self`.
    pub fn pre(&self, other: &Transform) -> Transform {
        other.then(self)
    }

    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// The inverse, or `None` for a singular (non-invertible) transform.
    pub fn invert(&self) -> Option<Transform> {
        let det = self.determinant();
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        Some(Transform::new(
            self.d / det,
            -self.b / det,
            -self.c / det,
            self.a / det,
            (self.c * self.f - self.d * self.e) / det,
            (self.b * self.e - self.a * self.f) / det,
        ))
    }

    pub fn apply(&self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// Applies the linear part only (no translation): for directions.
    pub fn apply_vector(&self, v: Point) -> Point {
        Point::new(self.a * v.x + self.c * v.y, self.b * v.x + self.d * v.y)
    }

    pub fn is_identity(&self) -> bool {
        *self == Transform::IDENTITY
    }

    /// True when the transform maps axis-aligned rectangles to axis-aligned
    /// rectangles (no rotation or skew; flips and scales are allowed).
    pub fn is_axis_aligned(&self) -> bool {
        self.b == 0.0 && self.c == 0.0
    }

    /// Geometric mean scale factor, used to scale stroke widths after a
    /// transform has been resolved. Exact for uniform scaling and rotation.
    pub fn mean_scale(&self) -> f64 {
        self.determinant().abs().sqrt()
    }

    pub fn is_finite(&self) -> bool {
        [self.a, self.b, self.c, self.d, self.e, self.f]
            .iter()
            .all(|v| v.is_finite())
    }

    /// The six coefficients in `cm` order.
    pub fn coefficients(&self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }

    /// Element-wise closeness for tests and de-duplication.
    pub fn approx_eq(&self, other: &Transform, eps: f64) -> bool {
        self.coefficients()
            .iter()
            .zip(other.coefficients().iter())
            .all(|(x, y)| (x - y).abs() <= eps)
    }
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{} {} {} {} {} {}]",
            self.a, self.b, self.c, self.d, self.e, self.f
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_then_scale_applies_in_order() {
        let t = Transform::translate(10.0, 20.0).then(&Transform::scale(2.0, 3.0));
        let p = t.apply(Point::new(1.0, 1.0));
        assert_eq!(p, Point::new(22.0, 63.0));
        let u = Transform::scale(2.0, 3.0).then(&Transform::translate(10.0, 20.0));
        assert_eq!(u.apply(Point::new(1.0, 1.0)), Point::new(12.0, 23.0));
    }

    #[test]
    fn rect_intersections() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        assert_eq!(a.intersect(&b), Some(Rect::new(5.0, 5.0, 5.0, 5.0)));
        assert_eq!(a.intersect(&Rect::new(10.0, 0.0, 5.0, 5.0)), None);
        assert_eq!(a.union(&b), Rect::new(0.0, 0.0, 15.0, 15.0));
        assert!(Rect::new(0.0, 0.0, 0.0, 5.0).is_empty());
    }

    #[test]
    fn transformed_bounds_of_rotated_rect() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = r.transformed_bounds(&Transform::rotate(std::f64::consts::FRAC_PI_4));
        let s = 10.0 * std::f64::consts::FRAC_1_SQRT_2;
        assert!((b.x + s).abs() < 1e-9);
        assert!((b.width - 2.0 * s).abs() < 1e-9);
        assert!((b.y - 0.0).abs() < 1e-9);
        assert!((b.height - 2.0 * s).abs() < 1e-9);
    }
}
