//! The display list, its device-space flattening, bounds, and hit-testing.

use crate::clip::{Clip, ClipStack};
use crate::color::Paint;
use crate::geom::{Point, Rect, Size, Transform};
use crate::item::{Group, Item, ItemId, SourceRange};
use crate::path::{FillRule, Path, StrokeStyle};

/// Default curve-flattening tolerance in points used by hit-testing.
pub const HIT_FLATTEN_TOLERANCE: f64 = 0.05;

/// One page's worth of primitives in paint order (first item painted first).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DisplayList {
    pub page_size: Size,
    pub items: Vec<Item>,
}

/// Maximum number of nested groups a display list may contain; the JSON
/// reader refuses deeper documents and [`DisplayList::validate`] reports
/// deeper in-memory trees (GH46).
///
/// Rationale: the display-list contract states no nesting bound, so this is
/// the crate's own. Every recursive walker here (validate, flatten, the PDF
/// and JSON writers, the schema reader) spends one frame per group level, and
/// the schema reader's frame is the heaviest: in a debug `cargo test` worker
/// thread (2 MiB stack) 190 nested groups read successfully and 200 overflowed
/// on mac-m1max-a (d-q222 measured 175 safe / 200 overflow independently on
/// GH46; release builds reach 1000). 64 keeps ~3x headroom under the tightest
/// measurement, and no authored tree (TikZ scopes, nested `pgfpicture`s)
/// approaches it.
pub const MAX_GROUP_DEPTH: usize = 64;

/// A structural problem found by [`DisplayList::validate`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationError {
    NonFinitePageSize,
    NonPositivePageSize,
    NonFiniteGeometry(ItemId),
    SingularTransform(ItemId),
    DuplicateId(ItemId),
    InvalidSourceRange(ItemId),
    /// Group `id` sits `depth` groups deep (itself included), over
    /// [`MAX_GROUP_DEPTH`]; its children were not examined.
    NestingTooDeep {
        id: ItemId,
        depth: usize,
        limit: usize,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::NonFinitePageSize => write!(f, "page size is not finite"),
            ValidationError::NonPositivePageSize => write!(f, "page size must be positive"),
            ValidationError::NonFiniteGeometry(id) => {
                write!(f, "item {} has non-finite geometry", id.0)
            }
            ValidationError::SingularTransform(id) => {
                write!(f, "item {} has a singular transform", id.0)
            }
            ValidationError::DuplicateId(id) => {
                write!(f, "item id {} is used more than once", id.0)
            }
            ValidationError::InvalidSourceRange(id) => {
                write!(f, "item {} has end_byte < start_byte", id.0)
            }
            ValidationError::NestingTooDeep { id, depth, limit } => write!(
                f,
                "item {} is nested {depth} groups deep, over the limit of {limit}",
                id.0
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

impl DisplayList {
    pub fn new(page_size: Size) -> DisplayList {
        DisplayList {
            page_size,
            items: Vec::new(),
        }
    }

    pub fn page_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.page_size.width, self.page_size.height)
    }

    /// Checks finiteness, positive page size, invertible transforms, unique
    /// ids, well-formed source ranges, and group nesting within
    /// [`MAX_GROUP_DEPTH`]. Returns every problem found; a group over the
    /// nesting limit is reported once and its subtree is not entered, so this
    /// walk itself never recurses past the limit.
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if !(self.page_size.width.is_finite() && self.page_size.height.is_finite()) {
            errors.push(ValidationError::NonFinitePageSize);
        } else if !(self.page_size.width > 0.0 && self.page_size.height > 0.0) {
            errors.push(ValidationError::NonPositivePageSize);
        }
        let mut seen = std::collections::BTreeSet::new();
        fn walk(
            items: &[Item],
            group_depth: usize,
            seen: &mut std::collections::BTreeSet<ItemId>,
            errors: &mut Vec<ValidationError>,
        ) {
            for item in items {
                let id = item.id();
                if !seen.insert(id) {
                    errors.push(ValidationError::DuplicateId(id));
                }
                if let Some(s) = item.source()
                    && s.end_byte < s.start_byte
                {
                    errors.push(ValidationError::InvalidSourceRange(id));
                }
                let finite = match item {
                    Item::Rule(r) => r.rect.is_finite() && r.paint.alpha.is_finite(),
                    Item::PathFill(p) => p.path.is_finite() && p.paint.alpha.is_finite(),
                    Item::PathStroke(p) => {
                        p.path.is_finite()
                            && p.style.width.is_finite()
                            && p.style.miter_limit.is_finite()
                    }
                    Item::Image(i) => {
                        i.width_pt.is_finite()
                            && i.height_pt.is_finite()
                            && i.transform.is_finite()
                            && i.alpha.is_finite()
                    }
                    Item::Group(g) => g.transform.is_finite() && g.opacity.is_finite(),
                };
                if !finite {
                    errors.push(ValidationError::NonFiniteGeometry(id));
                }
                match item {
                    Item::Image(i) if i.transform.invert().is_none() => {
                        errors.push(ValidationError::SingularTransform(id));
                    }
                    Item::Group(g) => {
                        if g.transform.invert().is_none() {
                            errors.push(ValidationError::SingularTransform(id));
                        }
                        let depth = group_depth + 1;
                        if depth > MAX_GROUP_DEPTH {
                            errors.push(ValidationError::NestingTooDeep {
                                id,
                                depth,
                                limit: MAX_GROUP_DEPTH,
                            });
                        } else {
                            walk(&g.items, depth, seen, errors);
                        }
                    }
                    _ => {}
                }
            }
        }
        walk(&self.items, 0, &mut seen, &mut errors);
        errors
    }

    /// Resolves groups, transforms, clips, and opacity into device-space
    /// primitives. Deterministic: the same list always yields the same output.
    pub fn flatten(&self) -> DeviceList {
        let mut out = DeviceList {
            page_size: self.page_size,
            items: Vec::new(),
        };
        let ctx = Context {
            transform: Transform::IDENTITY,
            clip: ClipStack::new(),
            alpha: 1.0,
            ancestors: Vec::new(),
            source: None,
        };
        flatten_items(&self.items, &ctx, &mut out.items);
        out
    }

    /// Bounding box of all painted geometry, clipped to each item's clip
    /// stack (not to the page). `None` for an empty list.
    pub fn bounds(&self) -> Option<Rect> {
        self.flatten().bounds()
    }

    /// Leaf ids under `p`, topmost first. See [`DeviceList::hit`].
    pub fn hit(&self, p: Point) -> Vec<ItemId> {
        self.flatten().hit(p)
    }
}

#[derive(Clone)]
struct Context {
    transform: Transform,
    clip: ClipStack,
    alpha: f64,
    ancestors: Vec<ItemId>,
    source: Option<SourceRange>,
}

fn flatten_items(items: &[Item], ctx: &Context, out: &mut Vec<DeviceItem>) {
    for item in items {
        match item {
            Item::Group(g) => flatten_group(g, ctx, out),
            _ => {
                let source = item.source().cloned().or_else(|| ctx.source.clone());
                let shape = match item {
                    Item::Rule(r) => {
                        let paint = Paint {
                            color: r.paint.color,
                            alpha: r.paint.alpha * ctx.alpha,
                        };
                        if ctx.transform.is_axis_aligned() {
                            DeviceShape::Rule {
                                rect: r.rect.transformed_bounds(&ctx.transform),
                                paint,
                            }
                        } else {
                            DeviceShape::Fill {
                                path: Path::rect(r.rect).transformed(&ctx.transform),
                                rule: FillRule::NonZero,
                                paint,
                            }
                        }
                    }
                    Item::PathFill(pf) => DeviceShape::Fill {
                        path: pf.path.transformed(&ctx.transform),
                        rule: pf.rule,
                        paint: Paint {
                            color: pf.paint.color,
                            alpha: pf.paint.alpha * ctx.alpha,
                        },
                    },
                    Item::PathStroke(ps) => {
                        let k = ctx.transform.mean_scale();
                        let mut style = ps.style.clone();
                        style.width *= k;
                        if let Some(d) = style.dash.as_mut() {
                            for v in d.array.iter_mut() {
                                *v *= k;
                            }
                            d.phase *= k;
                        }
                        DeviceShape::Stroke {
                            path: ps.path.transformed(&ctx.transform),
                            style,
                            paint: Paint {
                                color: ps.paint.color,
                                alpha: ps.paint.alpha * ctx.alpha,
                            },
                        }
                    }
                    Item::Image(img) => DeviceShape::Image {
                        content_hash: img.content_hash.clone(),
                        width_pt: img.width_pt,
                        height_pt: img.height_pt,
                        transform: img.transform.then(&ctx.transform),
                        alpha: img.alpha * ctx.alpha,
                    },
                    Item::Group(_) => unreachable!(),
                };
                out.push(DeviceItem {
                    id: item.id(),
                    ancestors: ctx.ancestors.clone(),
                    source,
                    clip: ctx.clip.clone(),
                    shape,
                });
            }
        }
    }
}

fn flatten_group(g: &Group, ctx: &Context, out: &mut Vec<DeviceItem>) {
    let transform = g.transform.then(&ctx.transform);
    let mut clip = ctx.clip.clone();
    if let Some(c) = &g.clip {
        clip.push(c.transformed(&transform));
    }
    let mut ancestors = ctx.ancestors.clone();
    ancestors.push(g.id);
    let child = Context {
        transform,
        clip,
        alpha: ctx.alpha * g.opacity,
        ancestors,
        source: g.source.clone().or_else(|| ctx.source.clone()),
    };
    flatten_items(&g.items, &child, out);
}

/// A device-space primitive produced by [`DisplayList::flatten`].
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceItem {
    pub id: ItemId,
    /// Enclosing group ids, outermost first.
    pub ancestors: Vec<ItemId>,
    /// The leaf's own source range, or the nearest ancestor's.
    pub source: Option<SourceRange>,
    /// Device-space clips that apply to this item.
    pub clip: ClipStack,
    pub shape: DeviceShape,
}

/// Device-space geometry. Alpha is the effective (group-multiplied) value.
#[derive(Clone, Debug, PartialEq)]
pub enum DeviceShape {
    Rule {
        rect: Rect,
        paint: Paint,
    },
    Fill {
        path: Path,
        rule: FillRule,
        paint: Paint,
    },
    /// `style.width` and dashes are already scaled by the mean scale of the
    /// resolved transform (exact for uniform scale and rotation).
    Stroke {
        path: Path,
        style: StrokeStyle,
        paint: Paint,
    },
    /// `transform` maps the image box `[0,w]×[0,h]` into device space.
    Image {
        content_hash: String,
        width_pt: f64,
        height_pt: f64,
        transform: Transform,
        alpha: f64,
    },
}

impl DeviceItem {
    /// Unclipped bounds of the geometry.
    pub fn geometry_bounds(&self) -> Option<Rect> {
        match &self.shape {
            DeviceShape::Rule { rect, .. } => Some(rect.normalized()),
            DeviceShape::Fill { path, .. } => path.bounds(),
            DeviceShape::Stroke { path, style, .. } => {
                path.bounds().map(|b| b.expand(style.outset()))
            }
            DeviceShape::Image {
                width_pt,
                height_pt,
                transform,
                ..
            } => Some(Rect::new(0.0, 0.0, *width_pt, *height_pt).transformed_bounds(transform)),
        }
    }

    /// Geometry bounds intersected with the clip stack.
    pub fn bounds(&self) -> Option<Rect> {
        let g = self.geometry_bounds()?;
        if g.is_empty() && !matches!(self.shape, DeviceShape::Rule { .. }) {
            return None;
        }
        self.clip.clip_rect(g)
    }

    /// Whether `p` hits this item, honouring clips. Fills use exact
    /// containment; strokes hit within `width/2 + tolerance` of the centre
    /// line (dashes are ignored: the whole path is hittable); rules and
    /// images use closed containment.
    pub fn hit(&self, p: Point, tolerance: f64) -> bool {
        if !self.clip.contains(p, HIT_FLATTEN_TOLERANCE) {
            return false;
        }
        match &self.shape {
            DeviceShape::Rule { rect, .. } => rect.normalized().expand(tolerance).contains(p),
            DeviceShape::Fill { path, rule, .. } => {
                path.contains(p, *rule, HIT_FLATTEN_TOLERANCE)
                    || (tolerance > 0.0
                        && path.distance_to_outline(p, HIT_FLATTEN_TOLERANCE) <= tolerance)
            }
            DeviceShape::Stroke { path, style, .. } => {
                path.distance_to_outline(p, HIT_FLATTEN_TOLERANCE)
                    <= style.width.abs() / 2.0 + tolerance
            }
            DeviceShape::Image {
                width_pt,
                height_pt,
                transform,
                ..
            } => match transform.invert() {
                Some(inv) => Rect::new(0.0, 0.0, *width_pt, *height_pt)
                    .expand(tolerance)
                    .contains(inv.apply(p)),
                None => false,
            },
        }
    }
}

/// A flattened page.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DeviceList {
    pub page_size: Size,
    pub items: Vec<DeviceItem>,
}

impl DeviceList {
    /// Union of every item's clipped bounds.
    pub fn bounds(&self) -> Option<Rect> {
        self.items
            .iter()
            .filter_map(DeviceItem::bounds)
            .reduce(|a, b| a.union(&b))
    }

    /// Ids of leaves under `p`, topmost (last painted) first, with zero
    /// tolerance.
    pub fn hit(&self, p: Point) -> Vec<ItemId> {
        self.hit_with_tolerance(p, 0.0)
    }

    /// Like [`DeviceList::hit`] with a distance tolerance in device points.
    pub fn hit_with_tolerance(&self, p: Point, tolerance: f64) -> Vec<ItemId> {
        self.hit_items(p, tolerance)
            .into_iter()
            .map(|i| i.id)
            .collect()
    }

    /// The hit items themselves, topmost first.
    pub fn hit_items(&self, p: Point, tolerance: f64) -> Vec<&DeviceItem> {
        self.items
            .iter()
            .rev()
            .filter(|i| i.hit(p, tolerance))
            .collect()
    }

    /// The source range of the topmost hit, if any has one.
    pub fn source_at(&self, p: Point, tolerance: f64) -> Option<&SourceRange> {
        self.hit_items(p, tolerance)
            .into_iter()
            .find_map(|i| i.source.as_ref())
    }
}

/// Convenience for building a rectangular clip.
pub fn rect_clip(r: Rect) -> Clip {
    Clip::Rect(r)
}
