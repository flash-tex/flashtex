//! Clip shapes and the clip stack.
//!
//! A clip stack is a conjunction: a point is visible only when it lies inside
//! every entry. Entries are stored in the coordinate space they were pushed in
//! (device space after flattening).

use crate::geom::{Point, Rect, Transform};
use crate::path::{FillRule, Path};

/// One clip shape.
#[derive(Clone, Debug, PartialEq)]
pub enum Clip {
    Rect(Rect),
    Path { path: Path, rule: FillRule },
}

impl Clip {
    pub fn path(path: Path, rule: FillRule) -> Clip {
        Clip::Path { path, rule }
    }

    /// Bounding box of the clip region (`None` for an empty clip).
    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Clip::Rect(r) => (!r.is_empty()).then_some(r.normalized()),
            Clip::Path { path, .. } => path.bounds(),
        }
    }

    pub fn contains(&self, p: Point, tolerance: f64) -> bool {
        match self {
            Clip::Rect(r) => r.normalized().contains(p),
            Clip::Path { path, rule } => path.contains(p, *rule, tolerance),
        }
    }

    /// Applies `t`. A rectangle stays a rectangle only under axis-aligned
    /// transforms; otherwise it becomes a path clip.
    pub fn transformed(&self, t: &Transform) -> Clip {
        match self {
            Clip::Rect(r) => {
                if t.is_axis_aligned() {
                    Clip::Rect(r.transformed_bounds(t))
                } else {
                    Clip::Path {
                        path: Path::rect(*r).transformed(t),
                        rule: FillRule::NonZero,
                    }
                }
            }
            Clip::Path { path, rule } => Clip::Path {
                path: path.transformed(t),
                rule: *rule,
            },
        }
    }
}

/// An ordered conjunction of clips.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ClipStack {
    clips: Vec<Clip>,
}

impl ClipStack {
    pub fn new() -> ClipStack {
        ClipStack::default()
    }

    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    pub fn clips(&self) -> &[Clip] {
        &self.clips
    }

    pub fn push(&mut self, clip: Clip) {
        self.clips.push(clip);
    }

    pub fn pushed(&self, clip: Clip) -> ClipStack {
        let mut s = self.clone();
        s.push(clip);
        s
    }

    pub fn pop(&mut self) -> Option<Clip> {
        self.clips.pop()
    }

    /// Intersection of all entries' bounding boxes. `None` means the stack
    /// admits nothing; an empty stack returns `Some(unbounded)` expressed as
    /// `None` from `bounds_or`, so use [`ClipStack::clip_rect`] with a page.
    pub fn clip_rect(&self, page: Rect) -> Option<Rect> {
        let mut acc = page;
        for c in &self.clips {
            acc = acc.intersect(&c.bounds()?)?;
        }
        Some(acc)
    }

    /// Whether `p` is inside every clip.
    pub fn contains(&self, p: Point, tolerance: f64) -> bool {
        self.clips.iter().all(|c| c.contains(p, tolerance))
    }

    pub fn transformed(&self, t: &Transform) -> ClipStack {
        ClipStack {
            clips: self.clips.iter().map(|c| c.transformed(t)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_intersects_rects() {
        let page = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut s = ClipStack::new();
        s.push(Clip::Rect(Rect::new(10.0, 10.0, 50.0, 50.0)));
        s.push(Clip::Rect(Rect::new(30.0, 0.0, 50.0, 20.0)));
        assert_eq!(s.clip_rect(page), Some(Rect::new(30.0, 10.0, 30.0, 10.0)));
        assert!(s.contains(Point::new(35.0, 15.0), 0.1));
        assert!(!s.contains(Point::new(20.0, 15.0), 0.1));
        s.push(Clip::Rect(Rect::new(90.0, 90.0, 5.0, 5.0)));
        assert_eq!(s.clip_rect(page), None);
    }

    #[test]
    fn rotated_rect_clip_becomes_path() {
        let c = Clip::Rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(matches!(
            c.transformed(&Transform::rotate(0.3)),
            Clip::Path { .. }
        ));
        assert!(matches!(
            c.transformed(&Transform::scale(2.0, -1.0)),
            Clip::Rect(_)
        ));
    }
}
