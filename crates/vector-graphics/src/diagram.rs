//! Diagram geometry helpers: arrows, polylines, circles and ellipses as
//! Bézier approximations, text anchor boxes (position and size only; no
//! glyphs), and grids. Everything returns plain [`Path`]s or [`Rect`]s in the
//! caller's coordinate space.

use crate::geom::{Point, Rect, Size};
use crate::path::Path;

/// Control-point distance for a quarter circle of radius 1 approximated by a
/// cubic Bézier; radial error is at most ~2.7e-4 of the radius.
pub const KAPPA: f64 = 0.552_284_749_830_793_4;

/// A circle of four cubic segments, starting at the rightmost point and
/// going through the bottom (positive y) first.
pub fn circle(center: Point, radius: f64) -> Path {
    ellipse(center, radius, radius)
}

/// An axis-aligned ellipse of four cubic segments.
pub fn ellipse(center: Point, rx: f64, ry: f64) -> Path {
    let (cx, cy) = (center.x, center.y);
    let kx = KAPPA * rx;
    let ky = KAPPA * ry;
    let mut p = Path::new();
    p.move_to(Point::new(cx + rx, cy))
        .cubic_to(
            Point::new(cx + rx, cy + ky),
            Point::new(cx + kx, cy + ry),
            Point::new(cx, cy + ry),
        )
        .cubic_to(
            Point::new(cx - kx, cy + ry),
            Point::new(cx - rx, cy + ky),
            Point::new(cx - rx, cy),
        )
        .cubic_to(
            Point::new(cx - rx, cy - ky),
            Point::new(cx - kx, cy - ry),
            Point::new(cx, cy - ry),
        )
        .cubic_to(
            Point::new(cx + kx, cy - ry),
            Point::new(cx + rx, cy - ky),
            Point::new(cx + rx, cy),
        )
        .close();
    p
}

/// An open or closed polyline through `points`. Fewer than one point gives an
/// empty path.
pub fn polyline(points: &[Point], closed: bool) -> Path {
    let mut p = Path::new();
    let mut iter = points.iter();
    if let Some(first) = iter.next() {
        p.move_to(*first);
        for pt in iter {
            p.line_to(*pt);
        }
        if closed {
            p.close();
        }
    }
    p
}

/// Arrow head style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowHead {
    /// Length along the shaft.
    pub length: f64,
    /// Full width across the shaft.
    pub width: f64,
    /// `true` for a closed triangle to fill; `false` for an open `>` to stroke.
    pub filled: bool,
}

impl Default for ArrowHead {
    fn default() -> Self {
        ArrowHead {
            length: 8.0,
            width: 6.0,
            filled: true,
        }
    }
}

/// The two paths of an arrow: the shaft (to stroke) and the head (to fill or
/// stroke according to [`ArrowHead::filled`]). The shaft stops at the base of
/// a filled head so the stroke does not poke through the tip.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrow {
    pub shaft: Path,
    pub head: Path,
}

/// A straight arrow from `from` to `to`. A zero-length arrow yields an empty
/// head and a degenerate shaft.
pub fn arrow(from: Point, to: Point, head: ArrowHead) -> Arrow {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len == 0.0 {
        return Arrow {
            shaft: polyline(&[from, to], false),
            head: Path::new(),
        };
    }
    let ux = dx / len;
    let uy = dy / len;
    let base = Point::new(to.x - ux * head.length, to.y - uy * head.length);
    let half = head.width / 2.0;
    let left = Point::new(base.x - uy * half, base.y + ux * half);
    let right = Point::new(base.x + uy * half, base.y - ux * half);
    let shaft_end = if head.filled { base } else { to };
    let mut head_path = Path::new();
    if head.filled {
        head_path.move_to(to).line_to(left).line_to(right).close();
    } else {
        head_path.move_to(left).line_to(to).line_to(right);
    }
    Arrow {
        shaft: polyline(&[from, shaft_end], false),
        head: head_path,
    }
}

/// Where a text box's anchor point sits relative to the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
    /// Left end of the baseline; `ascent` is the distance above it.
    BaselineLeft {
        ascent_pt: u32,
    },
}

/// The rectangle a text box of `size` occupies when anchored at `position`.
/// Glyph placement is a consumer concern; this fixes only the box.
pub fn text_anchor_box(position: Point, size: Size, anchor: Anchor) -> Rect {
    let (w, h) = (size.width, size.height);
    let (dx, dy) = match anchor {
        Anchor::TopLeft => (0.0, 0.0),
        Anchor::Top => (w / 2.0, 0.0),
        Anchor::TopRight => (w, 0.0),
        Anchor::Left => (0.0, h / 2.0),
        Anchor::Center => (w / 2.0, h / 2.0),
        Anchor::Right => (w, h / 2.0),
        Anchor::BottomLeft => (0.0, h),
        Anchor::Bottom => (w / 2.0, h),
        Anchor::BottomRight => (w, h),
        Anchor::BaselineLeft { ascent_pt } => (0.0, ascent_pt as f64),
    };
    Rect::new(position.x - dx, position.y - dy, w, h)
}

/// A grid of `columns × rows` cells over `area`, as one path of open line
/// segments (including the outer edges). Zero columns or rows gives an empty
/// path.
pub fn grid(area: Rect, columns: u32, rows: u32) -> Path {
    let mut p = Path::new();
    if columns == 0 || rows == 0 {
        return p;
    }
    let area = area.normalized();
    for i in 0..=columns {
        let x = area.x + area.width * i as f64 / columns as f64;
        p.move_to(Point::new(x, area.min_y()))
            .line_to(Point::new(x, area.max_y()));
    }
    for j in 0..=rows {
        let y = area.y + area.height * j as f64 / rows as f64;
        p.move_to(Point::new(area.min_x(), y))
            .line_to(Point::new(area.max_x(), y));
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::FillRule;

    #[test]
    fn arrow_head_geometry() {
        let a = arrow(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            ArrowHead {
                length: 10.0,
                width: 6.0,
                filled: true,
            },
        );
        let hb = a.head.bounds().unwrap();
        assert_eq!(hb, Rect::new(90.0, -3.0, 10.0, 6.0));
        assert!(
            a.head
                .contains(Point::new(95.0, 0.0), FillRule::NonZero, 0.01)
        );
        let sb = a.shaft.bounds().unwrap();
        assert_eq!(sb.max_x(), 90.0);
    }

    #[test]
    fn anchor_boxes() {
        let s = Size::new(20.0, 10.0);
        assert_eq!(
            text_anchor_box(Point::new(50.0, 50.0), s, Anchor::Center),
            Rect::new(40.0, 45.0, 20.0, 10.0)
        );
        assert_eq!(
            text_anchor_box(Point::new(50.0, 50.0), s, Anchor::BottomRight),
            Rect::new(30.0, 40.0, 20.0, 10.0)
        );
        assert_eq!(
            text_anchor_box(
                Point::new(0.0, 100.0),
                s,
                Anchor::BaselineLeft { ascent_pt: 8 }
            ),
            Rect::new(0.0, 92.0, 20.0, 10.0)
        );
    }

    #[test]
    fn grid_has_expected_lines() {
        let g = grid(Rect::new(0.0, 0.0, 30.0, 20.0), 3, 2);
        // (3+1) + (2+1) lines, each MoveTo + LineTo.
        assert_eq!(g.commands().len(), 2 * (4 + 3));
        assert_eq!(g.bounds().unwrap(), Rect::new(0.0, 0.0, 30.0, 20.0));
    }
}
