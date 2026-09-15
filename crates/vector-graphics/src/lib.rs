//! FlashTeX vector graphics: an original, dependency-free display-primitive
//! model for rules, filled and stroked paths, placed images, and groups with
//! transforms, clips, and opacity.
//!
//! This crate is a **proposal for later consumer integration** (rendering-v2,
//! after ABI agreement). It does not change runtime-v1, whose page items are
//! text-only; nothing here is wired into the compiler, the Mac shell, or
//! `crates/pdf` yet.
//!
//! Coordinate convention: points (pt), origin at the page's top-left corner,
//! `y` growing downwards — the same as runtime-v1 text items. Only
//! [`pdf::content_stream`] flips to PDF's bottom-left convention.
//!
//! Modules:
//! - [`geom`]: [`Point`], [`Size`], [`Rect`], affine [`Transform`].
//! - [`path`]: [`Path`] with lines and Béziers, exact bounds, tolerance-bound
//!   flattening, containment, and [`StrokeStyle`].
//! - [`color`]: [`Color`] (gray/RGB/CMYK) and [`Paint`] (straight alpha).
//! - [`clip`]: [`Clip`] and [`ClipStack`].
//! - [`item`]: the primitives and [`SourceRange`].
//! - [`display_list`]: [`DisplayList`], device-space flattening, bounds, and
//!   hit-testing.
//! - [`pdf`]: content-stream fragment generation.
//! - [`json`]: hand-written JSON writer and reader.
//! - [`diagram`]: arrows, polylines, circles, ellipses, anchor boxes, grids.

pub mod clip;
pub mod color;
pub mod diagram;
pub mod display_list;
pub mod geom;
pub mod item;
pub mod json;
pub mod path;
pub mod pdf;
pub mod tikz;

pub use clip::{Clip, ClipStack};
pub use color::{Color, Paint};
pub use display_list::{DeviceItem, DeviceList, DeviceShape, DisplayList, ValidationError};
pub use geom::{Point, Rect, Size, Transform};
pub use item::{Group, Image, Item, ItemId, PathFill, PathStroke, Pattern, Rule, SourceRange};
pub use path::{Dash, FillRule, LineCap, LineJoin, Path, PathCommand, Polyline, StrokeStyle};
