//! Display primitives.
//!
//! Every primitive carries a stable [`ItemId`] chosen by the producer (the
//! crate never renumbers) and an optional [`SourceRange`] for click-to-source.
//! Coordinates are in the enclosing group's local space (top-left origin,
//! y down, points).

use crate::clip::Clip;
use crate::color::{Color, Paint};
use crate::geom::{Rect, Transform};
use crate::path::{FillRule, Path, StrokeStyle};

/// Producer-assigned identifier. Stable across flattening and serialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub u64);

/// Source mapping in the runtime-v1 convention: zero-based, end-exclusive
/// UTF-8 byte offsets into `path` at the compiled revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRange {
    pub path: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl SourceRange {
    pub fn new(path: impl Into<String>, start_byte: usize, end_byte: usize) -> SourceRange {
        SourceRange {
            path: path.into(),
            start_byte,
            end_byte,
        }
    }
}

/// An axis-aligned filled rectangle (TeX rule, fraction bar, table border).
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: ItemId,
    pub rect: Rect,
    pub paint: Paint,
    pub source: Option<SourceRange>,
}

/// A TikZ/PGF tiling pattern and the colour used to paint its cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub name: String,
    pub color: Color,
}

/// A filled path.
#[derive(Clone, Debug, PartialEq)]
pub struct PathFill {
    pub id: ItemId,
    pub path: Path,
    pub rule: FillRule,
    pub paint: Paint,
    pub pattern: Option<Pattern>,
    pub source: Option<SourceRange>,
}

/// A stroked path. Stroke widths are in the item's local space and scale with
/// the enclosing group transforms.
#[derive(Clone, Debug, PartialEq)]
pub struct PathStroke {
    pub id: ItemId,
    pub path: Path,
    pub style: StrokeStyle,
    pub paint: Paint,
    pub source: Option<SourceRange>,
}

/// A placed raster image. The pixels are *not* here: `content_hash` names them
/// (for example `sha256:<hex>`), and decoding is a consumer concern.
///
/// The image occupies the box `[0, width_pt] × [0, height_pt]` in its own
/// space, first pixel row at `y = 0`; `transform` maps that box into the
/// enclosing group's space.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub id: ItemId,
    pub content_hash: String,
    pub width_pt: f64,
    pub height_pt: f64,
    pub transform: Transform,
    /// Straight alpha applied to the whole image.
    pub alpha: f64,
    pub source: Option<SourceRange>,
}

/// A container with its own coordinate space, optional clip, and opacity.
///
/// `transform` maps the children's space into the parent's. `clip` is
/// expressed in the children's space. `opacity` multiplies into every
/// descendant's alpha when the list is flattened; this is a per-primitive
/// approximation and not a PDF transparency group (overlapping children do
/// not composite as one).
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: ItemId,
    pub transform: Transform,
    pub clip: Option<Clip>,
    pub opacity: f64,
    pub items: Vec<Item>,
    pub source: Option<SourceRange>,
}

impl Group {
    pub fn new(id: ItemId) -> Group {
        Group {
            id,
            transform: Transform::IDENTITY,
            clip: None,
            opacity: 1.0,
            items: Vec::new(),
            source: None,
        }
    }
}

/// Any primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Rule(Rule),
    PathFill(PathFill),
    PathStroke(PathStroke),
    Image(Image),
    Group(Group),
}

impl Item {
    pub fn id(&self) -> ItemId {
        match self {
            Item::Rule(r) => r.id,
            Item::PathFill(p) => p.id,
            Item::PathStroke(p) => p.id,
            Item::Image(i) => i.id,
            Item::Group(g) => g.id,
        }
    }

    pub fn source(&self) -> Option<&SourceRange> {
        match self {
            Item::Rule(r) => r.source.as_ref(),
            Item::PathFill(p) => p.source.as_ref(),
            Item::PathStroke(p) => p.source.as_ref(),
            Item::Image(i) => i.source.as_ref(),
            Item::Group(g) => g.source.as_ref(),
        }
    }

    /// Convenience constructor for an opaque black rule without source.
    pub fn rule(id: u64, rect: Rect) -> Item {
        Item::Rule(Rule {
            id: ItemId(id),
            rect,
            paint: Paint::BLACK,
            source: None,
        })
    }

    pub fn fill(id: u64, path: Path, paint: Paint) -> Item {
        Item::PathFill(PathFill {
            id: ItemId(id),
            path,
            rule: FillRule::NonZero,
            paint,
            pattern: None,
            source: None,
        })
    }

    pub fn stroke(id: u64, path: Path, style: StrokeStyle, paint: Paint) -> Item {
        Item::PathStroke(PathStroke {
            id: ItemId(id),
            path,
            style,
            paint,
            source: None,
        })
    }
}
