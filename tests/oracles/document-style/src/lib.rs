//! FlashTeX document style model.
//!
//! An original Rust model of what LaTeX's `article` class decides about a
//! document before any text is set: page geometry, the font size table,
//! paragraph and sectioning spacing, and list measurements — plus a small
//! style tree with measured inheritance and a user overlay.
//!
//! No TeX engine is linked, shelled out to, or required. The default values
//! were transcribed from the class files shipped with BasicTeX (TeX Live
//! 2026) and cross-checked against a running pdflatex; see the README for
//! every file path and number.
//!
//! All lengths are TeX points (72.27 per inch). Convert with
//! [`Pt::to_bp`] or [`PageLayout::text_area_bp`] for PDF user space.
//!
//! ```
//! use flashtex_document_style::{Block, BaseSize, ClassOptions, Paper, Stylesheet};
//!
//! let sheet = Stylesheet::article(ClassOptions { paper: Paper::Letter, size: BaseSize::Pt12 });
//! let page = sheet.page_layout();
//! assert_eq!(page.text_area.width.0, 390.0);
//! let body = sheet.resolve(&[Block::Document, Block::Section(1), Block::Paragraph]);
//! assert_eq!(body.font_size.0, 12.0);
//! assert_eq!(body.baselineskip.0, 14.5);
//! ```

pub mod fonts;
pub mod geometry;
pub mod json;
pub mod length;
pub mod serialize;
pub mod style;

pub use fonts::{
    BaseSize, FontParams, FontSize, ListLevelParams, PARSKIP, SectionAfter, SectionSpec, SizeName,
    SizeParams, font_size, indent_after_heading, list_level, section_spec, size_params,
};
pub use geometry::{
    ClassOptions, Geometry, LatexPageParams, PageLayout, Paper, Rect, apply_geometry,
    article_page_params,
};
pub use json::{JsonError, Value};
pub use length::{LengthError, Pt, Skip};
pub use style::{
    Alignment, Block, DeltaRule, InlineStyle, ListKind, ListNestingTooDeep, ListStyle,
    MAX_LIST_NESTING_DEPTH, ResolvedStyle, StyleDelta, Stylesheet,
};
