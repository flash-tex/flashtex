//! FlashTeX math layout: an original Rust implementation of TeX's math
//! typesetting rules (TeXbook Appendix G) that turns a math list into
//! explicit boxes carrying glyph identity and rule geometry.
//!
//! Pipeline: build a [`MathList`] of [`Atom`]s → [`layout`] it in a
//! [`Style`] against a [`MathFontMetrics`] provider → get a [`MathBox`] tree →
//! [`positioned_runs`] flattens it into glyphs and rules in points.
//!
//! No TeX engine is involved at runtime. The Computer Modern adapter embeds
//! metrics extracted from TFM files at development time; see `README.md`.

pub mod ams;
pub mod ams_tfm;
pub mod boxes;
pub mod cm;
pub mod cm_tfm;
#[doc(hidden)]
pub mod fixtures;
pub mod layout;
pub mod mathlist;
pub mod metrics;
pub mod source;
pub mod spacing;
pub mod style;
pub mod tfm;
pub mod times;

pub use boxes::{
    BoxKind, Child, Flex, GlueOrder, GlueSet, GlueSign, GlueTotals, MathBox, Packed,
    PositionedGlyph, PositionedRule, PositionedRuns, positioned_runs,
};
pub use cm::CmMathMetrics;
pub use layout::{Layout, Limitation, layout, layout_with_report};
pub use mathlist::{
    Atom, AtomClass, BigSizing, Limits, MathFlex, MathList, Nucleus, TextPiece, TextStyle,
};
pub use metrics::{FontId, Glyph, MathFontMetrics, MathParams, OpenTypeMathConstants, SizeClass};
pub use source::{SourceSpan, SourceTag};
pub use spacing::{Space, between};
pub use style::{Style, StyleLevel};
pub use times::TimesApproxMetrics;
